// Builds zaptap/places.js from GeoNames cities15000 + curated landmarks,
// validated against the world-atlas 50m country polygons.
//
// Usage: node build.js <data-dir> <out-file> [<worker-json>]
// <data-dir> must hold cities15000.txt (download.geonames.org/export/dump),
// countries-50m.json (world-atlas@2.0.2) and all.csv
// (github.com/lukes/ISO-3166-Countries-with-Regional-Codes).
// Needs d3-geo and topojson-client installed next to this script.
const fs = require("fs");
const path = require("path");
const d3 = require("d3-geo");
const topojson = require("topojson-client");

const DATA = process.argv[2];
const OUT = process.argv[3];
const WORKER_OUT = process.argv[4];

// Zividian homelands (+ their overseas bits). ISO 3166 numeric.
const BANNED = {
  "250": "France", "258": "France", "540": "France", "666": "France", "663": "France", "652": "France", "260": "France",
  "840": "United States", "580": "United States", "850": "United States", "316": "United States", "016": "United States", "630": "United States",
  "392": "Japan", "120": "Cameroon", "356": "India", "398": "Kazakhstan", "380": "Italy",
  "578": "Norway", "744": "Norway", "074": "Norway",
  "040": "Austria", "688": "Serbia",
  "372": "Ireland", "638": "France", "254": "France", "474": "France", "312": "France", "175": "France", "876": "France",
  "581": "United States", "156": "China", "344": "China", "446": "China", "410": "South Korea",
  "276": "Germany", "076": "Brazil", "616": "Poland", "792": "Turkey", "752": "Sweden",
  "528": "Netherlands", "533": "Netherlands", "531": "Netherlands", "534": "Netherlands", "535": "Netherlands",
  "036": "Australia", "574": "Australia", "162": "Australia", "166": "Australia", "334": "Australia",
  "144": "Sri Lanka", "246": "Finland", "248": "Finland",
  "826": "United Kingdom", "831": "United Kingdom", "832": "United Kingdom", "833": "United Kingdom", "292": "United Kingdom",
  "238": "United Kingdom", "060": "United Kingdom", "136": "United Kingdom", "092": "United Kingdom", "500": "United Kingdom",
  "796": "United Kingdom", "660": "United Kingdom", "654": "United Kingdom", "612": "United Kingdom", "086": "United Kingdom", "239": "United Kingdom",
};

function parseCsv(text) {
  const rows = [];
  for (const line of text.trim().split("\n")) {
    const row = []; let cur = ""; let q = false;
    for (const ch of line) {
      if (ch === '"') q = !q;
      else if (ch === "," && !q) { row.push(cur); cur = ""; }
      else cur += ch;
    }
    row.push(cur); rows.push(row);
  }
  return rows;
}

const iso = parseCsv(fs.readFileSync(path.join(DATA, "all.csv"), "utf8")).slice(1);
const a2num = {}, numInfo = {};
for (const r of iso) {
  const [name, a2, , num, , region, sub, inter] = r;
  a2num[a2] = num;
  let cont = region;
  if (region === "Americas") cont = inter === "South America" ? "South America" : "North America";
  if (!region) cont = num === "158" ? "Asia" : "Antarctica";
  numInfo[num] = { cont };
}

