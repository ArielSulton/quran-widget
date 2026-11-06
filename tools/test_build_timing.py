"""Unit tests for the pure offset-choice logic of tools/build_timing.py (no audio, no torch)."""
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent))
import build_timing as bt  # noqa: E402

MODES = {"long": 4200, "short": 50}


# ---- cluster_offsets -----------------------------------------------------------------
def test_cluster_single_linkage_gap_125():
    # 125 ms gaps are linked (<= 125), 126 ms are not
    assert bt.cluster_offsets([0, 125, 250]) == [[0, 125, 250]]
    assert bt.cluster_offsets([0, 126]) == [[0], [126]]


def test_cluster_sorts_and_splits():
    assert bt.cluster_offsets([4355, 4343, 4288, 4301, 3381]) == [[3381], [4288, 4301, 4343, 4355]]


def test_cluster_empty_is_error():
    with pytest.raises(ValueError):
        bt.cluster_offsets([])


# ---- choose_offset: real data points (build/offsets) ---------------------------------
def test_surah_80_unique_cluster_high():
    r = bt.choose_offset([4355, 4343, 4288, 4301, 3381], 189.093, MODES)
    # cluster {4288,4301,4343,4355}: median of the 4 = (4301+4343)/2 = 4322
    assert (r["offset_ms"], r["confidence"]) == (4322, "high")
    assert [c["size"] for c in r["clusters"]] == [1, 4]


def test_surah_85_even_median_floors():
    r = bt.choose_offset([4281, 4191, 4211, 4266, 3316], 132.144, MODES)
    # cluster {4191,4211,4266,4281}: (4211+4266)/2 = 4238.5 -> floor 4238; spread 90 <= 150
    assert (r["offset_ms"], r["confidence"]) == (4238, "high")


def test_surah_90_tie_goes_to_long_mode_and_is_low():
    r = bt.choose_offset([4150, 4172, 3167, 3223], 93.599, MODES)
    # {3167,3223} vs {4150,4172}: tied at size 2 -> closest to long mode (4200)
    assert (r["offset_ms"], r["confidence"]) == (4161, "low")


def test_surah_86_size2_cluster_beats_singleton_and_is_medium():
    # {3374,3383} is the only cluster with >= 2 members, so it wins over the lone 4348
    # even though 4348 is nearer the class mode: agreement beats a single probe.
    r = bt.choose_offset([4348, 3374, 3383], 76.115, MODES)
    assert (r["offset_ms"], r["confidence"]) == (3378, "medium")


def test_surah_113_two_singletons_use_short_mode_low():
    r = bt.choose_offset([76, 992], 21.362, MODES)
    assert (r["offset_ms"], r["confidence"]) == (76, "low")  # 76 is closer to 50 than 992


def test_surah_83_own_cluster_high_near_971():
    r = bt.choose_offset([1022, 956, 944, 1006, 971], 196.593, MODES)
    assert (r["offset_ms"], r["confidence"]) == (971, "high")


def test_surah_78_negative_offset_high():
    r = bt.choose_offset([-3126, -3122, -3082, -3092, -3085], 255.0, MODES)
    assert (r["offset_ms"], r["confidence"]) == (-3092, "high")


# ---- choose_offset: synthetic edge cases ---------------------------------------------
def test_empty_list_is_error():
    with pytest.raises(ValueError):
        bt.choose_offset([], 30.0, MODES)


def test_single_probe_is_low_and_uses_class_mode():
    r = bt.choose_offset([777], 30.0, MODES)
    assert (r["offset_ms"], r["confidence"]) == (777, "low")
    assert r["clusters"][0]["members"] == [777]


def test_two_agreeing_probes_medium_even_median():
    r = bt.choose_offset([100, 101], 30.0, MODES)
    assert (r["offset_ms"], r["confidence"]) == (100, "medium")  # (100+101)//2


def test_three_members_but_wide_spread_is_medium():
    # one cluster (gaps 100), size 3, spread 200 > 150
    r = bt.choose_offset([0, 100, 200], 100.0, MODES)
    assert (r["offset_ms"], r["confidence"]) == (100, "medium")


def test_three_members_tight_is_high():
    r = bt.choose_offset([10, 20, 30], 100.0, MODES)
    assert (r["offset_ms"], r["confidence"]) == (20, "high")


def test_negative_even_median_floors_toward_minus_infinity():
    r = bt.choose_offset([-101, -100], 100.0, MODES)
    assert r["offset_ms"] == -101  # floor(-100.5)


