"""Tests for the --flagged helpers of tools/spotcheck.py."""
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import spotcheck  # noqa: E402


def test_worst_flagged_is_first_fallbacks_by_ayah_plus_onsets_by_delta():
    flags = [
        {"ayah": 2, "kind": "onset", "delta_ms": -900},
        {"ayah": 9, "kind": "ctc_fallback", "delta_ms": 0},
        {"ayah": 4, "kind": "onset", "delta_ms": 1200},
        {"ayah": 5, "kind": "ctc_fallback", "delta_ms": 0},
        {"ayah": 6, "kind": "onset", "delta_ms": 400},
        {"ayah": 3, "kind": "ctc_fallback", "delta_ms": 0},
        {"ayah": 7, "kind": "ctc_fallback", "delta_ms": 0},
        {"ayah": 8, "kind": "onset", "delta_ms": 350},
    ]
    got = [(f["kind"], f["ayah"]) for f in spotcheck.worst_flagged(flags, 3)]
    # 3 lowest-numbered fallbacks (3, 5, 7), then the 3 biggest measured onset moves (4, 2, 6):
    # onset-delta boundaries are NOT crowded out by fallbacks.
    assert got == [("ctc_fallback", 3), ("ctc_fallback", 5), ("ctc_fallback", 7),
                   ("onset", 4), ("onset", 2), ("onset", 6)]
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


def test_plan_clips_flagged_plus_first_and_last():
    plan = spotcheck.plan_clips(FLAGS, STARTS, count=3)
    assert [(c["ayah"], c["start_ms"]) for c in plan] == [(1, 1500), (2, 5000), (3, 9000), (4, 14000), (5, 20000)]
    by = {c["ayah"]: c["reasons"] for c in plan}
    assert by[1] == ["first ayah"]
    assert by[2] == ["ctc_fallback"]
    assert by[3] == ["onset delta +350 ms"]
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


# ---- "Prioritas dengar" section --------------------------------------------------------------
def test_priority_section_is_at_the_top_with_order_and_tiers():
    prio = [{"surah": 78, "name": "An-Naba'", "ayah": 1, "start_ms": 132, "file": "78/a.wav",
             "note": "Track Spotify mulai di dalam rekaman referensi."},
            {"surah": 97, "name": "Al-Qadr", "ayah": 4, "start_ms": 18160, "file": "97/b.wav",
             "note": "Ayat 4 berdurasi 25 detik."}]
    md = spotcheck.render_index(ENTRIES, prio)
    assert md.index("## Prioritas dengar") < md.index("## Surah 112")
    assert md.index("78/a.wav") < md.index("97/b.wav") < md.index("## Surah 112")
    assert "Track Spotify mulai di dalam rekaman referensi." in md
    # plain statement of how the clips were chosen
    assert "tiga batas ctc_fallback pertama (nomor ayat terkecil)" in md
    assert "bukan yang terburuk" in md.lower()
    assert "ayat pertama dan terakhir" in md


def test_priority_notes_from_summary_unresolved_and_extra():
    summary = {"unresolved": [{"surah": 78, "issue": "Spotify starts 3.1 s into the reference"}],
               "surahs": {"078": {"offset_ms": -3092, "first_ayah_spotify_ms": -2960}}}
    entries = [{"surah": 78, "name": "An-Naba'", "clips": [
                    {"ayah": 1, "start_ms": 132, "reasons": ["first ayah"], "file": "78/f.wav"},
                    {"ayah": 40, "start_ms": 230938, "reasons": ["last ayah"], "file": "78/l.wav"}]},
               {"surah": 97, "name": "Al-Qadr", "clips": [
                    {"ayah": 1, "start_ms": 48, "reasons": ["first ayah"], "file": "97/f.wav"},
                    {"ayah": 5, "start_ms": 43282, "reasons": ["last ayah"], "file": "97/l.wav"}]}]
    starts = {97: [48, 7528, 13168, 18160, 43282]}
    prio = spotcheck.priority_clips(summary, entries, starts)
    assert [(p["surah"], p["ayah"], p["file"]) for p in prio] == [(78, 1, "78/f.wav"), (97, 5, "97/l.wav")]
    assert "-2960 ms" in prio[0]["note"] and "ayat 1" in prio[0]["note"]
    assert "Spotify starts 3.1 s into the reference" in prio[0]["note"]
    assert "ayat 4" in prio[1]["note"] and "25,1 detik" in prio[1]["note"]     # 43282-18160


