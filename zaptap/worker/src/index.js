// ZapTap API: hands out today's puzzle one round at a time, scores guesses
// server-side and keeps a daily leaderboard. Runs on Cloudflare Workers + D1
// (both free tier).
//
// The daily puzzle is picked with an HMAC of the date under SEED_SECRET, so
// future puzzles can't be computed from the public code. Days roll over at
// midnight Oslo time.

import { geoBounds, geoContains, geoDistance } from "d3-geo";
import { feature } from "topojson-client";
import world from "world-atlas/countries-50m.json";
import DATA from "./places.json";

const EARTH_KM = 6371;
const WEIGHTS = [1, 1, 2, 3, 3];
const TIERS = [1, 1, 2, 3, 3];
const EPOCH = "2026-10-29"; // ZapTap #1
const TZ = "Europe/Oslo";

// ---------------------------------------------------------------- geography

// Same synthetic ids as the build script for features without an ISO code.
const SHAPES = feature(world, world.objects.countries).features.map((f, i) => ({
  id: f.id || "x" + i,
  feature: f,
  bounds: geoBounds(f),
}));

function countryAt(lonlat) {
  for (const { id, feature: f, bounds: [[w, s], [e, n]] } of SHAPES) {
    const [lon, lat] = lonlat;
    if (lat < s || lat > n) continue;
    if (w <= e ? lon < w || lon > e : lon < w && lon > e) continue;
    if (geoContains(f, lonlat)) return id;
  }
  return null;
}
const continentOf = (id) => (id && DATA.countries[id] ? DATA.countries[id][1] : null);

// 100 within 22 km, 0 past 16,250 km, 81 at 1,000 km. Same curve as the client.
function distanceScore(km) {
  if (km <= 22) return 100;
  if (km >= 16250) return 0;
  return Math.round(100 * (1 - Math.pow((km - 22) / (16250 - 22), 0.591)));
}

// ---------------------------------------------------------------- days

const dayFormat = new Intl.DateTimeFormat("en-CA", { timeZone: TZ, year: "numeric", month: "2-digit", day: "2-digit" });
const timeFormat = new Intl.DateTimeFormat("en-GB", { timeZone: TZ, hour: "2-digit", minute: "2-digit", second: "2-digit", hourCycle: "h23" });

const today = (d = new Date()) => dayFormat.format(d);

function secondsToMidnight(d = new Date()) {
  const [h, m, s] = timeFormat.format(d).split(":").map(Number);
  return 86400 - (h * 3600 + m * 60 + s);
}

function dayNumber(key) {
  const [y, m, d] = key.split("-").map(Number);
  const [ey, em, ed] = EPOCH.split("-").map(Number);
  return Math.round((Date.UTC(y, m - 1, d) - Date.UTC(ey, em - 1, ed)) / 864e5) + 1;
}

// ---------------------------------------------------------------- puzzle

