"""Tests for tools/crosscheck.py (independent Whisper cross-check of ayah start times).

Everything except the ``slow`` test is offline and works on synthetic word lists.
"""
import json
from pathlib import Path
from types import SimpleNamespace

import pytest

import crosscheck as cc

ROOT = Path(__file__).resolve().parent.parent
DATA = json.loads((ROOT / "data" / "quran" / "juz30.json").read_text(encoding="utf-8"))


def ayat_ar(surah: int) -> list[str]:
    return [a["ar"] for a in DATA[str(surah)]["ayat"]]


# Surah 112, normalised: قل هو الله احد | الله الصمد | لم يلد ولم يولد | ولم يكن له كفوا احد
AYAT_112 = ayat_ar(112)


def W(word: str, start: int, end: int | None = None) -> dict:
    return {"word": word, "start_ms": start, "end_ms": start + 300 if end is None else end}


def clean_112() -> list[dict]:
    """A perfect Whisper transcription of 112, starts 60/2538/4682/7218 ms."""
    return [
        W("قل", 60), W("هو", 400), W("الله", 800), W("أحد", 1500),
        W("الله", 2538), W("الصمد", 3000),
        W("لم", 4682), W("يلد", 5000), W("ولم", 5600), W("يولد", 6000),
        W("ولم", 7218), W("يكن", 7600), W("له", 8000), W("كفوا", 8300), W("أحد", 8900),
    ]


STARTS_112 = [60, 2538, 4682, 7218]


# ----------------------------------------------------------------------------------
# normalisation
# ----------------------------------------------------------------------------------


def test_normalize_strips_diacritics_and_unifies_alef():
    assert cc.normalize_word("ٱللَّهُ") == "الله"
    assert cc.normalize_word("أَحَدٌ") == "احد"
    assert cc.normalize_word("إِيَّاكَ") == "اياك"
    assert cc.normalize_word("آمَنُوا") == "امنوا"


def test_normalize_unifies_yaa_taa_marbuta_waw_hamza():
    assert cc.normalize_word("عَلَىٰ") == cc.normalize_word("علي") == "علي"
    assert cc.normalize_word("رَحْمَةٌ") == cc.normalize_word("رحمه") == "رحمه"
    assert cc.normalize_word("مُؤْمِن") == cc.normalize_word("مومن")
    assert cc.normalize_word("شَيْءٍ") == cc.normalize_word("شي")
    assert cc.normalize_word("ک") == cc.normalize_word("ك")  # Persian kaf
    assert cc.normalize_word("ی") == cc.normalize_word("ي")  # Persian yeh


def test_normalize_removes_tatweel_punctuation_and_quranic_marks():
    assert cc.normalize_word("اللـــه") == "الله"
    assert cc.normalize_word("الله،") == "الله"
    assert cc.normalize_word("«الله»؟") == "الله"
    assert cc.normalize_word("أَحَدٌ ۝١") == "احد"  # end-of-ayah sign + digit
    assert cc.normalize_word("ۖ") == ""
    assert cc.normalize_word("hello 123") == ""  # nothing Arabic left


def test_normalize_strips_small_waw_and_small_yeh():
    # U+06E5 / U+06E6 are category Lm (not marks): they must not survive as letters
    assert cc.normalize_word("لَهُ\u06e5") == "له"
    assert cc.normalize_word("بِهِ\u06e6") == "به"
    assert cc.word_similarity(cc.normalize_word("لَهُ\u06e5"), "له") == 1.0


def test_normalize_strips_all_quranic_annotation_marks():
    for cp in ("\u0670", "\u06d6", "\u06da", "\u06df", "\u06e1", "\u06e2", "\u06e8",
               "\u06ea", "\u06ed", "\u08d3", "\u08e4", "\u08ff", "\u0640", "\u06dd",
               "\u06de", "\u06e9"):
        assert cc.normalize_word("ب" + cp + "ت") == "بت", hex(ord(cp))


def test_normalize_real_ayat_exact_word_lists():
    assert cc.normalize_words(DATA["112"]["ayat"][3]["ar"]) == ["ولم", "يكن", "له", "كفوا", "احد"]
    assert cc.normalize_words(DATA["78"]["ayat"][14]["ar"]) == ["لنخرج", "به", "حبا", "ونباتا"]
    assert cc.normalize_words(DATA["80"]["ayat"][34]["ar"]) == ["وامه", "وابيه"]