def test_class_boundary_45s():
    assert bt.track_class(44.999) == "short"
    assert bt.track_class(45.0) == "long"
    assert bt.choose_offset([0, 1000], 44.9, {"long": 1000, "short": 0})["offset_ms"] == 0
    assert bt.choose_offset([0, 1000], 45.0, {"long": 1000, "short": 0})["offset_ms"] == 1000


def test_tie_equidistant_picks_lower_offset():
    r = bt.choose_offset([0, 100_000], 10.0, {"long": 0, "short": 50_000})
    assert r["offset_ms"] == 0 and r["confidence"] == "low"


def test_ambiguous_without_class_mode_is_error():
    with pytest.raises(ValueError):
        bt.choose_offset([0, 1000], 10.0, {"long": 4200})


def test_largest_cluster_tie_restricted_to_tied_candidates():
    # two clusters of size 2 and a nearer singleton: the singleton must not win
    r = bt.choose_offset([0, 10, 5000, 5010, 2600], 100.0, {"long": 2600, "short": 0})
    assert r["confidence"] == "low"
    assert r["offset_ms"] in (5, 5005)


# ---- class_modes ---------------------------------------------------------------------
def test_class_modes_uses_only_high_and_medium():
    items = [
        ([4200, 4210, 4220], 100.0),        # long, high -> 4210
        ([4300, 4310], 60.0),               # long, medium -> 4305
        ([0, 4000], 80.0),                  # long, ambiguous -> ignored
        ([40, 50, 60], 20.0),               # short, high -> 50
        ([900, 910], 20.0),                 # short, medium -> 905
        ([1000], 20.0),                     # short, ambiguous -> ignored
    ]
    m = bt.class_modes(items)
    assert m == {"long": 4257, "short": 477}   # medians (4210,4305)->4257 ; (50,905)->477


def test_class_modes_missing_class_is_error():
    with pytest.raises(ValueError):
        bt.class_modes([([1, 2, 3], 100.0)])  # no short-class data


def test_class_modes_from_real_short_class_example():
    items = [([75, 57], 13.0), ([26, 46, 976], 27.053), ([4000, 4010], 100.0)]
    m = bt.class_modes(items)
    assert m["short"] == (66 + 36) // 2


# ---- crosscheck_counts / assemble (pure, synthetic cache) ------------------------------
def _fake_inputs_and_cache():
    kept = [n for n in bt.check_timing.SURAHS if n not in bt.check_timing.EXCLUDED]
    probes = {n: {"surah": n, "offsets_ms": [100, 110, 120], "track_s": 100.0} for n in kept}
    spotify = {n: 100_000 for n in range(78, 115)}
    spotify[101], spotify[104] = 80_100, 65_200
    spotify[112] = 13_000                       # the only short-class surah
    probes[112] = {"surah": 112, "offsets_ms": [60, 66, 72], "track_s": 13.0}
    cache = {n: {"starts_ms": [1000, 5000], "flags": [], "ref_s": 96.0} for n in kept}
    cache[112]["ref_s"] = 11.0
    cache[112]["flags"] = [{"ayah": 2, "kind": "ctc_fallback", "delta_ms": 0}]
    for n in bt.check_timing.EXCLUDED:
        cache[n] = {"ref_s": 49.2}
    return {"ayat": {}, "spotify_ms": spotify, "probes": probes}, cache


def test_assemble_outputs_only_timeline_keys_and_derived_fields():
    inp, cache = _fake_inputs_and_cache()
    timings, summary = bt.assemble(inp, cache)
    assert set(timings) == {n for n in range(78, 115)} - {101, 104}
    assert set(timings[112]) == {"surah", "spotify_duration_ms", "offset_ms", "ayah_starts_ms"}
    assert timings[112] == {"surah": 112, "spotify_duration_ms": 13_000, "offset_ms": 66,
                            "ayah_starts_ms": [1000, 5000]}
    e = summary["surahs"]["112"]
    assert e["D_s"] == 2.0 and e["trailing_silence_s"] == 1.934   # D - offset
    assert summary["class_modes"] == {"long": 110, "short": 66}
    assert timings[90]["offset_ms"] == 110
    assert e["flagged_boundaries"] == [{"ayah": 2, "kind": "ctc_fallback", "delta_ms": 0, "start_ms": 5000}]
    assert "crosscheck" not in e and "crosscheck" in summary["notes"]
    assert "skipped" in summary["notes"]["crosscheck"]
    assert set(summary["excluded"]) == {"101", "104"}
    assert summary["excluded"]["101"]["ref_s"] == 49.2


def test_assemble_refuses_missing_probe_file():
    inp, cache = _fake_inputs_and_cache()
    del inp["probes"][90]
    with pytest.raises(ValueError, match="90"):
        bt.assemble(inp, cache)
