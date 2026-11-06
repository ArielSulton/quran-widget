# Bundled font: Amiri Quran

- Family: Amiri Quran Regular, Version 1.001 (`1.001;ALIF;AmiriQuran-Regular`), designed by Khaled Hosny (Alif Type).
- Copyright: Copyright 2010-2022 The Amiri Project Authors (https://github.com/aliftype/amiri).
- License: SIL Open Font License 1.1, full text in [OFL.txt](OFL.txt), also installed with the app as `licenses/AmiriQuran-OFL.txt` (copied from https://github.com/aliftype/amiri). The font declares no Reserved Font Name.
- File: `AmiriQuran-Regular.woff2`, SHA-256 `d2300497794c78746ad11ce3ead5c2c246cc9b0aeaaa35308288a3e211853035`.
- Derivation: the original `AmiriQuran.ttf` (133 376 bytes, SHA-256 `a406aa685f307d96327deecaae6d69322d28e30cd7ec8929badbee19a7e6c585`) was only re-wrapped as WOFF2 (`TTFont(src).flavor = "woff2"`, fontTools 4.60.1). Glyph and layout tables unchanged; only `head` metadata (checksum, modified time, WOFF2 flag) differs from the source TTF. Nothing was subset.
- Coverage check: `.venv/bin/python -I tools/check_font_coverage.py` (every code point of `data/quran/juz30.json` and the basmalah, plus GPOS `mark`/`mkmk`).
