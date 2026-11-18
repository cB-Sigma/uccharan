#!/usr/bin/env python3
"""End-to-end check: does a real voice sound the same when uccharan replaces
the reference phonemizer?

Each sentence is synthesised twice through a Piper voice with sherpa-onnx
(>= 2.0, ``GenerationConfig.phoneme_codepoints``): once from the reference
phonemizer's phonemes and once from uccharan's. Both recordings are transcribed
by the same speech recogniser and scored against the original text.

    python tools/eval/e2e_tts_asr.py --lang en-us \
        --voice /models/piper-en-us-amy-low/en_US-amy-low.onnx \
        --asr whisper-onnx:/models/whisper-base/base --n 500

Needs (not installed by uccharan): sherpa-onnx >= 2.0, jiwer, numpy, an
espeak-ng for the reference side, a Piper voice (``.onnx`` + ``tokens.txt``),
and an ASR model: ``whisper-onnx:<prefix>`` (sherpa-onnx Whisper export,
``<prefix>-encoder.int8.onnx`` …) or ``faster-whisper:<dir>``.

When both phoneme streams of a sentence are identical the audio is
synthesised once and used for both sides (the model input is the same).
Only aggregate numbers and a few example lines are written.
"""

from __future__ import annotations

import argparse
import json
import random
import re
import time
import unicodedata
from pathlib import Path

from common import VOICES, load_oracle, read_tsv, run_uccharan

DATA = Path(__file__).resolve().parent / "data"


def reference_codepoints(oracle, text: str, voice: str) -> list[list[int]]:
    """Piper-shaped reference stream: clauses joined with their terminators,
    one list per sentence."""
    sents: list[str] = []
    cur = ""
    for ipa, term in oracle.clauses(text, voice):
        ipa = re.sub(r"\([a-z-]+\)", "", ipa)
        cur += ipa + term
        if term in ".!?":
            sents.append(cur)
            cur = ""
        else:
            cur += " "
    if cur.strip():
        sents.append(cur.strip())
    return [[ord(c) for c in unicodedata.normalize("NFD", s).replace("g", "ɡ")] for s in sents]


def uccharan_codepoints(texts: list[str], lang: str, binary: str | None) -> list[list[list[int]]]:
    out = run_uccharan(texts, lang, "sherpa-onnx", punctuation=True, binary=binary)
    return [[[int(x) for x in s.split()] for s in line.split(" | ") if s.strip()] for line in out]


def normalise(text: str, lang: str) -> str:
    text = unicodedata.normalize("NFC", text).lower()
    if lang == "hi":
        text = text.replace("़", "").replace("ँ", "ं").replace("।", " ")
        text = re.sub(r"[^ऀ-ॿ\s]", " ", text)
    elif lang.startswith("en"):
        text = text.replace("’", "'")
        text = re.sub(r"[^a-z0-9'\s]", " ", text)
    else:
        # letters (with their marks) and digits of any script
        text = text.replace("’", "'").replace("।", " ")
        text = "".join(c if unicodedata.category(c)[0] in "LMN" or c == "'" else " " for c in text)
    return " ".join(text.split())


def pick_sentences(lang: str, n: int) -> list[tuple[str, str]]:
    base = "en" if lang.startswith("en") else "fr" if lang.startswith("fr") else lang
    rows = read_tsv(DATA / f"{base}_heldout.tsv")
    if base == "en":
        rows = [r for r in rows if 30 <= len(r[1]) <= 120 and not re.search(r"[0-9]", r[1]) and r[1].isascii()]
    elif re.search(r"[A-Za-z]", "".join(t for _, t in rows[:50])) and not re.search(r"[\u0900-\u0dff\u0600-\u06ff]", "".join(t for _, t in rows[:50])):
        # Latin-script languages: sentences without digits
        rows = [r for r in rows if 30 <= len(r[1]) <= 160 and not re.search(r"[0-9]", r[1])]
    else:
        # other scripts: no Latin letters and no digits of any script
        rows = [r for r in rows if 15 <= len(r[1]) <= 160 and not re.search(r"[A-Za-z\d]", r[1])]
    random.Random(7).shuffle(rows)
    return rows[:n]