def test_every_juz30_word_normalises_to_base_arabic_letters_only():
    import unicodedata

    seen = 0
    for surah, entry in DATA.items():
        for i, a in enumerate(entry["ayat"], start=1):
            words = cc.normalize_words(a["ar"])
            assert words, (surah, i)
            for w in words:
                seen += 1
                for ch in w:
                    assert unicodedata.category(ch) == "Lo" and "\u0621" <= ch <= "\u064a", (
                        surah, i, w, hex(ord(ch)), unicodedata.category(ch))
    assert seen > 500


def test_normalize_words_splits_and_drops_empty():
    assert cc.normalize_words("قُلْ هُوَ  ٱللَّهُ ۚ أَحَدٌ") == ["قل", "هو", "الله", "احد"]
    assert cc.normalize_words("") == []
    assert cc.normalize_words("، . ۝") == []


# ----------------------------------------------------------------------------------
# similarity
# ----------------------------------------------------------------------------------


def test_word_similarity():
    assert cc.word_similarity("الله", "الله") == 1.0
    assert cc.word_similarity("الصمد", "الصمت") == pytest.approx(0.8)
    assert cc.word_similarity("", "الله") == 0.0
    assert cc.word_similarity("قل", "ضرب") < 0.4


# ----------------------------------------------------------------------------------
# matching and classification (pure logic, synthetic Whisper words)
# ----------------------------------------------------------------------------------


def run(words, starts=STARTS_112, ayat=AYAT_112, tol=300):
    return cc.crosscheck_words(words, ayat, starts, tolerance_ms=tol)


def test_clean_transcription_all_ok_with_exact_numbers():
    res = run(clean_112())
    assert res == [
        {"ayah": 1, "aligner_ms": 60, "whisper_ms": 60, "delta_ms": 0, "status": "ok"},
        {"ayah": 2, "aligner_ms": 2538, "whisper_ms": 2538, "delta_ms": 0, "status": "ok"},
        {"ayah": 3, "aligner_ms": 4682, "whisper_ms": 4682, "delta_ms": 0, "status": "ok"},
        {"ayah": 4, "aligner_ms": 7218, "whisper_ms": 7218, "delta_ms": 0, "status": "ok"},
    ]


def test_delta_is_whisper_minus_aligner_and_ints():
    words = clean_112()
    words[4] = W("الله", 2700)  # 162 ms late
    r = run(words)[1]
    assert r["delta_ms"] == 162 and r["whisper_ms"] == 2700 and r["status"] == "ok"
    assert all(isinstance(r[k], int) for k in ("ayah", "aligner_ms", "whisper_ms", "delta_ms"))


@pytest.mark.parametrize(
    "delta,status",
    [(0, "ok"), (299, "ok"), (300, "ok"), (301, "disagree"), (-299, "ok"), (-300, "ok"),
     (-301, "disagree"), (-1500, "disagree")],
)
def test_status_around_tolerance(delta, status):
    words = clean_112()
    words[6] = W("لم", 4682 + delta)  # ayah 3 first word
    # only late deltas below the 2nd word's start (5000 ms) stay observable: earliest wins
    r = run(words)[2]
    assert (r["delta_ms"], r["status"]) == (delta, status)


def test_custom_tolerance():
    words = clean_112()
    words[4] = W("الله", 2538 + 150)
    assert run(words, tol=149)[1]["status"] == "disagree"
    assert run(words, tol=150)[1]["status"] == "ok"


def test_fuzzy_misspelled_words_still_match():
    words = clean_112()
    words[5] = W("الصمت", 3000)      # ayah 2, 2nd word misspelled
    words[2] = W("الاه", 800)        # ayah 1, 3rd word misspelled
    words[6] = W("لمم", 4690)        # ayah 3, 1st word misspelled (sim 0.67)
    res = run(words)
    assert [r["status"] for r in res] == ["ok"] * 4
    assert res[2]["whisper_ms"] == 4690 and res[2]["delta_ms"] == 8


def test_merged_whisper_words_use_merged_word_start():
    # "لم يلد" transcribed as one word "لميلد"
    words = [w for w in clean_112() if w["word"] not in ("لم", "يلد")]
    words.insert(6, W("لميلد", 4700))
    res = run(words)
    assert [r["status"] for r in res] == ["ok"] * 4
    assert res[2]["whisper_ms"] == 4700 and res[2]["delta_ms"] == 18


def test_split_whisper_word_uses_first_part_start():
    # "الصمد" -> "ال" "صمد"; ayah 2 itself starts with الله, ayah 1's tail is fine
    words = clean_112()
    words[4:6] = [W("ال", 2538), W("له", 2700), W("الصمد", 3000)]  # "الله" split
    res = run(words)
    assert res[1]["whisper_ms"] == 2538 and res[1]["status"] == "ok"
    # and a split on the very first word of ayah 4: "ولم" -> "و" "لم"
    words = clean_112()
    i = [k for k, w in enumerate(words) if w["start_ms"] == 7218][0]
    words[i:i + 1] = [W("و", 7240), W("لم", 7330)]
    res = run(words)
    assert res[3]["whisper_ms"] == 7240 and res[3]["status"] == "ok"


