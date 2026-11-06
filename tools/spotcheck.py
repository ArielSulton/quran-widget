#!/usr/bin/env python3
"""Export listening clips around random ayah boundaries for manual verification.

    python -I tools/spotcheck.py <surah> [--mp3 PATH] [--seed N] [--count 3]
    python -I tools/spotcheck.py --flagged [<surah>]


For each of ``count`` random ayah boundaries (ayah 2..N) a clip from 1 s before to 1 s
after the aligned boundary is written to build/spotcheck/<surah>/ (git-ignored, never
committed) with the boundary at exactly the 1.0 s mark of the clip.  The boundary times
are printed so a human can listen and verify that the cut point sits exactly where the
next ayah begins.

``--flagged`` instead reads data/timing/_summary.json and exports the +-1 s clips of up to
``--count`` (default 3) WORST low-confidence boundaries per surah (ctc_fallback first, then by
|delta_ms|) PLUS the first and last ayah start of every surah, into
build/spotcheck_flagged/<surah>/, using the shipped data/timing/NNN.json starts (no
re-alignment).  A full run also writes build/spotcheck_flagged/INDEX.md (what to listen for,
per clip) and play_all.sh (plays every clip in order with ffplay, label first).  The boundary is at exactly 1.000 s in every clip; for a boundary
earlier than 1 s the clip is padded with leading silence.
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
    """Write [boundary-1s, boundary+1s] to ``out``. Returns the actual boundary offset in s.

    A boundary earlier than ``margin_ms`` gets leading silence so it still sits at 1.000 s.
    """
    start_ms = max(0, boundary_ms - margin_ms)
    pad_ms = margin_ms - (boundary_ms - start_ms)
    cmd = ["ffmpeg", "-v", "error", "-nostdin", "-y", "-ss", f"{start_ms / 1000:.3f}",
           "-t", f"{(2 * margin_ms - pad_ms) / 1000:.3f}", "-i", align.ffmpeg_input(mp3), "-ac", "1"]
    if pad_ms > 0:
        cmd += ["-af", f"adelay={pad_ms}:all=1"]
    subprocess.run(cmd + ["-ar", "44100", out.as_posix()], check=True)
    return margin_ms / 1000


def worst_flagged(flags: list[dict], count: int = 3) -> list[dict]:
    """The ``count`` worst flagged boundaries: ctc_fallback first, then largest |delta_ms|."""
    return sorted(flags, key=lambda f: (f["kind"] != "ctc_fallback", -abs(f["delta_ms"]), f["ayah"]))[:count]


def _flag_reason(f: dict) -> str:
    return "ctc_fallback" if f["kind"] == "ctc_fallback" else f"{f['kind']} delta {f['delta_ms']:+d} ms"


def plan_clips(flags: list[dict], starts: list[int], count: int = 3) -> list[dict]:
    """Clips to export for one surah, ordered by ayah: the ``count`` worst flagged boundaries
    plus the FIRST ayah start and the LAST ayah start (even when unflagged), deduplicated."""
    by_ayah: dict[int, list[str]] = {}
    for f in worst_flagged(flags, count):
        by_ayah.setdefault(f["ayah"], []).append(_flag_reason(f))
    by_ayah.setdefault(1, []).insert(0, "first ayah")
    by_ayah.setdefault(len(starts), []).append("last ayah")
    return [{"ayah": a, "start_ms": starts[a - 1], "reasons": by_ayah[a]} for a in sorted(by_ayah)]


def fmt_time(ms: int) -> str:
    return f"{ms // 60000:02d}:{ms % 60000 // 1000:02d}.{ms % 1000:03d}"


def _label(surah: int, name: str, c: dict) -> str:
    return (f"Surah {surah} {name} - ayat {c['ayah']} mulai @ {fmt_time(c['start_ms'])} "
            f"({', '.join(c['reasons'])}) - batas di detik 1.0 klip")


def render_index(entries: list[dict]) -> str:
    out = ["# Klip pemeriksaan batas ayat (rekaman referensi publik)", "",
           "Tiap klip 2 detik: batas ayat berada tepat di detik 1.0 (1 detik sebelum, 1 detik sesudah).",
           "Dengarkan apakah ayat berikutnya mulai persis di tengah klip.",
           "`first ayah` / `last ayah` = awal ayat pertama / awal ayat terakhir (selalu diekspor); "
           "`ctc_fallback` / `onset delta` = batas bertanda ragu dari aligner.", ""]
    for e in entries:
        out += [f"## Surah {e['surah']} - {e['name']}", "",
                "| Ayat | Waktu di rekaman referensi | Alasan | Berkas |", "|---|---|---|---|"]
        for c in e["clips"]:
            out.append(f"| ayat {c['ayah']} mulai | {fmt_time(c['start_ms'])} | "
                       f"{', '.join(c['reasons'])} | `{c['file']}` |")
        out.append("")
    return "\n".join(out)


def _sh(s: str) -> str:
    return "'" + s.replace("'", "'\\''") + "'"


def render_play_all(entries: list[dict]) -> str:
    lines = ["#!/usr/bin/env bash", "# Memutar semua klip berurutan; label dicetak sebelum tiap klip.",
             'cd "$(dirname "$0")" || exit 1', ""]
    for e in entries:
        for c in e["clips"]:
            lines.append(f"echo {_sh(_label(e['surah'], e['name'], c))}")
            lines.append(f"ffplay -nodisp -autoexit -loglevel error {_sh(c['file'])}")
    return "\n".join(lines) + "\n"


def export_flagged(only_surah: int | None, count: int) -> int:
    summary = json.loads((ROOT / "data" / "timing" / "_summary.json").read_text(encoding="utf-8"))
    names = json.loads((ROOT / "data" / "quran" / "juz30.json").read_text(encoding="utf-8"))
    root_out = ROOT / "build" / "spotcheck_flagged"
    entries, total = [], 0
    for key, e in sorted(summary["surahs"].items()):
        n = int(key)
        if only_surah is not None and n != only_surah:
            continue
        starts = json.loads((ROOT / "data" / "timing" / f"{n:03d}.json").read_text(encoding="utf-8"))["ayah_starts_ms"]
        plan = plan_clips(e["flagged_boundaries"], starts, count)
        mp3 = align.ensure_mp3(n)
        outdir = root_out / str(n)
        outdir.mkdir(parents=True, exist_ok=True)
        for old in outdir.glob("*.wav"):
            old.unlink()
        for c in plan:
            slug = "_".join(r.split()[0] for r in c["reasons"]).replace("ctc_fallback", "ctc")
            out = outdir / f"s{n:03d}_a{c['ayah']:02d}_at_{c['start_ms']}ms_{slug}.wav"
            off = export_clip(mp3, c["start_ms"], out)
            c["file"] = out.relative_to(root_out).as_posix()
            total += 1
            print(f"surah {n}: ayah {c['ayah']} start {c['start_ms']} ms ({', '.join(c['reasons'])}); "
                  f"in clip at {off:.3f} s -> {out.relative_to(ROOT)}")
        entries.append({"surah": n, "name": names[str(n)]["name"], "clips": plan})
    if only_surah is None:  # index/script cover the whole juz only on a full run
        (root_out / "INDEX.md").write_text(render_index(entries), encoding="utf-8")
        sh = root_out / "play_all.sh"
        sh.write_text(render_play_all(entries), encoding="utf-8")
        sh.chmod(0o755)
    print(f"{total} clip(s) written")
    return 0


def main(argv=None) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("surah", type=int, nargs="?")
    ap.add_argument("--flagged", action="store_true",
                    help="export the worst low-confidence boundaries listed in data/timing/_summary.json")
    ap.add_argument("--mp3", default=None)
    ap.add_argument("--seed", type=int, default=None)
    ap.add_argument("--count", type=int, default=3)
    a = ap.parse_args(argv)
    if a.flagged:
        return export_flagged(a.surah, a.count)
    if a.surah is None:
        ap.error("surah is required unless --flagged is given")

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
