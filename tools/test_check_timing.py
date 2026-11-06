"""Tests for the per-file rules of tools/check_timing.py (synthetic files, no audio)."""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import check_timing as ct  # noqa: E402


def _write(tmp_path: Path, obj, name="112.json") -> Path:
    p = tmp_path / name
    p.write_text(json.dumps(obj), encoding="utf-8")
    return p


def good():
    return {"surah": 112, "spotify_duration_ms": 13000, "offset_ms": 66,
            "ayah_starts_ms": [1000, 4000, 7000, 10000]}


def test_good_file_passes(tmp_path):
    assert ct.check_file(_write(tmp_path, good()), 4, 13000) == []


def test_extra_or_missing_key_fails(tmp_path):
    d = good(); d["flags"] = []
    assert "keys must be exactly" in ct.check_file(_write(tmp_path, d), 4, 13000)[0]
    d = good(); del d["offset_ms"]
    assert "keys must be exactly" in ct.check_file(_write(tmp_path, d), 4, 13000)[0]


def test_wrong_filename_ayah_count_and_duration_fail(tmp_path):
    p = _write(tmp_path, good(), "113.json")
    assert any("does not match the file name" in e for e in ct.check_file(p, 4, 13000))
    assert any("juz30.json has 5" in e for e in ct.check_file(_write(tmp_path, good()), 5, 13000))
    assert any("!= fixture" in e for e in ct.check_file(_write(tmp_path, good()), 4, 14000))


def test_not_increasing_and_small_gap_fail(tmp_path):
    d = good(); d["ayah_starts_ms"] = [1000, 1000, 7000, 10000]
    assert any("is not after" in e for e in ct.check_file(_write(tmp_path, d), 4, 13000))
    d["ayah_starts_ms"] = [1000, 1499, 7000, 10000]
    assert any("gap 499 ms" in e for e in ct.check_file(_write(tmp_path, d), 4, 13000))
    d["ayah_starts_ms"] = [1000, 1500, 7000, 10000]
    assert ct.check_file(_write(tmp_path, d), 4, 13000) == []


def test_last_start_plus_offset_must_be_inside_track(tmp_path):
    d = good(); d["offset_ms"] = 3000
    assert any("last start" in e for e in ct.check_file(_write(tmp_path, d), 4, 13000))
    d["offset_ms"] = 2999
    assert ct.check_file(_write(tmp_path, d), 4, 13000) == []


def test_offset_bound(tmp_path):
    d = good(); d["offset_ms"] = -10001
    assert any("|offset_ms|" in e for e in ct.check_file(_write(tmp_path, d), 4, 13000))


def test_non_integer_values_fail(tmp_path):
    d = good(); d["offset_ms"] = 1.5
    assert any("must be an integer" in e for e in ct.check_file(_write(tmp_path, d), 4, 13000))
    d = good(); d["ayah_starts_ms"] = [1000, "x", 3, 4]
    assert any("list of integers" in e for e in ct.check_file(_write(tmp_path, d), 4, 13000))


def test_shipped_excluded_set_is_exactly_101_and_104_with_reasons():
    assert set(ct.EXCLUDED) == {101, 104}
    assert all(len(r) > 20 for r in ct.EXCLUDED.values())


# ---- whole-directory check() and _summary.json consistency -------------------------------
def _mini_world(tmp_path, monkeypatch):
    """Two-surah world: surah 112 shipped, surah 113 excluded."""
    quran = tmp_path / "q.json"
    quran.write_text(json.dumps({"112": {"ayat": [{}] * 4}, "113": {"ayat": [{}] * 5}}), encoding="utf-8")
    titles = tmp_path / "t.json"
    titles.write_text(json.dumps([{"surah": 112, "title": "x", "duration_ms": 13000},
                                  {"surah": 113, "title": "y", "duration_ms": 21000}]), encoding="utf-8")
    monkeypatch.setattr(ct, "SURAHS", range(112, 114))
    tdir = tmp_path / "timing"
    tdir.mkdir()
    _write(tdir, good())
    summary = {"notes": {"crosscheck": "intentionally skipped"}, "class_modes": {"long": 1, "short": 2}, "excluded": {"113": {"reason": "r"}},
               "surahs": {"112": {"offset_ms": 66, "confidence": "high", "ayah_count": 4,
                                  "spotify_duration_ms": 13000,
                                  "flagged_boundaries": [{"ayah": 2, "start_ms": 4000}]}}}
    (tdir / "_summary.json").write_text(json.dumps(summary), encoding="utf-8")
    return tdir, quran, titles, {113: "different recording of enough length"}


def test_check_dir_good_world_passes_and_summary_not_treated_as_timing(tmp_path, monkeypatch):
    tdir, quran, titles, ex = _mini_world(tmp_path, monkeypatch)
    assert ct.check(tdir, quran, titles, ex) == []


def test_check_dir_reports_missing_extra_and_excluded_present(tmp_path, monkeypatch):
    tdir, quran, titles, ex = _mini_world(tmp_path, monkeypatch)
    (tdir / "112.json").unlink()
    errs = ct.check(tdir, quran, titles, ex)
    assert any("112.json is missing" in e for e in errs)
    _write(tdir, {**good(), "surah": 113}, "113.json")
    assert any("113.json present but surah 113 is excluded" in e for e in ct.check(tdir, quran, titles, ex))


def test_check_dir_summary_must_agree_with_files(tmp_path, monkeypatch):
    tdir, quran, titles, ex = _mini_world(tmp_path, monkeypatch)
    s = json.loads((tdir / "_summary.json").read_text())
    s["surahs"]["112"]["offset_ms"] = 67
    s["surahs"]["112"]["flagged_boundaries"] = [{"ayah": 2, "start_ms": 4001}]
    (tdir / "_summary.json").write_text(json.dumps(s))
    errs = ct.check(tdir, quran, titles, ex)
    assert any("offset_ms 67" in e for e in errs)
    assert any("flagged boundary" in e for e in errs)
    (tdir / "_summary.json").unlink()
    assert ct.check(tdir, quran, titles, ex) == ["_summary.json is missing"]


def test_check_dir_summary_must_state_crosscheck_skipped_and_carry_no_crosscheck_counts(tmp_path, monkeypatch):
    tdir, quran, titles, ex = _mini_world(tmp_path, monkeypatch)
    s = json.loads((tdir / "_summary.json").read_text())
    s["surahs"]["112"]["crosscheck"] = {"ok": 3}
    (tdir / "_summary.json").write_text(json.dumps(s))
    assert any("crosscheck" in e and "112" in e for e in ct.check(tdir, quran, titles, ex))
    del s["surahs"]["112"]["crosscheck"], s["notes"]
    (tdir / "_summary.json").write_text(json.dumps(s))
    assert any("notes.crosscheck" in e for e in ct.check(tdir, quran, titles, ex))
