#!/usr/bin/env python3
"""Recogniser confusions for the sound-alike evaluation.

Sentences of ``data/items.jsonl`` are spoken by synthetic voices
(sherpa-onnx >= 2.0 with phonemes from uccharan; two voices for personal
names in English sentences, one otherwise; half of the sentences without a
vocabulary term) and transcribed by small speech recognisers. What the recogniser wrote in place of a vocabulary
term is the kind of error the matcher has to undo.

    python tools/eval/soundalike/make_confusions.py --models /path/to/models \
        --shard 0/2 --out tools/eval/soundalike/data/raw-0.jsonl

``--models`` is a directory with the sherpa-onnx exports used here:
``piper-en-us-amy-low``, ``piper-hi-in-rohan-medium``, ``kokoro-v1.0-int8``
(voices), ``whisper-tiny``, ``whisper-base``, ``moonshine-tiny-en``
(recognisers). Missing models are skipped. The run can be stopped and
started again: finished lines are kept. ``--merge raw-0.jsonl raw-1.jsonl
--out data/confusions.jsonl`` joins the shards.

The voices read uccharan's own phonemes (improved mode; the Hinglish option
for romanised Hindi), so the pronunciation of a term is uccharan's reading
of its spelling, or of its spoken form when the term set gives one. The
recognisers are independent of uccharan.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]

# Voices per context: (name, model directory, kind, speaker id).
VOICES = {
    "en": [("amy", "piper-en-us-amy-low", "piper", 0), ("kokoro-3", "kokoro-v1.0-int8", "kokoro", 3),
           ("kokoro-11", "kokoro-v1.0-int8", "kokoro", 11), ("kokoro-24", "kokoro-v1.0-int8", "kokoro", 24)],
    "hi": [("rohan", "piper-hi-in-rohan-medium", "piper", 0), ("kokoro-31", "kokoro-v1.0-int8", "kokoro", 31),
           ("kokoro-33", "kokoro-v1.0-int8", "kokoro", 33)],
}
VOICES["hing"] = VOICES["hi"]
# Recognisers per context: (name, model, language).
ASRS = {
    "en": [("whisper-tiny", "whisper-tiny", "en"), ("whisper-base", "whisper-base", "en"), ("moonshine-tiny", "moonshine-tiny-en", "en")],
    "hi": [("whisper-base-hi", "whisper-base", "hi")],
    "hing": [("whisper-base-hi", "whisper-base", "hi"), ("whisper-base-en", "whisper-base", "en")],
}


def find_binary(explicit: str | None) -> str:
    for c in (explicit, os.environ.get("UCCHARAN_BIN"), str(ROOT / "target" / "release" / "uccharan"), shutil.which("uccharan")):
        if c and Path(c).exists():
            return c
    raise SystemExit("uccharan binary not found: run `cargo build --release` or set UCCHARAN_BIN")


def phonemes(binary: str, lines: list[str], lang: str, profile: str, extra: tuple[str, ...]) -> list[str]:
    args = [binary, "-l", lang, "-p", profile, "--improved", *extra]
    out = subprocess.run(args, input="\n".join(lines) + "\n", capture_output=True, text=True, check=True).stdout.split("\n")
    return out[: len(lines)]


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--models", type=Path)
    ap.add_argument("--merge", type=Path, nargs="+", help="join shard outputs into --out (in the order of the items) and stop")
    ap.add_argument("--items", type=Path, default=HERE / "data" / "items.jsonl")
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--shard", default="0/1", help="K/N: take every Nth item starting at K")
    ap.add_argument("--limit", type=int, default=0)
    ap.add_argument("--threads", type=int, default=2)
    ap.add_argument("--uccharan-bin")
    args = ap.parse_args()
    if args.merge:
        order = {json.loads(line)["id"]: n for n, line in enumerate(args.items.read_text(encoding="utf-8").splitlines())}
        source = {json.loads(line)["id"]: json.loads(line).get("source") for line in args.items.read_text(encoding="utf-8").splitlines()}
        rows = {}
        for f in args.merge:
            for line in f.read_text(encoding="utf-8").splitlines():
                r = json.loads(line)
                if source.get(r["id"]):
                    r["source"] = source[r["id"]]
                rows[(order[r["id"]], r["voice"])] = r
        with args.out.open("w", encoding="utf-8") as f:
            for key in sorted(rows):
                f.write(json.dumps(rows[key], ensure_ascii=False) + "\n")
        print(len(rows), "utterances")
        return 0
    if not args.models:
        ap.error("--models is required")
    import numpy as np
    import sherpa_onnx as so

    binary = find_binary(args.uccharan_bin)
    k, n = (int(x) for x in args.shard.split("/"))
    items = [json.loads(line) for line in args.items.read_text(encoding="utf-8").splitlines()]
    # A fixed pseudo-random order, so that a run that is stopped early has
    # covered a balanced sample.
    items.sort(key=lambda it: hashlib.sha1(it["id"].encode()).hexdigest())
    items = [it for it in items if it["kind"] != "none" or int(hashlib.sha1(it["id"].encode()).hexdigest(), 16) % 2 == 0]
    items = [it for i, it in enumerate(items) if i % n == k]
    if args.limit:
        items = items[: args.limit]
    done = set()
    if args.out.exists():
        for line in args.out.read_text(encoding="utf-8").splitlines():
            r = json.loads(line)
            done.add((r["id"], r["voice"]))

    # Phonemes for every item, per voice kind.
    ph: dict[tuple[str, str], str] = {}
    for ctx, lang, extra in (("en", "en-us", ()), ("hi", "hi", ()), ("hing", "hi", ("--hinglish",))):
        sub = [it for it in items if it["ctx"] == ctx]
        for profile in ("sherpa-onnx", "kokoro"):
            for it, line in zip(sub, phonemes(binary, [it["tts"] for it in sub], lang, profile, extra)):
                ph[(it["id"], profile)] = line

    tts_cache: dict[str, object] = {}
    kokoro_vocab: set[str] = set()

    def tts(model: str, kind: str):
        if model not in tts_cache:
            p = args.models / model
            if not p.is_dir():
                tts_cache[model] = None
            elif kind == "piper":
                onnx = next(x for x in p.glob("*.onnx"))
                cfg = so.OfflineTtsModelConfig(vits=so.OfflineTtsVitsModelConfig(model=str(onnx), tokens=str(p / "tokens.txt")), num_threads=args.threads)
                tts_cache[model] = so.OfflineTts(so.OfflineTtsConfig(model=cfg))
            else:
                cfg = so.OfflineTtsModelConfig(kokoro=so.OfflineTtsKokoroModelConfig(
                    model=str(next(p.glob("model*.onnx"))), voices=str(p / "voices.bin"), tokens=str(p / "tokens.txt")), num_threads=args.threads)
                tts_cache[model] = so.OfflineTts(so.OfflineTtsConfig(model=cfg))
                for line in (p / "tokens.txt").read_text(encoding="utf-8").split("\n"):
                    if line:
                        kokoro_vocab.add(" " if line.startswith(" ") else line.rsplit(" ", 1)[0])
        return tts_cache[model]

    asr_cache: dict[tuple[str, str], object] = {}

    def asr(model: str, lang: str):
        key = (model, lang)
        if key not in asr_cache:
            p = args.models / model
            if not p.is_dir():
                asr_cache[key] = None
            elif model.startswith("whisper"):
                size = model.split("-", 1)[1]
                asr_cache[key] = so.OfflineRecognizer.from_whisper(
                    encoder=str(p / f"{size}-encoder.int8.onnx"), decoder=str(p / f"{size}-decoder.int8.onnx"), tokens=str(p / f"{size}-tokens.txt"),
                    language=lang, task="transcribe", num_threads=args.threads)
            else:
                asr_cache[key] = so.OfflineRecognizer.from_moonshine(
                    preprocessor=str(p / "preprocess.onnx"), encoder=str(p / "encode.int8.onnx"), uncached_decoder=str(p / "uncached_decode.int8.onnx"),
                    cached_decoder=str(p / "cached_decode.int8.onnx"), tokens=str(p / "tokens.txt"), num_threads=args.threads)
        return asr_cache[key]

    out = args.out.open("a", encoding="utf-8")
    t0 = time.time()
    made = 0
    for idx, it in enumerate(items):
        ctx = it["ctx"]
        voices = VOICES[ctx]
        h = int(hashlib.sha1(it["id"].encode()).hexdigest(), 16)
        count = 2 if ctx == "en" and it["kind"] == "name" else 1
        chosen = [voices[(h + j * (1 + h // 7 % (len(voices) - 1))) % len(voices)] for j in range(count)]
        chosen = list(dict.fromkeys(chosen))
        for name, model, kind, sid in chosen:
            if (it["id"], name) in done:
                continue
            engine = tts(model, kind)
            if engine is None:
                continue
            g = so.GenerationConfig()
            if kind == "piper":
                line = ph[(it["id"], "sherpa-onnx")]
                g.phoneme_codepoints = [[int(x) for x in s.split()] for s in line.split(" | ") if s.strip()]
            else:
                g.sid = sid
                g.phoneme_codepoints = [[ord(c) for c in ph[(it["id"], "kokoro")] if c in kokoro_vocab]]
            if not g.phoneme_codepoints or not g.phoneme_codepoints[0]:
                continue
            audio = engine.generate(it["tts"], g)
            samples = np.asarray(audio.samples, dtype=np.float32)
            if samples.size == 0:
                continue
            asrs = ASRS[ctx]
            if it["kind"] == "none" and ctx == "en":
                asrs = [asrs[h % len(asrs)], asrs[(h + 1) % len(asrs)]]
            hyps = {}
            for aname, amodel, alang in asrs:
                rec = asr(amodel, alang)
                if rec is None:
                    continue
                s = rec.create_stream()
                s.accept_waveform(audio.sample_rate, samples)
                rec.decode_stream(s)
                hyps[aname] = s.result.text.strip()
            row = {key: it[key] for key in ("id", "ctx", "kind", "split", "ref") if key in it}
            for key in ("term", "hi", "say"):
                if key in it:
                    row[key] = it[key]
            row.update({"voice": name, "hyp": hyps})
            out.write(json.dumps(row, ensure_ascii=False) + "\n")
            out.flush()
            made += 1
        if (idx + 1) % 50 == 0:
            print(f"{idx + 1}/{len(items)} items, {made} utterances, {time.time() - t0:.0f}s", file=sys.stderr, flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