def test_missing_first_word_uses_earliest_matched_of_first_three():
    words = [w for w in clean_112() if not (w["word"] == "لم" and w["start_ms"] == 4682)]
    r = run(words)[2]
    assert r["whisper_ms"] == 5000  # "يلد" is the earliest matched of the first words
    assert r["delta_ms"] == 318 and r["status"] == "disagree"  # honest: biased late


def test_first_two_missing_uses_third_word():
    words = [w for w in clean_112() if w["start_ms"] not in (7218, 7600)]
    r = run(words)[3]
    assert r["whisper_ms"] == 8000  # "له"
    assert r["status"] == "disagree"


def test_only_first_three_words_are_considered():
    # ayah 4 = ولم يكن له كفوا احد: first three missing, 4th/5th present -> unmatched
    words = [w for w in clean_112() if w["start_ms"] not in (7218, 7600, 8000)]
    r = run(words)[3]
    assert r["status"] == "unmatched"
    assert r["whisper_ms"] is None and r["delta_ms"] is None and r["aligner_ms"] == 7218


def test_garbage_opening_is_unmatched_never_ok():
    words = clean_112()
    for k in (6, 7):                       # ayah 3 first two words
        words[k] = W("شكرا", words[k]["start_ms"])
    words[8] = W("مرحبا", 5600)            # ... and the third
    r = run(words)[2]
    assert r["status"] == "unmatched" and r["whisper_ms"] is None and r["delta_ms"] is None
    assert [x["status"] for x in run(words)] == ["ok", "ok", "unmatched", "ok"]


def test_empty_whisper_output_is_all_unmatched():
    res = run([])
    assert [r["status"] for r in res] == ["unmatched"] * 4
    assert [r["aligner_ms"] for r in res] == STARTS_112


def test_hallucinated_extra_words_are_ignored():
    words = [W("شكرا", 0, 50), W("لكم", 55, 59)] + clean_112()
    words.insert(8, W("اشترك", 3500))      # inserted mid-surah
    words.append(W("ترجمة", 9500))         # trailing hallucination
    res = run(words)
    assert [r["status"] for r in res] == ["ok"] * 4
    assert [r["whisper_ms"] for r in res] == STARTS_112


def test_repeated_words_across_ayat_are_matched_in_order():
    # "الله" ends ayah 1's phrase region and starts ayah 2: the ayah-2 start must be the
    # SECOND "الله", not the first.
    res = run(clean_112())
    assert res[0]["whisper_ms"] == 60 and res[1]["whisper_ms"] == 2538


def test_whisper_words_without_arabic_are_dropped():
    words = clean_112()
    words.insert(3, W("...", 900))
    words.insert(0, W("[موسيقى]", 0, 10))   # brackets removed, word kept as a hallucination
    words.insert(0, W("123", 0, 5))
    assert [r["status"] for r in run(words)] == ["ok"] * 4


def test_ayah_with_one_word():
    ayat = ["قُلْ", "ٱلصَّمَدُ"]
    words = [W("قل", 100), W("الصمد", 900)]
    res = cc.crosscheck_words(words, ayat, [100, 950])
    assert [r["status"] for r in res] == ["ok", "ok"] and res[1]["delta_ms"] == -50


def test_length_mismatch_is_an_error():
    with pytest.raises(ValueError, match="starts_ms"):
        cc.crosscheck_words(clean_112(), AYAT_112, [60, 2538])
    with pytest.raises(ValueError):
        cc.crosscheck_words(clean_112(), AYAT_112, [60, 2538, 4682, 7218, 9000])


def test_crosscheck_surah_validates_before_running_whisper(monkeypatch):
    def boom(_):
        raise AssertionError("whisper must not run on bad input")

    monkeypatch.setattr(cc, "whisper_word_times", boom)
    with pytest.raises(ValueError):
        cc.crosscheck_surah("/nonexistent.mp3", AYAT_112, [60, 2538])


def test_crosscheck_surah_uses_whisper_words(monkeypatch):
    monkeypatch.setattr(cc, "whisper_word_times", lambda p: clean_112())
    res = cc.crosscheck_surah("x.mp3", AYAT_112, STARTS_112)
    assert [r["status"] for r in res] == ["ok"] * 4