# ---- outlier surahs (default offset ~1 s away from siblings of the same class) --------------
def _outlier_world():
    summary = {"unresolved": [],
               "class_modes": {"long": 4000, "short": 100},
               "surahs": {"086": {"offset_ms": 3000, "class": "long"},
                          "107": {"offset_ms": 1000, "class": "short"}}}
    entries = [{"surah": 86, "name": "At-Tariq", "clips": [
                   {"ayah": 1, "start_ms": 500, "reasons": ["first ayah"], "file": "86/f.wav"},
                   {"ayah": 17, "start_ms": 60000, "reasons": ["last ayah"], "file": "86/l.wav"}]},
               {"surah": 107, "name": "Al-Ma'un", "clips": [
                   {"ayah": 1, "start_ms": 900, "reasons": ["first ayah"], "file": "107/f.wav"},
                   {"ayah": 7, "start_ms": 20000, "reasons": ["last ayah"], "file": "107/l.wav"}]}]
    return summary, entries


def test_priority_lists_outlier_first_ayah_clips_with_numbers_from_summary(monkeypatch):
    monkeypatch.setattr(spotcheck, "OUTLIER_SURAHS", (86, 107))
    summary, entries = _outlier_world()
    prio = spotcheck.priority_clips(summary, entries, {})
    assert [(p["surah"], p["ayah"], p["file"]) for p in prio] == [(86, 1, "86/f.wav"), (107, 1, "107/f.wav")]
    assert "3000 ms" in prio[0]["note"] and "4000 ms" in prio[0]["note"] and "1000 ms" in prio[0]["note"]
    assert "long" in prio[0]["note"] and "geser sinkron" in prio[0]["note"]
    assert "1000 ms" in prio[1]["note"] and "100 ms" in prio[1]["note"] and "900 ms" in prio[1]["note"]
    assert "short" in prio[1]["note"]


def test_outliers_come_after_unresolved_and_last_ayah_entries(monkeypatch):
    monkeypatch.setattr(spotcheck, "OUTLIER_SURAHS", (86,))
    monkeypatch.setattr(spotcheck, "PRIORITY_LAST_AYAH", (97,))
    summary, entries = _outlier_world()
    summary["unresolved"] = [{"surah": 78, "issue": "x"}]
    summary["surahs"]["078"] = {"offset_ms": -3092, "first_ayah_spotify_ms": -2960}
    entries = [{"surah": 78, "name": "An-Naba'", "clips": [{"ayah": 1, "start_ms": 132, "reasons": [], "file": "78/f.wav"}]},
               {"surah": 97, "name": "Al-Qadr", "clips": [{"ayah": 5, "start_ms": 43282, "reasons": [], "file": "97/l.wav"}]},
               entries[0]]
    prio = spotcheck.priority_clips(summary, entries, {97: [48, 7528, 13168, 18160, 43282]})
    assert [p["surah"] for p in prio] == [78, 97, 86]


def test_outlier_surahs_exist_in_the_shipped_summary_and_are_off_their_class_mode():
    import json
    summary = json.loads((spotcheck.ROOT / "data" / "timing" / "_summary.json").read_text(encoding="utf-8"))
    assert spotcheck.OUTLIER_SURAHS == (86, 93, 100, 107)
    for n in spotcheck.OUTLIER_SURAHS:
        e = summary["surahs"][f"{n:03d}"]
        assert abs(e["offset_ms"] - summary["class_modes"][e["class"]]) > 700
