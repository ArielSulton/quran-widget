#!/usr/bin/env python3
"""Forced alignment of Quran ayat against the reference recording (Yasser Ad-Dosari,
https://server11.mp3quran.net/yasser/NNN.mp3).

Public API
    align_surah(mp3_path, ayat_ar)        -> list[int]  # ayah start times, ms, reference time
    align_surah_detail(mp3_path, ayat_ar) -> dict       # {"starts_ms": [...], "flags": [...]}
    ensure_mp3(surah)                     -> str        # download (if needed) into the user cache

CLI
    python -I tools/align.py <surah_number 78-114> [--mp3 PATH]
    -> {"surah": N, "ayah_starts_ms": [...], "flags": [...]}

Method
    1. The mp3 is decoded with ffmpeg to 16 kHz mono float32.
    2. torchaudio's MMS_FA (wav2vec2 CTC, 1100+ languages) produces frame posteriors
       (20 ms frames) over a romanised alphabet.  The Arabic text is romanised by a small
       deterministic rule set (``romanize``), then CTC forced alignment (Viterbi) gives
       one time span per character.
    3. Only the ayat text is aligned.  All 37 Juz 30 reference recordings start directly
       with ayah 1 (no isti'adzah / basmala).  A recording that does not is detected by a
       guard (first ayah later than 2500 ms) and rejected with "preamble suspected".
    4. Boundary definition: the start of an ayah is the moment the reciter begins the
       first word of the ayah.  The CTC estimate (first character span) is refined with
       the short-time energy inside a +/-500 ms window: the pause nearest the estimate is
       found and the onset is the instant the energy leaves it.  When there is no usable
       pause the CTC estimate is kept and reported as a "ctc_fallback" flag.
    5. The final list is validated (strictly increasing, >= 0, < duration); violations
       raise AlignmentError instead of being clamped.

Flags (``align_surah_detail``): one entry ``{"ayah": n (1-based), "kind": "onset" |
"ctc_fallback", "delta_ms": refined - raw_ctc}`` for every boundary that fell back to the
raw CTC estimate or moved more than 300 ms away from it.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import unicodedata
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SURAH_RANGE = (78, 114)
MAX_FIRST_START_MS = 2500  # later than this => a spoken preamble is suspected
SAMPLE_RATE = 16000
FRAME = 320  # samples per model frame (20 ms)
REF_URL = "https://server11.mp3quran.net/yasser/{n:03d}.mp3"
REF_DIR = Path(os.environ.get("XDG_CACHE_HOME") or Path.home() / ".cache") / "quran-widget" / "ref"
MODEL_CACHE = Path(os.environ.get("DOSARI_MODEL_CACHE", ROOT / ".venv" / "torch_cache"))

# ----------------------------------------------------------------------------------
# Romanisation (Arabic -> MMS_FA alphabet: a-z and ')
# ----------------------------------------------------------------------------------
_CONS = {
    "ب": "b", "ت": "t", "ث": "th", "ج": "j", "ح": "h", "خ": "kh", "د": "d", "ذ": "dh",
    "ر": "r", "ز": "z", "س": "s", "ش": "sh", "ص": "s", "ض": "d", "ط": "t", "ظ": "z",
    "ع": "'", "غ": "gh", "ف": "f", "ق": "q", "ك": "k", "ل": "l", "م": "m", "ن": "n",
    "ه": "h", "و": "w", "ي": "y", "ى": "a", "ة": "t", "ء": "'", "أ": "'", "إ": "'",
    "ؤ": "'", "ئ": "'", "آ": "'a", "ٱ": "",
}
_FATHA, _DAMMA, _KASRA = "َ", "ُ", "ِ"
_FATHATAN, _DAMMATAN, _KASRATAN = "ً", "ٌ", "ٍ"
_SHADDA, _SUKUN, _DAGGER_ALEF = "ّ", "ْ", "ٰ"
_SMALL_WAW, _SMALL_YEH = "ۥ", "ۦ"  # as used in ۥ ۦ (U+06E5/U+06E6)
_VOWEL = {_FATHA: "a", _DAMMA: "u", _KASRA: "i", _FATHATAN: "an", _DAMMATAN: "un",
          _KASRATAN: "in"}
_SUN = set("تثدذرزسشصضطظلن")


def _split_marks(word: str):
    """[(base_letter, [marks...]), ...] for one word (Arabic letters + combining marks)."""
    out = []
    for ch in word:
        if unicodedata.category(ch).startswith("M") or ch in "ۥۦۢۡ":
            if out:
                out[-1][1].append(ch)
        elif "\u0600" <= ch <= "\u06ff":
            out.append((ch, []))
    return out


def romanize_word(word: str, at_start: bool) -> str:
    """Approximate recited pronunciation, lower-case a-z and ' only."""
    letters = _split_marks(word)
    out: list[str] = []
    last_vowel = ""  # last short vowel emitted (a/u/i), used to detect madd letters

    def vowel_out(v: str):
        # a long vowel is one stretched token: never repeat the same vowel back to back
        if out and out[-1].endswith(v[0]) and len(v) == 1:
            return
        out.append(v)

    for idx, (ch, marks) in enumerate(letters):
        mset = set(marks)
        vowel = next((_VOWEL[m] for m in marks if m in _VOWEL), "")
        has_other = bool(mset & {_SHADDA, _SUKUN}) or bool(vowel)
        if ch == "ٱ":  # alef wasla: only pronounced when starting the utterance
            if at_start and idx == 0:
                vowel_out("a")
                last_vowel = "a"
            continue
        if ch == "ا":
            if "\u0654" in mset:  # hamza above on alef
                out.append("'")
            elif not has_other and last_vowel == "a":
                continue  # madd alef: lengthening of the previous 'a'
            if vowel:
                vowel_out(vowel)
                last_vowel = vowel[0]
            continue
        if ch == "آ":
            if last_vowel != "a":
                out.append("'")
            vowel_out("a")
            last_vowel = "a"
            continue
        if ch == "ل" and idx == 1 and letters[0][0] == "ٱ" and not has_other:
            nxt = letters[2] if len(letters) > 2 else None
            if nxt and nxt[0] in _SUN and _SHADDA in nxt[1]:
                continue  # sun-letter assimilation: lam of the article is silent
        if ch in "وي" and not has_other and "\u0654" not in mset:
            if (ch == "و" and last_vowel == "u") or (ch == "ي" and last_vowel == "i"):
                continue  # madd waw / madd yaa
        if "\u0654" in mset and ch in "وي":
            out.append("'")  # hamza seated on waw / yaa
        else:
            out.append(_CONS.get(ch, ""))
        if vowel:
            if len(vowel) == 2:
                out.append(vowel)
            else:
                vowel_out(vowel)
            last_vowel = vowel[0]
        elif _SUKUN in mset:
            last_vowel = ""
        for m in marks:
            if m == _DAGGER_ALEF:
                vowel_out("a")
                last_vowel = "a"
            elif m == _SMALL_WAW:
                vowel_out("u")
                last_vowel = "u"
            elif m == _SMALL_YEH:
                vowel_out("i")
                last_vowel = "i"
    return re.sub(r"[^a-z']", "", "".join(out))


