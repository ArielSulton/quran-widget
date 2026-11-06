#!/usr/bin/env python3
"""Export listening clips around random ayah boundaries for manual verification.

    python -I tools/spotcheck.py <surah> [--mp3 PATH] [--seed N] [--count 3]

For each of ``count`` random ayah boundaries (ayah 2..N) a clip from 1 s before to 1 s
after the aligned boundary is written to build/spotcheck/<surah>/ (git-ignored, never
committed) with the boundary at exactly the 1.0 s mark of the clip.  The boundary times
are printed so a human can listen and verify that the cut point sits exactly where the
next ayah begins.
"""
from __future__ import annotations

import argparse
import json
import random
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import align  # noqa: E402

ROOT = align.ROOT


def export_clip(mp3: str, boundary_ms: int, out: Path, margin_ms: int = 1000) -> float:
    """Write [boundary-1s, boundary+1s] to ``out``. Returns the actual boundary offset in s."""
    start_ms = max(0, boundary_ms - margin_ms)
    offset = (boundary_ms - start_ms) / 1000
    subprocess.run(
        ["ffmpeg", "-v", "error", "-nostdin", "-y", "-ss", f"{start_ms / 1000:.3f}",
         "-t", f"{2 * margin_ms / 1000:.3f}", "-i", align.ffmpeg_input(mp3), "-ac", "1", "-ar", "44100", out.as_posix()],
        check=True,
    )
    return offset


def main(argv=None) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("surah", type=int)
    ap.add_argument("--mp3", default=None)
    ap.add_argument("--seed", type=int, default=None)
    ap.add_argument("--count", type=int, default=3)
    a = ap.parse_args(argv)

    data = json.loads((ROOT / "data" / "quran" / "juz30.json").read_text(encoding="utf-8"))
    ayat = [x["ar"] for x in data[str(a.surah)]["ayat"]]
    mp3 = a.mp3 or align.ensure_mp3(a.surah)
    starts = align.align_surah(mp3, ayat)

    candidates = list(range(1, len(starts)))  # boundaries *into* ayah 2..N
    if not candidates:
        print("surah has a single ayah: nothing to spot-check", file=sys.stderr)
        return 0
    rng = random.Random(a.seed)
    picks = sorted(rng.sample(candidates, min(a.count, len(candidates))))

    outdir = ROOT / "build" / "spotcheck" / str(a.surah)
    outdir.mkdir(parents=True, exist_ok=True)
    for old in outdir.glob("*.wav"):
        old.unlink()
    print(f"surah {a.surah}: ayah starts (ms) = {starts}")
    for i in picks:
        b = starts[i]
        out = outdir / f"s{a.surah:03d}_a{i}-to-a{i + 1}_at_{b}ms.wav"
        off = export_clip(mp3, b, out)
        print(f"  boundary ayah {i} -> ayah {i + 1} at {b} ms ({b / 1000:.3f} s); "
              f"in clip at {off:.3f} s -> {out.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