const topo = JSON.parse(fs.readFileSync(path.join(DATA, "countries-50m.json"), "utf8"));
const feats = topojson.feature(topo, topo.objects.countries).features;
// Short, friendly display names.
const NAME_FIX = {
  "United States of America": "United States", "Dem. Rep. Congo": "DR Congo", "Central African Rep.": "Central African Republic",
  "Bosnia and Herz.": "Bosnia and Herzegovina", "Dominican Rep.": "Dominican Republic", "Eq. Guinea": "Equatorial Guinea",
  "S. Sudan": "South Sudan", "Solomon Is.": "Solomon Islands", "Côte d'Ivoire": "Côte d'Ivoire", "eSwatini": "Eswatini",
  "Faeroe Is.": "Faroe Islands", "W. Sahara": "Western Sahara", "Fr. Polynesia": "French Polynesia", "Cayman Is.": "Cayman Islands",
  "Falkland Is.": "Falkland Islands", "Marshall Is.": "Marshall Islands", "N. Mariana Is.": "Northern Mariana Islands",
  "Antigua and Barb.": "Antigua and Barbuda", "St. Vin. and Gren.": "Saint Vincent and the Grenadines", "St. Kitts and Nevis": "Saint Kitts and Nevis",
  "Turks and Caicos Is.": "Turks and Caicos Islands", "British Virgin Is.": "British Virgin Islands", "U.S. Virgin Is.": "U.S. Virgin Islands",
  "Cook Is.": "Cook Islands", "Wallis and Futuna Is.": "Wallis and Futuna", "São Tomé and Principe": "São Tomé and Príncipe",
  "Fr. S. Antarctic Lands": "French Southern Lands", "Heard I. and McDonald Is.": "Heard and McDonald Islands",
  "S. Geo. and the Is.": "South Georgia", "Br. Indian Ocean Ter.": "British Indian Ocean Territory", "Pitcairn Is.": "Pitcairn Islands",
  "St. Pierre and Miquelon": "Saint Pierre and Miquelon", "N. Cyprus": "Northern Cyprus", Macedonia: "North Macedonia",
};
// Countries without an ISO id in the topology get a synthetic key.
// Several features can share an id (e.g. Australia and Ashmore and Cartier Is.):
// name the country after its largest feature.
const countries = {};
const biggest = {};
feats.forEach((f, i) => {
  if (!f.id) f.id = "x" + i;
  const area = d3.geoArea(f);
  if (biggest[f.id] > area) return;
  biggest[f.id] = area;
  const name = NAME_FIX[f.properties.name] || f.properties.name;
  let cont = (numInfo[f.id] || {}).cont;
  if (!cont) cont = { Kosovo: "Europe", "N. Cyprus": "Asia", Somaliland: "Africa", "Siachen Glacier": "Asia", "Indian Ocean Ter.": "Oceania" }[f.properties.name] || "Asia";
  if (f.id === "010") cont = "Antarctica";
  countries[f.id] = [name, cont];
});

function countryAt(lon, lat) {
  for (const f of feats) if (d3.geoContains(f, [lon, lat])) return f.id;
  return null;
}

// For coastal points that fall just outside the simplified coastline:
// probe rings of increasing radius and take the first country found.
function nearestCountry(lon, lat) {
  for (const km of [3, 6, 10, 15, 20, 30]) {
    const found = new Set();
    for (let a = 0; a < 360; a += 15) {
      const r = km / 6371;
      const la = lat * Math.PI / 180, lo = lon * Math.PI / 180, b = a * Math.PI / 180;
      const la2 = Math.asin(Math.sin(la) * Math.cos(r) + Math.cos(la) * Math.sin(r) * Math.cos(b));
      const lo2 = lo + Math.atan2(Math.sin(b) * Math.sin(r) * Math.cos(la), Math.cos(r) - Math.sin(la) * Math.sin(la2));
      const id = countryAt(lo2 * 180 / Math.PI, la2 * 180 / Math.PI);
      if (id) found.add(id);
    }
    if (found.size === 1) return [...found][0];
    if (found.size > 1) return null;
  }
  return null;
}

const places = [];
const rejects = [];
// Places on contested ground: kept wherever the border data puts them, and
// shown with this label instead of a country ("" shows no country at all).
const CONTESTED = {
  "Jerusalem|IL": "",
  "Sevastopol|UA": "Crimea",
  "Hargeysa|SO": "",
  "Borama|SO": "",
  "Nicosia|CY": "",
};

