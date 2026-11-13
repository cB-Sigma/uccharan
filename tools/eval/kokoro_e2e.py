#!/usr/bin/env python3
"""End-to-end check for Kokoro: synthesise sentences through a Kokoro model
with sherpa-onnx (`GenerationConfig.phoneme_codepoints`) from two front ends
and compare speech-recognition word error rates.

* English: misaki's tokens (written by `kokoro_eval.py --dump`) against
  uccharan's Kokoro English front end (`--misaki DIR` for its dictionaries).
* Hindi: the reference phonemizer's `hi` output through the published
  espeak → Kokoro mapping against uccharan's `kokoro` profile.

    python tools/eval/kokoro_e2e.py --model DIR --lang en-us --reference dump.en-us.jsonl --misaki DIR \
        --asr whisper-onnx:/models/whisper-base/base --sid 3 --n 300 --out tools/eval/results
"""

from __future__ import annotations

import argparse
import json
import random
import re
import time
from pathlib import Path

from common import VOICES, load_oracle, read_tsv, run_uccharan
from conformance import KOKORO_MAP
from e2e_tts_asr import bootstrap, make_asr, normalise

DATA = Path(__file__).resolve().parent / "data"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--model", type=Path, required=True, help="sherpa-onnx Kokoro model directory (model*.onnx, voices.bin, tokens.txt)")
    ap.add_argument("--lang", choices=["en-us", "en-gb", "hi"], required=True)
    ap.add_argument("--reference", type=Path, help="misaki output (JSON lines) for English")
    ap.add_argument("--misaki", type=Path, help="make uccharan use misaki dictionaries from this directory instead of its built-in copy")
    ap.add_argument("--asr", required=True)
    ap.add_argument("--sid", type=int, default=0)
    ap.add_argument("--n", type=int, default=300)
    ap.add_argument("--out", type=Path)
    ap.add_argument("--uccharan-bin")
    args = ap.parse_args()
    import jiwer
    import sherpa_onnx

    model = next(args.model.glob("model*.onnx"))
    tts = sherpa_onnx.OfflineTts(sherpa_onnx.OfflineTtsConfig(model=sherpa_onnx.OfflineTtsModelConfig(
        kokoro=sherpa_onnx.OfflineTtsKokoroModelConfig(model=str(model), voices=str(args.model / "voices.bin"), tokens=str(args.model / "tokens.txt")),
        num_threads=2)))
    vocab = {line.rsplit(" ", 1)[0] if not line.startswith(" ") else " " for line in (args.model / "tokens.txt").read_text(encoding="utf-8").split("\n") if line}
    if args.lang == "hi":
        rows = [r for r in read_tsv(DATA / "hi_heldout.tsv") if 15 <= len(r[1]) <= 120 and not re.search(r"[A-Za-z0-9०-९]", r[1])]
        random.Random(7).shuffle(rows)
        rows = rows[: args.n]
        oracle = load_oracle()
        refs = []
        for _, t in rows:
            r = re.sub(r"\([a-z-]+\)", "", oracle.phonemize(t, VOICES["hi"], tie=True))
            for a, b in KOKORO_MAP:
                r = r.replace(a, b)
            refs.append(r.replace("^", "").replace("-", "") + "।"[:0] + ("." if not r.endswith((".", "?", "!")) else ""))
        items = [{"id": i, "text": t, "ref": r} for (i, t), r in zip(rows, refs)]
        ours = run_uccharan([x["text"] for x in items], "hi", "kokoro", normalize="natural", punctuation=True, binary=args.uccharan_bin)
    else:
        items = [json.loads(line) for line in args.reference.read_text(encoding="utf-8").splitlines()]
        items = [x for x in items if 30 <= len(x["text"]) <= 120 and not re.search(r"[0-9]", x["text"]) and x["text"].isascii()]
        random.Random(7).shuffle(items)
        items = items[: args.n]
        for x in items:
            x["ref"] = x["misaki"]
        ours = run_uccharan([x["text"] for x in items], args.lang, "kokoro", punctuation=True, binary=args.uccharan_bin, extra=("--misaki", str(args.misaki)) if args.misaki else ())
    asr = make_asr(args.asr, args.lang)

    def synth(tokens: str, text: str):
        g = sherpa_onnx.GenerationConfig()
        g.sid = args.sid
        g.phoneme_codepoints = [[ord(c) for c in tokens if c in vocab]]
        a = tts.generate(text, g)
        return a.samples, a.sample_rate

    records = []
    t0 = time.time()
    for k, (x, o) in enumerate(zip(items, ours)):
        same = x["ref"].strip() == o.strip()
        sr, rate = synth(x["ref"], x["text"])
        hr = asr(sr, rate)
        hu = hr if same else asr(*synth(o, x["text"]))
        records.append({"same": same, "ref_text": normalise(x["text"], args.lang), "asr_reference": normalise(hr, args.lang), "asr_uccharan": normalise(hu, args.lang),
                        "seconds": len(sr) / rate})
        if (k + 1) % 25 == 0:
            print(f"{k + 1}/{len(items)} {time.time() - t0:.0f}s", flush=True)
    records = [r for r in records if r["ref_text"]]
    refs_t = [r["ref_text"] for r in records]
    pairs = []
    for r in records:
        w = len(r["ref_text"].split())
        pairs.append((round(jiwer.wer(r["ref_text"], r["asr_reference"]) * w), round(jiwer.wer(r["ref_text"], r["asr_uccharan"]) * w), w))
    lo, hi = bootstrap(pairs)
    wer_r = jiwer.wer(refs_t, [r["asr_reference"] for r in records])
    wer_u = jiwer.wer(refs_t, [r["asr_uccharan"] for r in records])
    res = {
        "language": args.lang, "model": args.model.name, "sid": args.sid, "tts": f"sherpa-onnx {sherpa_onnx.__version__} (phoneme_codepoints)",
        "asr": args.asr.split(":")[0] + ":" + Path(args.asr.split(":", 1)[1]).name,
        "reference_front_end": "misaki 0.9.4 (spaCy en_core_web_sm, espeak-ng fallback)" if args.lang != "hi" else "espeak-ng 1.52.0 hi + published Kokoro mapping",
        "sentences": len(records), "audio_minutes": sum(r["seconds"] for r in records) / 60,
        "identical_tokens": sum(r["same"] for r in records),
        "wer_reference": wer_r, "wer_uccharan": wer_u, "wer_difference": wer_u - wer_r, "wer_difference_95ci": [lo, hi],
        "better_worse_same": [sum(b < a for a, b, _ in pairs), sum(b > a for a, b, _ in pairs), sum(a == b for a, b, _ in pairs)],
    }
    print(json.dumps(res, indent=1, ensure_ascii=False))
    if args.out:
        args.out.mkdir(parents=True, exist_ok=True)
        (args.out / f"kokoro_e2e_{args.lang}.json").write_text(json.dumps(res, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
