#!/usr/bin/env python3
"""Build data/timing/NNN.json (+ _summary.json) for Juz 30 from the reference alignment.

    python -I tools/build_timing.py [--threads N] [--shard K/N] [--refresh]

Pipeline per non-excluded surah (see tools/check_timing.py:EXCLUDED):
  1. ayah starts in the PUBLIC REFERENCE recording: tools/align.py ``align_surah_detail``
     (kept: its ``flags`` = low-confidence boundaries);
  2. (Whisper cross-check: intentionally SKIPPED by user decision; the boundaries are verified by
     listening to the clips of ``tools/spotcheck.py --flagged``; _summary.json says so.)
  3. offset = (time in Spotify) - (time in reference), chosen from the per-probe offsets in
     build/offsets/NNN.json (read-only input) by ``choose_offset`` below;
  4. spotify_duration_ms from tests/fixtures/spotify_titles.json.

Outputs (deterministic): data/timing/NNN.json with ONLY the 4 Timeline keys, and
data/timing/_summary.json (confidence, clusters, probe offsets, flagged boundaries,
track/ref durations, class modes, excluded surahs and reasons).

The slow part (alignment) is cached in build/timing_cache/NNN.json (git-ignored,
keyed on the mp3 size, the ayat text and ``ALIGN_CACHE_VERSION``); --refresh recomputes it.
NOTE: even a run that is 100% cache hits calls ``align.ensure_mp3`` for every surah (also the
excluded ones, for their reference duration), so a default run may DOWNLOAD missing reference
mp3s into the per-user cache and needs them present otherwise.  --shard K/N computes only
every N-th surah (to run several processes side by side) and writes no output.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import check_timing  # noqa: E402  (stdlib only)

ROOT = check_timing.ROOT
TIMING_DIR = check_timing.TIMING_DIR
OFFSETS_DIR = ROOT / "build" / "offsets"
CACHE_DIR = ROOT / "build" / "timing_cache"
LONG_TRACK_S = 45.0       # Spotify track >= 45 s is class "long"
# Bump when the logic or thresholds of tools/align.py change (MMS_FA alignment, onset snapping,
# flag rules): cache entries made by an older aligner are then recomputed instead of reused.
ALIGN_CACHE_VERSION = 1
CLUSTER_GAP_MS = 125      # single-linkage gap for offset clustering
HIGH_MIN_SIZE = 3
HIGH_MAX_SPREAD_MS = 150
CLASSES = ("long", "short")
CROSSCHECK_NOTE = ("Whisper cross-check intentionally skipped (user decision); boundaries are verified "
                   "by listening to the clips from tools/spotcheck.py --flagged")


# ---------------------------------------------------------------------------------
# Offset choice (pure)
# ---------------------------------------------------------------------------------
def _median(sorted_values: list[int]) -> int:
    """Median of a sorted int list; for an even count the floor of the mean of the middle two."""
    n = len(sorted_values)
    if n % 2:
        return sorted_values[n // 2]
    return (sorted_values[n // 2 - 1] + sorted_values[n // 2]) // 2


def cluster_offsets(offsets_ms: list[int], gap_ms: int = CLUSTER_GAP_MS) -> list[list[int]]:
    """Single-linkage clusters (consecutive sorted values at most ``gap_ms`` apart)."""
    if not offsets_ms:
        raise ValueError("no probe offsets")
    values = sorted(offsets_ms)
    clusters = [[values[0]]]
    for v in values[1:]:
        if v - clusters[-1][-1] <= gap_ms:
            clusters[-1].append(v)
        else:
            clusters.append([v])
    return clusters


def track_class(track_s: float) -> str:
    return "long" if track_s >= LONG_TRACK_S else "short"


def _unique_choice(clusters: list[list[int]]):
    """(offset, confidence) if there is a unique largest cluster with >= 2 members, else None."""
    biggest = max(len(c) for c in clusters)
    top = [c for c in clusters if len(c) == biggest]
    if biggest < 2 or len(top) != 1:
        return None
    c = top[0]
    high = len(c) >= HIGH_MIN_SIZE and c[-1] - c[0] <= HIGH_MAX_SPREAD_MS
    return _median(c), "high" if high else "medium"


def choose_offset(offsets_ms: list[int], track_s: float, class_modes: dict[str, int]) -> dict:
    """Pick the offset for one surah from its per-probe offsets (see the module docstring).

    A unique largest cluster with >= 2 members wins (median of that cluster; ``high`` when it
    has >= 3 members and spread <= 150 ms, else ``medium``).  A cluster of two agreeing probes
    beats a lone probe even if the lone one is nearer the class mode.  Otherwise (tie for the
    largest, or only singletons) the tied-largest cluster whose median is closest to the class
    mode (ties: the lower offset) wins and the confidence is ``low``.
    """
    clusters = cluster_offsets(offsets_ms)
    info = [{"median_ms": _median(c), "size": len(c), "members": c} for c in clusters]
    chosen = _unique_choice(clusters)
    if chosen is None:
        cls = track_class(track_s)
        if cls not in class_modes:
            raise ValueError(f"ambiguous offsets {sorted(offsets_ms)} and no class mode for {cls!r}")
        mode = class_modes[cls]
        biggest = max(len(c) for c in clusters)
        cands = [_median(c) for c in clusters if len(c) == biggest]
        chosen = (min(cands, key=lambda m: (abs(m - mode), m)), "low")
    return {"offset_ms": chosen[0], "confidence": chosen[1], "clusters": info}


def class_modes(items: list[tuple[list[int], float]]) -> dict[str, int]:
    """Per class (long/short): median of the offsets of the high/medium surahs of that class.

    ``items`` = [(probe_offsets_ms, track_s), ...].  Ambiguous (low) surahs do not contribute.
    """
    pools: dict[str, list[int]] = {c: [] for c in CLASSES}
    for offsets, track_s in items:
        chosen = _unique_choice(cluster_offsets(offsets))
        if chosen is not None:
            pools[track_class(track_s)].append(chosen[0])
    missing = [c for c in CLASSES if not pools[c]]
    if missing:
        raise ValueError(f"no high/medium offsets to compute the class mode of {missing}")
    return {c: _median(sorted(pools[c])) for c in CLASSES}


# ---------------------------------------------------------------------------------
# Inputs
# ---------------------------------------------------------------------------------
def load_inputs() -> dict:
    quran = json.loads((ROOT / "data" / "quran" / "juz30.json").read_text(encoding="utf-8"))
    titles = json.loads((ROOT / "tests" / "fixtures" / "spotify_titles.json").read_text(encoding="utf-8"))
    spotify = {t["surah"]: t["duration_ms"] for t in titles}
    ayat = {n: [a["ar"] for a in quran[str(n)]["ayat"]] for n in check_timing.SURAHS}
    probes = {}
    for n in check_timing.SURAHS:
        p = OFFSETS_DIR / f"{n:03d}.json"
        if p.exists():
            d = json.loads(p.read_text(encoding="utf-8"))
            if d["surah"] != n:
                raise ValueError(f"{p}: surah {d['surah']} != {n}")
            probes[n] = d
    return {"ayat": ayat, "spotify_ms": spotify, "probes": probes}


# ---------------------------------------------------------------------------------
# Slow part: alignment, cached
# ---------------------------------------------------------------------------------
def _cache_key(mp3: str, ayat: list[str]) -> dict:
    return {"align_version": ALIGN_CACHE_VERSION, "mp3_size": os.path.getsize(mp3),
            "ayat_sha256": hashlib.sha256("\n".join(ayat).encode("utf-8")).hexdigest()}


def compute_surah(n: int, ayat: list[str], refresh: bool, threads: int) -> dict:
    import align
    import torch

    mp3 = align.ensure_mp3(n)
    key = _cache_key(mp3, ayat)
    path = CACHE_DIR / f"{n:03d}.json"
    if path.exists() and not refresh:
        c = json.loads(path.read_text(encoding="utf-8"))
        if c.get("key") == key:
            return c
    align._model()                  # loads MMS_FA and applies align's own thread setting first
    torch.set_num_threads(threads)  # then cap it so parallel shards do not oversubscribe
    res = align.align_surah_detail(mp3, ayat)          # raises AlignmentError on bad data
    c = {"key": key, "starts_ms": res["starts_ms"], "flags": res["flags"],
         "ref_s": round(align.probe_audio(mp3), 3)}
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(c) + "\n", encoding="utf-8")
    return c


# ---------------------------------------------------------------------------------
# Assembly (pure given the cache)
# ---------------------------------------------------------------------------------
def assemble(inp: dict, cache: dict[int, dict]) -> tuple[dict[int, dict], dict]:
    kept = [n for n in check_timing.SURAHS if n not in check_timing.EXCLUDED]
    missing_probe = [n for n in kept if n not in inp["probes"]]
    if missing_probe:
        raise ValueError(f"no build/offsets file for non-excluded surahs {missing_probe}")
    items = [(inp["probes"][n]["offsets_ms"], inp["spotify_ms"][n] / 1000) for n in kept]
    modes = class_modes(items)
    timings, per = {}, {}
    for n in kept:
        probes = inp["probes"][n]["offsets_ms"]
        sp_ms = inp["spotify_ms"][n]
        ch = choose_offset(probes, sp_ms / 1000, modes)
        c = cache[n]
        starts = c["starts_ms"]
        timings[n] = {"surah": n, "spotify_duration_ms": sp_ms, "offset_ms": ch["offset_ms"],
                      "ayah_starts_ms": starts}
        d_s = round(sp_ms / 1000 - c["ref_s"], 3)
        per[f"{n:03d}"] = {
            "offset_ms": ch["offset_ms"], "first_ayah_spotify_ms": starts[0] + ch["offset_ms"],
            "confidence": ch["confidence"],
            "class": track_class(sp_ms / 1000), "clusters": ch["clusters"],
            "probe_offsets_ms": probes, "ayah_count": len(starts),
            "spotify_duration_ms": sp_ms, "track_s": inp["probes"][n]["track_s"],
            "ref_s": c["ref_s"], "D_s": d_s,
            "trailing_silence_s": round(d_s - ch["offset_ms"] / 1000, 3),
            "flagged_boundaries": [{**f, "start_ms": starts[f["ayah"] - 1]} for f in c["flags"]],
        }
    excluded = {}
    for n, reason in check_timing.EXCLUDED.items():
        excluded[f"{n:03d}"] = {"reason": reason, "spotify_s": inp["spotify_ms"][n] / 1000,
                                "ref_s": cache[n]["ref_s"] if n in cache else None}
    unresolved = [{"surah": n, "issue": check_timing.FIRST_AYAH_BEFORE_TRACK[n]}
                  for n in sorted(check_timing.FIRST_AYAH_BEFORE_TRACK) if n in timings]
    summary = {"notes": {"crosscheck": CROSSCHECK_NOTE}, "unresolved": unresolved,
               "class_modes": modes, "excluded": excluded, "surahs": per}
    return timings, summary


def write_outputs(timings: dict[int, dict], summary: dict) -> None:
    TIMING_DIR.mkdir(parents=True, exist_ok=True)
    for p in TIMING_DIR.glob("[0-9][0-9][0-9].json"):
        if int(p.stem) not in timings:
            p.unlink()  # stale file for an excluded surah
    for n, t in timings.items():
        (TIMING_DIR / f"{n:03d}.json").write_text(
            json.dumps(t, separators=(",", ":")) + "\n", encoding="utf-8")
    (TIMING_DIR / "_summary.json").write_text(json.dumps(summary, indent=1) + "\n", encoding="utf-8")


def ref_duration_only(n: int) -> dict:
    """Cache-like record with just the reference duration (for excluded surahs)."""
    import align
    return {"ref_s": round(align.probe_audio(align.ensure_mp3(n)), 3)}


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--threads", type=int, default=6, help="torch threads for the alignment (default 6)")
    ap.add_argument("--shard", default=None, metavar="K/N",
                    help="compute only every N-th surah starting at K (0-based); write nothing")
    ap.add_argument("--refresh", action="store_true", help="ignore build/timing_cache")
    a = ap.parse_args(argv)
    inp = load_inputs()
    kept = [n for n in check_timing.SURAHS if n not in check_timing.EXCLUDED]
    todo = kept
    if a.shard:
        k, m = (int(x) for x in a.shard.split("/"))
        todo = [n for i, n in enumerate(kept) if i % m == k]
    cache, failures = {}, {}
    for n in todo:
        try:
            cache[n] = compute_surah(n, inp["ayat"][n], a.refresh, a.threads)
            print(f"surah {n}: {len(cache[n]['starts_ms'])} ayat, {len(cache[n]['flags'])} flagged", flush=True)
        except Exception as e:  # report every failure; nothing is clamped or skipped silently
            failures[n] = f"{type(e).__name__}: {e}"
            print(f"surah {n}: FAILED {failures[n]}", file=sys.stderr, flush=True)
    if failures:
        print(f"{len(failures)} surah(s) failed: {sorted(failures)}", file=sys.stderr)
        return 1
    if a.shard:
        return 0
    for n in kept:
        if n not in cache:
            cache[n] = compute_surah(n, inp["ayat"][n], False, a.threads)
    for n in check_timing.EXCLUDED:
        cache[n] = ref_duration_only(n)
    timings, summary = assemble(inp, cache)
    write_outputs(timings, summary)
    print(f"wrote {len(timings)} timing files + _summary.json to {TIMING_DIR}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
