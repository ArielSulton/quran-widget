#!/usr/bin/env python3
"""Sanity check of ayah start times with Whisper: right words, right order; not a precision check.

This tool flags GROSS errors (a wrong or missing ayah, a boundary about a second or more off).
It does NOT validate the 300 ms accuracy requirement: Whisper word timestamps are themselves
only good to roughly +-200 ms and are biased early after pauses, so an "ok" does not prove a
start is within 300 ms and a "disagree" does not prove the aligner is wrong.  The human
listening gate is not replaced.

It cross-checks the start times computed by tools/align.py.  align.py (torchaudio MMS_FA forced alignment + energy-onset snapping) is the primary method.
This tool is a second opinion that shares no model, no tokenisation and no romanisation
with it: faster-whisper (CTranslate2, int8, CPU) transcribes the same recording blind (the
Quran text is NOT given to Whisper, no prompt) with word-level timestamps, and the
transcript is then matched to the known ayat text.

Public API
    whisper_word_times(mp3_path)                      -> [{"word", "start_ms", "end_ms"}, ...]
    crosscheck_surah(mp3_path, ayat_ar, starts_ms, tolerance_ms=300) -> [entry, ...]
    crosscheck_words(words, ayat_ar, starts_ms, tolerance_ms=300)    -> [entry, ...]  (pure)
    entry = {"ayah": i (1-based), "aligner_ms", "whisper_ms" | None, "delta_ms" | None,
             "status": "ok" | "disagree" | "unmatched"}

Matching
    Arabic is normalised (diacritics, tatweel, Quranic marks, punctuation and digits removed;
    alef/hamza, yaa/alef-maqsura, taa-marbuta, waw-hamza variants unified).  The Whisper word
    sequence is then globally aligned to the sequence of all ayah words (dynamic programming,
    Needleman-Wunsch style) with a fuzzy, character-weighted word similarity
    (1 - Levenshtein / max length) and four extra moves: one reference word <-> two Whisper
    words (Whisper split a word), two reference words <-> one Whisper word (Whisper merged
    words), and gaps on either side (missed / hallucinated words).  A pairing counts as a
    match only when its similarity is >= MATCH_MIN_SIM.
    The Whisper start of an ayah is the start time of the earliest matched Whisper word among
    the ayah's first three words.  When none of them is matched the status is "unmatched"
    (never silently "ok").  delta_ms = whisper_ms - aligner_ms; |delta| > tolerance_ms is
    "disagree".  Note that if the ayah's first word was missed but the 2nd/3rd was matched,
    whisper_ms is later than the true start by that word's duration, which shows up as a
    (conservative) "disagree".

CLI
    python -I tools/crosscheck.py <surah 78-114> [--json] [--model SIZE] [--mp3 PATH]
                                  [--starts PATH]
    --starts PATH: check given starts (JSON list of ms, or an object with "ayah_starts_ms",
    e.g. the output of align.py) instead of re-aligning.
    exit 0: every ayah "ok";  1: any "disagree"/"unmatched" (or the run failed);  2: usage.
    Run with ``nice -n 19`` and at most 2 threads on a shared machine (set below).
"""
from __future__ import annotations

import argparse
import json
import os
import sys
import unicodedata
from pathlib import Path

os.environ.setdefault("OMP_NUM_THREADS", "2")  # before ctranslate2 is imported

sys.path.insert(0, str(Path(__file__).resolve().parent))
import align  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_MODEL = "small"
CPU_THREADS = 2
WHISPER_CACHE = Path(os.environ.get("XDG_CACHE_HOME") or Path.home() / ".cache") / "quran-widget" / "whisper"
MATCH_MIN_SIM = 0.6      # minimum word similarity for a pairing to count as matched
GAP_COST = 0.7           # per character of a word skipped on either side
OPENING_WORDS = 3        # an ayah start is taken from its first 1-3 words

