"""Builds ZapTap's base globe textures from NASA GIBS (public domain):

  ../earth.webp       Blue Marble shaded relief, 4096 x 2048
  ../earth-water.webp water, 4096 x 2048, lossless: red = inland water (lakes, rivers),
                      green = sea; the globe paints inland water and dark sea
                      the colour of the deep sea, and keeps bright shallows

Inland water is what the MODIS water mask calls water and the OpenStreetMap
land mask calls land: MODIS alone also marks the sea, which would paint over
shallow seas like the Bahamas banks. The land is shrunk a little first, as
the two masks don't draw coastlines in quite the same place and the sliver in
between would outline every coast. Sea is where OpenStreetMap has no land,
kept a pixel away from the coast so it doesn't eat into the coastline.

Both come from zoom level 3 of the GIBS EPSG:4326 grid (10 x 5 tiles of
512 px, exactly 360 x 180 degrees). Usage: python3 earth.py [<out-dir>]
"""

import io
import sys
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from PIL import Image, ImageChops, ImageFilter

GIBS = "https://gibs.earthdata.nasa.gov/wmts/epsg4326/best"
IMAGERY = GIBS + "/BlueMarble_ShadedRelief/default/500m/3/{y}/{x}.jpeg"
WATER = GIBS + "/MODIS_Water_Mask/default/250m/3/{y}/{x}.png"
LAND = GIBS + "/OSM_Land_Mask/default/250m/3/{y}/{x}.png"
COLS, ROWS, TILE = 10, 5, 512
SIZE = (4096, 2048)


def fetch(url):
    with urllib.request.urlopen(url, timeout=60) as r:
        return Image.open(io.BytesIO(r.read()))


def mosaic(template, mode, convert):
    out = Image.new(mode, (COLS * TILE, ROWS * TILE))
    jobs = [(x, y) for y in range(ROWS) for x in range(COLS)]
    with ThreadPoolExecutor(8) as pool:
        tiles = pool.map(lambda xy: fetch(template.format(x=xy[0], y=xy[1])), jobs)
        for (x, y), tile in zip(jobs, tiles):
            out.paste(convert(tile), (x * TILE, y * TILE))
    return out


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).parent.parent)
    imagery = mosaic(IMAGERY, "RGB", lambda t: t.convert("RGB"))
    imagery.resize(SIZE, Image.LANCZOS).save(out / "earth.webp", quality=80, method=6)
    # Each mask is its opaque part. Averaging when shrinking keeps rivers
    # thinner than a pixel as partial water instead of losing them.
    alpha = lambda t: t.convert("RGBA").getchannel("A")
    water = mosaic(WATER, "L", alpha)
    land = mosaic(LAND, "L", alpha)
    inland = ImageChops.multiply(water, land.filter(ImageFilter.MinFilter(5)))  # 2 px in from the coast
    sea = ImageChops.invert(land.filter(ImageFilter.MaxFilter(3)))  # 1 px out from the coast
    blank = Image.new("L", land.size, 0)
    rgb = Image.merge("RGB", (inland, sea, blank))
    rgb.resize(SIZE, Image.BOX).save(out / "earth-water.webp", lossless=True, quality=100, method=6)


if __name__ == "__main__":
    main()