def asr_language(lang: str) -> str:
    """Whisper's code for a language tag (en-us -> en, pt-br -> pt; Bhojpuri,
    Maithili, Magahi, Dogri and Bodo are transcribed as Hindi)."""
    return "hi" if lang in ("bho", "mai", "mag", "doi", "brx") else lang.split("-")[0]


def make_asr(spec: str, lang: str):
    kind, path = spec.split(":", 1)
    if kind == "whisper-onnx":
        import numpy as np
        import sherpa_onnx

        rec = sherpa_onnx.OfflineRecognizer.from_whisper(
            encoder=f"{path}-encoder.int8.onnx", decoder=f"{path}-decoder.int8.onnx", tokens=f"{path}-tokens.txt",
            num_threads=2, language=asr_language(lang), task="transcribe")

        def run(samples, rate):
            s = rec.create_stream()
            s.accept_waveform(rate, np.asarray(samples, dtype=np.float32))
            rec.decode_stream(s)
            return s.result.text

        return run
    if kind == "faster-whisper":
        import numpy as np
        from faster_whisper import WhisperModel

        model = WhisperModel(path, device="cpu", compute_type="int8", cpu_threads=2)

        def run(samples, rate):
            x = np.asarray(samples, dtype=np.float32)
            if rate != 16000:
                idx = np.linspace(0, len(x) - 1, int(len(x) * 16000 / rate))
                x = np.interp(idx, np.arange(len(x)), x).astype(np.float32)
            segs, _ = model.transcribe(x, language=asr_language(lang), beam_size=1, condition_on_previous_text=False)
            return " ".join(s.text for s in segs)

        return run
    raise SystemExit(f"unknown ASR spec {spec!r}")