# ----------------------------------------------------------------------------------
# Normalisation
# ----------------------------------------------------------------------------------
_LETTER_MAP = {
    "أ": "ا", "إ": "ا", "آ": "ا", "ٱ": "ا", "ٲ": "ا", "ٳ": "ا",   # alef variants
    "ى": "ي", "ی": "ي", "ې": "ي", "ئ": "ي",                        # yaa / alef maqsura / hamza on yaa
    "ة": "ه",                                                      # taa marbuta
    "ؤ": "و",                                                      # hamza on waw
    "ک": "ك",                                                      # Persian kaf
    "ء": "",                                                       # bare hamza
}
_MAP = str.maketrans(_LETTER_MAP)


def _is_base_letter(ch: str) -> bool:
    """A real Arabic base letter: category Lo inside an Arabic block.

    This excludes by construction everything that is not a base letter: harakat and Quranic
    annotation marks (Mn: U+064B-U+0655, U+0670, U+06D6-U+06ED incl. U+06DF/U+06E1, U+08D3-U+08FF),
    tatweel U+0640 and the small waw / small yeh U+06E5/U+06E6 (all category Lm), the end-of-ayah
    sign U+06DD (Cf), the sign characters U+06DE/U+06E9 (So), digits and punctuation.
    """
    if unicodedata.category(ch) != "Lo":
        return False
    return ("\u0600" <= ch <= "\u06ff" or "\u0750" <= ch <= "\u077f"
            or "\u08a0" <= ch <= "\u08ff")


def normalize_word(text: str) -> str:
    """Arabic base letters only, no diacritics/tatweel/small letters/punctuation, variants unified."""
    return "".join(ch for ch in unicodedata.normalize("NFC", text).translate(_MAP)
                   if _is_base_letter(ch))


def normalize_words(text: str) -> list[str]:
    """Normalised, non-empty words of ``text``."""
    return [w for w in (normalize_word(t) for t in text.split()) if w]


# ----------------------------------------------------------------------------------
# Fuzzy matching
# ----------------------------------------------------------------------------------


def _levenshtein(a: str, b: str) -> int:
    if len(a) < len(b):
        a, b = b, a
    prev = list(range(len(b) + 1))
    for i, ca in enumerate(a, start=1):
        cur = [i]
        for j, cb in enumerate(b, start=1):
            cur.append(min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + (ca != cb)))
        prev = cur
    return prev[-1]


def word_similarity(a: str, b: str) -> float:
    """1 - Levenshtein/max(len): 1.0 identical, 0.0 nothing in common (or an empty word)."""
    n = max(len(a), len(b))
    if n == 0 or not a or not b:
        return 0.0
    return 1.0 - _levenshtein(a, b) / n


def _pair_score(a: str, b: str) -> tuple[float, float]:
    sim = word_similarity(a, b)
    return (2 * sim - 1) * max(len(a), len(b)), sim


def align_words(ref: list[str], hyp: list[str]) -> dict[int, tuple[int, float]]:
    """Globally align ``hyp`` (Whisper) to ``ref`` (known text); both already normalised.

    Returns {ref_index: (first_hyp_index, similarity)} for every reference word that is part
    of a pairing with similarity >= MATCH_MIN_SIM.
    """
    n, m = len(ref), len(hyp)
    NEG = float("-inf")
    dp = [[NEG] * (m + 1) for _ in range(n + 1)]
    back: list[list[tuple | None]] = [[None] * (m + 1) for _ in range(n + 1)]
    dp[0][0] = 0.0
    for i in range(n + 1):
        for j in range(m + 1):
            cur = dp[i][j]
            if cur == NEG:
                continue
            moves = []
            if i < n:  # reference word missed by Whisper
                moves.append((i + 1, j, -GAP_COST * len(ref[i]), ("skip_ref",)))
            if j < m:  # hallucinated / extra Whisper word
                moves.append((i, j + 1, -GAP_COST * len(hyp[j]), ("skip_hyp",)))
            if i < n and j < m:
                s, sim = _pair_score(ref[i], hyp[j])
                moves.append((i + 1, j + 1, s, ("pair", sim)))
            if i + 1 < n and j < m:  # Whisper merged two reference words
                s, sim = _pair_score(ref[i] + ref[i + 1], hyp[j])
                moves.append((i + 2, j + 1, s, ("merge", sim)))
            if i < n and j + 1 < m:  # Whisper split one reference word
                s, sim = _pair_score(ref[i], hyp[j] + hyp[j + 1])
                moves.append((i + 1, j + 2, s, ("split", sim)))
            for ni, nj, s, tag in moves:
                if cur + s > dp[ni][nj]:
                    dp[ni][nj] = cur + s
                    back[ni][nj] = (i, j, tag)
    matches: dict[int, tuple[int, float]] = {}
    i, j = n, m
    while (i, j) != (0, 0):
        pi, pj, tag = back[i][j]
        kind = tag[0]
        if kind in ("pair", "merge", "split") and tag[1] >= MATCH_MIN_SIM:
            for r in range(pi, i):
                matches[r] = (pj, tag[1])
        i, j = pi, pj
    return matches


