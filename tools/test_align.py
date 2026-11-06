"""Tests for tools/align.py. Downloads are cached in the per-user cache dir (ensure_mp3)."""
import functools
import json
import shutil
import subprocess
from pathlib import Path

import numpy as np
import pytest

import align

ROOT = Path(__file__).resolve().parent.parent
DATA = json.loads((ROOT / "data" / "quran" / "juz30.json").read_text(encoding="utf-8"))

# Known-good boundaries (ms) from the reviewed run, pinned with +/-100 ms tolerance.
PINS = {
    112: [60, 2538, 4682, 7218],
    108: [100, 5298, 8033],
    114: [60, 6328, 11128, 14988, 22698, 29092],
}


def ayat_ar(surah: int) -> list[str]:
    return [a["ar"] for a in DATA[str(surah)]["ayat"]]


def duration_ms(path: str) -> float:
    out = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0",
         align.ffmpeg_input(path)],
        check=True, capture_output=True, text=True,
    ).stdout.strip()
    return float(out) * 1000


@functools.lru_cache(maxsize=None)
def detail(surah: int) -> dict:
    return align.align_surah_detail(align.ensure_mp3(surah), ayat_ar(surah))


@pytest.fixture(scope="session")
def mp3_112() -> str:
    return align.ensure_mp3(112)


# ---- brief tests ------------------------------------------------------------------


def test_al_ikhlas_starts_are_valid(mp3_112):
    starts = align.align_surah(mp3_112, ayat_ar(112))
    assert len(starts) == 4
    assert all(isinstance(s, int) for s in starts)
    assert starts[0] >= 0
    assert all(b > a for a, b in zip(starts, starts[1:])), starts
    assert starts[-1] < duration_ms(mp3_112)


@pytest.mark.parametrize("surah,count", [(108, 3), (114, 6)])
def test_value_count_matches_ayat(surah, count):
    starts = detail(surah)["starts_ms"]
    assert len(starts) == count == len(DATA[str(surah)]["ayat"])
    assert all(b > a for a, b in zip(starts, starts[1:])), starts


# ---- regression pins --------------------------------------------------------------


@pytest.mark.parametrize("surah", sorted(PINS))
def test_known_boundaries_within_100ms(surah):
    starts = detail(surah)["starts_ms"]
    assert len(starts) == len(PINS[surah])
    for i, (got, want) in enumerate(zip(starts, PINS[surah]), start=1):
        assert abs(got - want) <= 100, f"surah {surah} ayah {i}: {got} vs pinned {want}"


@pytest.mark.parametrize("surah", sorted(PINS))
def test_detail_shape_and_flags(surah):
    d = detail(surah)
    assert set(d) == {"starts_ms", "flags"}
    for f in d["flags"]:
        assert set(f) == {"ayah", "kind", "delta_ms"}
        assert f["kind"] in ("onset", "ctc_fallback")
        assert 1 <= f["ayah"] <= len(d["starts_ms"])
        assert isinstance(f["delta_ms"], int)


# ---- validator --------------------------------------------------------------------


def test_validate_accepts_good_list():
    align.validate_starts([0, 10, 2000], 5000.0)


@pytest.mark.parametrize(
    "starts,dur,needle",
    [
        ([10, 10, 20], 1000.0, "ayah 2"),  # equal => not strictly increasing
        ([10, 30, 20], 1000.0, "ayah 3"),  # decreasing
        ([-1, 5], 1000.0, "ayah 1"),  # negative
        ([5, 1000], 1000.0, "ayah 2"),  # == duration
        ([5, 2000], 1000.0, "ayah 2"),  # beyond duration
        ([5, 999, 1500, 1600], 1000.0, "ayah 3"),  # two beyond: no silent clamp to dupes
    ],
)
def test_validate_rejects_bad_lists(starts, dur, needle):
    with pytest.raises(align.AlignmentError) as e:
        align.validate_starts(starts, dur, label="surah 99")
    msg = str(e.value)
    assert needle in msg and "surah 99" in msg


# ---- preamble guard / romanisation guard ------------------------------------------


def test_preamble_guard_raises_when_first_ayah_late():
    align.check_no_preamble(2500)  # boundary value is allowed
    with pytest.raises(align.AlignmentError, match="preamble suspected"):
        align.check_no_preamble(2501)


def test_empty_romanisation_raises_instead_of_borrowing_previous_ayah():
    with pytest.raises(ValueError, match="ayah 2"):
        align.align_frames(None, ["قُلْ", "َُ"])


