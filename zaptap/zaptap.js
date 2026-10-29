(() => {
  "use strict";

  const DATA = window.ZAPTAP_DATA;
  const EARTH_KM = 6371;
  const WEIGHTS = [1, 1, 2, 3, 3];
  const TIERS = [1, 1, 2, 3, 3]; // place difficulty per round
  const EPOCH = "2026-10-29"; // ZapTap #1
  const STORE_KEY = "zaptap:v1";
  // Served by the ZapTap Worker (behind Zivid sign-in): server-run daily and
  // leaderboard. Anywhere else: offline daily, no leaderboard.
  const ONLINE = !!window.ZAPTAP_SERVER;
  const TOPO_HI = "https://cdn.jsdelivr.net/npm/world-atlas@2.0.2/countries-50m.json";
  const TOPO_LO = "https://cdn.jsdelivr.net/npm/world-atlas@2.0.2/countries-110m.json";
  const MIN_ZOOM = 0.8;
  const MAX_ZOOM = 60;

  const $ = (id) => document.getElementById(id);

  // ---------------------------------------------------------------- storage

  function loadStore() {
    try {
      return JSON.parse(localStorage.getItem(STORE_KEY)) || {};
    } catch (e) {
      return {};
    }
  }
  const store = Object.assign(
    { days: {}, practice: { played: 0, best: 0, sum: 0 }, achievements: {}, shares: 0, name: null },
    loadStore()
  );
  function save() {
    try {
      localStorage.setItem(STORE_KEY, JSON.stringify(store));
    } catch (e) {
      // Private mode or storage full: the game still works, it just won't remember.
    }
  }

  // ---------------------------------------------------------------- dates

  // Everyone plays the same day: it rolls over at midnight Oslo time.
  const pad = (n) => String(n).padStart(2, "0");
  const osloDay = new Intl.DateTimeFormat("en-CA", { timeZone: "Europe/Oslo", year: "numeric", month: "2-digit", day: "2-digit" });
  const osloTime = new Intl.DateTimeFormat("en-GB", { timeZone: "Europe/Oslo", hour: "2-digit", minute: "2-digit", second: "2-digit", hourCycle: "h23" });
  function dayKey(d = new Date()) {
    return osloDay.format(d);
  }
  function secondsToNextDay(d = new Date()) {
    const [h, m, s] = osloTime.format(d).split(":").map(Number);
    return 86400 - (h * 3600 + m * 60 + s);
  }
  function keyToDate(key) {
    const [y, m, d] = key.split("-").map(Number);
    return new Date(y, m - 1, d);
  }
  function dayNumber(key) {
    const [y, m, d] = key.split("-").map(Number);
    const [ey, em, ed] = EPOCH.split("-").map(Number);
    return Math.round((Date.UTC(y, m - 1, d) - Date.UTC(ey, em - 1, ed)) / 864e5) + 1;
  }
  function prettyDate(key, opts = { weekday: "long", day: "numeric", month: "long" }) {
    return keyToDate(key).toLocaleDateString(undefined, opts);
  }
  function previousKey(key) {
    const [y, m, d] = key.split("-").map(Number);
    return new Date(Date.UTC(y, m - 1, d - 1)).toISOString().slice(0, 10);
  }

  // ---------------------------------------------------------------- puzzle

  function hashStr(s) {
    let h = 2166136261;
    for (let i = 0; i < s.length; i++) {
      h ^= s.charCodeAt(i);
      h = Math.imul(h, 16777619);
    }
    return h >>> 0;
  }
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

  // Five places, one per round, from five different countries and
  // at most two per continent.
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
      return { name: pick[0], lat: pick[1], lon: pick[2], id: pick[4], country: pick[6] || DATA.countries[pick[4]][0] };
    });
  }

  // 100 within 22 km, 0 past 16,250 km, 81 at 1,000 km.
  function distanceScore(km) {
    if (km <= 22) return 100;
    if (km >= 16250) return 0;
    return Math.round(100 * (1 - Math.pow((km - 22) / (16250 - 22), 0.591)));
  }

  const scoreEmoji = (s) => (s >= 95 ? "🎯" : s >= 80 ? "🟩" : s >= 60 ? "🟨" : s >= 35 ? "🟧" : "🟥");

  // ---------------------------------------------------------------- geography

  let topoHi = null;
  let topoLo = null;
  let countryShapes = []; // { id, feature, bounds }

  // Vector land, only drawn while the satellite texture loads or without WebGL2.
  const drawable = (topo) => ({ land: topojson.feature(topo, topo.objects.land) });

  function prepareGeo(hi) {
    topoHi = drawable(hi);
    // Same synthetic ids as the build script for features without an ISO code.
    countryShapes = topojson.feature(hi, hi.objects.countries).features.map((f, i) => ({
      id: f.id || "x" + i,
      feature: f,
      bounds: d3.geoBounds(f),
    }));
  }

  function inBounds([[w, s], [e, n]], [lon, lat]) {
    if (lat < s || lat > n) return false;
    return w <= e ? lon >= w && lon <= e : lon >= w || lon <= e;
  }

  function countryAt(lonlat) {
    for (const c of countryShapes) {
      if (inBounds(c.bounds, lonlat) && d3.geoContains(c.feature, lonlat)) return c.id;
    }
    return null;
  }
  const continentOf = (id) => (id && DATA.countries[id] ? DATA.countries[id][1] : null);

  // ---------------------------------------------------------------- globe

  const canvas = $("globe"); // 2D overlay: pins, arcs, labels; also receives input
  const ctx = canvas.getContext("2d");
  const starsCanvas = $("stars");
  const earth = window.createEarth($("earth"), () => requestDraw());
  const projection = d3.geoOrthographic().clipAngle(90).precision(0.4);
  const geoPath = d3.geoPath(projection, ctx);
  const graticule = d3.geoGraticule10();
  const view = { lon: 15, lat: 20, zoom: 1 };
  let W = 0;
  let H = 0;
  let DPR = 1;
  let baseRadius = 200;
  let colors = {};
  let interacting = false;
  let idleTimer = 0;
  let frame = 0;

  function readColors() {
    const cs = getComputedStyle(document.documentElement);
    for (const k of ["guess", "answer"]) {
      colors[k] = cs.getPropertyValue("--" + k).trim();
    }
  }

  // A fixed, seeded starfield behind the globe.
  function drawStars() {
    starsCanvas.width = Math.round(W * DPR);
    starsCanvas.height = Math.round(H * DPR);
    const c = starsCanvas.getContext("2d");
    c.setTransform(DPR, 0, 0, DPR, 0, 0);
    const rand = mulberry32(42);
    const n = Math.round((W * H) / 1800);
    for (let i = 0; i < n; i++) {
      const big = rand() < 0.06;
      c.globalAlpha = 0.25 + rand() * (big ? 0.75 : 0.45);
      c.fillStyle = rand() < 0.15 ? "#cfe0ff" : "#ffffff";
      c.beginPath();
      c.arc(rand() * W, rand() * H, big ? 0.9 + rand() * 0.8 : 0.3 + rand() * 0.6, 0, 2 * Math.PI);
      c.fill();
    }
  }

  function centerY() {
    return H * 0.52;
  }

  function resize() {
    const r = canvas.getBoundingClientRect();
    W = r.width;
    H = r.height;
    DPR = Math.min(window.devicePixelRatio || 1, 2);
    canvas.width = Math.round(W * DPR);
    canvas.height = Math.round(H * DPR);
    baseRadius = Math.max(80, Math.min(W / 2 - 6, H / 2 - 70));
    drawStars();
    requestDraw();
    scheduleDetail();
  }

  function requestDraw() {
    if (!frame) frame = requestAnimationFrame(draw);
  }

  function markInteracting() {
    interacting = true;
    clearTimeout(idleTimer);
    idleTimer = setTimeout(() => {
      interacting = false;
      requestDraw();
      scheduleDetail();
    }, 180);
  }

  // Once the view settles, ask for sharper imagery of what's on screen.
  let detailTimer = 0;
  function scheduleDetail() {
    if (!earth) return;
    clearTimeout(detailTimer);
    detailTimer = setTimeout(() => {
      if (!interacting && W) earth.updateDetail(visibleBounds());
    }, 120);
  }

  function visibleBounds() {
    const R = baseRadius * view.zoom;
    const cx = W / 2;
    const cy = centerY();
    let lonMin = Infinity;
    let lonMax = -Infinity;
    let latMin = Infinity;
    let latMax = -Infinity;
    const N = 14;
    for (let i = 0; i <= N; i++) {
      for (let j = 0; j <= N; j++) {
        let x = (W * i) / N;
        let y = (H * j) / N;
        const d = Math.hypot(x - cx, y - cy);
        if (d > R * 0.999) {
          // Off the globe: use the nearest point on its edge instead.
          x = cx + ((x - cx) * R * 0.999) / d;
          y = cy + ((y - cy) * R * 0.999) / d;
          if (x < 0 || x > W || y < 0 || y > H) continue;
        }
        const ll = projection.invert([x, y]);
        if (!ll || Number.isNaN(ll[0])) continue;
        const dl = ((ll[0] - view.lon + 540) % 360) - 180;
        lonMin = Math.min(lonMin, dl);
        lonMax = Math.max(lonMax, dl);
        latMin = Math.min(latMin, ll[1]);
        latMax = Math.max(latMax, ll[1]);
      }
    }
    // Grid sampling misses the corners between samples: pad by a cell.
    const padLon = (lonMax - lonMin) / N;
    const padLat = (latMax - latMin) / N;
    lonMin = view.lon + lonMin - padLon;
    lonMax = view.lon + lonMax + padLon;
    latMin = Math.max(-90, latMin - padLat);
    latMax = Math.min(90, latMax + padLat);
    for (const pole of [90, -90]) {
      const p = projection([0, pole]);
      if (isVisible([0, pole]) && p[0] >= 0 && p[0] <= W && p[1] >= 0 && p[1] <= H) {
        lonMin = view.lon - 180;
        lonMax = view.lon + 180;
        if (pole > 0) latMax = 90;
        else latMin = -90;
      }
    }
    return { lonMin, lonMax, latMin, latMax, pxPerDeg: (R * DPR * Math.PI) / 180 };
  }

  const isVisible = (lonlat) => d3.geoDistance(lonlat, [view.lon, view.lat]) < Math.PI / 2 - 0.005;

  function drawPin([lon, lat], color, label) {
    if (!isVisible([lon, lat])) return;
    const [x, y] = projection([lon, lat]);
    ctx.save();
    ctx.shadowColor = "rgba(0,0,0,0.35)";
    ctx.shadowBlur = 6;
    ctx.shadowOffsetY = 2;
    ctx.beginPath();
    ctx.moveTo(x, y);
    ctx.bezierCurveTo(x - 4, y - 8, x - 9, y - 12, x - 9, y - 19);
    ctx.arc(x, y - 19, 9, Math.PI, 0);
    ctx.bezierCurveTo(x + 9, y - 12, x + 4, y - 8, x, y);
    ctx.fillStyle = color;
    ctx.fill();
    ctx.restore();
    ctx.beginPath();
    ctx.arc(x, y - 19, 3.4, 0, 2 * Math.PI);
    ctx.fillStyle = "#fff";
    ctx.fill();
    if (label) {
      ctx.font = "600 13px 'IBM Plex Sans', system-ui, sans-serif";
      const w = ctx.measureText(label).width;
      const lx = Math.min(Math.max(x + 14, 8), W - w - 18);
      const ly = y - 30;
      ctx.fillStyle = "rgba(6, 10, 24, 0.82)";
      roundRect(lx - 6, ly - 13, w + 12, 20, 6);
      ctx.fill();
      ctx.fillStyle = "#fff";
      ctx.fillText(label, lx, ly + 2);
    }
  }

  function roundRect(x, y, w, h, r) {
    ctx.beginPath();
    ctx.moveTo(x + r, y);
    ctx.arcTo(x + w, y, x + w, y + h, r);
    ctx.arcTo(x + w, y + h, x, y + h, r);
    ctx.arcTo(x, y + h, x, y, r);
    ctx.arcTo(x, y, x + w, y, r);
    ctx.closePath();
  }

  function drawArc(a, b, color) {
    ctx.beginPath();
    geoPath({ type: "LineString", coordinates: [a, b] });
    ctx.setLineDash([6, 5]);
    ctx.lineWidth = 2.2;
    ctx.strokeStyle = color;
    ctx.stroke();
    ctx.setLineDash([]);
  }

  function draw() {
    frame = 0;
    if (!W) return;
    ctx.setTransform(DPR, 0, 0, DPR, 0, 0);
    ctx.clearRect(0, 0, W, H);
    projection
      .translate([W / 2, centerY()])
      .scale(baseRadius * view.zoom)
      .rotate([-view.lon, -view.lat]);

    if (earth && earth.ready) {
      earth.render({ width: W, height: H, dpr: DPR, cx: W / 2, cy: centerY(), radius: baseRadius * view.zoom, lon: view.lon, lat: view.lat });
    } else {
      drawVectorGlobe();
    }

    // Overlays
    if (game) {
      const shown = game.phase === "done" ? game.rounds.map((r, i) => [r, game.targets[i]]) : [];
      if (game.phase === "reveal") {
        const i = game.rounds.length - 1;
        shown.push([game.rounds[i], game.targets[i]]);
      }
      for (const [r, t] of shown) drawArc(r.g, [t.lon, t.lat], "#fff");
      for (const [r] of shown) drawPin(r.g, colors.guess);
      for (const [, t] of shown) drawPin([t.lon, t.lat], colors.answer, t.name);
      if (game.phase === "guess" && pending) drawPin(pending, colors.guess);
    }
  }

  function drawVectorGlobe() {
    ctx.beginPath();
    geoPath({ type: "Sphere" });
    ctx.fillStyle = "#0a2268";
    ctx.fill();

    ctx.beginPath();
    geoPath(graticule);
    ctx.lineWidth = 1;
    ctx.strokeStyle = "rgba(150, 180, 255, 0.08)";
    ctx.stroke();

    const geo = interacting || !topoHi ? topoLo : topoHi;
    if (geo) {
      ctx.beginPath();
      geoPath(geo.land);
      ctx.fillStyle = "#4f6b3a";
      ctx.fill();
    }

    ctx.beginPath();
    geoPath({ type: "Sphere" });
    ctx.lineWidth = 1.5;
    ctx.strokeStyle = "rgba(90, 140, 255, 0.6)";
    ctx.stroke();
  }

  // Animate the view; lon takes the short way round.
  let anim = null;
  function animateTo(target, ms = 900) {
    if (anim) anim.stop();
    const from = { ...view };
    let dLon = ((target.lon - from.lon + 540) % 360) - 180;
    const toZoom = target.zoom ?? from.zoom;
    const ease = d3.easeCubicInOut;
    anim = d3.timer((t) => {
      const k = ease(Math.min(1, t / ms));
      view.lon = from.lon + dLon * k;
      view.lat = from.lat + ((target.lat ?? from.lat) - from.lat) * k;
      view.zoom = Math.exp(Math.log(from.zoom) + (Math.log(toZoom) - Math.log(from.zoom)) * k);
      markInteracting();
      requestDraw();
      if (k >= 1) {
        anim.stop();
        anim = null;
        view.lon = ((view.lon + 540) % 360) - 180;
      }
    });
  }

  function fitPoints(a, b) {
    const mid = d3.geoInterpolate(a, b)(0.5);
    const theta = d3.geoDistance(a, b);
    const zoom = Math.max(1, Math.min(24, 0.55 / Math.sin(Math.max(theta / 2, 0.0005))));
    animateTo({ lon: mid[0], lat: mid[1], zoom });
  }

  // ---------------------------------------------------------------- input

  const pointers = new Map();
  let gesture = null;

  function localPoint(e) {
    const r = canvas.getBoundingClientRect();
    return [e.clientX - r.left, e.clientY - r.top];
  }

  canvas.addEventListener("pointerdown", (e) => {
    canvas.setPointerCapture(e.pointerId);
    pointers.set(e.pointerId, localPoint(e));
    if (anim) {
      anim.stop();
      anim = null;
    }
    if (pointers.size === 1) {
      gesture = { start: localPoint(e), lon: view.lon, lat: view.lat, moved: false, multi: false, t: performance.now() };
    } else if (pointers.size === 2) {
      const [p, q] = [...pointers.values()];
      gesture = { ...gesture, multi: true, pinch: Math.hypot(p[0] - q[0], p[1] - q[1]), zoom: view.zoom };
    }
  });

  canvas.addEventListener("pointermove", (e) => {
    if (!pointers.has(e.pointerId) || !gesture) return;
    pointers.set(e.pointerId, localPoint(e));
    if (pointers.size >= 2 && gesture.pinch) {
      const [p, q] = [...pointers.values()];
      const d = Math.hypot(p[0] - q[0], p[1] - q[1]);
      view.zoom = clampZoom(gesture.zoom * (d / gesture.pinch));
    } else if (pointers.size === 1 && !gesture.multi) {
      const [x, y] = localPoint(e);
      const dx = x - gesture.start[0];
      const dy = y - gesture.start[1];
      if (!gesture.moved && Math.hypot(dx, dy) < 6) return;
      gesture.moved = true;
      canvas.classList.add("dragging");
      const degPerPx = 180 / Math.PI / (baseRadius * view.zoom);
      view.lon = gesture.lon - dx * degPerPx;
      view.lat = Math.max(-89, Math.min(89, gesture.lat + dy * degPerPx));
    } else {
      return;
    }
    markInteracting();
    requestDraw();
  });

  function endPointer(e) {
    if (!pointers.has(e.pointerId)) return;
    pointers.delete(e.pointerId);
    canvas.classList.remove("dragging");
    if (pointers.size === 0 && gesture) {
      const g = gesture;
      gesture = null;
      if (e.type === "pointerup" && !g.moved && !g.multi && performance.now() - g.t < 700) {
        onTap(localPoint(e));
      }
    } else if (pointers.size === 1 && gesture) {
      // Pinch ended with one finger still down: keep dragging from here.
      const [p] = [...pointers.values()];
      gesture = { start: p, lon: view.lon, lat: view.lat, moved: true, multi: false, t: 0 };
    }
  }
  canvas.addEventListener("pointerup", endPointer);
  canvas.addEventListener("pointercancel", endPointer);

  canvas.addEventListener(
    "wheel",
    (e) => {
      e.preventDefault();
      view.zoom = clampZoom(view.zoom * Math.exp(-e.deltaY * (e.ctrlKey ? 0.01 : 0.0015)));
      markInteracting();
      requestDraw();
    },
    { passive: false }
  );

  const clampZoom = (z) => Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, z));
  function zoomBy(f) {
    animateTo({ lon: view.lon, lat: view.lat, zoom: clampZoom(view.zoom * f) }, 250);
  }
  $("zoom-in").addEventListener("click", () => zoomBy(1.8));
  $("zoom-out").addEventListener("click", () => zoomBy(1 / 1.8));

  function onTap([x, y]) {
    if (!game || game.phase !== "guess") return;
    const r = Math.hypot(x - W / 2, y - centerY());
    if (r > baseRadius * view.zoom) return;
    const lonlat = projection.invert([x, y]);
    if (!lonlat || Number.isNaN(lonlat[0])) return;
    pending = lonlat;
    setAction("Confirm guess", true);
    requestDraw();
  }

  // ---------------------------------------------------------------- server

  // Same origin as the page, so the Cloudflare Access cookie identifies the player.
  async function api(path, body) {
    const opts = body === undefined ? {} : { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) };
    const res = await fetch(path, opts);
    if (res.status === 401 || (res.redirected && !res.url.includes("/api/"))) {
      // The sign-in expired: reloading lets Access log the player in again.
      // The flag stops a reload loop if something else is wrong.
      if (!sessionStorage.getItem("zaptap:relogin")) {
        sessionStorage.setItem("zaptap:relogin", "1");
        location.reload();
      }
      const err = new Error("not_signed_in");
      err.code = "not_signed_in";
      throw err;
    }
    sessionStorage.removeItem("zaptap:relogin");
    const data = await res.json().catch(() => ({}));
    if (!res.ok) {
      const err = new Error(data.error || "http_" + res.status);
      err.code = data.error;
      throw err;
    }
    return data;
  }

  // ---------------------------------------------------------------- game

  let game = null; // { mode, key, targets, rounds, rec, phase }
  let pending = null;
  let busy = false;

  function dailyRecord(key) {
    if (!store.days[key]) {
      store.days[key] = { targets: ONLINE ? [] : pickTargets(mulberry32(hashStr("zaptap:" + key))), rounds: [] };
      save();
    }
    return store.days[key];
  }

  async function startDaily() {
    if (busy) return;
    closeSheets();
    if (!ONLINE) {
      const key = dayKey();
      const rec = dailyRecord(key);
      game = { mode: "daily", key, targets: rec.targets, rounds: rec.rounds, rec, phase: "guess" };
      if (rec.rounds.length >= 5) return showFinishedMap(true);
      return startRound();
    }

    busy = true;
    let st;
    try {
      st = await api("/api/state", {});
    } catch (e) {
      toast("Couldn't reach the ZapTap server. Try again in a bit.");
      return;
    } finally {
      busy = false;
    }
    store.name = st.name;

    // Mirror the server's view locally, keeping what only this browser knows.
    const rec = dailyRecord(st.day);
    const rounds = [];
    for (const r of st.rounds) {
      rec.targets[r.round] = targetFrom(r);
      if (r.score !== undefined) rounds.push({ ...(rec.rounds[r.round] || {}), ...roundFrom(r) });
    }
    rec.rounds = rounds;
    if (st.done) rec.total = st.total;
    else delete rec.total; // e.g. a game played offline before the server existed
    save();
    game = { mode: "daily", key: st.day, targets: rec.targets, rounds: rec.rounds, rec, phase: "guess" };
    if (st.done) return showFinishedMap(true);
    startRound();
  }

  const targetFrom = (r) => ({
    name: r.name,
    country: r.country,
    id: r.countryId,
    lon: r.answer ? r.answer[0] : undefined,
    lat: r.answer ? r.answer[1] : undefined,
  });
  const roundFrom = (r) => ({ g: r.guess, km: r.km, s: r.score, h: r.inCountry ? 1 : 0, b: r.bonus || undefined });

  function startPractice() {
    const targets = pickTargets(mulberry32((Math.random() * 2 ** 32) >>> 0));
    game = { mode: "practice", targets, rounds: [], phase: "guess" };
    closeSheets();
    startRound();
  }

  function runningTotal() {
    return game.rounds.reduce((sum, r, i) => sum + r.s * WEIGHTS[i], 0);
  }

  function startRound() {
    const i = game.rounds.length;
    const t = game.targets[i];
    game.phase = "guess";
    pending = null;
    $("prompt").hidden = false;
    $("dock").hidden = false;
    $("reveal").hidden = true;
    $("round-label").textContent = `${game.mode === "practice" ? "Practice · " : ""}Round ${i + 1} / 5`;
    $("round-weight").textContent = "×" + WEIGHTS[i];
    $("running-total").textContent = runningTotal();
    $("place-name").textContent = t.name;
    $("place-country").textContent = t.country;
    setAction("Tap the globe to guess", false);
    animateTo({ lon: view.lon, lat: Math.max(-35, Math.min(45, view.lat)), zoom: 1 }, 600);
    requestDraw();
  }

  function setAction(label, enabled) {
    const b = $("action-btn");
    b.textContent = label;
    b.disabled = !enabled;
  }

  async function confirmGuess() {
    const i = game.rounds.length;
    const t = game.targets[i];
    const g = [round4(pending[0]), round4(pending[1])];

    if (game.mode === "daily" && ONLINE) {
      game.phase = "checking";
      setAction("Checking…", false);
      let res;
      try {
        res = await api("/api/guess", { day: game.key, round: i, lon: g[0], lat: g[1] });
      } catch (e) {
        game.phase = "guess";
        if (["day_over", "wrong_round", "done"].includes(e.code)) {
          toast(e.code === "day_over" ? "A new ZapTap has started!" : "Syncing with the server…");
          return startDaily();
        }
        toast("Couldn't reach the server. Try again.");
        setAction("Confirm guess", true);
        return;
      }
      Object.assign(t, targetFrom(res.result));
      if (res.next) game.targets[i + 1] = targetFrom(res.next);
      if (res.done) game.rec.total = res.total;
      return applyRound(roundFrom(res.result), i);
    }

    const km = d3.geoDistance(g, [t.lon, t.lat]) * EARTH_KM;
    let s = distanceScore(km);
    let b;
    const hit = countryAt(g);
    if (game.mode === "daily") {
      if (hit === t.id && s < 25) {
        s = 25;
        b = "country";
      } else if (hit && hit !== t.id && continentOf(hit) === continentOf(t.id) && s < 10) {
        s = 10;
        b = "continent";
      }
    }
    applyRound({ g, km: Math.round(km * 10) / 10, s, h: hit === t.id ? 1 : 0, b }, i);
  }

  function applyRound(round, i) {
    const t = game.targets[i];
    round.c = countryAt(round.g) || undefined; // for achievements
    game.rounds.push(round);
    if (game.mode === "daily") save();
    game.phase = "reveal";
    pending = null;

    const w = WEIGHTS[i];
    $("reveal").hidden = false;
    $("reveal-score").textContent = round.s;
    $("reveal-distance").textContent = `${formatKm(round.km)} away${w > 1 ? ` · ×${w} = ${round.s * w}` : ""}`;
    $("reveal-note").textContent =
      round.b === "country" ? "Right country: bumped up to 25." : round.b === "continent" ? "Right continent: bumped up to 10." : verdict(round.s);
    $("running-total").textContent = runningTotal();
    setAction(game.rounds.length < 5 ? "Next round" : "See results", true);
    fitPoints(round.g, [t.lon, t.lat]);
  }

  const round4 = (x) => Math.round(x * 1e4) / 1e4;

  function formatKm(km) {
    return km < 10 ? `${km.toFixed(1)} km` : `${Math.round(km).toLocaleString()} km`;
  }

  function verdict(s) {
    if (s === 100) return "Bullseye. Zivid-grade accuracy.";
    if (s >= 90) return "Sub-millimetre point cloud vibes.";
    if (s >= 75) return "Solid. The robot would pick that one.";
    if (s >= 50) return "Right neighbourhood, wrong street.";
    if (s >= 25) return "Needs a recalibration.";
    return "Was the camera even on?";
  }

  function finish() {
    const total = runningTotal();
    if (game.mode === "daily") {
      game.rec.total = total;
      game.rec.finishedAt = Date.now();
    } else {
      store.practice.played += 1;
      store.practice.sum += total;
      store.practice.best = Math.max(store.practice.best, total);
    }
    save();
    showFinishedMap(false);
    showResults();
    checkAchievements({ game });
  }

  function showFinishedMap(openResults) {
    game.phase = "done";
    pending = null;
    $("prompt").hidden = false;
    $("dock").hidden = false;
    $("reveal").hidden = true;
    $("round-label").textContent = game.mode === "daily" ? `ZapTap #${dayNumber(game.key)}` : "Practice";
    $("round-weight").textContent = "done";
    $("running-total").textContent = "";
    $("place-name").textContent = `${runningTotal()} / 1000`;
    $("place-country").textContent = game.mode === "daily" ? prettyDate(game.key) : "Practice round";
    setAction("Results", true);
    animateTo({ lon: view.lon, lat: view.lat, zoom: 1 }, 700);
    if (openResults) showResults();
  }

  $("action-btn").addEventListener("click", () => {
    if (!game) return;
    if (game.phase === "guess" && pending) confirmGuess();
    else if (game.phase === "reveal") game.rounds.length < 5 ? startRound() : finish();
    else if (game.phase === "done") showResults();
  });

  document.addEventListener("keydown", (e) => {
    if (e.key !== "Enter" || document.querySelector("dialog[open]")) return;
    const b = $("action-btn");
    if (!b.disabled && !$("dock").hidden && document.activeElement !== b) b.click();
  });

  // ---------------------------------------------------------------- sheets

  function openSheet(id) {
    closeSheets();
    const d = $(id);
    if (!d.open) d.showModal();
  }
  function closeSheets() {
    document.querySelectorAll("dialog[open]").forEach((d) => d.close());
  }
  document.querySelectorAll("[data-close]").forEach((b) => b.addEventListener("click", closeSheets));
  document.querySelectorAll("dialog").forEach((d) =>
    d.addEventListener("click", (e) => {
      if (e.target === d && d.id !== "name-dlg") d.close(); // backdrop
    })
  );

  function showHome() {
    const key = dayKey();
    const rec = store.days[key];
    $("home-date").textContent = `#${dayNumber(key)} · ${prettyDate(key)}`;
    const play = $("play-daily");
    if (rec && rec.rounds.length >= 5 && rec.total !== undefined) play.textContent = `See today's result (${rec.total})`;
    else if (rec && rec.rounds.length > 0) play.textContent = `Resume today (round ${rec.rounds.length + 1} / 5)`;
    else play.textContent = "Play today";
    $("home-board").hidden = !ONLINE;
    const st = computeStats();
    const bits = [];
    if (ONLINE && store.name) bits.push(`Playing as ${store.name}`);
    if (st.played) bits.push(`🔥 ${st.streak}-day streak`, `best ${st.best}`);
    $("home-streak").textContent = bits.join(" · ");
    openSheet("home");
  }

  // ---------------------------------------------------------------- results

  let myRank = null; // { rank, of } on today's board, once known

  function showResults() {
    const total = runningTotal();
    const daily = game.mode === "daily";
    $("results-title").textContent = daily
      ? `ZapTap #${dayNumber(game.key)} · ${prettyDate(game.key, { day: "numeric", month: "short" })}`
      : "Practice round";
    $("results-total").textContent = total;
    const list = $("results-rounds");
    list.replaceChildren(
      ...game.rounds.map((r, i) => {
        const t = game.targets[i];
        const li = document.createElement("li");
        li.innerHTML = `<span>${scoreEmoji(r.s)}</span>
          <span><div class="r-name"></div><div class="r-sub"></div></span>
          <span class="r-score">${r.s * WEIGHTS[i]}<small>${r.s} ×${WEIGHTS[i]}</small></span>`;
        li.querySelector(".r-name").textContent = t.name;
        li.querySelector(".r-sub").textContent = `${t.country} · ${formatKm(r.km)}`;
        return li;
      })
    );
    $("results-board").hidden = !(daily && ONLINE);
    $("results-rank").hidden = true;
    openSheet("results");
    tickCountdown();
    if (daily && ONLINE) refreshRank();
  }

  async function refreshRank() {
    try {
      const board = await api(`/api/leaderboard?day=${game.key}`);
      const i = board.entries.findIndex((e) => e.name === store.name);
      if (i < 0) return;
      myRank = { day: board.day, rank: i + 1, of: board.entries.length };
      $("results-rank").textContent = `${medal(i + 1)} ${ordinal(i + 1)} of ${board.entries.length} today so far`;
      $("results-rank").hidden = false;
    } catch (e) {
      // The rank is a nice-to-have.
    }
  }

  const medal = (n) => ["🥇", "🥈", "🥉"][n - 1] || "🏅";
  function ordinal(n) {
    const s = ["th", "st", "nd", "rd"];
    const v = n % 100;
    return n + (s[(v - 20) % 10] || s[v] || s[0]);
  }

  function shareText() {
    const daily = game.mode === "daily";
    const head = daily ? `ZapTap #${dayNumber(game.key)} ⚡ ${runningTotal()}/1000` : `ZapTap practice ⚡ ${runningTotal()}/1000`;
    const line = game.rounds.map((r) => scoreEmoji(r.s) + r.s).join(" ");
    const rank = daily && myRank && myRank.day === game.key ? `\n${medal(myRank.rank)} ${ordinal(myRank.rank)} of ${myRank.of} so far` : "";
    return `${head}\n${line}${rank}\n${location.host}${location.pathname}`;
  }

  $("share-btn").addEventListener("click", async () => {
    const text = shareText();
    try {
      await navigator.clipboard.writeText(text);
      toast("Copied to clipboard");
    } catch (e) {
      if (navigator.share) {
        try {
          await navigator.share({ text });
        } catch (err) {
          return; // User dismissed the share sheet.
        }
      } else {
        toast("Couldn't copy, sorry");
        return;
      }
    }
    store.shares += 1;
    save();
    checkAchievements({});
  });

  let countdownTimer = 0;
  function tickCountdown() {
    clearTimeout(countdownTimer);
    if (!$("results").open) return;
    const s = secondsToNextDay();
    $("next-countdown").textContent = `Next ZapTap in ${pad(Math.floor(s / 3600))}:${pad(Math.floor((s % 3600) / 60))}:${pad(s % 60)}`;
    countdownTimer = setTimeout(tickCountdown, 1000);
  }

  // ---------------------------------------------------------------- name

  let afterName = null;
  function askName(then) {
    afterName = then;
    $("name-input").value = store.name || "";
    $("name-error").textContent = "";
    openSheet("name-dlg");
    setTimeout(() => $("name-input").focus(), 50);
  }

  $("name-form").addEventListener("submit", async (e) => {
    e.preventDefault();
    const name = $("name-input").value.trim();
    if (!name) return;
    const btn = $("name-save");
    btn.disabled = true;
    try {
      const res = await api("/api/name", { name });
      store.name = res.name;
      save();
      closeSheets();
      const then = afterName;
      afterName = null;
      if (then) then();
    } catch (err) {
      $("name-error").textContent =
        err.code === "name_taken" ? "Someone already uses that name. Try another one." : err.code === "bad_name" ? "Use 1 to 24 characters." : "Couldn't reach the server. Try again.";
    } finally {
      btn.disabled = false;
    }
  });
  $("name-cancel").addEventListener("click", () => {
    afterName = null;
    closeSheets();
  });

  // ---------------------------------------------------------------- leaderboard

  let boardDay = null;

  async function showBoard(day = dayKey()) {
    boardDay = day;
    $("board-tabs").querySelectorAll("button").forEach((b) => b.classList.toggle("on", b.dataset.day === (day === dayKey() ? "today" : "yesterday")));
    $("board-title").textContent = `ZapTap #${dayNumber(day)} · ${prettyDate(day, { weekday: "short", day: "numeric", month: "short" })}`;
    $("board-list").innerHTML = `<li class="muted">Loading…</li>`;
    $("board-note").textContent = "";
    $("board-me").textContent = store.name ? `You're playing as ${store.name}.` : "Play today's ZapTap to get on the board.";
    $("board-rename").hidden = !store.name;
    if (!$("board").open) openSheet("board");
    let board;
    try {
      board = await api(`/api/leaderboard?day=${day}`);
    } catch (e) {
      $("board-list").innerHTML = `<li class="muted">Couldn't load the leaderboard.</li>`;
      return;
    }
    if (boardDay !== day) return;
    renderBoard(board);
    checkAchievements({ board });
  }

  function renderBoard(board) {
    const list = $("board-list");
    if (!board.entries.length) {
      list.innerHTML = `<li class="muted">No finished games yet. Be the first!</li>`;
    } else {
      list.replaceChildren(
        ...board.entries.map((e, i) => {
          const li = document.createElement("li");
          if (e.name === store.name) li.className = "me";
          li.innerHTML = `<span class="b-rank">${i < 3 ? medal(i + 1) : i + 1}</span><span class="b-name"></span><span class="b-emoji">${e.scores
            .map(scoreEmoji)
            .join("")}</span><span class="b-total">${e.total}</span>`;
          li.querySelector(".b-name").textContent = e.name;
          return li;
        })
      );
    }
    const notes = [];
    if (board.playing) notes.push(`${board.playing} still playing`);
    notes.push(board.final ? "Final results." : "Resets at midnight Oslo time.");
    $("board-note").textContent = notes.join(" · ");
  }

  $("board-tabs").addEventListener("click", (e) => {
    const b = e.target.closest("button");
    if (!b) return;
    showBoard(b.dataset.day === "today" ? dayKey() : previousKey(dayKey()));
  });
  $("board-rename").addEventListener("click", () => askName(() => showBoard(boardDay || dayKey())));

  // ---------------------------------------------------------------- achievements

  const NYDALEN = [10.7652, 59.9494]; // Zivid HQ
  const bannedIds = new Set(DATA.bannedIds);

  const ACHIEVEMENTS = [
    { id: "first", icon: "📸", name: "First Capture", desc: "Finish your first daily ZapTap.", check: (c) => c.stats.played >= 1 },
    { id: "calibrated", icon: "🎯", name: "Calibrated", desc: "Score 100 on a round.", check: (c) => c.rounds.some((r) => r.s === 100) },
    { id: "submm", icon: "🔬", name: "Sub-millimetre Accuracy", desc: "Drop a pin within 5 km of a target.", check: (c) => c.rounds.some((r) => r.km <= 5) },
    { id: "hdr", icon: "🌈", name: "HDR Capture", desc: "Score 80+ on all five rounds of a daily.", check: (c) => c.days.some((d) => d.rounds.every((r) => r.s >= 80)) },
    { id: "dense", icon: "☁️", name: "Dense Point Cloud", desc: "Score 900+ on a daily.", check: (c) => c.stats.best >= 900 },
    { id: "impact", icon: "💥", name: "Make Impact", desc: "Score 90+ on both ×3 rounds of a daily.", check: (c) => c.days.some((d) => d.rounds[3].s >= 90 && d.rounds[4].s >= 90) },
    {
      id: "motion",
      icon: "🦾",
      name: "Motion Planned",
      desc: "Score higher on every round than on the one before.",
      check: (c) => c.games.some((g) => g.rounds.length === 5 && g.rounds.every((r, i) => i === 0 || r.s > g.rounds[i - 1].s)),
    },
    { id: "handeye", icon: "🤝", name: "Hand-Eye Calibrated", desc: "Score 700+ three days in a row.", check: (c) => c.runOf((d) => d.total >= 700) >= 3 },
    { id: "bin", icon: "📦", name: "Bin Picker", desc: "Finish 10 dailies.", check: (c) => c.stats.played >= 10 },
    { id: "depal", icon: "🏗️", name: "Depalletizer", desc: "Finish 50 dailies.", check: (c) => c.stats.played >= 50 },
    { id: "streak7", icon: "🔁", name: "Always-On Camera", desc: "Play 7 days in a row.", check: (c) => c.stats.maxStreak >= 7 },
    { id: "streak30", icon: "🏭", name: "Industrial Grade", desc: "Play 30 days in a row.", check: (c) => c.stats.maxStreak >= 30 },
    {
      id: "global",
      icon: "🌍",
      name: "Global Customer Base",
      desc: "Land in the right country on six different continents.",
      check: (c) => new Set(c.days.flatMap((d) => d.rounds.map((r, i) => r.h && continentOf(d.targets[i] && d.targets[i].id)).filter(Boolean))).size >= 6,
    },
    { id: "lunch", icon: "🥪", name: "Second Lunch", desc: "Finish a daily between 14:00 and 15:00.", check: (c) => c.finishedHour(14, 15) },
    { id: "practice", icon: "🧪", name: "Lab Rig", desc: "Finish 10 practice rounds.", check: () => store.practice.played >= 10 },
    { id: "trust", icon: "📣", name: "Build Trust", desc: "Share 5 results with the team.", check: () => store.shares >= 5 },
    { id: "podium", icon: "🥉", name: "Podium Pick", desc: "Finish top 3 on a day's final leaderboard (3+ players).", check: (c) => c.finalRank > 0 && c.finalRank <= 3 },
    { id: "top", icon: "🥇", name: "Top of the Pallet", desc: "Finish first on a day's final leaderboard (3+ players).", check: (c) => c.finalRank === 1 },
    { id: "homesick", icon: "🏠", name: "Homesick", desc: "Drop a pin in a country a Zividian comes from.", secret: true, check: (c) => c.rounds.some((r) => bannedIds.has(r.c)) },
    {
      id: "nydalen",
      icon: "🏢",
      name: "Back to Nydalen",
      desc: "Drop a pin within 25 km of the Oslo office.",
      secret: true,
      check: (c) => c.rounds.some((r) => d3.geoDistance(r.g, NYDALEN) * EARTH_KM <= 25),
    },
    { id: "artifact", icon: "👾", name: "Artifact Detected", desc: "Score 0 on a round.", secret: true, check: (c) => c.rounds.some((r) => r.s === 0) },
    { id: "recal", icon: "🔧", name: "Needs Recalibration", desc: "Finish a daily under 300.", secret: true, check: (c) => c.days.some((d) => d.total < 300) },
    { id: "night", icon: "🌙", name: "Night Shift", desc: "Finish a daily between midnight and 05:00.", secret: true, check: (c) => c.finishedHour(0, 5) },
    {
      id: "studio",
      icon: "🎓",
      name: "Zivid Studio Pro",
      desc: "Unlock 12 other achievements.",
      check: () => Object.keys(store.achievements).filter((id) => id !== "studio").length >= 12,
    },
  ];

  function achievementContext({ game: g, board }) {
    const stats = computeStats();
    const days = stats.done.map(([, d]) => d);
    const games = [...days];
    if (g && g.mode === "practice") games.push(g);
    let finalRank = 0;
    if (board && board.final && board.entries.length >= 3 && store.name) {
      finalRank = board.entries.findIndex((e) => e.name === store.name) + 1;
    }
    return {
      stats,
      days,
      games,
      rounds: games.flatMap((x) => x.rounds),
      finalRank,
      finishedHour: (from, to) => days.some((d) => d.finishedAt && new Date(d.finishedAt).getHours() >= from && new Date(d.finishedAt).getHours() < to),
      runOf(pred) {
        let best = 0;
        let run = 0;
        let prev = null;
        for (const [key, d] of [...stats.done].reverse()) {
          run = pred(d) ? (prev && previousKey(key) === prev ? run + 1 : 1) : 0;
          best = Math.max(best, run);
          prev = pred(d) ? key : null;
        }
        return best;
      },
    };
  }

  function checkAchievements(input, quiet = false) {
    const fresh = [];
    for (let pass = 0; pass < 2; pass++) {
      const ctx = achievementContext(input); // recomputed: "studio" depends on the others
      for (const a of ACHIEVEMENTS) {
        if (store.achievements[a.id]) continue;
        try {
          if (!a.check(ctx)) continue;
        } catch (e) {
          continue;
        }
        store.achievements[a.id] = dayKey();
        fresh.push(a);
      }
    }
    if (!fresh.length) return;
    save();
    if (!quiet) for (const a of fresh) toast(`${a.icon} Achievement unlocked: ${a.name}`, 2600);
  }

  function showAchievements() {
    const got = ACHIEVEMENTS.filter((a) => store.achievements[a.id]).length;
    $("ach-count").textContent = `${got} of ${ACHIEVEMENTS.length} unlocked`;
    $("ach-list").replaceChildren(
      ...ACHIEVEMENTS.map((a) => {
        const on = !!store.achievements[a.id];
        const hidden = a.secret && !on;
        const li = document.createElement("li");
        li.className = on ? "on" : "";
        li.innerHTML = `<span class="a-icon"></span><span><div class="a-name"></div><div class="a-desc"></div></span>`;
        li.querySelector(".a-icon").textContent = hidden ? "❔" : a.icon;
        li.querySelector(".a-name").textContent = hidden ? "Secret achievement" : a.name;
        li.querySelector(".a-desc").textContent = hidden
          ? "Keep playing to find out."
          : on
          ? `${a.desc} Unlocked ${prettyDate(store.achievements[a.id], { day: "numeric", month: "short" })}.`
          : a.desc;
        return li;
      })
    );
    openSheet("achievements");
  }

  // Final leaderboards can award achievements: look at yesterday's quietly.
  async function checkYesterdayBoard() {
    const y = previousKey(dayKey());
    const rec = store.days[y];
    if (!ONLINE || !store.name || !rec || rec.rounds.length < 5) return;
    try {
      checkAchievements({ board: await api(`/api/leaderboard?day=${y}`) });
    } catch (e) {
      // Try again next visit.
    }
  }

  $("results-close").addEventListener("click", closeSheets);
  $("results-practice").addEventListener("click", startPractice);
  $("results-board").addEventListener("click", () => showBoard(game.key));
  $("play-practice").addEventListener("click", startPractice);
  $("play-daily").addEventListener("click", startDaily);
  $("home-board").addEventListener("click", () => showBoard());
  $("home-btn").addEventListener("click", showHome);
  $("help-btn").addEventListener("click", () => openSheet("help"));
  $("stats-btn").addEventListener("click", showStats);
  $("board-btn").addEventListener("click", () => (ONLINE ? showBoard() : toast("The leaderboard isn't set up yet.")));
  $("ach-btn").addEventListener("click", showAchievements);

  // ---------------------------------------------------------------- stats

  function computeStats() {
    const done = Object.entries(store.days)
      .filter(([, d]) => d.rounds.length >= 5 && typeof d.total === "number")
      .sort(([a], [b]) => (a < b ? 1 : -1));
    const totals = done.map(([, d]) => d.total);
    const doneSet = new Set(done.map(([k]) => k));
    const today = dayKey();
    let streak = 0;
    let k = doneSet.has(today) ? today : previousKey(today);
    while (doneSet.has(k)) {
      streak++;
      k = previousKey(k);
    }
    let maxStreak = 0;
    let run = 0;
    let prev = null;
    for (const [key] of [...done].reverse()) {
      run = prev && previousKey(key) === prev ? run + 1 : 1;
      maxStreak = Math.max(maxStreak, run);
      prev = key;
    }
    return {
      done,
      played: done.length,
      avg: totals.length ? Math.round(totals.reduce((a, b) => a + b, 0) / totals.length) : 0,
      best: totals.length ? Math.max(...totals) : 0,
      streak,
      maxStreak,
      today: store.days[today] && store.days[today].rounds.length >= 5 ? store.days[today].total : null,
    };
  }

  function showStats() {
    const st = computeStats();
    const pr = store.practice;
    const tiles = [
      [st.played, "Played"],
      [st.avg || "–", "Average"],
      [st.best || "–", "Best"],
      [st.streak, "Streak"],
      [st.maxStreak, "Max streak"],
      [pr.played ? Math.round(pr.sum / pr.played) : "–", `Practice avg (${pr.played})`],
    ];
    $("stat-tiles").innerHTML = tiles.map(([v, l]) => `<div class="tile"><b>${v}</b><span>${l}</span></div>`).join("");

    const buckets = [
      ["900+", 900, 1001],
      ["800s", 800, 900],
      ["700s", 700, 800],
      ["600s", 600, 700],
      ["500s", 500, 600],
      ["< 500", 0, 500],
    ];
    const counts = buckets.map(([, lo, hi]) => st.done.filter(([, d]) => d.total >= lo && d.total < hi).length);
    const max = Math.max(1, ...counts);
    $("stat-dist").innerHTML = buckets
      .map(([label, lo, hi], i) => {
        const today = st.today !== null && st.today >= lo && st.today < hi ? " today" : "";
        return `<div class="dist__row"><span>${label}</span><div class="dist__bar${today}" style="width:${Math.max(8, (counts[i] / max) * 100)}%">${counts[i]}</div></div>`;
      })
      .join("");

    $("stat-history").innerHTML = st.done.length
      ? st.done
          .slice(0, 10)
          .map(
            ([key, d]) =>
              `<li><span>#${dayNumber(key)} · ${prettyDate(key, { weekday: "short", day: "numeric", month: "short" })}</span><span>${d.rounds
                .map((r) => scoreEmoji(r.s))
                .join("")} ${d.total}</span></li>`
          )
          .join("")
      : `<li><span class="muted">No finished days yet.</span><span></span></li>`;
    openSheet("stats");
  }

  // ---------------------------------------------------------------- misc

  // Toasts queue up so several achievements don't overwrite each other.
  const toasts = [];
  let toastBusy = false;
  function toast(msg, ms = 1800) {
    toasts.push([msg, ms]);
    if (!toastBusy) nextToast();
  }
  function nextToast() {
    const item = toasts.shift();
    const t = $("toast");
    if (!item) {
      toastBusy = false;
      return;
    }
    toastBusy = true;
    t.textContent = item[0];
    // As a popover it sits in the top layer, above any open dialog.
    if (t.showPopover) {
      if (t.matches(":popover-open")) t.hidePopover();
      t.showPopover();
    }
    requestAnimationFrame(() => t.classList.add("show"));
    setTimeout(() => {
      t.classList.remove("show");
      setTimeout(nextToast, 250);
    }, item[1]);
  }

  function renderFlags() {
    $("help-flags").innerHTML = "";
    for (const [name, flag] of DATA.banned) {
      const s = document.createElement("span");
      s.textContent = `${flag} ${name}`;
      $("help-flags").append(s);
    }
  }

  // ---------------------------------------------------------------- boot

  // On GitHub Pages, send players to the real game once it's deployed.
  if (!ONLINE && window.ZAPTAP_HOME) {
    location.replace(window.ZAPTAP_HOME);
    return;
  }
  readColors();
  renderFlags();
  checkAchievements({}, true); // backfill from history without fanfare
  if (earth) {
    earth.loadBase("earth.jpg").catch(() => console.warn("ZapTap: couldn't load earth.jpg, using vector map"));
  }
  window.addEventListener("resize", resize);
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
    readColors();
    requestDraw();
  });
  resize();

  // Low-res first so the globe shows up quickly, then the detailed borders.
  fetch(TOPO_LO)
    .then((r) => r.json())
    .then((lo) => {
      topoLo = drawable(lo);
      $("loading").hidden = true;
      requestDraw();
      showHome();
      return fetch(TOPO_HI)
        .then((r) => r.json())
        .then((hi) => {
          prepareGeo(hi);
          requestDraw();
          checkYesterdayBoard();
        });
    })
    .catch(() => {
      $("loading").textContent = "Couldn't load the map. Check your connection and reload.";
    });

  // Slow idle spin until the first game starts.
  d3.timer(() => {
    if (game || gesture || anim || !topoLo) return;
    view.lon += 0.04;
    requestDraw();
  });
})();
