#!/usr/bin/env python3
"""
Generate tray icon assets from Font Awesome brain SVG (CC BY 4.0).

Setup (project-local venv):
    python3 -m venv .venv
    .venv/bin/pip install cairosvg pillow

Usage:
    curl -sL https://raw.githubusercontent.com/FortAwesome/Font-Awesome/6.x/svgs/solid/brain.svg -o /tmp/brain.svg
    .venv/bin/python scripts/gen_icons.py /tmp/brain.svg

Run once; commit the generated PNGs. Do not re-run unless icon design changes.
"""
import io
import sys

import cairosvg
from PIL import Image, ImageDraw

SIZE = 32


def svg_to_image(svg_path: str, tint: tuple) -> Image.Image:
    png = cairosvg.svg2png(url=svg_path, output_width=SIZE, output_height=SIZE)
    img = Image.open(io.BytesIO(png)).convert("RGBA")
    pixels = img.load()
    for y in range(img.height):
        for x in range(img.width):
            _, _, _, a = pixels[x, y]
            if a > 0:
                pixels[x, y] = (*tint, a)
    return img


def add_dot(img: Image.Image, color: tuple) -> Image.Image:
    out = img.copy()
    draw = ImageDraw.Draw(out)
    r = 5
    draw.ellipse([SIZE - r * 2, SIZE - r * 2, SIZE, SIZE], fill=(*color, 255))
    return out


if len(sys.argv) != 2:
    print("Usage: python scripts/gen_icons.py <brain.svg>")
    sys.exit(1)

svg = sys.argv[1]

normal = svg_to_image(svg, (255, 255, 255))
normal.save("icons/brain_normal.png")

alert = add_dot(normal, (220, 50, 50))
alert.save("icons/brain_alert.png")

unavailable = add_dot(normal, (160, 160, 160))
unavailable.save("icons/brain_unavailable.png")

print("Generated: icons/brain_normal.png  icons/brain_alert.png  icons/brain_unavailable.png")