def test_flatten_segments_converts_to_integer_ms():
    segs = [
        SimpleNamespace(words=[SimpleNamespace(word=" قل", start=0.06, end=0.3999),
                               SimpleNamespace(word=" هو", start=0.4, end=0.6)]),
        SimpleNamespace(words=None),
        SimpleNamespace(words=[SimpleNamespace(word=" الله،", start=2.5384, end=3.0)]),
    ]
    assert cc.flatten_segments(segs) == [
        {"word": "قل", "start_ms": 60, "end_ms": 400},
        {"word": "هو", "start_ms": 400, "end_ms": 600},
        {"word": "الله،", "start_ms": 2538, "end_ms": 3000},
    ]


# ----------------------------------------------------------------------------------
# CLI
# ----------------------------------------------------------------------------------


@pytest.mark.parametrize("bad", ["50", "77", "115", "0"])
def test_cli_rejects_surah_outside_78_114(bad):
    with pytest.raises(SystemExit) as e:
        cc.main([bad])
    assert e.value.code == 2


def _stub(monkeypatch, words):
    monkeypatch.setattr(cc, "limit_torch_threads", lambda *a: None)
    monkeypatch.setattr(cc.align, "ensure_mp3", lambda s: "/fake.mp3")
    monkeypatch.setattr(cc.align, "align_surah_detail",
                        lambda p, a: {"starts_ms": list(STARTS_112), "flags": []})
    monkeypatch.setattr(cc, "whisper_word_times", lambda p, model=None: words)


def test_cli_exit_0_and_table_when_all_ok(monkeypatch, capsys):
    _stub(monkeypatch, clean_112())
    assert cc.main(["112"]) == 0
    out = capsys.readouterr().out
    assert "ayah" in out and "whisper" in out and "status" in out
    assert out.count("ok") >= 4 and "disagree" not in out


def test_cli_exit_1_on_disagree_or_unmatched(monkeypatch, capsys):
    words = clean_112()
    words[4] = W("الله", 3300)
    _stub(monkeypatch, words)
    assert cc.main(["112"]) == 1
    assert "disagree" in capsys.readouterr().out
    _stub(monkeypatch, [])
    assert cc.main(["112"]) == 1
    assert "unmatched" in capsys.readouterr().out


def test_cli_json(monkeypatch, capsys):
    _stub(monkeypatch, clean_112())
    assert cc.main(["112", "--json"]) == 0
    out = json.loads(capsys.readouterr().out)
    assert out["surah"] == 112 and len(out["results"]) == 4
    assert out["results"][0] == {"ayah": 1, "aligner_ms": 60, "whisper_ms": 60,
                                 "delta_ms": 0, "status": "ok"}


def test_cli_starts_file_list_and_object(monkeypatch, capsys, tmp_path):
    _stub(monkeypatch, clean_112())

    def no_align(p, a):
        raise AssertionError("must not re-align when --starts is given")

    monkeypatch.setattr(cc.align, "align_surah_detail", no_align)
    f = tmp_path / "s.json"
    f.write_text("[60, 2538, 4682, 7218]")
    assert cc.main(["112", "--starts", str(f), "--json"]) == 0
    assert [r["aligner_ms"] for r in json.loads(capsys.readouterr().out)["results"]] == STARTS_112
    f.write_text('{"surah": 112, "ayah_starts_ms": [60, 2538, 5000, 7218], "flags": []}')
    assert cc.main(["112", "--starts", str(f)]) == 1
    assert "disagree" in capsys.readouterr().out


@pytest.mark.parametrize("content", ["[60, 2538]", '{"x": 1}', '["a", 1, 2, 3]', "not json", "[]"])
def test_cli_starts_file_bad_content_exits_1(monkeypatch, capsys, tmp_path, content):
    _stub(monkeypatch, clean_112())
    f = tmp_path / "s.json"
    f.write_text(content)
    assert cc.main(["112", "--starts", str(f)]) == 1
    assert "error" in capsys.readouterr().err


def test_cli_starts_file_missing_exits_1(monkeypatch, capsys, tmp_path):
    _stub(monkeypatch, clean_112())
    assert cc.main(["112", "--starts", str(tmp_path / "nope.json")]) == 1


# ----------------------------------------------------------------------------------
# slow integration test (real model, real recording): pytest -m slow
# ----------------------------------------------------------------------------------


@pytest.mark.slow
def test_surah_112_end_to_end_all_ayat_matched():
    import align

    mp3 = align.ensure_mp3(112)
    cc.limit_torch_threads()
    starts = align.align_surah_detail(mp3, AYAT_112)["starts_ms"]
    res = cc.crosscheck_surah(mp3, AYAT_112, starts)
    assert len(res) == 4
    assert all(r["status"] != "unmatched" for r in res), res