# ----------------------------------------------------------------------------------
# Cross-check logic (pure)
# ----------------------------------------------------------------------------------


def crosscheck_words(words: list[dict], ayat_ar: list[str], starts_ms: list[int],
                     tolerance_ms: int = 300) -> list[dict]:
    """Compare ``starts_ms`` with the ayah starts found in the Whisper ``words``."""
    if len(starts_ms) != len(ayat_ar):
        raise ValueError(
            f"starts_ms has {len(starts_ms)} entries but ayat_ar has {len(ayat_ar)}")
    ref: list[str] = []
    bounds: list[tuple[int, int]] = []
    for i, text in enumerate(ayat_ar, start=1):
        ws = normalize_words(text)
        if not ws:
            raise ValueError(f"ayah {i} has no Arabic words: {text!r}")
        bounds.append((len(ref), len(ref) + len(ws)))
        ref.extend(ws)
    hyp, hyp_start = [], []
    for w in words:
        nw = normalize_word(w["word"])
        if nw:
            hyp.append(nw)
            hyp_start.append(w["start_ms"])
    matches = align_words(ref, hyp) if hyp else {}

    out = []
    for i, ((lo, hi), aligner_ms) in enumerate(zip(bounds, starts_ms), start=1):
        cands = [hyp_start[matches[r][0]] for r in range(lo, min(hi, lo + OPENING_WORDS))
                 if r in matches]
        if not cands:
            out.append({"ayah": i, "aligner_ms": aligner_ms, "whisper_ms": None,
                        "delta_ms": None, "status": "unmatched"})
            continue
        whisper_ms = min(cands)
        delta = whisper_ms - aligner_ms
        out.append({"ayah": i, "aligner_ms": aligner_ms, "whisper_ms": whisper_ms,
                    "delta_ms": delta,
                    "status": "disagree" if abs(delta) > tolerance_ms else "ok"})
    return out


# ----------------------------------------------------------------------------------
# Whisper
# ----------------------------------------------------------------------------------


def flatten_segments(segments) -> list[dict]:
    """faster-whisper segments -> [{"word", "start_ms", "end_ms"}] (whitespace stripped)."""
    out = []
    for seg in segments:
        for w in getattr(seg, "words", None) or []:
            text = w.word.strip()
            if text:
                out.append({"word": text, "start_ms": int(round(w.start * 1000)),
                            "end_ms": int(round(w.end * 1000))})
    return out


_MODELS: dict[str, object] = {}


def _load_model(size: str):
    if size not in _MODELS:
        from faster_whisper import WhisperModel

        WHISPER_CACHE.mkdir(parents=True, exist_ok=True)
        _MODELS[size] = WhisperModel(size, device="cpu", compute_type="int8",
                                     cpu_threads=CPU_THREADS, num_workers=1,
                                     download_root=str(WHISPER_CACHE))
    return _MODELS[size]


def whisper_word_times(mp3_path: str, model: str | None = None) -> list[dict]:
    """Blind Arabic transcription with word timestamps (CPU, int8, <= 2 threads)."""
    size = model or os.environ.get("QURAN_WHISPER_MODEL") or DEFAULT_MODEL
    wave = align.decode_audio(mp3_path)  # 16 kHz mono float32 via the validated ffmpeg path
    segments, _info = _load_model(size).transcribe(
        wave, language="ar", word_timestamps=True, beam_size=5,
        condition_on_previous_text=False, vad_filter=False)
    return flatten_segments(segments)  # generator is consumed here


