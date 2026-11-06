#!/usr/bin/env python3
"""Hasilkan src-tauri/icons/source/simak-icon.svg (master 1024x1024, mandiri, tanpa <text>).

Huruf sin (U+0633) diambil dari font Amiri (SIL OFL 1.1) lalu diubah jadi path,
sehingga SVG tidak bergantung pada font apa pun saat dirender.

Pemakaian:  python3 -I scripts/gen-icon-svg.py /jalur/ke/Amiri-Bold.ttf > src-tauri/icons/source/simak-icon.svg
Lalu:       render ke PNG 1024x1024 dan jalankan `npx tauri icon <png>` (lihat README bagian ikon).
"""
import sys

from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont

SIZE = 1024
TILE = 824                    # grid ikon macOS: 100 px ruang di tiap sisi
INSET = (SIZE - TILE) // 2
RADIUS = round(TILE * 0.2237)  # sudut membulat ala macOS
GLYPH_W = 540                 # lebar huruf di kanvas


def glyph_path(font_path: str) -> str:
    font = TTFont(font_path)
    gs = font.getGlyphSet()
    name = font.getBestCmap()[0x0633]
    bp = BoundsPen(gs)
    gs[name].draw(bp)
    x0, y0, x1, y1 = bp.bounds
    k = GLYPH_W / (x1 - x0)
    cx, cy = SIZE / 2, SIZE / 2 + 6  # sedikit di bawah tengah agar terlihat seimbang
    # font: y ke atas; SVG: y ke bawah. Pusatkan kotak batas huruf di (cx, cy).
    tx = cx - k * (x0 + x1) / 2
    ty = cy + k * (y0 + y1) / 2
    pen = SVGPathPen(gs, ntos=lambda v: f"{v:.1f}".rstrip("0").rstrip("."))
    gs[name].draw(TransformPen(pen, (k, 0, 0, -k, tx, ty)))
    return pen.getCommands()


def main() -> None:
    d = glyph_path(sys.argv[1])
    print(f'''<svg xmlns="http://www.w3.org/2000/svg" width="{SIZE}" height="{SIZE}" viewBox="0 0 {SIZE} {SIZE}">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#0F5B4A"/>
      <stop offset="1" stop-color="#1FA88A"/>
    </linearGradient>
    <linearGradient id="gloss" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#FFFFFF" stop-opacity="0.11"/>
      <stop offset="1" stop-color="#FFFFFF" stop-opacity="0"/>
    </linearGradient>
    <linearGradient id="rim" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#FFFFFF" stop-opacity="0.45"/>
      <stop offset="0.5" stop-color="#FFFFFF" stop-opacity="0"/>
      <stop offset="1" stop-color="#000000" stop-opacity="0.18"/>
    </linearGradient>
    <clipPath id="tile"><rect x="{INSET}" y="{INSET}" width="{TILE}" height="{TILE}" rx="{RADIUS}"/></clipPath>
    <filter id="tile-shadow" x="-10%" y="-10%" width="120%" height="125%">
      <feGaussianBlur in="SourceAlpha" stdDeviation="14"/>
      <feOffset dy="12" result="b"/>
      <feFlood flood-color="#02201A" flood-opacity="0.38"/>
      <feComposite in2="b" operator="in"/>
    </filter>
    <filter id="glyph-shadow" x="-20%" y="-30%" width="140%" height="170%">
      <feGaussianBlur in="SourceAlpha" stdDeviation="12"/>
      <feOffset dy="14" result="b"/>
      <feFlood flood-color="#02201A" flood-opacity="0.35"/>
      <feComposite in2="b" operator="in"/>
    </filter>
  </defs>

  <!-- bayangan tile -->
  <rect x="{INSET}" y="{INSET}" width="{TILE}" height="{TILE}" rx="{RADIUS}" filter="url(#tile-shadow)"/>
  <!-- tile + gradien -->
  <rect x="{INSET}" y="{INSET}" width="{TILE}" height="{TILE}" rx="{RADIUS}" fill="url(#bg)"/>
  <!-- kilau kaca di bagian atas -->
  <g clip-path="url(#tile)">
    <rect x="{INSET}" y="{INSET}" width="{TILE}" height="{round(TILE * 0.45)}" fill="url(#gloss)"/>
  </g>
  <!-- tepi dalam halus -->
  <rect x="{INSET + 1.5}" y="{INSET + 1.5}" width="{TILE - 3}" height="{TILE - 3}" rx="{RADIUS - 1.5}" fill="none" stroke="url(#rim)" stroke-width="3"/>
  <!-- huruf sin (path hasil konversi Amiri Bold, OFL) -->
  <path d="{d}" fill="#000000" filter="url(#glyph-shadow)"/>
  <path d="{d}" fill="#FFFFFF"/>
</svg>''')


main()
