"""Command line: ``uccharan-train <label|select|coverage|score|mfa-dict> …``."""

from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Sequence
from pathlib import Path

from .. import __version__
from .coverage import coverage, read_alignments
from .label import PROFILES, label
from .manifest import Utterance, from_rows, read_manifest, read_texts
from .mfa import mfa_dictionary
from .score import score, score_many
from .script import select_script
from .symbols import SymbolTable


def _common(p: argparse.ArgumentParser) -> None:
    p.add_argument("-l", "--lang", default="en-us", help="language (default en-us)")
    p.add_argument("--improved", action="store_true", help="improved mode instead of the reference-compatible readings")
    p.add_argument("--wordlist", action="append", default=[], metavar="FILE", help="pronunciation word list (TSV, PLS or JSON); may be repeated")


def _symbols(arg: str | None) -> SymbolTable | None:
    if arg is None:
        return None
    return SymbolTable.matcha() if arg == "matcha" else SymbolTable.from_file(arg)


def _wordlists(args: argparse.Namespace) -> list[str]:
    return [Path(f).read_text(encoding="utf-8") for f in args.wordlist]


def _labelled_or_manifest(path: str, layout: str | None) -> tuple[list[Utterance], list[dict[str, object]] | None]:
    """A manifest, or the ``labels.jsonl`` that ``label`` wrote."""
    if path.endswith(".jsonl"):
        with open(path, encoding="utf-8") as fh:
            rows = [json.loads(line) for line in fh if line.strip()]
        if rows and "phonemes" in rows[0]:
            return [], rows
        return from_rows(rows), None
    return read_manifest(path, layout=layout), None


def _cmd_label(args: argparse.Namespace) -> int:
    rows = read_manifest(args.manifest, layout=args.layout, text_column=args.text_column)
    replace = {}
    for item in args.replace:
        if "=" not in item:
            raise SystemExit(f"--replace expects FROM=TO, got {item!r}")
        a, b = item.split("=", 1)
        replace[a] = b
    done = label(
        rows,
        args.lang,
        profile=args.profile,
        improved=args.improved,
        normalize=args.normalize,
        hinglish=args.hinglish,
        wordlists=_wordlists(args),
        symbols=_symbols(args.symbols),
        extend=args.extend,
        replace=replace,
        stress=not args.no_stress,
    )
    for i, name in enumerate(args.wordlist):
        done.wordlists[i]["file"] = Path(name).name
    written = done.write(args.out, formats=args.format or ("piper", "matcha", "icefall"), validation=args.validation)
    warned = [u for u in done.utterances if u.warnings]
    for u in warned[: args.show]:
        for w in u.warnings:
            print(f"{args.manifest}:{u.line}: {u.id}: {w}", file=sys.stderr)
    if len(warned) > args.show:
        print(f"… {len(warned) - args.show} more utterances with warnings (see report.json)", file=sys.stderr)
    print(f"{len(done.utterances)} utterances, {len(done.symbols)} symbols, {len(warned)} with warnings; wrote {len(written)} files to {args.out}")
    if done.outside:
        print("symbols outside the target set: " + " ".join(f"{s} (U+{ord(s):04X}) ×{n}" for s, n in sorted(done.outside.items())), file=sys.stderr)
        return 2 if args.strict else 0
    return 0


def _cmd_select(args: argparse.Namespace) -> int:
    from ._phones import split_sentences

    sentences: list[str] = []
    for f in args.corpus:
        text = Path(f).read_text(encoding="utf-8-sig")
        sentences.extend(text.splitlines() if args.lines else split_sentences(text))
    s = select_script(
        sentences,
        args.lang,
        improved=args.improved,
        wordlists=_wordlists(args),
        max_sentences=args.max_sentences,
        min_words=args.min_words,
        max_words=args.max_words,
        max_chars=args.max_chars,
        allow_digits=args.allow_digits,
        allow_mixed_script=args.allow_mixed_script,
        min_symbol_sentences=args.min_symbol_sentences,
        symbols=_symbols(args.symbols),
        stress=args.stress,
        prefer=args.prefer,
    )
    if args.out:
        s.write(args.out)
    else:
        sys.stdout.write(s.text())
    st = s.stats
    ph, di = st["phones"], st["diphones"]
    print(
        f"{st['sentences']} sentences ({st['words']} words) from {st['candidates']} candidates of {st['corpus_sentences']}; "
        f"phones {ph['covered']}/{ph['total']}, diphones {di['covered']}/{di['total']}",
        file=sys.stderr,
    )
    return 0


