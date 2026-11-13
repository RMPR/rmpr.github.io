"""Builds ZapTap's base globe textures from NASA GIBS (public domain):

  ../earth.jpg        Blue Marble shaded relief, 4096 x 2048
  ../earth-water.png  MODIS water mask (lakes, rivers, sea), 4096 x 2048, grey
                      (255 = water); the globe paints it the colour of the sea

Both come from zoom level 3 of the GIBS EPSG:4326 grid (10 x 5 tiles of
512 px, exactly 360 x 180 degrees). Usage: python3 earth.py [<out-dir>]
"""

import io
import sys
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from PIL import Image

GIBS = "https://gibs.earthdata.nasa.gov/wmts/epsg4326/best"
IMAGERY = GIBS + "/BlueMarble_ShadedRelief/default/500m/3/{y}/{x}.jpeg"
WATER = GIBS + "/MODIS_Water_Mask/default/250m/3/{y}/{x}.png"
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
    imagery.resize(SIZE, Image.LANCZOS).save(out / "earth.jpg", quality=84, optimize=True, progressive=True)
    # Water is the opaque part of the mask. Averaging when shrinking keeps
    # rivers thinner than a pixel as partial water instead of losing them.
    water = mosaic(WATER, "L", lambda t: t.convert("RGBA").getchannel("A"))
    water.resize(SIZE, Image.BOX).save(out / "earth-water.png", optimize=True)


if __name__ == "__main__":
    main()
