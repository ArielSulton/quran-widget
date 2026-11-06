#!/usr/bin/env python3
"""Validate the shipped per-surah timing files in data/timing/.

    python3 -I tools/check_timing.py

Per ``NNN.json`` (the 4 keys of the Rust ``Timeline``): exactly those keys, ``surah`` equals
the file name, the ayah count equals data/quran/juz30.json, starts strictly increasing with
gaps >= 500 ms, last start + offset < spotify_duration, first start + offset >= 0 (ayah 1 may not
begin before the Spotify track does, except the documented ``FIRST_AYAH_BEFORE_TRACK``
exceptions, which must be exact), |offset| <= 10 s, spotify_duration equal to tests/fixtures/spotify_titles.json.  Exactly the surahs of Juz 30 minus ``EXCLUDED``
must be present, and ``_summary.json`` must agree with the files.  Prints ``OK`` and the
excluded surahs (to be reported to the user), or every problem found, exit 1.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TIMING_DIR = ROOT / "data" / "timing"
KEYS = {"surah", "spotify_duration_ms", "offset_ms", "ayah_starts_ms"}
SURAHS = range(78, 115)
MIN_GAP_MS = 500
MAX_ABS_OFFSET_MS = 10_000

# Surahs deliberately left out of v1 (the widget then shows the waiting state for them).
EXCLUDED = {
    101: "Spotify track is a different recording from the public reference (Spotify 80.1 s vs "
         "reference 49.2 s): the reference ayah times do not apply to it",
    104: "Spotify track is a different recording from the public reference (Spotify 65.2 s vs "
         "reference 41.5 s): the reference ayah times do not apply to it",
}

# Surahs whose first ayah starts BEFORE the Spotify track does (first start + offset < 0), i.e.
# the Spotify version begins inside the reference recording.  Exact: the check fails if a surah
# is listed but does not need it, and if any other surah violates the rule.  Each entry is an
# unresolved item that is reported in _summary.json ("unresolved").
FIRST_AYAH_BEFORE_TRACK = {
    78: "The Spotify track begins about 3.1 s into the reference recording (offset -3092 ms, five "
        "tight probes), so ayah 1 (reference 132 ms) is already in progress at Spotify position 0. "
        "UNRESOLVED: confirm by listening to build/spotcheck_flagged/78/s078_a01_at_132ms_first.wav "
        "and to the start of the Spotify track.",
}


def _is_int(x) -> bool:
    return isinstance(x, int) and not isinstance(x, bool)


def check_file(path: Path, ayah_count: int, spotify_ms: int,
               first_before_track: str | None = None) -> list[str]:
    name = path.name
    errs: list[str] = []
    try:
        d = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as e:
        return [f"{name}: unreadable: {e}"]
    if not isinstance(d, dict) or set(d) != KEYS:
        got = sorted(d) if isinstance(d, dict) else type(d).__name__
        return [f"{name}: keys must be exactly {sorted(KEYS)}, got {got}"]
    for k in KEYS:
        if k == "ayah_starts_ms":
            continue
        if not _is_int(d[k]):
            errs.append(f"{name}: {k} must be an integer, got {d[k]!r}")
    starts = d["ayah_starts_ms"]
    if not isinstance(starts, list) or not all(_is_int(s) for s in starts):
        return errs + [f"{name}: ayah_starts_ms must be a list of integers"]
    if errs:
        return errs
    if f"{d['surah']:03d}.json" != name:
        errs.append(f"{name}: surah {d['surah']} does not match the file name")
    if len(starts) != ayah_count:
        errs.append(f"{name}: {len(starts)} ayah starts, juz30.json has {ayah_count} ayat")
    if starts and starts[0] < 0:
        errs.append(f"{name}: first start {starts[0]} is negative")
    for i in range(1, len(starts)):
        gap = starts[i] - starts[i - 1]
        if gap <= 0:
            errs.append(f"{name}: ayah {i + 1} start {starts[i]} is not after {starts[i - 1]}")
        elif gap < MIN_GAP_MS:
            errs.append(f"{name}: ayah {i}->{i + 1} gap {gap} ms < {MIN_GAP_MS} ms")
    if starts:
        first_spotify = starts[0] + d["offset_ms"]
        if first_spotify < 0 and first_before_track is None:
            errs.append(f"{name}: first ayah starts at Spotify {first_spotify} ms, before the Spotify "
                        f"track starts (first start {starts[0]} + offset {d['offset_ms']} < 0); wrong "
                        f"offset sign? or add a documented FIRST_AYAH_BEFORE_TRACK exception")
        elif first_spotify >= 0 and first_before_track is not None:
            errs.append(f"{name}: FIRST_AYAH_BEFORE_TRACK exception is not needed "
                        f"(first ayah at Spotify {first_spotify} ms)")
    if abs(d["offset_ms"]) > MAX_ABS_OFFSET_MS:
        errs.append(f"{name}: |offset_ms| {d['offset_ms']} > {MAX_ABS_OFFSET_MS}")
    if starts and starts[-1] + d["offset_ms"] >= d["spotify_duration_ms"]:
        errs.append(f"{name}: last start {starts[-1]} + offset {d['offset_ms']} >= "
                    f"spotify_duration_ms {d['spotify_duration_ms']}")
    if d["spotify_duration_ms"] != spotify_ms:
        errs.append(f"{name}: spotify_duration_ms {d['spotify_duration_ms']} != fixture {spotify_ms}")
    return errs


def check(timing_dir: Path = TIMING_DIR, quran_path: Path | None = None,
          titles_path: Path | None = None, excluded: dict[int, str] | None = None,
          first_before_track: dict[int, str] | None = None) -> list[str]:
    excluded = EXCLUDED if excluded is None else excluded
    first_before_track = FIRST_AYAH_BEFORE_TRACK if first_before_track is None else first_before_track
    quran = json.loads((quran_path or ROOT / "data" / "quran" / "juz30.json").read_text(encoding="utf-8"))
    titles = json.loads((titles_path or ROOT / "tests" / "fixtures" / "spotify_titles.json")
                        .read_text(encoding="utf-8"))
    spotify = {t["surah"]: t["duration_ms"] for t in titles}
    errs: list[str] = []
    for n, reason in excluded.items():
        if n not in SURAHS or not reason.strip():
            errs.append(f"EXCLUDED[{n}] must be a Juz 30 surah with a non-empty reason")

    pat = re.compile(r"^\d{3}\.json$")
    found = {int(p.stem): p for p in sorted(timing_dir.glob("*.json")) if pat.match(p.name)}
    expected = {n for n in SURAHS if n not in excluded}
    for n in sorted(expected - set(found)):
        errs.append(f"{n:03d}.json is missing")
    for n in sorted(set(found) - expected):
        errs.append(f"{n:03d}.json present but surah {n} is "
                    f"{'excluded' if n in excluded else 'outside Juz 30'}")
    for n, reason in first_before_track.items():
        if n not in expected or n not in found or not reason.strip():
            errs.append(f"FIRST_AYAH_BEFORE_TRACK exception for surah {n}: must be a shipped surah "
                        f"with a non-empty reason")
    files: dict[int, dict] = {}
    for n in sorted(expected & set(found)):
        if str(n) not in quran or n not in spotify:
            errs.append(f"{n:03d}: no ayat in juz30.json or no duration in spotify_titles.json")
            continue
        e = check_file(found[n], len(quran[str(n)]["ayat"]), spotify[n], first_before_track.get(n))
        errs += e
        if not e:
            files[n] = json.loads(found[n].read_text(encoding="utf-8"))
    errs += check_summary(timing_dir / "_summary.json", files, excluded, first_before_track)
    return errs


def check_summary(path: Path, files: dict[int, dict], excluded: dict[int, str],
                  first_before_track: dict[int, str] | None = None) -> list[str]:
    first_before_track = FIRST_AYAH_BEFORE_TRACK if first_before_track is None else first_before_track
    if not path.exists():
        return ["_summary.json is missing"]
    try:
        s = json.loads(path.read_text(encoding="utf-8"))
    except ValueError as e:
        return [f"_summary.json: invalid JSON: {e}"]
    errs: list[str] = []
    ex = s.get("excluded")
    if not isinstance(ex, dict) or {int(k) for k in ex} != set(excluded):
        errs.append(f"_summary.json: 'excluded' must list exactly {sorted(excluded)}")
    per = s.get("surahs")
    if not isinstance(per, dict) or {int(k) for k in per} != set(files):
        return errs + ["_summary.json: 'surahs' must list exactly the shipped timing files"]
    if not isinstance(s.get("class_modes"), dict):
        errs.append("_summary.json: 'class_modes' missing")
    notes = s.get("notes")
    if not isinstance(notes, dict) or "skipped" not in str(notes.get("crosscheck", "")):
        errs.append("_summary.json: notes.crosscheck must state that the Whisper cross-check was skipped")
    unresolved = s.get("unresolved")
    expected_unresolved = [{"surah": n, "issue": first_before_track[n]}
                           for n in sorted(first_before_track) if n in files]
    if unresolved != expected_unresolved:
        errs.append(f"_summary.json: 'unresolved' must be exactly {expected_unresolved!r}, got {unresolved!r}")
    for n, d in files.items():
        e = per[f"{n:03d}"]
        if e.get("first_ayah_spotify_ms") != d["ayah_starts_ms"][0] + d["offset_ms"]:
            errs.append(f"_summary.json {n:03d}: first_ayah_spotify_ms {e.get('first_ayah_spotify_ms')} "
                        f"!= first start + offset {d['ayah_starts_ms'][0] + d['offset_ms']}")
        if e.get("offset_ms") != d["offset_ms"]:
            errs.append(f"_summary.json {n:03d}: offset_ms {e.get('offset_ms')} != file {d['offset_ms']}")
        if e.get("ayah_count") != len(d["ayah_starts_ms"]):
            errs.append(f"_summary.json {n:03d}: ayah_count disagrees with the file")
        if e.get("spotify_duration_ms") != d["spotify_duration_ms"]:
            errs.append(f"_summary.json {n:03d}: spotify_duration_ms disagrees with the file")
        if "crosscheck" in e:
            errs.append(f"_summary.json {n:03d}: unexpected 'crosscheck' counts (the check was skipped)")
        if e.get("confidence") not in ("high", "medium", "low"):
            errs.append(f"_summary.json {n:03d}: bad confidence {e.get('confidence')!r}")
        for f in e.get("flagged_boundaries", []):
            if not 1 <= f["ayah"] <= len(d["ayah_starts_ms"]) or \
                    f["start_ms"] != d["ayah_starts_ms"][f["ayah"] - 1]:
                errs.append(f"_summary.json {n:03d}: flagged boundary {f} does not match the file")
    return errs


def main() -> int:
    errs = check()
    if errs:
        print("FAIL")
        for e in errs:
            print("  " + e)
        return 1
    n = len([p for p in TIMING_DIR.glob("*.json") if re.match(r"^\d{3}\.json$", p.name)])
    print(f"OK: {n} timing files")
    print("unresolved (first ayah before the Spotify track starts; confirm by listening):")
    for k, v in sorted(FIRST_AYAH_BEFORE_TRACK.items()):
        print(f"  {k}: {v}")
    print("excluded from v1 (the widget shows the waiting state for these):")
    for k, v in sorted(EXCLUDED.items()):
        print(f"  {k}: {v}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