def _cmd_coverage(args: argparse.Namespace) -> int:
    from .label import LabelledUtterance

    rows, labelled = _labelled_or_manifest(args.dataset, args.layout)
    data: object
    if labelled is not None:
        data = [
            LabelledUtterance(
                id=str(r["id"]),
                audio=str(r.get("audio", "")),
                speaker=str(r.get("speaker", "")),
                lang=str(r.get("lang") or args.lang),
                text=str(r.get("text", "")),
                normalized=str(r.get("normalized_text", "")),
                phonemes=str(r["phonemes"]),
                duration=float(r["duration"]) if r.get("duration") is not None else None,  # type: ignore[arg-type]
            )
            for r in labelled
        ]
    else:
        data = rows
    report = coverage(
        data,
        args.lang,
        improved=args.improved,
        reference=read_texts(args.reference) if args.reference else None,
        symbols=_symbols(args.symbols),
        alignments=read_alignments(args.alignments, tier=args.tier) if args.alignments else None,
        vocabulary=read_texts(args.vocabulary) if args.vocabulary else None,
        min_count=args.min_count,
        max_phones=args.max_phones,
        max_seconds=args.max_seconds,
        max_oov=args.max_oov,
        stress=args.stress,
    )
    if args.json:
        Path(args.json).write_text(report.to_json(), encoding="utf-8")
    sys.stdout.write(report.summary())
    return 0


