// ZapTap server: serves the game, hands out today's puzzle one round at a
// time, scores guesses and keeps a daily leaderboard. Runs on Cloudflare
// Workers + D1 behind Cloudflare Access (all free tier), so players sign in
// with their Zivid Microsoft account.
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

// Cloudflare Access signs in the player and passes us a signed JWT, either as
// the Cf-Access-Jwt-Assertion header or the CF_Authorization cookie. We check
// it ourselves rather than trusting that every request went through Access.

const b64url = (s) => Uint8Array.from(atob(s.replace(/-/g, "+").replace(/_/g, "/") + "=".repeat((4 - (s.length % 4)) % 4)), (c) => c.charCodeAt(0));
const b64json = (s) => JSON.parse(new TextDecoder().decode(b64url(s)));
const teamUrl = (env) => (env.ACCESS_TEAM_DOMAIN.includes("://") ? env.ACCESS_TEAM_DOMAIN : "https://" + env.ACCESS_TEAM_DOMAIN);

let accessKeys = null;
let accessKeysAt = 0;
async function accessKey(env, kid) {
  for (let attempt = 0; attempt < 2; attempt++) {
    if (!accessKeys || Date.now() - accessKeysAt > 3600e3 || attempt === 1) {
      const res = await fetch(teamUrl(env) + "/cdn-cgi/access/certs");
      if (!res.ok) throw new Error("couldn't fetch Access certs: " + res.status);
      const { keys } = await res.json();
      accessKeys = await Promise.all(
        keys.map(async (k) => ({ kid: k.kid, key: await crypto.subtle.importKey("jwk", k, { name: "RSASSA-PKCS1-v1_5", hash: "SHA-256" }, false, ["verify"]) }))
      );
      accessKeysAt = Date.now();
    }
    const found = accessKeys.find((k) => k.kid === kid);
    if (found) return found.key; // otherwise the keys may have rotated: refetch once
  }
  return null;
}

function cookie(request, name) {
  const m = (request.headers.get("Cookie") || "").match(new RegExp("(?:^|;\\s*)" + name + "=([^;]+)"));
  return m ? m[1] : null;
}

async function identify(request, env) {
  // Local development only: `wrangler dev` has no Access in front of it.
  if (env.DEV_AUTH === "header") {
    const email = (request.headers.get("X-Dev-Email") || "").toLowerCase();
    if (!email) throw new HttpError(401, "not_signed_in");
    return { email, jwt: null };
  }
  if (!env.ACCESS_TEAM_DOMAIN || !env.ACCESS_AUD || env.ACCESS_AUD.startsWith("REPLACE")) throw new Error("Cloudflare Access is not configured");
  const jwt = request.headers.get("Cf-Access-Jwt-Assertion") || cookie(request, "CF_Authorization");
  if (!jwt) throw new HttpError(401, "not_signed_in");
  const [h, p, sig] = jwt.split(".");
  let header;
  let payload;
  try {
    header = b64json(h);
    payload = b64json(p);
  } catch (e) {
    throw new HttpError(401, "bad_session");
  }
  const key = header.alg === "RS256" ? await accessKey(env, header.kid) : null;
  const valid = key && (await crypto.subtle.verify("RSASSA-PKCS1-v1_5", key, b64url(sig), new TextEncoder().encode(h + "." + p)));
  const aud = [].concat(payload.aud || []);
  if (!valid || !aud.includes(env.ACCESS_AUD) || payload.iss !== teamUrl(env) || !(payload.exp * 1000 > Date.now())) {
    throw new HttpError(401, "bad_session");
  }
  const email = String(payload.email || "").toLowerCase();
  const domain = (env.ALLOWED_EMAIL_DOMAIN || "").toLowerCase();
  if (!email || (domain && !email.endsWith("@" + domain))) throw new HttpError(403, "not_allowed");
  return { email, jwt };
}

function cleanName(name) {
  if (typeof name !== "string") return null;
  const n = name.normalize("NFC").replace(/[\u0000-\u001f\u007f]/g, "").replace(/\s+/g, " ").trim().slice(0, 24).trim();
  return n.length >= 1 ? n : null;
}

// The name from the Microsoft account if Access knows it, else from the
// email address: paul.mairo@zivid.com -> Paul Mairo.
async function defaultName(env, who) {
  if (who.jwt) {
    try {
      const res = await fetch(teamUrl(env) + "/cdn-cgi/access/get-identity", { headers: { Cookie: "CF_Authorization=" + who.jwt } });
      const name = res.ok ? cleanName((await res.json()).name) : null;
      if (name) return name;
    } catch (e) {
      // Fall back to the email address.
    }
  }
  const local = who.email.split("@")[0];
  return cleanName(local.split(/[._-]+/).map((w) => w.charAt(0).toUpperCase() + w.slice(1)).join(" ")) || "Zividian";
}

// Players are keyed by a hash of their email; the first visit creates them
// with a unique display name they can change later.
async function playerFor(request, env) {
  const who = await identify(request, env);
  const id = await sha256Hex(who.email);
  const row = await env.DB.prepare("SELECT name FROM players WHERE id = ?").bind(id).first();
  if (row) return { id, name: row.name };
  const base = await defaultName(env, who);
  for (let n = 1; n < 100; n++) {
    const name = n === 1 ? base : `${base.slice(0, 21)} ${n}`;
    try {
      await env.DB.prepare("INSERT INTO players (id, name, name_key, created_at) VALUES (?, ?, ?, ?)")
        .bind(id, name, name.toLowerCase(), Date.now())
        .run();
      return { id, name };
    } catch (e) {
      const again = await env.DB.prepare("SELECT name FROM players WHERE id = ?").bind(id).first();
      if (again) return { id, name: again.name }; // created by a parallel request
    }
  }
  throw new Error("couldn't find a free name for " + base);
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
  const guesses = await guessesFor(env, day, player.id);
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
  await env.DB.prepare("UPDATE players SET name = ?, name_key = ? WHERE id = ?").bind(name, key, player.id).run();
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

const json = (data, status) => new Response(JSON.stringify(data), { status, headers: { "Content-Type": "application/json", "Cache-Control": "no-store" } });

export default {
  // Static files (the game itself) are served by Workers Assets before this
  // runs; everything else lands here.
  async fetch(request, env) {
    const url = new URL(request.url);
    try {
      if (url.pathname === "/config.js") {
        return new Response("window.ZAPTAP_SERVER = true;\n", { headers: { "Content-Type": "text/javascript", "Cache-Control": "no-store" } });
      }
      if (!url.pathname.startsWith("/api/")) return env.ASSETS.fetch(request);
      const player = await playerFor(request, env);
      if (request.method === "GET" && url.pathname === "/api/leaderboard") {
        const day = url.searchParams.get("day") || today();
        if (!/^\d{4}-\d{2}-\d{2}$/.test(day) || day > today()) throw new HttpError(400, "bad_day");
        return json(await leaderboard(env, day), 200);
      }
      if (request.method === "POST") {
        const body = await request.json().catch(() => ({}));
        if (url.pathname === "/api/state") return json(await state(env, player), 200);
        if (url.pathname === "/api/guess") return json(await guess(env, player, body), 200);
        if (url.pathname === "/api/name") return json(await setName(env, player, body), 200);
      }
      throw new HttpError(404, "not_found");
    } catch (e) {
      if (e instanceof HttpError) return json({ error: e.message }, e.status);
      console.error(e);
      return json({ error: "server_error" }, 500);
    }
  },
};
