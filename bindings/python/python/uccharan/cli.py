"""Command line: ``uccharan [options] [TEXT ...]`` (reads stdin when no text is given)."""

from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Sequence

from . import LANGUAGES, PROFILES, __version__, phonemize


def main(argv: Sequence[str] | None = None) -> int:
    ap = argparse.ArgumentParser(prog="uccharan", description="Text to phonemes for neural TTS.")
    ap.add_argument("text", nargs="*", help="text to phonemize; standard input is read line by line when omitted")
    ap.add_argument("-l", "--lang", default="en-us", help=f"language: {', '.join(LANGUAGES)} (default en-us)")
    ap.add_argument("-p", "--profile", default="espeak-ipa", choices=PROFILES)
    ap.add_argument("--normalize", default="natural", choices=["natural", "espeak", "off"])
    ap.add_argument("--improved", action="store_true", help="cleaner, not reference-compatible output")
    ap.add_argument("--hinglish", action="store_true", help="detect romanised Hindi (experimental, lang hi)")
    ap.add_argument("--no-punctuation", action="store_true", help="drop clause punctuation (espeak-ipa)")
    ap.add_argument("--json", action="store_true", help="print one JSON value per input line")
    ap.add_argument("-V", "--version", action="version", version=f"uccharan {__version__}")
    args = ap.parse_args(argv)

    def emit(line: str) -> None:
        try:
            out = phonemize(  # type: ignore[call-overload]
                line,
                args.lang,
                args.profile,
                normalize=args.normalize,
                improved=args.improved,
                hinglish=args.hinglish,
                punctuation=not args.no_punctuation,
            )
        except ValueError as exc:
            ap.error(str(exc))
        if args.json or not isinstance(out, str):
            print(json.dumps(out, ensure_ascii=False))
        else:
            print(out)

    if args.text:
        for t in args.text:
            emit(t)
    else:
        for line in sys.stdin:
            emit(line.rstrip("\n"))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
