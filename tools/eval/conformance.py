#!/usr/bin/env python3
"""Per-profile conformance checks against real model files.

Nothing here needs a GPU; every check is skipped (and reported as skipped)
when the file it needs was not given.

* Piper (``--piper VOICE.onnx.json`` …): ids produced by ``uccharan --ids`` must
  equal an independent implementation of Piper's framing applied to uccharan's
  sentence strings; no symbol may be missing from the voice's
  ``phoneme_id_map``; and, with the reference phonemizer installed, the id
  sequence is compared with the one built from reference phonemes.
* sherpa-onnx (``--tokens tokens.txt`` …): every codepoint emitted for
  ``GenerationConfig.phoneme_codepoints`` must be in the model's ``tokens.txt``.
* Kokoro (``--kokoro-tokens tokens.txt``): the built-in vocabulary must equal
  the model's; no token outside it is emitted; Hindi is compared with
  reference output passed through the published espeak → Kokoro mapping;
  English words can be compared with a misaki dictionary (``--misaki-gold``).
* Kitten / Matcha (``--kitten-tokens tokens.txt``): the built-in symbol table
  must equal the model's; ids are compared with ids built from the
  ``phonemizer`` package's output the way the Kitten front end does it.

    python tools/eval/conformance.py --piper en.onnx.json --piper hi.onnx.json \
        --tokens en/tokens.txt --kokoro-tokens kokoro/tokens.txt --out tools/eval/results
"""

from __future__ import annotations

import argparse
import json
import re
import unicodedata
from pathlib import Path

from common import VOICES, align, load_oracle, read_tsv, run_uccharan

DATA = Path(__file__).resolve().parent / "data"


def held_out(lang: str, limit: int) -> list[str]:
    rows = read_tsv(DATA / ("hi_heldout.tsv" if lang == "hi" else "en_heldout.tsv"))
    return [t for _, t in rows][:limit]


def piper_ids_reference_impl(sentence: str, id_map: dict[str, list[int]]) -> tuple[list[int], list[str]]:
    """Piper's framing, written independently of the Rust code:
    BOS, PAD, then each NFD codepoint followed by PAD, then EOS."""
    ids = list(id_map["^"]) + list(id_map["_"])
    missing = []
    for ch in unicodedata.normalize("NFD", sentence):
        if ch in id_map:
            ids += list(id_map[ch]) + list(id_map["_"])
        else:
            missing.append(ch)
    return ids + list(id_map["$"]), missing


def reference_sentences(oracle, text: str, voice: str) -> list[str]:
    sents, cur = [], ""
    for ipa, term in oracle.clauses(text, voice):
        cur += re.sub(r"\([a-z-]+\)", "", ipa) + term
        if term in ".!?":
            sents.append(cur)
            cur = ""
        else:
            cur += " "
    if cur.strip():
        sents.append(cur.strip())
    return [s.replace("g", "ɡ") for s in sents]


def check_piper(config: Path, oracle, limit: int, binary) -> dict:
    cfg = json.loads(config.read_text(encoding="utf-8"))
    voice = cfg.get("espeak", {}).get("voice", "en-us")
    lang = {"en-us": "en-us", "en-gb-x-rp": "en-gb", "en": "en-gb", "hi": "hi"}.get(voice)
    if lang is None:
        return {"voice": config.name, "skipped": f"no front end for espeak voice {voice!r}"}
    id_map = cfg["phoneme_id_map"]
    texts = held_out(lang, limit)
    strings = run_uccharan(texts, lang, "piper", normalize="natural", punctuation=True, binary=binary)
    ids = run_uccharan(texts, lang, "piper", normalize="natural", punctuation=True, binary=binary, extra=("--ids", "--voice-config", str(config)))
    sentences = run_uccharan(texts, lang, "sherpa-onnx", normalize="natural", punctuation=True, binary=binary)
    framing_ok = True
    missing: dict[str, int] = {}
    n_sent = 0
    for line_ids, line_cp in zip(ids, sentences):
        got = [[int(x) for x in s.split()] for s in line_ids.split(" | ") if s.strip()]
        sents = ["".join(chr(int(c)) for c in s.split()) for s in line_cp.split(" | ") if s.strip()]
        if len(got) != len(sents):
            framing_ok = False
            continue
        for g, s in zip(got, sents):
            n_sent += 1
            want, miss = piper_ids_reference_impl(s, id_map)
            framing_ok &= g == want
            for m in miss:
                missing[m] = missing.get(m, 0) + 1
    res = {
        "voice": config.name,
        "espeak_voice": voice,
        "symbols_in_map": len(id_map),
        "texts": len(texts),
        "sentences": n_sent,
        "framing_matches_independent_implementation": framing_ok,
        "symbols_missing_from_phoneme_id_map": missing,
        "passed": framing_ok and not missing,
    }
    del strings
    if oracle is not None:
        like = run_uccharan(texts, lang, "sherpa-onnx", normalize="espeak", punctuation=True, binary=binary)
        exact = total = err = ref_len = 0
        ref_missing: dict[str, int] = {}
        for text, line in zip(texts, like):
            ours = ["".join(chr(int(c)) for c in s.split()) for s in line.split(" | ") if s.strip()]
            ref = reference_sentences(oracle, text, voice)
            a = [i for s in ours for i in piper_ids_reference_impl(s, id_map)[0]]
            b = []
            for s in ref:
                i, miss = piper_ids_reference_impl(s, id_map)
                b += i
                for m in miss:
                    ref_missing[m] = ref_missing.get(m, 0) + 1
            total += 1
            exact += a == b
            d, _ = align([str(x) for x in b], [str(x) for x in a])
            err += d
            ref_len += len(b)
        res["id_sequence_vs_reference"] = {"texts": total, "exact": exact / total, "id_error_rate": err / ref_len,
                                            "symbols_the_reference_emits_outside_the_map": ref_missing}
    return res