# ---- refine_onset (synthetic energy envelope) -------------------------------------


def make_edb(duration_s, pauses, speech=-15.0, pause=-40.0):
    n = int(duration_s * align.SAMPLE_RATE / align.HOP)
    edb = np.full(n, speech)
    for a, b in pauses:
        for i in range(n):
            if a <= align._t(i) <= b:
                edb[i] = pause
    return edb


def test_refine_snaps_to_nearby_pause_end():
    edb = make_edb(3.0, [(1.0, 1.4)])
    t, kind = align.refine_onset(edb, 1.2, 0.0)  # CTC estimate inside the pause
    assert kind == "onset" and abs(t - 1.4) < 0.05


def test_refine_ignores_pause_beyond_window():
    edb = make_edb(3.0, [(1.4, 1.9)])  # pause ends 0.9 s after the estimate
    t, kind = align.refine_onset(edb, 1.0, 0.0)
    assert kind == "ctc_fallback" and t == 1.0


def test_refine_never_moves_more_than_window():
    edb = make_edb(4.0, [(0.5, 0.7), (2.0, 2.5)])
    for est in np.arange(0.4, 3.0, 0.1):
        t, kind = align.refine_onset(edb, float(est), 0.0)
        assert abs(t - est) <= 0.5 + 1e-6, (est, t, kind)


def test_refine_first_ayah_leading_silence():
    edb = make_edb(3.0, [(0.0, 0.3)])
    t, kind = align.refine_onset(edb, 0.1, 0.0, first=True)
    assert kind == "onset" and abs(t - 0.3) < 0.05


def test_refine_first_ayah_does_not_snap_to_word_gap():
    edb = make_edb(3.0, [(0.8, 0.95)])  # intra-ayah gap
    t, kind = align.refine_onset(edb, 0.05, 0.0, first=True)
    assert abs(t - 0.05) < 0.1


# ---- file handling / CLI ----------------------------------------------------------


def test_probe_rejects_html_error_page(tmp_path):
    bad = tmp_path / "x.mp3"
    bad.write_text("<html>" + "Not Found " * 500 + "</html>")
    with pytest.raises(align.AlignmentError):
        align.probe_audio(str(bad))


def test_ensure_mp3_does_not_trust_html_download(tmp_path, monkeypatch):
    def fake_download(url, dest):
        Path(dest).write_text("<html>" + "oops " * 600 + "</html>")

    monkeypatch.setattr(align, "_download", fake_download)
    with pytest.raises(align.AlignmentError):
        align.ensure_mp3(112, dest_root=tmp_path)
    assert not (tmp_path / "112" / "112.mp3").exists()


def test_ensure_mp3_replaces_cached_html(tmp_path, monkeypatch, mp3_112):
    d = tmp_path / "112"
    d.mkdir()
    (d / "112.mp3").write_text("<html>" + "oops " * 600 + "</html>")
    monkeypatch.setattr(align, "_download", lambda url, dest: shutil.copy(mp3_112, dest))
    p = align.ensure_mp3(112, dest_root=tmp_path)
    assert align.probe_audio(p) > 5


def test_default_cache_is_per_user_not_tmp():
    assert not str(align.REF_DIR).startswith("/tmp")
    assert "quran-widget" in str(align.REF_DIR)


@pytest.mark.parametrize("bad", ["http://example.com/a.mp3", "file:/etc/passwd", "concat:a|b"])
def test_protocol_like_paths_rejected(bad):
    with pytest.raises(ValueError):
        align.ffmpeg_input(bad)


def test_missing_or_non_regular_path_rejected(tmp_path):
    with pytest.raises(FileNotFoundError):
        align.ffmpeg_input(str(tmp_path / "nope.mp3"))
    with pytest.raises(FileNotFoundError):
        align.ffmpeg_input(str(tmp_path))  # a directory


def test_ffmpeg_input_uses_file_prefix(mp3_112):
    assert align.ffmpeg_input(mp3_112) == "file:" + str(Path(mp3_112).resolve())


@pytest.mark.parametrize("bad", ["50", "77", "115", "0"])
def test_cli_rejects_surah_outside_78_114(bad):
    with pytest.raises(SystemExit) as e:
        align.main([bad])
    assert e.value.code == 2


def test_cli_json_includes_flags(capsys, mp3_112):
    assert align.main(["112", "--mp3", mp3_112]) == 0
    out = json.loads(capsys.readouterr().out)
    assert out["surah"] == 112 and len(out["ayah_starts_ms"]) == 4
    assert isinstance(out["flags"], list)
