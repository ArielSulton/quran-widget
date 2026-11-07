#!/usr/bin/env python3
"""Verify that the bundled Arabic font can render everything the widget displays.

Checks (exit status 1 and one line per problem when any fails):
  * every code point of the Arabic text in data/quran/juz30.json, plus the basmalah shown
    by src/viewModel.ts, is in the font's cmap (no tofu boxes);
  * the font has GPOS `mark` and `mkmk` features (Quranic marks are positioned, not just present);
  * the name table identifies Amiri Quran under the SIL Open Font License.

Usage: python tools/check_font_coverage.py [FONT] [QURAN_JSON]
Needs fontTools (+ brotli for .woff2): .venv/bin/pip install fonttools brotli
"""
import json
import re
import sys
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_FONT = ROOT / "src" / "assets" / "fonts" / "AmiriQuran-Regular.woff2"
DEFAULT_DATA = ROOT / "data" / "quran" / "juz30.json"
VIEW_MODEL = ROOT / "src" / "viewModel.ts"
REQUIRED_FEATURES = ("mark", "mkmk")


def _basmalah() -> str:
    """The basmalah string from src/viewModel.ts (so the check follows the app)."""
    m = re.search(r'ar:\s*"([^"]+)"', VIEW_MODEL.read_text(encoding="utf-8"))
    if not m:
        raise RuntimeError(f"basmalah not found in {VIEW_MODEL}")
    return m.group(1)


def _end_of_ayah_chars() -> str:
    """Characters the UI adds around each ayah (src/main.ts): NBSP, U+06DD and the Arabic-Indic digits of src/viewModel.ts."""
    src = VIEW_MODEL.read_text(encoding="utf-8")
    digits = re.search(r'ARABIC_INDIC_DIGITS\s*=\s*"([^"]+)"', src)
    if not digits:
        raise RuntimeError(f"ARABIC_INDIC_DIGITS not found in {VIEW_MODEL}")
    return "\u00a0\u06dd" + digits.group(1)


def used_codepoints(data_path: Path = DEFAULT_DATA) -> set[int]:
    data = json.loads(Path(data_path).read_text(encoding="utf-8"))
    text = _basmalah() + "".join(a["ar"] for s in data.values() for a in s["ayat"])
    text += _end_of_ayah_chars()
    return {ord(c) for c in text}


def _open(font_path: Path):
    from fontTools.ttLib import TTFont

    return TTFont(str(font_path))


def _name(font, name_id: int) -> str:
    rec = font["name"].getName(name_id, 3, 1, 0x409)
    return rec.toUnicode() if rec else ""


def font_identity(font_path: Path) -> dict:
    font = _open(font_path)
    return {
        "family": _name(font, 1),
        "version": _name(font, 5),
        "copyright": _name(font, 0),
        "license": _name(font, 13),
    }


def check_font(font_path: Path, codepoints: set[int], require_identity: bool = True) -> list[str]:
    font = _open(font_path)
    problems = []
    cmap = font.getBestCmap()
    for cp in sorted(codepoints - set(cmap)):
        problems.append(f"missing U+{cp:04X} {unicodedata.name(chr(cp), '?')}")

    gpos = font["GPOS"].table if "GPOS" in font else None
    tags = {r.FeatureTag for r in gpos.FeatureList.FeatureRecord} if gpos else set()
    for feature in REQUIRED_FEATURES:
        if feature not in tags:
            problems.append(f"font has no GPOS '{feature}' feature (marks would not be positioned)")

    if require_identity:
        info = font_identity(font_path)
        if info["family"] != "Amiri Quran":
            problems.append(f"family is {info['family']!r}, expected 'Amiri Quran'")
        if "SIL Open Font License" not in info["license"]:
            problems.append(f"license is {info['license']!r}, expected the SIL Open Font License")
    return problems


def main(argv: list[str]) -> int:
    font = Path(argv[1]) if len(argv) > 1 else DEFAULT_FONT
    data = Path(argv[2]) if len(argv) > 2 else DEFAULT_DATA
    cps = used_codepoints(data)
    problems = check_font(font, cps)
    for p in problems:
        print(f"FAIL {p}")
    if problems:
        return 1
    info = font_identity(font)
    print(f"OK {info['family']} {info['version']}: {len(cps)} distinct code points covered, "
          f"GPOS mark+mkmk present, {info['license'][:48]}...")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