function mulberry32(a) {
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const byTier = { 1: [], 2: [], 3: [] };
DATA.places.forEach((p, i) => byTier[p[3]].push(i));

// Five places from five different countries, at most two per continent.
function pickTargets(rand) {
  const usedCountries = new Set();
  const perContinent = {};
  return TIERS.map((tier) => {
    const pool = byTier[tier];
    let pick = null;
    for (let tries = 0; tries < 500 && !pick; tries++) {
      const p = DATA.places[pool[Math.floor(rand() * pool.length)]];
      const cont = DATA.countries[p[4]][1];
      if (usedCountries.has(p[4])) continue;
      if ((perContinent[cont] || 0) >= 2 && tries < 400) continue;
      pick = p;
    }
    usedCountries.add(pick[4]);
    const cont = DATA.countries[pick[4]][1];
    perContinent[cont] = (perContinent[cont] || 0) + 1;
    return { name: pick[0], lat: pick[1], lon: pick[2], id: pick[4], country: DATA.countries[pick[4]][0] };
  });
}

const puzzles = new Map();
async function puzzleFor(env, day) {
  if (!puzzles.has(day)) {
    if (!env.SEED_SECRET) throw new Error("SEED_SECRET is not set");
    const key = await crypto.subtle.importKey("raw", new TextEncoder().encode(env.SEED_SECRET), { name: "HMAC", hash: "SHA-256" }, false, ["sign"]);
    const mac = await crypto.subtle.sign("HMAC", key, new TextEncoder().encode("zaptap:" + day));
    puzzles.set(day, pickTargets(mulberry32(new DataView(mac).getUint32(0))));
    if (puzzles.size > 8) puzzles.delete(puzzles.keys().next().value);
  }
  return puzzles.get(day);
}

// ---------------------------------------------------------------- players

async function sha256Hex(s) {
  const buf = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(s));
  return [...new Uint8Array(buf)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

// The browser keeps a random token; we only ever store its hash.
async function playerFor(env, token) {
  if (typeof token !== "string" || !/^[0-9a-f-]{36}$/i.test(token)) throw new HttpError(400, "bad_token");
  const id = await sha256Hex(token.toLowerCase());
  const row = await env.DB.prepare("SELECT name FROM players WHERE id = ?").bind(id).first();
  return { id, name: row ? row.name : null };
}

function cleanName(name) {
  if (typeof name !== "string") return null;
  const n = name.normalize("NFC").replace(/[\u0000-\u001f\u007f]/g, "").replace(/\s+/g, " ").trim();
  return n.length >= 1 && n.length <= 24 ? n : null;
}

// ---------------------------------------------------------------- game

function roundView(target, i, guess) {
  const r = { round: i, name: target.name, country: target.country, countryId: target.id, weight: WEIGHTS[i] };
  if (guess) {
    Object.assign(r, {
      guess: [guess.lon, guess.lat],
      km: guess.km,
      score: guess.score,
      points: guess.points,
      bonus: guess.bonus,
      inCountry: !!guess.in_country,
      answer: [target.lon, target.lat],
    });
  }
  return r;
}

async function guessesFor(env, day, playerId) {
  const { results } = await env.DB.prepare("SELECT * FROM guesses WHERE day = ? AND player = ? ORDER BY round").bind(day, playerId).all();
  return results;
}

// Played rounds (with answers) plus the next round's name, never further ahead.
async function state(env, player) {
  const day = today();
  const targets = await puzzleFor(env, day);
  const guesses = player.name ? await guessesFor(env, day, player.id) : [];
  const rounds = guesses.map((g) => roundView(targets[g.round], g.round, g));
  if (rounds.length < 5) rounds.push(roundView(targets[rounds.length], rounds.length));
  return {
    day,
    number: dayNumber(day),
    nextIn: secondsToMidnight(),
    name: player.name,
    rounds,
    done: guesses.length === 5,
    total: guesses.reduce((sum, g) => sum + g.points, 0),
  };
}

async function guess(env, player, body) {
  if (!player.name) throw new HttpError(409, "need_name");
  const day = today();
  if (body.day !== day) throw new HttpError(409, "day_over");
  const lon = Number(body.lon);
  const lat = Number(body.lat);
  if (!Number.isFinite(lon) || !Number.isFinite(lat) || Math.abs(lon) > 180 || Math.abs(lat) > 90) throw new HttpError(400, "bad_guess");
  const targets = await puzzleFor(env, day);
  const played = await guessesFor(env, day, player.id);
  const round = played.length;
  if (round >= 5) throw new HttpError(409, "done");
  if (body.round !== round) throw new HttpError(409, "wrong_round");

  const t = targets[round];
  const km = geoDistance([lon, lat], [t.lon, t.lat]) * EARTH_KM;
  let score = distanceScore(km);
  let bonus = null;
  const hit = countryAt([lon, lat]);
  if (hit === t.id && score < 25) {
    score = 25;
    bonus = "country";
  } else if (hit && hit !== t.id && continentOf(hit) === continentOf(t.id) && score < 10) {
    score = 10;
    bonus = "continent";
  }
  const g = {
    lon: Math.round(lon * 1e4) / 1e4,
    lat: Math.round(lat * 1e4) / 1e4,
    km: Math.round(km * 10) / 10,
    score,
    points: score * WEIGHTS[round],
    bonus,
    in_country: hit === t.id ? 1 : 0,
  };
  try {
    await env.DB.prepare(
      "INSERT INTO guesses (day, player, round, lon, lat, km, score, points, bonus, in_country, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
      .bind(day, player.id, round, g.lon, g.lat, g.km, g.score, g.points, g.bonus, g.in_country, Date.now())
      .run();
  } catch (e) {
    throw new HttpError(409, "wrong_round"); // a parallel request already took this round
  }
  const total = played.reduce((sum, p) => sum + p.points, 0) + g.points;
  return {
    result: roundView(t, round, g),
    next: round < 4 ? roundView(targets[round + 1], round + 1) : null,
    total,
    done: round === 4,
  };
}

async function setName(env, player, body) {
  const name = cleanName(body.name);
  if (!name) throw new HttpError(400, "bad_name");
  const key = name.toLowerCase();
  const taken = await env.DB.prepare("SELECT id FROM players WHERE name_key = ?").bind(key).first();
  if (taken && taken.id !== player.id) throw new HttpError(409, "name_taken");
  await env.DB.prepare(
    "INSERT INTO players (id, name, name_key, created_at) VALUES (?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET name = excluded.name, name_key = excluded.name_key"
  )
    .bind(player.id, name, key, Date.now())
    .run();
  return { name };
}

async function leaderboard(env, day) {
  const { results } = await env.DB.prepare(
    "SELECT g.player, p.name, g.round, g.score, g.points, g.created_at FROM guesses g JOIN players p ON p.id = g.player WHERE g.day = ? ORDER BY g.player, g.round"
  )
    .bind(day)
    .all();
  const byPlayer = new Map();
  for (const r of results) {
    if (!byPlayer.has(r.player)) byPlayer.set(r.player, { name: r.name, scores: [], total: 0, finishedAt: 0 });
    const e = byPlayer.get(r.player);
    e.scores.push(r.score);
    e.total += r.points;
    e.finishedAt = Math.max(e.finishedAt, r.created_at);
  }
  const all = [...byPlayer.values()];
  const entries = all
    .filter((e) => e.scores.length === 5)
    .sort((a, b) => b.total - a.total || a.finishedAt - b.finishedAt)
    .map(({ name, scores, total }) => ({ name, scores, total }));
  return { day, number: dayNumber(day), final: day < today(), playing: all.length - entries.length, entries };
}

// ---------------------------------------------------------------- http

class HttpError extends Error {
  constructor(status, code) {
    super(code);
    this.status = status;
  }
}

function cors(request, env) {
  const origin = request.headers.get("Origin") || "";
  const allowed = (env.ALLOWED_ORIGINS || "").split(",").map((s) => s.trim());
  return allowed.includes(origin) || allowed.includes("*")
    ? { "Access-Control-Allow-Origin": origin || "*", "Access-Control-Allow-Methods": "GET, POST, OPTIONS", "Access-Control-Allow-Headers": "Content-Type", Vary: "Origin" }
    : {};
}

const json = (data, status, headers) =>
  new Response(JSON.stringify(data), { status, headers: { "Content-Type": "application/json", "Cache-Control": "no-store", ...headers } });

export default {
  async fetch(request, env) {
    const headers = cors(request, env);
    if (request.method === "OPTIONS") return new Response(null, { status: 204, headers });
    const url = new URL(request.url);
    try {
      if (request.method === "GET" && url.pathname === "/api/leaderboard") {
        const day = url.searchParams.get("day") || today();
        if (!/^\d{4}-\d{2}-\d{2}$/.test(day) || day > today()) throw new HttpError(400, "bad_day");
        return json(await leaderboard(env, day), 200, headers);
      }
      if (request.method === "POST" && url.pathname.startsWith("/api/")) {
        const body = await request.json().catch(() => ({}));
        const player = await playerFor(env, body.token);
        if (url.pathname === "/api/state") return json(await state(env, player), 200, headers);
        if (url.pathname === "/api/guess") return json(await guess(env, player, body), 200, headers);
        if (url.pathname === "/api/name") return json(await setName(env, player, body), 200, headers);
      }
      if (url.pathname === "/") return json({ ok: true, day: today() }, 200, headers);
      throw new HttpError(404, "not_found");
    } catch (e) {
      if (e instanceof HttpError) return json({ error: e.message }, e.status, headers);
      console.error(e);
      return json({ error: "server_error" }, 500, headers);
    }
  },
};
