"""Build data/quran/juz30.json from Tanzil downloads (never from memory).

Usage:
  python3 -I tools/build_quran_data.py QURAN_UTHMANI_TXT TRANSLATION_ID_TXT OUT_JSON

Inputs are Tanzil 'txt-2' files (lines `surah|ayah|text`, '#' comment lines):
  quran-uthmani.txt  (https://tanzil.net/download/, Uthmani, Text with aya numbers, with pause marks and sajdah signs included: marks=true&sajdah=true)
  id.indonesian.txt  (https://tanzil.net/trans/id.indonesian, Kemenag RI)
Text is copied verbatim, except that the basmala Tanzil prepends to ayah 1 of each
surah (except 1 and 9) is removed, because it is not part of ayah 1.
"""
import json
import sys
from pathlib import Path

NAMES = {
    78: "An-Naba'", 79: "An-Nazi'at", 80: "'Abasa", 81: "At-Takwir", 82: "Al-Infitar",
    83: "Al-Mutaffifin", 84: "Al-Insyiqaq", 85: "Al-Buruj", 86: "At-Tariq", 87: "Al-A'la",
    88: "Al-Gasyiyah", 89: "Al-Fajr", 90: "Al-Balad", 91: "Asy-Syams", 92: "Al-Lail",
    93: "Ad-Duha", 94: "Al-Insyirah", 95: "At-Tin", 96: "Al-'Alaq", 97: "Al-Qadr",
    98: "Al-Bayyinah", 99: "Az-Zalzalah", 100: "Al-'Adiyat", 101: "Al-Qari'ah",
    102: "At-Takasur", 103: "Al-'Asr", 104: "Al-Humazah", 105: "Al-Fil", 106: "Quraisy",
    107: "Al-Ma'un", 108: "Al-Kausar", 109: "Al-Kafirun", 110: "An-Nasr", 111: "Al-Lahab",
    112: "Al-Ikhlas", 113: "Al-Falaq", 114: "An-Nas",
}


def parse(path: Path) -> dict:
    out = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line or line.startswith("#"):
            continue
        s, a, text = line.split("|", 2)
        out[(int(s), int(a))] = text
    return out


def strip_basmala(text: str, basmala: str) -> str | None:
    """Return text without a leading basmala, or None if it has none.

    Tanzil writes the basmala of surahs 95 and 97 with an extra shadda on the
    ba, so words are compared with U+0651 (shadda) ignored.
    """
    n = len(basmala.split())
    words = text.split(" ")
    if len(words) > n and norm(" ".join(words[:n])) == norm(basmala):
        return " ".join(words[n:])
    return None


def norm(t: str) -> str:
    return t.replace("\u0651", "")


def main() -> int:
    if len(sys.argv) != 4:
        print(__doc__)
        return 2
    ar_src, id_src, out_path = (Path(p) for p in sys.argv[1:])
    ar, idn = parse(ar_src), parse(id_src)
    basmala = ar[(1, 1)]
    data, stripped = {}, 0
    for s, name in NAMES.items():
        ayat, n = [], 1
        while (s, n) in ar:
            a_text = ar[(s, n)]
            if n == 1:
                rest = strip_basmala(a_text, basmala)
                if rest is not None:
                    a_text = rest
                    stripped += 1
            ayat.append({"ar": a_text, "id": idn[(s, n)]})
            n += 1
        data[str(s)] = {"name": name, "ayat": ayat}
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(data, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print(f"wrote {out_path}: {len(data)} surah, {sum(len(v['ayat']) for v in data.values())} ayat, basmala stripped from {stripped} surah")
    return 0


if __name__ == "__main__":
    sys.exit(main())
