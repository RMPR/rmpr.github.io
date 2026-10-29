(() => {
  "use strict";

  const DATA = window.ZAPTAP_DATA;
  const EARTH_KM = 6371;
  const WEIGHTS = [1, 1, 2, 3, 3];
  const TIERS = [1, 1, 2, 3, 3]; // place difficulty per round
  const EPOCH = "2026-10-29"; // ZapTap #1
  const STORE_KEY = "zaptap:v1";
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
  const store = Object.assign({ days: {}, practice: { played: 0, best: 0, sum: 0 } }, loadStore());
  function save() {
    try {
      localStorage.setItem(STORE_KEY, JSON.stringify(store));
    } catch (e) {
      // Private mode or storage full: the game still works, it just won't remember.
    }
  }

  // ---------------------------------------------------------------- dates

  const pad = (n) => String(n).padStart(2, "0");
  function dayKey(d = new Date()) {
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
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
    const d = keyToDate(key);
    d.setDate(d.getDate() - 1);
    return dayKey(d);
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
      return { name: pick[0], lat: pick[1], lon: pick[2], id: pick[4], country: DATA.countries[pick[4]][0] };
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
  const bannedIds = new Set(DATA.bannedIds);

  const drawable = (topo) => ({
    land: topojson.feature(topo, topo.objects.land),
    banned: topojson.merge(topo, topo.objects.countries.geometries.filter((g) => bannedIds.has(g.id))),
  });

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

  const canvas = $("globe");
  const ctx = canvas.getContext("2d");
  const projection = d3.geoOrthographic().clipAngle(90).precision(0.4);
  const geoPath = d3.geoPath(projection, ctx);
  const graticule = d3.geoGraticule10();
  const view = { lon: 15, lat: 20, zoom: 1 };
  let W = 0;
  let H = 0;
  let DPR = 1;
  let baseRadius = 200;
  let colors = {};
  let hatch = null;
  let interacting = false;
  let idleTimer = 0;
  let frame = 0;

  function readColors() {
    const cs = getComputedStyle(document.documentElement);
    for (const k of ["ocean", "ocean-edge", "land", "land-edge", "banned", "grid", "guess", "answer", "text", "surface", "accent"]) {
      colors[k] = cs.getPropertyValue("--" + k).trim();
    }
    const tile = document.createElement("canvas");
    tile.width = tile.height = 7;
    const t = tile.getContext("2d");
    t.strokeStyle = colors.banned;
    t.lineWidth = 1.1;
    t.beginPath();
    t.moveTo(-1, 8);
    t.lineTo(8, -1);
    t.stroke();
    hatch = ctx.createPattern(tile, "repeat");
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
    baseRadius = Math.max(80, Math.min(W / 2 - 16, H / 2 - 70) * 0.96);
    requestDraw();
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
    }, 180);
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
      ctx.fillStyle = colors.surface;
      ctx.globalAlpha = 0.92;
      roundRect(lx - 6, ly - 13, w + 12, 20, 6);
      ctx.fill();
      ctx.globalAlpha = 1;
      ctx.fillStyle = colors.text;
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
    ctx.lineWidth = 2;
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

    ctx.beginPath();
    geoPath({ type: "Sphere" });
    ctx.fillStyle = colors.ocean;
    ctx.fill();

    ctx.beginPath();
    geoPath(graticule);
    ctx.lineWidth = 1;
    ctx.strokeStyle = colors.grid;
    ctx.stroke();

    const geo = interacting || !topoHi ? topoLo : topoHi;
    if (geo) {
      ctx.beginPath();
      geoPath(geo.land);
      ctx.fillStyle = colors.land;
      ctx.fill();
      ctx.lineWidth = 0.8;
      ctx.strokeStyle = colors["land-edge"];
      ctx.stroke();

      ctx.beginPath();
      geoPath(geo.banned);
      ctx.fillStyle = hatch;
      ctx.fill();
    }

    ctx.beginPath();
    geoPath({ type: "Sphere" });
    ctx.lineWidth = 1.2;
    ctx.strokeStyle = colors["ocean-edge"];
    ctx.stroke();

    // Overlays
    if (game) {
      const shown = game.phase === "done" ? game.rounds.map((r, i) => [r, game.targets[i]]) : [];
      if (game.phase === "reveal") {
        const i = game.rounds.length - 1;
        shown.push([game.rounds[i], game.targets[i]]);
      }
      for (const [r, t] of shown) drawArc(r.g, [t.lon, t.lat], colors.text);
      for (const [r] of shown) drawPin(r.g, colors.guess);
      for (const [, t] of shown) drawPin([t.lon, t.lat], colors.answer, t.name);
      if (game.phase === "guess" && pending) drawPin(pending, colors.guess);
    }
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

  // ---------------------------------------------------------------- game

  let game = null; // { mode, key, targets, rounds, phase }
  let pending = null;

  function dailyRecord(key) {
    if (!store.days[key]) {
      store.days[key] = { targets: pickTargets(mulberry32(hashStr("zaptap:" + key))), rounds: [] };
      save();
    }
    return store.days[key];
  }

  function startDaily() {
    const key = dayKey();
    const rec = dailyRecord(key);
    game = { mode: "daily", key, targets: rec.targets, rounds: rec.rounds, rec, phase: "guess" };
    closeSheets();
    if (rec.rounds.length >= 5) return showFinishedMap(true);
    startRound();
  }

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

  function confirmGuess() {
    const i = game.rounds.length;
    const t = game.targets[i];
    const target = [t.lon, t.lat];
    const km = d3.geoDistance(pending, target) * EARTH_KM;
    let s = distanceScore(km);
    let bonus = "";
    if (game.mode === "daily") {
      const hit = countryAt(pending);
      if (hit === t.id && s < 25) {
        s = 25;
        bonus = "Right country: bumped up to 25.";
      } else if (hit && hit !== t.id && continentOf(hit) === continentOf(t.id) && s < 10) {
        s = 10;
        bonus = "Right continent: bumped up to 10.";
      }
    }
    const round = { g: [round4(pending[0]), round4(pending[1])], km: Math.round(km), s };
    game.rounds.push(round);
    if (game.mode === "daily") save();
    game.phase = "reveal";
    pending = null;

    $("reveal").hidden = false;
    $("reveal-score").textContent = s;
    $("reveal-distance").textContent = `${formatKm(km)} away${WEIGHTS[i] > 1 ? ` · ×${WEIGHTS[i]} = ${s * WEIGHTS[i]}` : ""}`;
    $("reveal-note").textContent = bonus || verdict(s);
    $("running-total").textContent = runningTotal();
    setAction(game.rounds.length < 5 ? "Next round" : "See results", true);
    fitPoints(round.g, target);
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
      save();
    } else {
      store.practice.played += 1;
      store.practice.sum += total;
      store.practice.best = Math.max(store.practice.best, total);
      save();
    }
    showFinishedMap(false);
    showResults();
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
      if (e.target === d) d.close(); // backdrop
    })
  );

  function showHome() {
    const key = dayKey();
    const rec = store.days[key];
    $("home-date").textContent = `#${dayNumber(key)} · ${prettyDate(key)}`;
    const play = $("play-daily");
    if (rec && rec.rounds.length >= 5) play.textContent = `See today's result (${rec.total})`;
    else if (rec && rec.rounds.length > 0) play.textContent = `Resume today (round ${rec.rounds.length + 1} / 5)`;
    else play.textContent = "Play today";
    const st = computeStats();
    $("home-streak").textContent = st.played ? `🔥 ${st.streak}-day streak · ${st.played} played · best ${st.best}` : "";
    openSheet("home");
  }

  function showResults() {
    const total = runningTotal();
    $("results-title").textContent =
      game.mode === "daily" ? `ZapTap #${dayNumber(game.key)} · ${prettyDate(game.key, { day: "numeric", month: "short" })}` : "Practice round";
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
    $("results-practice").hidden = false;
    openSheet("results");
    tickCountdown();
  }

  function shareText() {
    const head =
      game.mode === "daily" ? `ZapTap #${dayNumber(game.key)} ⚡ ${runningTotal()}/1000` : `ZapTap practice ⚡ ${runningTotal()}/1000`;
    const line = game.rounds.map((r) => scoreEmoji(r.s) + r.s).join(" ");
    return `${head}\n${line}\n${location.host}${location.pathname}`;
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
          // User dismissed the share sheet.
        }
      } else {
        toast("Couldn't copy, sorry");
      }
    }
  });

  let countdownTimer = 0;
  function tickCountdown() {
    clearTimeout(countdownTimer);
    if (!$("results").open) return;
    const now = new Date();
    const next = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1);
    const s = Math.max(0, Math.floor((next - now) / 1000));
    $("next-countdown").textContent = `Next ZapTap in ${pad(Math.floor(s / 3600))}:${pad(Math.floor((s % 3600) / 60))}:${pad(s % 60)}`;
    countdownTimer = setTimeout(tickCountdown, 1000);
  }

  $("results-close").addEventListener("click", closeSheets);
  $("results-practice").addEventListener("click", startPractice);
  $("play-practice").addEventListener("click", startPractice);
  $("play-daily").addEventListener("click", startDaily);
  $("home-btn").addEventListener("click", showHome);
  $("help-btn").addEventListener("click", () => openSheet("help"));
  $("stats-btn").addEventListener("click", showStats);

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

  let toastTimer = 0;
  function toast(msg) {
    const t = $("toast");
    t.textContent = msg;
    t.classList.add("show");
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => t.classList.remove("show"), 1800);
  }

  function renderFlags() {
    $("home-flags").textContent = DATA.banned.map(([, f]) => f).join("");
    $("home-flags").title = DATA.banned.map(([n]) => n).join(", ");
    $("help-flags").innerHTML = "";
    for (const [name, flag] of DATA.banned) {
      const s = document.createElement("span");
      s.textContent = `${flag} ${name}`;
      $("help-flags").append(s);
    }
  }

  // ---------------------------------------------------------------- boot

  readColors();
  renderFlags();
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