def romanize(text: str) -> list[str]:
    """Words of ``text`` romanised; empty words are dropped."""
    words = [w for w in text.split() if w]
    res = []
    for i, w in enumerate(words):
        r = romanize_word(w, at_start=(i == 0))
        # words glued to an elided alef wasla still start with the vowel of the previous word
        if r:
            res.append(r)
    return res


class AlignmentError(RuntimeError):
    """The alignment is not usable (bad audio, preamble suspected, invalid result)."""


# ----------------------------------------------------------------------------------
# Audio / model
# ----------------------------------------------------------------------------------

_PROTOCOL_LIKE = re.compile(r"^[A-Za-z][A-Za-z0-9+.\-]*:")


def ffmpeg_input(path: str) -> str:
    """Validate ``path`` as an existing regular file and return it as ``file:<abs path>``.

    The ``file:`` prefix makes ffmpeg/ffprobe treat the value strictly as a local file, so a
    name like ``http://...`` or ``concat:...`` can never select another protocol.
    """
    text = str(path)
    if _PROTOCOL_LIKE.match(text):
        raise ValueError(f"protocol-like path rejected: {text!r}")
    p = Path(text).resolve()
    if not p.is_file():
        raise FileNotFoundError(f"not an existing regular file: {text!r}")
    return "file:" + str(p)