function add(name, a2, lat, lon, tier, kind, isCapital = false) {
  const num = a2num[a2];
  if (!num || BANNED[num]) return false;
  const near = places.findIndex((p) => d3.geoDistance([p[2], p[1]], [lon, lat]) * 6371 < 15);
  if (near >= 0) {
    // A capital replaces a bigger city right next to it (Islamabad vs Rawalpindi);
    // anything else this close is a duplicate.
    if (isCapital && places[near][5] === "c" && places[near][4] === num) places.splice(near, 1);
    else return false;
  }
  let hit = countryAt(lon, lat);
  if (hit === null) hit = nearestCountry(lon, lat);
  const contested = CONTESTED[name + "|" + a2];
  const ok = contested === undefined ? hit === num : hit && !BANNED[hit];
  if (!ok) { rejects.push([name, a2, num, hit]); return false; }
  // A contested place takes the id of the shape it's in, for the country bonus.
  const place = [name, Math.round(lat * 1e4) / 1e4, Math.round(lon * 1e4) / 1e4, tier, hit, kind];
  if (contested !== undefined) place.push(contested);
  places.push(place);
  return true;
}

for (const [name, a2, lat, lon, tier] of require("./landmarks.js")) add(name, a2, lat, lon, tier, "l");

const lines = fs.readFileSync(path.join(DATA, "cities15000.txt"), "utf8").trim().split("\n");
const cities = lines.map((l) => {
  const c = l.split("\t");
  return { name: c[1], lat: +c[4], lon: +c[5], fcode: c[7], a2: c[8], pop: +c[14] };
}).filter((c) => c.fcode !== "PPLX" && c.fcode !== "PPLH" && (c.fcode === "PPLC" || c.pop >= 300000) && c.pop >= 15000)
  .sort((a, b) => b.pop - a.pop);

const R = 6371;
function dist(a, b) {
  return d3.geoDistance([a.lon, a.lat], [b.lon, b.lat]) * R;
}
const kept = [];
const perCountry = {};
for (const c of cities) {
  if (c.fcode !== "PPLC" && kept.some((k) => dist(k, c) < 60)) continue; // skip suburbs
  let tier;
  if ((c.fcode === "PPLC" && c.pop >= 1e6) || c.pop >= 5e6) tier = 1;
  else if ((c.fcode === "PPLC" && c.pop >= 2e5) || c.pop >= 1.5e6) tier = 2;
  else tier = 3;
  perCountry[c.a2] = perCountry[c.a2] || 0;
  if (c.fcode !== "PPLC" && perCountry[c.a2] >= 10) continue;
  if (add(c.name, c.a2, c.lat, c.lon, tier, "c", c.fcode === "PPLC")) { kept.push(c); perCountry[c.a2]++; }
}

const byTier = [1, 2, 3].map((t) => places.filter((p) => p[3] === t).length);
console.error("places", places.length, "by tier", byTier, "rejects", rejects.length);
if (process.env.VERBOSE) for (const r of rejects) console.error("reject", r.join(" | "));

const num2a2 = Object.fromEntries(Object.entries(a2num).map(([a2, n]) => [n, a2]));
const flag = (a2) => [...a2].map((ch) => String.fromCodePoint(0x1f1e6 + ch.charCodeAt(0) - 65)).join("");
const banned = [];
// Object key order puts "175" before "250", so name each country after its main id.
const MAIN = { France: "250", "United States": "840", Norway: "578", China: "156", "United Kingdom": "826", Netherlands: "528", Australia: "036", Finland: "246" };
for (const [num, name] of Object.entries(BANNED)) {
  if (!banned.some((b) => b[0] === name)) banned.push([name, flag(num2a2[MAIN[name] || num])]);
}
banned.sort((a, b) => a[0].localeCompare(b[0]));
const bannedIds = Object.keys(BANNED);
const out =
  "// Generated by build.js from GeoNames (CC BY 4.0) and curated landmarks. Do not edit by hand.\n" +
  "// Place: [name, lat, lon, tier (1 easy - 3 hard), country id, kind (c = city, l = landmark), country label if not the id's]\n" +
  "window.ZAPTAP_DATA = " + JSON.stringify({ banned, bannedIds, countries, places }) + ";\n";
if (OUT) fs.writeFileSync(OUT, out);
// The Worker picks the daily puzzle server-side from the same pool.
if (WORKER_OUT) fs.writeFileSync(WORKER_OUT, JSON.stringify({ bannedIds, countries, places }) + "\n");