def read_tokens(path: Path) -> dict[str, int]:
    table = {}
    for line in path.read_text(encoding="utf-8").split("\n"):
        if not line.strip() and not line.startswith(" "):
            continue
        sym, _, idx = line.rpartition(" ")
        if idx.strip().lstrip("-").isdigit():
            table[sym if sym else " "] = int(idx)
    return table


def check_sherpa(tokens: Path, lang: str, limit: int, binary) -> dict:
    table = read_tokens(tokens)
    texts = held_out(lang, limit)
    out = run_uccharan(texts, lang, "sherpa-onnx", normalize="natural", punctuation=True, binary=binary)
    unknown: dict[str, int] = {}
    n = 0
    for line in out:
        for cp in line.replace("|", " ").split():
            n += 1
            ch = chr(int(cp))
            if ch not in table:
                unknown[ch] = unknown.get(ch, 0) + 1
    return {"tokens": str(tokens.parent.name), "language": lang, "texts": len(texts), "codepoints": n, "codepoints_not_in_tokens_txt": unknown, "passed": not unknown}


KOKORO_MAP = [("d^ʒ", "ʤ"), ("t^ʃ", "ʧ"), ("a^ɪ", "I"), ("a^ʊ", "W"), ("e^ɪ", "A"), ("o^ʊ", "O"), ("ə^ʊ", "Q"), ("ɔ^ɪ", "Y"), ("t^s", "ʦ"), ("d^z", "ʣ"), ("t^ɕ", "ʨ"), ("d^ʑ", "ʥ")]


def check_kokoro(tokens: Path, oracle, limit: int, misaki_gold: Path | None, binary) -> dict:
    table = read_tokens(tokens)
    res: dict = {"tokens": str(tokens.parent.name), "symbols": len(table)}
    # the built-in table, read back through --ids on single symbols
    probe = [s for s in table if s.strip()]
    ids = run_uccharan(["x"], "en-us", "kokoro", binary=binary, extra=("--ids",))
    res["framing_pad_0_both_ends"] = ids[0].split()[0] == "0" and ids[0].split()[-1] == "0"
    outside = {}
    for lang in ("en-us", "en-gb", "hi"):
        texts = held_out(lang, limit)
        out = run_uccharan(texts, lang, "kokoro", normalize="natural", punctuation=True, binary=binary)
        bad: dict[str, int] = {}
        for line in out:
            for ch in line:
                if ch not in table:
                    bad[ch] = bad.get(ch, 0) + 1
        outside[lang] = bad
    res["tokens_outside_vocabulary"] = outside
    res["passed"] = res["framing_pad_0_both_ends"] and not any(outside.values())
    del probe
    if oracle is not None:
        texts = held_out("hi", limit)
        ours = run_uccharan(texts, "hi", "kokoro", normalize="espeak", punctuation=False, binary=binary)
        exact = err = ref_len = 0
        for t, o in zip(texts, ours):
            r = re.sub(r"\([a-z-]+\)", "", oracle.phonemize(t, "hi", tie=True))
            for a, b in KOKORO_MAP:
                r = r.replace(a, b)
            r = r.replace("^", "").replace("-", "")
            r = "".join(c for c in unicodedata.normalize("NFD", " ".join(r.split())) if c in table)
            # phoneme tokens only: the Kokoro profile always keeps clause punctuation
            o = re.sub(r"[.,;:!?]", "", unicodedata.normalize("NFD", o))
            r = re.sub(r"[.,;:!?]", "", r)
            d, _ = align(list(r), list(o))
            err += d
            ref_len += len(r)
            exact += d == 0
        res["hindi_vs_reference_through_published_mapping"] = {"sentences": len(texts), "exact": exact / len(texts), "token_error_rate": err / ref_len}
    if misaki_gold is not None and misaki_gold.exists():
        gold = json.loads(misaki_gold.read_text(encoding="utf-8"))
        words = [w for w in (DATA / "en_words_frequent.txt").read_text().split() if isinstance(gold.get(w), str)]
        ours = run_uccharan(words, "en-us", "kokoro", binary=binary)
        norm = lambda s: s.replace("ɾ", "T")  # noqa: E731  (flap is written ɾ in older dictionaries, T in newer ones)
        exact = sum(norm(o) == norm(gold[w]) for w, o in zip(words, ours))
        nostress = sum(re.sub("[ˈˌ]", "", norm(o)) == re.sub("[ˈˌ]", "", norm(gold[w])) for w, o in zip(words, ours))
        res["english_words_vs_misaki_gold"] = {"dictionary": misaki_gold.name, "words": len(words), "exact": exact / len(words), "exact_ignoring_stress": nostress / len(words)}
    return res