def probe_audio(path: str) -> float:
    """ffprobe sanity check: must be a real audio file. Returns the duration in seconds."""
    r = subprocess.run(
        ["ffprobe", "-v", "error", "-select_streams", "a:0", "-show_entries",
         "stream=codec_type:format=duration", "-of", "json", ffmpeg_input(path)],
        capture_output=True, text=True,
    )
    try:
        info = json.loads(r.stdout or "{}")
        dur = float(info["format"]["duration"])
        ok = r.returncode == 0 and info["streams"][0]["codec_type"] == "audio" and dur > 1.0
    except (KeyError, IndexError, ValueError, TypeError):
        ok = False
    if not ok:
        raise AlignmentError(f"{path}: not a valid audio file (ffprobe: {r.stderr.strip()[:200]})")
    return dur


def _download(url: str, dest: str | Path) -> None:
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    with urllib.request.urlopen(req, timeout=60) as r:
        if r.status != 200:
            raise AlignmentError(f"{url}: HTTP {r.status}")
        with open(dest, "wb") as f:
            while chunk := r.read(1 << 16):
                f.write(chunk)


def ensure_mp3(surah: int, dest_root: Path | str | None = None) -> str:
    """Return the reference mp3 for ``surah``, downloading into <cache>/NNN/NNN.mp3 if needed.

    The default cache is per user (``$XDG_CACHE_HOME`` or ``~/.cache``/quran-widget/ref).
    Downloaded data is untrusted: it is never executed, only passed to ffmpeg/ffprobe as a
    ``file:`` argument, and a cached or freshly downloaded file must pass an ffprobe sanity
    check (a saved HTML error page is discarded / rejected).
    """
    if not SURAH_RANGE[0] <= surah <= SURAH_RANGE[1]:
        raise ValueError(f"surah must be in {SURAH_RANGE[0]}-{SURAH_RANGE[1]}, got {surah}")
    root = Path(dest_root) if dest_root else REF_DIR
    d = root / f"{surah:03d}"
    p = d / f"{surah:03d}.mp3"
    if p.exists():
        try:
            probe_audio(str(p))
            return str(p)
        except AlignmentError:
            p.unlink()  # corrupt / error page cached earlier: fetch again
    d.mkdir(parents=True, exist_ok=True, mode=0o700)
    tmp = p.with_suffix(".part")
    try:
        _download(REF_URL.format(n=surah), tmp)
        if tmp.stat().st_size < 1000:
            raise AlignmentError(f"surah {surah}: download too small ({tmp.stat().st_size} bytes)")
        probe_audio(str(tmp))
        tmp.rename(p)
    finally:
        tmp.unlink(missing_ok=True)
    return str(p)


def decode_audio(path: str):
    import numpy as np

    raw = subprocess.run(
        ["ffmpeg", "-v", "error", "-nostdin", "-i", ffmpeg_input(path), "-f", "f32le",
         "-ac", "1", "-ar", str(SAMPLE_RATE), "-"],
        check=True, capture_output=True,
    ).stdout
    return np.frombuffer(raw, dtype=np.float32).copy()