def _cmd_score(args: argparse.Namespace) -> int:
    lists = _wordlists(args)
    if args.pairs:
        items = []
        with open(args.pairs, encoding="utf-8-sig") as fh:
            for n, line in enumerate(fh, start=1):
                if not line.strip():
                    continue
                if line.lstrip().startswith("{"):
                    items.append(json.loads(line))
                else:
                    cols = line.rstrip("\n").split("\t")
                    if len(cols) < 2:
                        raise SystemExit(f"{args.pairs}:{n}: expected reference<TAB>heard or id<TAB>reference<TAB>heard")
                    key = "heard_phonemes" if args.phonemes else "heard"
                    items.append({"id": cols[0], "reference": cols[1], key: cols[2]} if len(cols) >= 3 else {"reference": cols[0], key: cols[1]})
        result = score_many(items, lang=args.lang, heard_lang=args.heard_lang, improved=not args.compatible, threshold=args.threshold, wordlists=lists)
        if args.json:
            Path(args.json).write_text(json.dumps(result, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
        print(f"{result['count']} utterances, phoneme error {result['error_rate']:.2%} (weighted {result['weighted_error']:.2%}), mean score {result['mean_score']:.1%}, exact {result['exact']}")
        for w in result["words"][: args.show]:
            print(f"  {w['word']}: {w['errors']} of {w['occurrences']}")
        return 0
    if args.reference is None or (args.heard is None and args.heard_phonemes is None):
        raise SystemExit("score: give --pairs FILE, or --reference TEXT with --heard TEXT or --heard-phonemes IPA")
    s = score(
        args.reference,
        args.heard,
        lang=args.lang,
        heard_lang=args.heard_lang,
        heard_phonemes=args.heard_phonemes,
        improved=not args.compatible,
        threshold=args.threshold,
        wordlists=lists,
    )
    if args.json:
        Path(args.json).write_text(json.dumps(s.to_dict(), ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    sys.stdout.write(s.feedback())
    return 0 if not s.mispronounced else 1


def _cmd_mfa(args: argparse.Namespace) -> int:
    lines: list[str] = []
    for f in args.input:
        lines.extend(read_texts(f))
    text = mfa_dictionary(lines, args.lang, improved=args.improved, wordlists=_wordlists(args), stress=args.stress, both_modes=args.both_modes, foreign=args.foreign)
    if args.out:
        Path(args.out).write_text(text, encoding="utf-8", newline="\n")
        print(f"{text.count(chr(10))} pronunciations written to {args.out}", file=sys.stderr)
    else:
        sys.stdout.write(text)
    return 0


def main(argv: Sequence[str] | None = None) -> int:
    ap = argparse.ArgumentParser(prog="uccharan-train", description="Dataset labelling, reading scripts, coverage and pronunciation scoring.")
    ap.add_argument("-V", "--version", action="version", version=f"uccharan {__version__}")
    sub = ap.add_subparsers(dest="command", required=True)

    p = sub.add_parser("label", help="manifest to training-ready phonemes (Piper, Matcha-TTS, icefall), symbol table and voice.json")
    p.add_argument("manifest", help="metadata.csv (id|text), a Piper CSV, or JSON lines")
    p.add_argument("-o", "--out", required=True, help="output directory")
    _common(p)
    p.add_argument("--layout", choices=["ljspeech", "piper", "jsonl", "json"], help="manifest layout (default: by file name)")
    p.add_argument("--text-column", type=int, help="text column of a |-separated manifest, from 0")
    p.add_argument("-p", "--profile", default="piper", choices=PROFILES)
    p.add_argument("--normalize", default="natural", choices=["natural", "espeak", "off"])
    p.add_argument("--hinglish", action="store_true", help="detect romanised Hindi (lang hi)")
    p.add_argument("--symbols", metavar="FILE|matcha", help="target symbol set: a Piper voice config, a JSON map, a tokens.txt, or 'matcha' (default: built from the data)")
    p.add_argument("--extend", action="store_true", help="give new ids to symbols the target set lacks")
    p.add_argument("--replace", action="append", default=[], metavar="FROM=TO", help="rewrite a symbol before the table is built (for example ʱ=ɦ)")
    p.add_argument("--no-stress", action="store_true", help="drop stress marks")
    p.add_argument("--format", action="append", choices=["piper", "matcha", "icefall"], help="files to write (default: all)")
    p.add_argument("--validation", type=int, default=0, metavar="N", help="hold N utterances out as matcha/val.txt")
    p.add_argument("--strict", action="store_true", help="exit with status 2 when a symbol is outside the target set")
    p.add_argument("--show", type=int, default=20, help="warnings to print (default 20)")
    p.set_defaults(run=_cmd_label)

    p = sub.add_parser("select", help="pick a compact reading script that covers the phones and diphones")
    p.add_argument("corpus", nargs="+", help="text files")
    p.add_argument("-o", "--out", help="output directory (script.txt, script.tsv, coverage.json); default: the script on standard output")
    _common(p)
    p.add_argument("--lines", action="store_true", help="one sentence per line (default: split running text into sentences)")
    p.add_argument("-n", "--max-sentences", type=int)
    p.add_argument("--min-words", type=int, default=3)
    p.add_argument("--max-words", type=int, default=20)
    p.add_argument("--max-chars", type=int)
    p.add_argument("--allow-digits", action="store_true")
    p.add_argument("--allow-mixed-script", action="store_true")
    p.add_argument("--min-symbol-sentences", type=int, default=2, help="drop sentences with a phone seen in fewer sentences than this (default 2)")
    p.add_argument("--symbols", metavar="FILE|matcha", help="only sentences inside this symbol set")
    p.add_argument("--stress", action="store_true", help="count stressed vowels as phones of their own")
    p.add_argument("--prefer", choices=["sentences", "length"], default="sentences")
    p.set_defaults(run=_cmd_select)

    p = sub.add_parser("coverage", help="phone and diphone coverage, durations and outliers of a dataset")
    p.add_argument("dataset", help="a manifest, or labels.jsonl written by label")
    _common(p)
    p.add_argument("--layout", choices=["ljspeech", "piper", "jsonl", "json"])
    p.add_argument("--reference", metavar="FILE", help="text corpus of the language, one sentence per line: what should be covered")
    p.add_argument("--symbols", metavar="FILE|matcha", help="symbol table whose unused symbols are reported")
    p.add_argument("--alignments", metavar="PATH", help="TextGrid directory, JSON lines or CTM with phone times")
    p.add_argument("--tier", default="phones", help="TextGrid tier (default phones)")
    p.add_argument("--vocabulary", metavar="FILE", help="known words, one per line")
    p.add_argument("--min-count", type=int, default=5)
    p.add_argument("--max-phones", type=int)
    p.add_argument("--max-seconds", type=float)
    p.add_argument("--max-oov", type=float, default=0.3)
    p.add_argument("--stress", action="store_true")
    p.add_argument("--json", metavar="FILE", help="write the full report here")
    p.set_defaults(run=_cmd_coverage)

    p = sub.add_parser("score", help="phoneme error of a recogniser's output against a reference, word by word")
    _common(p)
    p.add_argument("--reference", help="reference text")
    p.add_argument("--heard", help="the recogniser's text")
    p.add_argument("--heard-phonemes", help="IPA from a phone recogniser")
    p.add_argument("--heard-lang", help="language of the recogniser's text (default: --lang)")
    p.add_argument("--pairs", metavar="FILE", help="JSON lines (reference, heard or heard_phonemes, id) or TSV (id, reference, heard)")
    p.add_argument("--phonemes", action="store_true", help="the heard column of a TSV --pairs file is IPA")
    p.add_argument("--compatible", action="store_true", help="read both sides in compatible mode (default: improved)")
    p.add_argument("--threshold", type=float, default=0.85, help="word score under which a word is reported (default 0.85)")
    p.add_argument("--json", metavar="FILE")
    p.add_argument("--show", type=int, default=20)
    p.set_defaults(run=_cmd_score)

    p = sub.add_parser("mfa-dict", help="pronunciation dictionary for the Montreal Forced Aligner from a word list or corpus")
    p.add_argument("input", nargs="+", help="text files (words or sentences)")
    p.add_argument("-o", "--out")
    _common(p)
    p.add_argument("--stress", action="store_true", help="keep stress marks on the phones")
    p.add_argument("--both-modes", action="store_true", help="list the compatible and the improved reading when they differ")
    p.add_argument("--foreign", action="store_true", help="keep words in another script")
    p.set_defaults(run=_cmd_mfa)

    args = ap.parse_args(argv)
    # Phonemes are not in the legacy Windows code pages: always read and write UTF-8.
    for stream in (sys.stdin, sys.stdout, sys.stderr):
        reconfigure = getattr(stream, "reconfigure", None)
        if reconfigure is not None:
            reconfigure(encoding="utf-8")
    try:
        return int(args.run(args))
    except (ValueError, OSError) as e:
        print(f"uccharan-train: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