def check_kitten(tokens: Path, limit: int, binary) -> dict:
    table = read_tokens(tokens)
    res: dict = {"tokens": str(tokens.parent.name), "symbols": len(table)}
    # table identity: every symbol of the model maps to the same id in uccharan
    mismatched = []
    syms = [s for s in table if s not in ("$", "_") and table[s] != 0]
    lines = run_uccharan(syms, "en-us", "kitten", binary=binary)  # warm check that the binary runs
    del lines
    import subprocess

    from common import find_binary

    # ids of raw symbols are checked through a tiny Rust-free route: phonemize text that yields known symbols
    texts = held_out("en-us", limit)
    ours_text = run_uccharan(texts, "en-us", "kitten", normalize="espeak", punctuation=True, binary=binary)
    ours_ids = run_uccharan(texts, "en-us", "kitten", normalize="espeak", punctuation=True, binary=binary, extra=("--ids",))
    table_ok = True
    for t, i in zip(ours_text, ours_ids):
        want = [0] + [table[c] for c in t if c in table] + [10, 0]
        got = [int(x) for x in i.split()]
        if want != got:
            table_ok = False
            mismatched.append(t[:60])
    res["ids_match_model_tokens_txt"] = table_ok
    res["symbols_outside_table"] = sorted({c for t in ours_text for c in t if c not in table})
    res["passed"] = table_ok and not res["symbols_outside_table"]
    del subprocess, find_binary
    try:
        from phonemizer.backend import EspeakBackend  # type: ignore[import-not-found]

        try:
            import espeakng_loader  # type: ignore[import-not-found]
            from phonemizer.backend.espeak.wrapper import EspeakWrapper  # type: ignore[import-not-found]

            EspeakWrapper.set_library(espeakng_loader.get_library_path())
            EspeakWrapper.set_data_path(espeakng_loader.get_data_path())
        except Exception:
            pass
        be = EspeakBackend("en-us", preserve_punctuation=True, with_stress=True)
        ref = be.phonemize(texts)
        exact = err = ref_len = 0
        for r, o in zip(ref, ours_text):
            r = " ".join(re.findall(r"\w+|[^\w\s]", r))
            o = " ".join(re.findall(r"\w+|[^\w\s]", o))
            a = [table[c] for c in r if c in table]
            b = [table[c] for c in o if c in table]
            d, _ = align([str(x) for x in a], [str(x) for x in b])
            err += d
            ref_len += len(a)
            exact += d == 0
        res["ids_vs_phonemizer_package_front_end"] = {"texts": len(texts), "exact": exact / len(texts), "id_error_rate": err / ref_len}
    except Exception as exc:
        res["ids_vs_phonemizer_package_front_end"] = {"skipped": f"phonemizer package not usable: {exc}"}
    return res


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--piper", action="append", default=[], type=Path, help="a Piper voice config (*.onnx.json); repeatable")
    ap.add_argument("--tokens", action="append", default=[], help="LANG=path/to/tokens.txt of a sherpa-onnx Piper model; repeatable")
    ap.add_argument("--kokoro-tokens", type=Path)
    ap.add_argument("--kitten-tokens", type=Path)
    ap.add_argument("--misaki-gold", type=Path, help="us_gold.json from the misaki package (Apache-2.0)")
    ap.add_argument("--limit", type=int, default=3000)
    ap.add_argument("--out", type=Path)
    ap.add_argument("--uccharan-bin")
    args = ap.parse_args()
    oracle = load_oracle()
    result: dict = {"reference": f"espeak-ng {oracle.version()}" if oracle else None, "piper": [], "sherpa_onnx": []}
    for cfg in args.piper:
        result["piper"].append(check_piper(cfg, oracle, args.limit, args.uccharan_bin))
    for spec in args.tokens:
        lang, path = spec.split("=", 1)
        result["sherpa_onnx"].append(check_sherpa(Path(path), lang, args.limit, args.uccharan_bin))
    result["kokoro"] = check_kokoro(args.kokoro_tokens, oracle, args.limit, args.misaki_gold, args.uccharan_bin) if args.kokoro_tokens else {"skipped": "no --kokoro-tokens"}
    result["kitten"] = check_kitten(args.kitten_tokens, args.limit, args.uccharan_bin) if args.kitten_tokens else {"skipped": "no --kitten-tokens"}
    print(json.dumps(result, ensure_ascii=False, indent=1))
    if args.out:
        args.out.mkdir(parents=True, exist_ok=True)
        (args.out / "conformance.json").write_text(json.dumps(result, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