def crosscheck_surah(mp3_path: str, ayat_ar: list[str], starts_ms: list[int],
                     tolerance_ms: int = 300) -> list[dict]:
    if len(starts_ms) != len(ayat_ar):  # fail before the expensive transcription
        raise ValueError(
            f"starts_ms has {len(starts_ms)} entries but ayat_ar has {len(ayat_ar)}")
    return crosscheck_words(whisper_word_times(mp3_path), ayat_ar, starts_ms, tolerance_ms)


# ----------------------------------------------------------------------------------
# CLI
# ----------------------------------------------------------------------------------


def load_starts(path: str) -> list[int]:
    """Read ayah starts from JSON: a list of ms, or an object with ``ayah_starts_ms``."""
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    if isinstance(data, dict):
        if "ayah_starts_ms" not in data:
            raise ValueError(f"{path}: JSON object has no 'ayah_starts_ms'")
        data = data["ayah_starts_ms"]
    if (not isinstance(data, list) or not data
            or not all(isinstance(x, int) and not isinstance(x, bool) for x in data)):
        raise ValueError(f"{path}: expected a non-empty JSON list of integer milliseconds")
    return data


def limit_torch_threads(n: int = CPU_THREADS) -> None:
    """align.py sizes torch to cpu_count/2 threads; cap it so the whole run stays <= n."""
    import torch

    align._model()  # loads MMS_FA and applies align's own thread setting first
    torch.set_num_threads(n)


def format_table(results: list[dict]) -> str:
    rows = [("ayah", "aligner_ms", "whisper_ms", "delta_ms", "status")]
    for r in results:
        rows.append((str(r["ayah"]), str(r["aligner_ms"]),
                     "-" if r["whisper_ms"] is None else str(r["whisper_ms"]),
                     "-" if r["delta_ms"] is None else f"{r['delta_ms']:+d}", r["status"]))
    widths = [max(len(row[c]) for row in rows) for c in range(5)]
    return "\n".join("  ".join(cell.rjust(w) for cell, w in zip(row, widths)) for row in rows)


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(
        description="Sanity check of ayah start times with Whisper: right words, right order; "
                    "flags gross errors; NOT a precision check (does not validate 300 ms).")
    ap.add_argument("surah", type=int, help="surah number, 78-114 (Juz 30)")
    ap.add_argument("--starts", default=None, metavar="PATH",
                    help="JSON list of ayah starts in ms, or an object with 'ayah_starts_ms' "
                         "(e.g. align.py output); checked instead of re-aligning")
    ap.add_argument("--json", action="store_true", help="machine-readable output")
    ap.add_argument("--model", default=None, help=f"Whisper model size (default {DEFAULT_MODEL})")
    ap.add_argument("--mp3", default=None, help="existing local mp3 file")
    a = ap.parse_args(argv)
    lo, hi = align.SURAH_RANGE
    if not lo <= a.surah <= hi:
        ap.error(f"surah must be in {lo}-{hi}, got {a.surah}")
    data = json.loads((ROOT / "data" / "quran" / "juz30.json").read_text(encoding="utf-8"))
    ayat = [x["ar"] for x in data[str(a.surah)]["ayat"]]
    try:
        mp3 = a.mp3 or align.ensure_mp3(a.surah)
        if a.starts:
            starts = load_starts(a.starts)
        else:
            limit_torch_threads()
            starts = align.align_surah_detail(mp3, ayat)["starts_ms"]
        kw = {"model": a.model} if a.model else {}
        words = whisper_word_times(mp3, **kw)
        results = crosscheck_words(words, ayat, starts)
    except (align.AlignmentError, ValueError, OSError) as e:
        print(f"error: surah {a.surah}: {e}", file=sys.stderr)
        return 1
    if a.json:
        print(json.dumps({"surah": a.surah, "results": results}, ensure_ascii=False))
    else:
        print(f"surah {a.surah}")
        print(format_table(results))
    return 0 if all(r["status"] == "ok" for r in results) else 1


if __name__ == "__main__":
    sys.exit(main())
