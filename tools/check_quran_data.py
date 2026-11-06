"""Validate data/quran/juz30.json (shape QuranData from src/viewModel.ts).

Usage: python3 -I tools/check_quran_data.py [path/to/juz30.json]
"""
import json
import sys
import unicodedata
from pathlib import Path

EXPECTED = {78: 40, 79: 46, 80: 42, 81: 29, 82: 19, 83: 36, 84: 25, 85: 22, 86: 17,
            87: 19, 88: 26, 89: 30, 90: 20, 91: 15, 92: 21, 93: 11, 94: 8, 95: 8,
            96: 19, 97: 5, 98: 8, 99: 8, 100: 11, 101: 11, 102: 8, 103: 3, 104: 9,
            105: 5, 106: 4, 107: 7, 108: 3, 109: 6, 110: 3, 111: 5, 112: 4, 113: 5,
            114: 6}
BASMALA = "بِسْمِ ٱللَّهِ ٱلرَّحْمَٰنِ ٱلرَّحِيمِ"  # compared ignoring shadda (U+0651)
DEFAULT = Path(__file__).resolve().parent.parent / "data" / "quran" / "juz30.json"


def has_tashkeel(text: str) -> bool:
    return any(unicodedata.category(c) == "Mn" for c in text)


def main() -> int:
    path = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as e:
        print(f"FAIL cannot load {path}: {e}")
        return 1
    errors = []
    if sorted(data.keys(), key=int) != [str(n) for n in sorted(EXPECTED)] or len(data) != 37:
        errors.append(f"expected exactly 37 surah keys 78..114, got {len(data)}")
    total = 0
    for num, count in EXPECTED.items():
        s = data.get(str(num))
        if s is None:
            continue
        if not isinstance(s.get("name"), str) or not s["name"].strip():
            errors.append(f"surah {num}: empty name")
        ayat = s.get("ayat", [])
        if len(ayat) != count:
            errors.append(f"surah {num}: expected {count} ayat, got {len(ayat)}")
        for i, a in enumerate(ayat, 1):
            ar, idn = a.get("ar"), a.get("id")
            if not isinstance(ar, str) or not ar.strip():
                errors.append(f"surah {num}:{i}: empty ar")
            elif not has_tashkeel(ar):
                errors.append(f"surah {num}:{i}: ar has no tashkeel")
            if not isinstance(idn, str) or not idn.strip():
                errors.append(f"surah {num}:{i}: empty id")
        if ayat and ayat[0].get("ar", "").replace("\u0651", "").startswith(BASMALA.replace("\u0651", "")):
            errors.append(f"surah {num}:1 still starts with basmala")
        total += len(ayat)
    if errors:
        print("FAIL")
        for e in errors[:30]:
            print(" -", e)
        return 1
    print(f"OK {len(data)} surah, {total} ayat")
    return 0


if __name__ == "__main__":
    sys.exit(main())