def bootstrap(pairs: list[tuple[int, int, int]], rounds: int = 2000) -> tuple[float, float]:
    """95 % interval of WER(uccharan) - WER(reference); pairs are
    (errors_reference, errors_uccharan, reference_words)."""
    rng = random.Random(11)
    n = len(pairs)
    diffs = []
    for _ in range(rounds):
        er = eu = words = 0
        for _ in range(n):
            a, b, w = pairs[rng.randrange(n)]
            er += a
            eu += b
            words += w
        diffs.append((eu - er) / max(1, words))
    diffs.sort()
    return diffs[int(0.025 * rounds)], diffs[int(0.975 * rounds)]


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--lang", choices=sorted(VOICES), required=True)
    ap.add_argument("--voice", required=True, help="path of the Piper voice .onnx (tokens.txt next to it)")
    ap.add_argument("--asr", required=True)
    ap.add_argument("--n", type=int, default=500)
    ap.add_argument("--out", type=Path)
    ap.add_argument("--state", type=Path, help="JSON-lines file to resume from / append to (kept out of the repository)")
    ap.add_argument("--uccharan-bin")
    args = ap.parse_args()

    import jiwer
    import sherpa_onnx

    oracle = load_oracle()
    if oracle is None:
        raise SystemExit("reference phonemizer not installed")
    voice = Path(args.voice)
    tts = sherpa_onnx.OfflineTts(sherpa_onnx.OfflineTtsConfig(model=sherpa_onnx.OfflineTtsModelConfig(
        vits=sherpa_onnx.OfflineTtsVitsModelConfig(model=str(voice), tokens=str(voice.parent / "tokens.txt")), num_threads=2)))
    asr = make_asr(args.asr, args.lang)
    rows = pick_sentences(args.lang, args.n)
    texts = [t for _, t in rows]
    ours = uccharan_codepoints(texts, args.lang, args.uccharan_bin)

    done: dict[str, dict] = {}
    if args.state and args.state.exists():
        for line in args.state.read_text(encoding="utf-8").splitlines():
            rec = json.loads(line)
            done[rec["id"]] = rec
    state = args.state.open("a", encoding="utf-8") if args.state else None

    def synth(cps: list[list[int]], text: str):
        g = sherpa_onnx.GenerationConfig()
        g.phoneme_codepoints = cps
        a = tts.generate(text, g)
        return a.samples, a.sample_rate

    t0 = time.time()
    records = []
    for k, ((sid, text), u) in enumerate(zip(rows, ours)):
        if sid in done:
            records.append(done[sid])
            continue
        r = reference_codepoints(oracle, text, VOICES[args.lang])
        same = r == u
        sr, rate = synth(r, text)
        hyp_r = asr(sr, rate)
        if same:
            hyp_u = hyp_r
        else:
            su, rate = synth(u, text)
            hyp_u = asr(su, rate)
        rec = {"id": sid, "same_phonemes": same, "ref_text": normalise(text, args.lang),
               "asr_reference": normalise(hyp_r, args.lang), "asr_uccharan": normalise(hyp_u, args.lang), "seconds_audio": len(sr) / rate}
        records.append(rec)
        if state:
            state.write(json.dumps(rec, ensure_ascii=False) + "\n")
            state.flush()
        if (k + 1) % 25 == 0:
            print(f"{k + 1}/{len(rows)}  {time.time() - t0:.0f}s", flush=True)

    records = [r for r in records if r["ref_text"]]
    refs = [r["ref_text"] for r in records]

    def score(key: str, subset=None) -> dict:
        rs = [r for r in records if subset is None or subset(r)]
        if not rs:
            return {"sentences": 0}
        return {"sentences": len(rs), "wer": jiwer.wer([r["ref_text"] for r in rs], [r[key] for r in rs]),
                "cer": jiwer.cer([r["ref_text"] for r in rs], [r[key] for r in rs])}

    pairs = []
    for r in records:
        w = len(r["ref_text"].split())
        ea = round(jiwer.wer(r["ref_text"], r["asr_reference"]) * w)
        eb = round(jiwer.wer(r["ref_text"], r["asr_uccharan"]) * w)
        pairs.append((ea, eb, w))
    lo, hi = bootstrap(pairs)
    differ = lambda r: not r["same_phonemes"]  # noqa: E731
    res = {
        "language": args.lang,
        "voice": voice.name,
        "tts": f"sherpa-onnx {sherpa_onnx.__version__} (phoneme_codepoints)",
        "asr": args.asr.split(":")[0] + ":" + Path(args.asr.split(":", 1)[1]).name,
        "reference": f"espeak-ng {oracle.version()} voice {VOICES[args.lang]}",
        "sentences": len(records),
        "audio_minutes": sum(r["seconds_audio"] for r in records) / 60,
        "sentences_with_identical_phonemes": sum(r["same_phonemes"] for r in records),
        "reference_phonemes": score("asr_reference"),
        "uccharan_phonemes": score("asr_uccharan"),
        "wer_difference_uccharan_minus_reference": score("asr_uccharan")["wer"] - score("asr_reference")["wer"],
        "wer_difference_95ci_paired_bootstrap": [lo, hi],
        "subset_where_phonemes_differ": {"reference_phonemes": score("asr_reference", differ), "uccharan_phonemes": score("asr_uccharan", differ)},
        "better_worse_same_sentences": {
            "uccharan_better": sum(b < a for a, b, _ in pairs),
            "uccharan_worse": sum(b > a for a, b, _ in pairs),
            "same": sum(a == b for a, b, _ in pairs),
        },
    }
    print(json.dumps(res, ensure_ascii=False, indent=1))
    if args.out:
        args.out.mkdir(parents=True, exist_ok=True)
        (args.out / f"e2e_{args.lang}_{voice.stem}.json").write_text(json.dumps(res, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
