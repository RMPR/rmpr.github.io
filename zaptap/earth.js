// Satellite globe for ZapTap: a WebGL2 fragment shader that draws an
// orthographic globe with exactly the same maths as d3.geoOrthographic, so
// the 2D overlay (pins, arcs) and tap inversion stay in d3.
//
// Imagery is NASA Blue Marble (public domain): a bundled 4K base texture,
// plus detail tiles streamed from NASA GIBS when zoomed in. Blue Marble only
// paints the deep sea and the largest lakes flat blue. An inland water mask
// (bundled, and built from GIBS tiles alongside the detail tiles) marks rivers
// and other lakes too, so they get the same colour; shallow seas keep theirs.
(() => {
  "use strict";

  const GIBS = "https://gibs.earthdata.nasa.gov/wmts/epsg4326/best/BlueMarble_ShadedRelief/default/500m";
  // Same grid as the imagery. Inland water = MODIS water inside OSM land.
  const WATER = "https://gibs.earthdata.nasa.gov/wmts/epsg4326/best/MODIS_Water_Mask/default/250m";
  const LAND = "https://gibs.earthdata.nasa.gov/wmts/epsg4326/best/OSM_Land_Mask/default/250m";
  const COAST_PX = 2; // how far in from the coast inland water starts
  const TILE = 512;
  const MAX_LEVEL = 7; // ~500 m per pixel
  const MIN_DETAIL_LEVEL = 4; // below this the base texture is as sharp
  const MAX_TILES = 30;

  const VERT = `#version 300 es
in vec2 aPos;
void main() { gl_Position = vec4(aPos, 0.0, 1.0); }`;

  const FRAG = `#version 300 es
precision highp float;
uniform vec2 uCenter;   // device px, origin bottom-left
uniform float uRadius;  // device px
uniform vec2 uRot;      // centre lon, lat (radians)
uniform float uGlow;    // halo width, device px
uniform sampler2D uBase;
uniform sampler2D uDetail;
uniform sampler2D uBaseWater;   // red = water
uniform sampler2D uDetailWater;
uniform vec4 uBox;      // detail box: lon min, lat max, lon span, lat span (radians)
uniform float uDetailOn;
uniform vec4 uWaterBox; // same for the detail water mask
uniform float uWaterOn;
uniform vec3 uOcean;
out vec4 outColor;
const float PI = 3.141592653589793;

void main() {
  vec2 p = (gl_FragCoord.xy - uCenter) / uRadius;
  float r = length(p);
  float xp = sqrt(max(0.0, 1.0 - r * r)); // depth towards the viewer

  // Inverse of d3's rotate([-lon, -lat]) followed by the orthographic projection.
  float c = cos(uRot.y), s = sin(uRot.y);
  float X = xp * c - p.y * s;
  float Z = p.y * c + xp * s;
  float lon = mod(atan(p.x, X) + uRot.x + PI, 2.0 * PI) - PI;
  float lat = asin(clamp(Z, -1.0, 1.0));

  // Longitude jumps at the antimeridian; take derivatives from whichever
  // parametrisation is continuous here so mipmapping doesn't draw a seam.
  float lonAlt = mod(lon, 2.0 * PI);
  vec2 g1 = vec2(dFdx(lon), dFdy(lon));
  vec2 g2 = vec2(dFdx(lonAlt), dFdy(lonAlt));
  vec2 gLon = vec2(abs(g1.x) < abs(g2.x) ? g1.x : g2.x, abs(g1.y) < abs(g2.y) ? g1.y : g2.y);
  vec2 gLat = vec2(dFdx(lat), dFdy(lat));

  vec2 uv = vec2((lon + PI) / (2.0 * PI), (0.5 * PI - lat) / PI);
  vec2 bdx = vec2(gLon.x / (2.0 * PI), -gLat.x / PI);
  vec2 bdy = vec2(gLon.y / (2.0 * PI), -gLat.y / PI);
  vec3 col = textureGrad(uBase, uv, bdx, bdy).rgb;
  float mask = textureGrad(uBaseWater, uv, bdx, bdy).r;

  if (uDetailOn > 0.5) {
    float dl = mod(lon - uBox.x, 2.0 * PI);
    float dy = uBox.y - lat;
    if (dl < uBox.z && dy >= 0.0 && dy <= uBox.w) {
      vec2 duv = vec2(dl / uBox.z, dy / uBox.w);
      vec2 ddx = vec2(gLon.x / uBox.z, -gLat.x / uBox.w);
      vec2 ddy = vec2(gLon.y / uBox.z, -gLat.y / uBox.w);
      col = textureGrad(uDetail, duv, ddx, ddy).rgb;
    }
  }
  if (uWaterOn > 0.5) {
    float dl = mod(lon - uWaterBox.x, 2.0 * PI);
    float dy = uWaterBox.y - lat;
    if (dl < uWaterBox.z && dy >= 0.0 && dy <= uWaterBox.w) {
      mask = textureGrad(uDetailWater, vec2(dl / uWaterBox.z, dy / uWaterBox.w),
                         vec2(gLon.x / uWaterBox.z, -gLat.x / uWaterBox.w), vec2(gLon.y / uWaterBox.z, -gLat.y / uWaterBox.w)).r;
    }
  }

  // Water is either Blue Marble's flat near-black sea or anything the water
  // mask marks (rivers, lakes): paint it all one brighter navy.
  float sea = 1.0 - smoothstep(0.035, 0.11, distance(col, vec3(0.008, 0.02, 0.078)));
  float water = max(sea, mask);
  vec3 landCol = pow(col, vec3(0.86)) * 1.12;
  col = mix(landCol, uOcean, water);

  // Soft limb darkening and a thin atmosphere rim.
  col *= 0.7 + 0.3 * pow(xp, 0.5);
  col += vec3(0.3, 0.5, 1.0) * pow(1.0 - xp, 5.0) * 0.5;

  float inside = clamp((1.0 - r) * uRadius + 0.5, 0.0, 1.0);
  float halo = exp(-max(r - 1.0, 0.0) * uRadius / uGlow) * 0.55 * (1.0 - inside);
  vec3 haloCol = vec3(0.32, 0.55, 1.0);
  outColor = vec4(col * inside + haloCol * halo, inside + halo);
}`;

  function compile(gl, type, src) {
    const sh = gl.createShader(type);
    gl.shaderSource(sh, src);
    gl.compileShader(sh);
    if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(sh));
    return sh;
  }

  function loadImage(url) {
    return new Promise((resolve, reject) => {
      const img = new Image();
      img.crossOrigin = "anonymous";
      img.onload = () => resolve(img);
      img.onerror = reject;
      img.src = url;
    });
  }

  window.createEarth = function (canvas, onChange) {
    const gl = canvas.getContext("webgl2", { alpha: true, premultipliedAlpha: true, antialias: false });
    if (!gl) return null;

    let prog;
    try {
      prog = gl.createProgram();
      gl.attachShader(prog, compile(gl, gl.VERTEX_SHADER, VERT));
      gl.attachShader(prog, compile(gl, gl.FRAGMENT_SHADER, FRAG));
      gl.linkProgram(prog);
      if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(prog));
    } catch (e) {
      console.warn("ZapTap: WebGL globe unavailable", e);
      return null;
    }
    gl.useProgram(prog);

    // One triangle covering the viewport.
    gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
    const aPos = gl.getAttribLocation(prog, "aPos");
    gl.enableVertexAttribArray(aPos);
    gl.vertexAttribPointer(aPos, 2, gl.FLOAT, false, 0, 0);

    const u = {};
    for (const name of ["uCenter", "uRadius", "uRot", "uGlow", "uBase", "uDetail", "uBaseWater", "uDetailWater", "uBox", "uDetailOn", "uWaterBox", "uWaterOn", "uOcean"]) {
      u[name] = gl.getUniformLocation(prog, name);
    }
    gl.uniform1i(u.uBase, 0);
    gl.uniform1i(u.uDetail, 1);
    gl.uniform1i(u.uBaseWater, 2);
    gl.uniform1i(u.uDetailWater, 3);
    gl.uniform3f(u.uOcean, 10 / 255, 34 / 255, 104 / 255);

    const maxTex = gl.getParameter(gl.MAX_TEXTURE_SIZE);
    const aniso = gl.getExtension("EXT_texture_filter_anisotropic");

    // Imagery is RGB; water masks are single-channel (R8, red = water).
    const RGB = { internal: gl.RGB8, format: gl.RGB, empty: [2, 5, 20] };
    const MASK = { internal: gl.R8, format: gl.RED, empty: [0] };

    function makeTexture(unit, wrapS, kind) {
      const tex = gl.createTexture();
      gl.activeTexture(gl.TEXTURE0 + unit);
      gl.bindTexture(gl.TEXTURE_2D, tex);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, wrapS);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      if (aniso) gl.texParameterf(gl.TEXTURE_2D, aniso.TEXTURE_MAX_ANISOTROPY_EXT, Math.min(8, gl.getParameter(aniso.MAX_TEXTURE_MAX_ANISOTROPY_EXT)));
      gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
      gl.texImage2D(gl.TEXTURE_2D, 0, kind.internal, 1, 1, 0, kind.format, gl.UNSIGNED_BYTE, new Uint8Array(kind.empty));
      return tex;
    }
    const baseTex = makeTexture(0, gl.REPEAT, RGB);
    const detailTex = makeTexture(1, gl.CLAMP_TO_EDGE, RGB);
    const baseWaterTex = makeTexture(2, gl.REPEAT, MASK);
    const detailWaterTex = makeTexture(3, gl.CLAMP_TO_EDGE, MASK);

    function upload(unit, tex, source, kind) {
      gl.activeTexture(gl.TEXTURE0 + unit);
      gl.bindTexture(gl.TEXTURE_2D, tex);
      gl.texImage2D(gl.TEXTURE_2D, 0, kind.internal, kind.format, gl.UNSIGNED_BYTE, source);
      gl.generateMipmap(gl.TEXTURE_2D);
    }

    let baseReady = false;
    let baseImg = null;
    let baseWaterImg = null;
    let detailOn = false;
    let detailBox = [0, 0, 1, 1];
    let waterOn = false;
    let waterBox = [0, 0, 1, 1];
    let detailKey = "";

    function loadBase(url, waterUrl) {
      // Without the mask the globe still works, rivers just keep their photo colour.
      loadImage(waterUrl)
        .then((img) => {
          baseWaterImg = img;
          upload(2, baseWaterTex, img, MASK);
          onChange();
        })
        .catch(() => console.warn("ZapTap: couldn't load " + waterUrl));
      return loadImage(url).then((img) => {
        baseImg = img;
        upload(0, baseTex, img, RGB);
        baseReady = true;
        onChange();
      });
    }

    function render({ width, height, dpr, cx, cy, radius, lon, lat }) {
      const w = Math.round(width * dpr);
      const h = Math.round(height * dpr);
      if (canvas.width !== w || canvas.height !== h) {
        canvas.width = w;
        canvas.height = h;
      }
      gl.viewport(0, 0, w, h);
      gl.clearColor(0, 0, 0, 0);
      gl.clear(gl.COLOR_BUFFER_BIT);
      const rad = Math.PI / 180;
      gl.uniform2f(u.uCenter, cx * dpr, h - cy * dpr);
      gl.uniform1f(u.uRadius, radius * dpr);
      gl.uniform2f(u.uRot, lon * rad, lat * rad);
      gl.uniform1f(u.uGlow, 16 * dpr);
      gl.uniform4f(u.uBox, detailBox[0] * rad, detailBox[1] * rad, detailBox[2] * rad, detailBox[3] * rad);
      gl.uniform1f(u.uDetailOn, detailOn ? 1 : 0);
      gl.uniform4f(u.uWaterBox, waterBox[0] * rad, waterBox[1] * rad, waterBox[2] * rad, waterBox[3] * rad);
      gl.uniform1f(u.uWaterOn, waterOn ? 1 : 0);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
    }

    // bounds: visible area in degrees, lonMin/lonMax unwrapped (may pass ±180).
    // pxPerDeg: device pixels per degree at the centre of the view.
    function updateDetail({ lonMin, lonMax, latMin, latMax, pxPerDeg }) {
      let z = Math.min(MAX_LEVEL, Math.ceil(Math.log2((pxPerDeg * 288) / TILE) - 0.25));
      for (; z >= MIN_DETAIL_LEVEL; z--) {
        const span = 288 / 2 ** z;
        const cols = Math.round(360 / span);
        const rows = Math.round(180 / span);
        const x0 = Math.floor((lonMin + 180) / span);
        const x1 = Math.floor((lonMax + 180) / span);
        const y0 = Math.max(0, Math.floor((90 - latMax) / span));
        const y1 = Math.min(rows - 1, Math.floor((90 - latMin) / span));
        const nx = Math.min(cols, x1 - x0 + 1);
        const ny = y1 - y0 + 1;
        if (nx * ny > MAX_TILES || nx * TILE > maxTex || ny * TILE > maxTex) continue;
        const key = [z, x0, y0, nx, ny].join("/");
        if (key === detailKey) return;
        detailKey = key;
        loadDetail(key, { z, span, cols, x0, y0, nx, ny });
        return;
      }
      // Zoomed out: the base texture is enough.
      detailKey = "";
      current = null;
      if (detailOn || waterOn) {
        detailOn = waterOn = false;
        onChange();
      }
    }

    const layer = (width, height) => {
      const c = document.createElement("canvas");
      c.width = width;
      c.height = height;
      return [c, c.getContext("2d")];
    };

    // Inland water from the stitched masks (both opaque where they say
    // water/land): MODIS water where OSM has land, with the land shrunk a
    // little first, as the two don't put coastlines in quite the same place
    // and the sliver would outline every coast. White on black, for the red
    // channel. Done on the whole stitched area so tile edges don't erode.
    function inlandWater(water, land) {
      const { width, height } = water;
      const [inner, i2] = layer(width, height);
      i2.drawImage(land, 0, 0);
      i2.globalCompositeOperation = "destination-in";
      for (const [dx, dy] of [[-COAST_PX, 0], [COAST_PX, 0], [0, -COAST_PX], [0, COAST_PX]]) i2.drawImage(land, dx, dy);
      const [out, o2] = layer(width, height);
      o2.drawImage(water, 0, 0);
      o2.globalCompositeOperation = "destination-in";
      o2.drawImage(inner, 0, 0);
      o2.globalCompositeOperation = "source-in";
      o2.fillStyle = "#fff";
      o2.fillRect(0, 0, width, height);
      o2.globalCompositeOperation = "destination-over";
      o2.fillStyle = "#000";
      o2.fillRect(0, 0, width, height);
      return out;
    }

    // ---------------------------------------------------------------- tiles

    // Recently used tiles, decoded off the main thread where the browser can,
    // shared by every update: panning back or a little only fetches what's new.
    const CACHE_MAX = 96; // about 1 MB each, decoded
    const cache = new Map(); // url -> Promise of a decoded image, or null if it failed
    const decoded = new Map(); // url -> decoded image, once ready

    function decode(url) {
      if (!window.createImageBitmap) return loadImage(url);
      return fetch(url, { mode: "cors" })
        .then((r) => (r.ok ? r.blob() : Promise.reject(new Error(r.status))))
        .then((blob) => createImageBitmap(blob));
    }

    function tile(url) {
      let p = cache.get(url);
      if (p) {
        cache.delete(url); // most recently used goes last
      } else {
        p = decode(url).then(
          (img) => {
            if (cache.get(url) !== p) {
              if (img.close) img.close(); // evicted while it was downloading
              return null;
            }
            decoded.set(url, img);
            return img;
          },
          () => null
        );
      }
      cache.set(url, p);
      while (cache.size > CACHE_MAX) {
        const old = cache.keys().next().value;
        const img = decoded.get(old);
        cache.delete(old);
        decoded.delete(old);
        if (img && img.close) img.close();
      }
      return p;
    }

    // ---------------------------------------------------------------- detail

    // The detail area shows straight away, filled from the base image plus
    // any tiles already cached, and sharpens as the rest arrive. Water masks
    // come one zoom level coarser (4x fewer tiles) and never hold up the
    // imagery; until they're in, the base water mask is used.
    let current = null;

    function loadDetail(key, { z, span, cols, x0, y0, nx, ny }) {
      const job = { key, dirty: false, waterReady: null, timer: 0 };
      current = job;

      const [imagery, c2] = layer(nx * TILE, ny * TILE);
      const box = [x0 * span - 180, 90 - y0 * span, nx * span, ny * span];
      const fromBase = (img, ctx, w, h, x, y, s, dx, dy, size) =>
        img && ctx.drawImage(img, (x * s * w) / 360, (y * s * h) / 180, (s * w) / 360, (s * h) / 180, dx, dy, size, size);

      let drawn = 0;
      const pending = [];
      for (let j = 0; j < ny; j++) {
        for (let i = 0; i < nx; i++) {
          const col = (((x0 + i) % cols) + cols) % cols;
          const url = `${GIBS}/${z}/${y0 + j}/${col}.jpeg`;
          tile(url); // touch, so this view's tiles stay cached
          const ready = decoded.get(url);
          if (ready) {
            c2.drawImage(ready, i * TILE, j * TILE);
            drawn++;
          } else {
            fromBase(baseImg, c2, baseImg && baseImg.width, baseImg && baseImg.height, col, y0 + j, span, i * TILE, j * TILE, TILE);
            pending.push([url, i, j]);
          }
        }
      }
      for (const [url, i, j] of pending) {
        tile(url).then((img) => {
          if (!img || current !== job) return;
          c2.drawImage(img, i * TILE, j * TILE);
          job.dirty = true;
          scheduleFlush(job, imagery, box);
        });
      }
      if (drawn) {
        job.dirty = true;
        flush(job, imagery, box);
      }

      // Water, one level coarser.
      const mz = z - 1;
      const mspan = span * 2;
      const mcols = Math.round(360 / mspan);
      const mx0 = Math.floor(x0 / 2);
      const my0 = Math.floor(y0 / 2);
      const mnx = Math.min(mcols, Math.floor((x0 + nx - 1) / 2) - mx0 + 1);
      const mny = Math.floor((y0 + ny - 1) / 2) - my0 + 1;
      const [water, w2] = layer(mnx * TILE, mny * TILE);
      const [land, l2] = layer(mnx * TILE, mny * TILE);
      const masks = [];
      for (let j = 0; j < mny; j++) {
        for (let i = 0; i < mnx; i++) {
          const col = (((mx0 + i) % mcols) + mcols) % mcols;
          const at = `${mz}/${my0 + j}/${col}.png`;
          masks.push(
            tile(`${WATER}/${at}`).then((img) => img && w2.drawImage(img, i * TILE, j * TILE)),
            tile(`${LAND}/${at}`).then((img) => img && l2.drawImage(img, i * TILE, j * TILE))
          );
        }
      }
      Promise.all(masks).then(() => {
        if (current !== job) return;
        job.waterReady = { canvas: inlandWater(water, land), box: [mx0 * mspan - 180, 90 - my0 * mspan, mnx * mspan, mny * mspan] };
        scheduleFlush(job, imagery, box);
      });
    }

    // Uploads are batched, so a burst of tiles costs one texture upload.
    function scheduleFlush(job, imagery, box) {
      if (job.timer) return;
      job.timer = setTimeout(() => {
        job.timer = 0;
        flush(job, imagery, box);
      }, 150);
    }

    function flush(job, imagery, box) {
      if (current !== job) return;
      if (job.dirty) {
        job.dirty = false;
        upload(1, detailTex, imagery, RGB);
        detailBox = box;
        detailOn = true;
      }
      if (job.waterReady) {
        upload(3, detailWaterTex, job.waterReady.canvas, MASK);
        waterBox = job.waterReady.box;
        waterOn = true;
        job.waterReady = null;
      }
      onChange();
    }

    return {
      render,
      loadBase,
      updateDetail,
      get ready() {
        return baseReady;
      },
    };
  };
})();
