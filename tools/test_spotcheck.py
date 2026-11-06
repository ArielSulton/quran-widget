"""Tests for the --flagged helpers of tools/spotcheck.py."""
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import spotcheck  # noqa: E402


def test_worst_flagged_orders_fallback_first_then_delta():
    flags = [
        {"ayah": 2, "kind": "onset", "delta_ms": -900},
        {"ayah": 3, "kind": "ctc_fallback", "delta_ms": 0},
        {"ayah": 4, "kind": "onset", "delta_ms": 1200},
        {"ayah": 5, "kind": "ctc_fallback", "delta_ms": 40},
        {"ayah": 6, "kind": "onset", "delta_ms": 400},
    ]
    got = [f["ayah"] for f in spotcheck.worst_flagged(flags, 3)]
    assert got == [5, 3, 4]
    assert spotcheck.worst_flagged([], 3) == []


def _wav_duration(p: Path) -> float:
    out = subprocess.run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of",
                          "default=nw=1:nk=1", str(p)], capture_output=True, text=True, check=True)
    return float(out.stdout)


def test_export_clip_is_2s_with_boundary_at_1s_even_near_start(tmp_path):
    src = tmp_path / "tone.wav"
    subprocess.run(["ffmpeg", "-v", "error", "-nostdin", "-y", "-f", "lavfi", "-i",
                    "sine=frequency=440:duration=5", str(src)], check=True)
    for boundary in (3000, 400):  # mid-file and earlier than the 1 s margin
        out = tmp_path / f"c{boundary}.wav"
        assert spotcheck.export_clip(str(src), boundary, out) == 1.0
        assert abs(_wav_duration(out) - 2.0) < 0.02


# ---- plan_clips: flagged + first/last, deduplicated --------------------------------------
STARTS = [1500, 5000, 9000, 14000, 20000]
FLAGS = [{"ayah": 3, "kind": "onset", "delta_ms": 350}, {"ayah": 5, "kind": "ctc_fallback", "delta_ms": 0},
         {"ayah": 2, "kind": "ctc_fallback", "delta_ms": 0}, {"ayah": 4, "kind": "onset", "delta_ms": -900}]


def test_plan_clips_worst_three_flagged_plus_first_and_last():
    plan = spotcheck.plan_clips(FLAGS, STARTS, count=3)
    assert [(c["ayah"], c["start_ms"]) for c in plan] == [(1, 1500), (2, 5000), (4, 14000), (5, 20000)]
    by = {c["ayah"]: c["reasons"] for c in plan}
    assert by[1] == ["first ayah"]
    assert by[2] == ["ctc_fallback"]
    assert by[4] == ["onset delta -900 ms"]
    assert by[5] == ["ctc_fallback", "last ayah"]          # flagged AND last: one clip, both reasons


def test_plan_clips_unflagged_surah_still_has_two_clips():
    plan = spotcheck.plan_clips([], [800, 4000, 9000], count=3)
    assert [(c["ayah"], c["reasons"]) for c in plan] == [(1, ["first ayah"]), (3, ["last ayah"])]


def test_plan_clips_single_ayah_surah_is_one_clip():
    plan = spotcheck.plan_clips([], [800], count=3)
    assert [(c["ayah"], c["reasons"]) for c in plan] == [(1, ["first ayah", "last ayah"])]


# ---- INDEX.md and play_all.sh ----------------------------------------------------------------
ENTRIES = [{"surah": 112, "name": "Al-Ikhlas", "clips": [
    {"ayah": 1, "start_ms": 1500, "reasons": ["first ayah"], "file": "112/s112_a1.wav"},
    {"ayah": 4, "start_ms": 83123, "reasons": ["ctc_fallback", "last ayah"], "file": "112/s112_a4.wav"}]}]


def test_index_lists_name_ayah_time_reason_file():
    md = spotcheck.render_index(ENTRIES)
    assert "112" in md and "Al-Ikhlas" in md
    assert "ayat 4 mulai" in md and "01:23.123" in md
    assert "ctc_fallback, last ayah" in md and "112/s112_a4.wav" in md
    assert "ayat 1 mulai" in md and "00:01.500" in md


def test_play_all_script_plays_in_order_with_labels_and_is_safe_to_quote():
    sh = spotcheck.render_play_all(ENTRIES)
    assert sh.startswith("#!/usr/bin/env bash")
    assert sh.count("ffplay -nodisp -autoexit") == 2
    assert sh.index("112/s112_a1.wav") < sh.index("112/s112_a4.wav")
    assert 'cd "$(dirname "$0")"' in sh
    assert "ayat 4 mulai" in sh
    odd = [{"surah": 1, "name": "It's", "clips": [{"ayah": 2, "start_ms": 5, "reasons": ["x"], "file": "1/a b.wav"}]}]
    assert "Surah 1 It'\\''s - ayat 2" in spotcheck.render_play_all(odd)
    assert "'1/a b.wav'" in spotcheck.render_play_all(odd)


def test_format_ref_time():
    assert spotcheck.fmt_time(0) == "00:00.000"
    assert spotcheck.fmt_time(83123) == "01:23.123"