_MODEL = None


def _model():
    global _MODEL
    if _MODEL is None:
        import torch
        import torchaudio

        MODEL_CACHE.mkdir(parents=True, exist_ok=True)
        torch.hub.set_dir(str(MODEL_CACHE))
        torch.set_num_threads(max(1, (os.cpu_count() or 2) // 2))
        bundle = torchaudio.pipelines.MMS_FA
        _MODEL = (bundle.get_model().eval(), bundle.get_dict())
    return _MODEL


def emissions(wave):
    """Log-posteriors, shape (T, 28) at 20 ms stride (star column removed).

    Long audio is processed in 20 s windows with 1 s context on both sides to bound
    attention memory.
    """
    import torch

    model, _ = _model()
    chunk, ctx = 20 * SAMPLE_RATE, 1 * SAMPLE_RATE
    x = torch.from_numpy(wave)
    n = x.shape[0]
    outs = []
    with torch.inference_mode():
        for start in range(0, n, chunk):
            lo = max(0, start - ctx)
            hi = min(n, start + chunk + ctx)
            em, _ = model(x[lo:hi][None])
            em = em[0, :, :-1]
            a = (start - lo) // FRAME
            b = a + min(chunk, n - start) // FRAME + (1 if min(chunk, n - start) % FRAME else 0)
            outs.append(em[a:b])
    return torch.cat(outs, 0)


# ----------------------------------------------------------------------------------
# Alignment
# ----------------------------------------------------------------------------------


def _forced(em, words_rom: list[str]):
    """Align romanised words; returns per-word (start_frame, end_frame, token scores)."""
    import torch
    import torchaudio.functional as F

    _, dic = _model()
    toks = [dic[c] for w in words_rom for c in w]
    targets = torch.tensor([toks], dtype=torch.int32)
    ali, scores = F.forced_align(em[None], targets, blank=0)
    spans = F.merge_tokens(ali[0], scores[0].exp())
    if len(spans) != len(toks):
        raise AlignmentError(f"alignment produced {len(spans)} spans for {len(toks)} tokens")
    res, k = [], 0
    for w in words_rom:
        sp = spans[k:k + len(w)]
        k += len(w)
        res.append((sp[0].start, sp[-1].end, [s.score for s in sp]))
    return res


def align_frames(em, ayat_ar: list[str]):
    """Align the ayat text only -> per-ayah (first_word_start, first_word_end) in frames.

    Returns (spans, score) where score is the mean token confidence.
    """
    words_per_ayah = [romanize(a) for a in ayat_ar]
    for i, ws in enumerate(words_per_ayah, start=1):
        if not ws:
            raise ValueError(f"ayah {i} romanises to no words: {ayat_ar[i - 1]!r}")
    words = [w for ws in words_per_ayah for w in ws]
    res = _forced(em, words)
    sc = [s for _, _, ss in res for s in ss]
    spans, k = [], 0
    for ws in words_per_ayah:
        fw = res[k]
        spans.append((fw[0], fw[1]))
        k += len(ws)
    return spans, sum(sc) / len(sc)


# ----------------------------------------------------------------------------------
# Onset refinement with short-time energy
# ----------------------------------------------------------------------------------
HOP = 80  # 5 ms
WIN = 400  # 25 ms
MIN_PAUSE_S = 0.12  # shortest low-energy run accepted as an ayah pause
WINDOW_S = 0.5  # snapping window half-width around the CTC estimate
FLAG_DELTA_MS = 300  # moves larger than this are reported in "flags"


def energy_db(wave):
    import numpy as np

    n = (len(wave) - WIN) // HOP + 1
    if n <= 0:
        return np.zeros(1)
    idx = np.arange(WIN)[None, :] + HOP * np.arange(n)[:, None]
    frames = wave[idx] * np.hanning(WIN)[None, :]
    e = (frames ** 2).mean(1)
    return 10 * np.log10(e + 1e-10)  # frame i is centred at (i*HOP + WIN/2) samples


def _idx(t_s: float) -> int:
    return int(round((t_s * SAMPLE_RATE - WIN / 2) / HOP))


def _t(i: int) -> float:
    return (i * HOP + WIN / 2) / SAMPLE_RATE


def _rise_start(seg, end: int, thr_on: float, thr_low: float):
    """Index in ``seg`` where the rise after ``end`` starts, or None if it never gets there."""
    j = end + 1
    while j < len(seg) and seg[j] < thr_on:
        j += 1
    if j >= len(seg):
        return None  # capped at the window edge: never reached speech level
    k = j
    while k > end + 1 and seg[k - 1] < seg[k] and seg[k - 1] > thr_low:
        k -= 1
    return k


def refine_onset(edb, est_s: float, min_s: float, first: bool = False):
    """Snap the CTC estimate ``est_s`` of an ayah's first character to the audible onset.

    Only the window [est-0.5 s, est+0.5 s] (not before ``min_s``) is inspected, and the
    result never differs from the estimate by more than 0.5 s.

    * Ayah 2..N: the pause (>= 120 ms below floor + 30 % of the range) whose end is nearest
      the estimate is chosen; the onset is the instant the energy leaves it.  Pauses
      shorter than 120 ms (word gaps) are ignored.
    * Ayah 1 (``first=True``): the start is the first rise out of silence from the start of
      the file, so an intra-ayah gap can never be picked; if the audio before the estimate
      is silent the aligner's value is kept.

    Returns (seconds, kind) where kind is 'onset' (energy evidence used) or 'ctc_fallback'
    (the CTC estimate is returned unchanged).
    """
    import numpy as np

    sm = np.convolve(np.pad(edb, 1, mode="edge"), np.ones(3) / 3, mode="valid")  # 15 ms
    il = 0 if first else max(0, _idx(max(min_s, est_s - WINDOW_S)))
    ih = min(len(sm) - 1, _idx(est_s + WINDOW_S))
    if ih - il < 8:
        return est_s, "ctc_fallback"
    seg = sm[il:ih + 1]
    peak = float(np.percentile(seg, 90))
    floor = max(float(seg.min()), peak - 40.0)  # digital silence (-100 dB) must not set the scale
    if peak - floor < 12:  # no real dynamic range => no audible pause
        return est_s, "ctc_fallback"
    thr_low = floor + 0.30 * (peak - floor)
    thr_on = floor + 0.45 * (peak - floor)
    if first:
        # leading silence = frames from the start that stay below thr_low
        lead = 0
        while lead < len(seg) - 1 and seg[lead] <= thr_low:
            lead += 1
        k = 0 if lead == 0 else _rise_start(seg, lead - 1, thr_on, thr_low)
        if k is None:
            return est_s, "ctc_fallback"
        t = _t(il + k)  # lead == 0: speech already running at the first frame
    else:
        low = seg <= thr_low
        runs, i = [], 0
        while i < len(low):
            if low[i]:
                j = i
                while j + 1 < len(low) and low[j + 1]:
                    j += 1
                if (j - i + 1) * HOP / SAMPLE_RATE >= MIN_PAUSE_S and j < len(seg) - 1:
                    runs.append((i, j))
                i = j + 1
            else:
                i += 1
        if not runs:
            return est_s, "ctc_fallback"
        est_i = _idx(est_s) - il
        end = min(runs, key=lambda r: abs(r[1] - est_i))[1]
        k = _rise_start(seg, end, thr_on, thr_low)
        if k is None:
            return est_s, "ctc_fallback"
        t = _t(il + k)
    if abs(t - est_s) > WINDOW_S:
        return est_s, "ctc_fallback"
    return t, "onset"


def validate_starts(starts_ms: list[int], duration_ms: float, label: str = "") -> None:
    """Raise AlignmentError unless starts are >= 0, strictly increasing and < duration."""
    where = f"{label}: " if label else ""
    prev = None
    for i, s in enumerate(starts_ms, start=1):
        if s < 0:
            raise AlignmentError(f"{where}ayah {i} start {s} ms is negative (starts={starts_ms})")
        if prev is not None and s <= prev:
            raise AlignmentError(
                f"{where}ayah {i} start {s} ms is not after ayah {i - 1} start {prev} ms "
                f"(starts={starts_ms})")
        if s >= duration_ms:
            raise AlignmentError(
                f"{where}ayah {i} start {s} ms is not before the file duration "
                f"{duration_ms:.0f} ms (starts={starts_ms})")
        prev = s


def check_no_preamble(first_start_ms: float) -> None:
    if first_start_ms > MAX_FIRST_START_MS:
        raise AlignmentError(
            f"preamble suspected: ayah 1 starts at {first_start_ms:.0f} ms "
            f"(> {MAX_FIRST_START_MS} ms); the recording may begin with isti'adzah/basmala, "
            "which this tool does not model")


def align_detail(mp3_path: str, ayat_ar: list[str]):
    wave = decode_audio(mp3_path)
    em = emissions(wave)
    spans, score = align_frames(em, ayat_ar)
    f = FRAME / SAMPLE_RATE
    raw = [sp[0] * f for sp in spans]
    check_no_preamble(raw[0] * 1000)
    edb = energy_db(wave)
    refined, kinds = [], []
    for i in range(len(spans)):
        floor_s = refined[-1] + 0.3 if refined else 0.0
        t, kind = refine_onset(edb, raw[i], floor_s, first=(i == 0))
        refined.append(t)
        kinds.append(kind)
    return {"raw": raw, "refined": refined, "kinds": kinds, "score": score,
            "duration": len(wave) / SAMPLE_RATE, "edb": edb}


def align_surah_detail(mp3_path: str, ayat_ar: list[str]) -> dict:
    """{"starts_ms": [...], "flags": [...]} - see the module docstring for the flag format."""
    d = align_detail(mp3_path, ayat_ar)
    starts = [int(round(t * 1000)) for t in d["refined"]]
    validate_starts(starts, d["duration"] * 1000, label=Path(mp3_path).name)
    check_no_preamble(starts[0])
    flags = []
    for i, (raw, ref, kind) in enumerate(zip(d["raw"], d["refined"], d["kinds"]), start=1):
        delta = int(round((ref - raw) * 1000))
        if kind == "ctc_fallback" or abs(delta) > FLAG_DELTA_MS:
            flags.append({"ayah": i, "kind": kind, "delta_ms": delta})
    return {"starts_ms": starts, "flags": flags}


def align_surah(mp3_path: str, ayat_ar: list[str]) -> list[int]:
    return align_surah_detail(mp3_path, ayat_ar)["starts_ms"]


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("surah", type=int, help="surah number, 78-114 (Juz 30)")
    ap.add_argument("--mp3", default=None, help="existing local mp3 file")
    a = ap.parse_args(argv)
    if not SURAH_RANGE[0] <= a.surah <= SURAH_RANGE[1]:
        ap.error(f"surah must be in {SURAH_RANGE[0]}-{SURAH_RANGE[1]}, got {a.surah}")
    data = json.loads((ROOT / "data" / "quran" / "juz30.json").read_text(encoding="utf-8"))
    ayat = [x["ar"] for x in data[str(a.surah)]["ayat"]]
    try:
        mp3 = a.mp3 or ensure_mp3(a.surah)
        res = align_surah_detail(mp3, ayat)
    except (AlignmentError, ValueError, OSError) as e:
        print(f"error: surah {a.surah}: {e}", file=sys.stderr)
        return 1
    print(json.dumps({"surah": a.surah, "ayah_starts_ms": res["starts_ms"],
                      "flags": res["flags"]}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
