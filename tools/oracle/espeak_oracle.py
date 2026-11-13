#!/usr/bin/env python3
"""Black-box oracle: run an *installed* espeak-ng and print its IPA output.

uccharan never links, bundles or reads espeak-ng. This helper only calls an
espeak-ng that the user installed separately, in one of two ways:

* ``library``: ``espeak_TextToPhonemes`` in a shared ``libespeak-ng`` located
  through the ``espeakng-loader`` wheel or the system loader (fast, used for
  evaluation runs);
* ``executable``: the ``espeak-ng`` program on ``PATH`` (one process per line).

Usage::

    python tools/oracle/espeak_oracle.py -v en-us < lines.txt > ipa.txt
    python tools/oracle/espeak_oracle.py -v hi "नमस्ते दुनिया"

One output line per input line.
"""

from __future__ import annotations

import argparse
import ctypes
import ctypes.util
import shutil
import subprocess
import sys


class Oracle:
    def phonemize(self, text: str, voice: str) -> str:
        raise NotImplementedError

    def version(self) -> str:
        return "unknown"


class LibraryOracle(Oracle):
    _IPA = 0x02
    _UTF8 = 1

    def __init__(self, library: str, data_path: str | None) -> None:
        lib = ctypes.CDLL(library)
        lib.espeak_Initialize.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_char_p, ctypes.c_int]
        lib.espeak_Initialize.restype = ctypes.c_int
        lib.espeak_SetVoiceByName.argtypes = [ctypes.c_char_p]
        lib.espeak_SetVoiceByName.restype = ctypes.c_int
        lib.espeak_TextToPhonemes.argtypes = [ctypes.POINTER(ctypes.c_char_p), ctypes.c_int, ctypes.c_int]
        lib.espeak_TextToPhonemes.restype = ctypes.c_char_p
        lib.espeak_Info.argtypes = [ctypes.POINTER(ctypes.c_char_p)]
        lib.espeak_Info.restype = ctypes.c_char_p
        # AUDIO_OUTPUT_RETRIEVAL = 1, espeakINITIALIZE_DONT_EXIT = 0x8000
        if lib.espeak_Initialize(1, 0, data_path.encode() if data_path else None, 0x8000) <= 0:
            raise OSError("espeak_Initialize failed")
        self._lib = lib
        self._voice: str | None = None

    def version(self) -> str:
        raw = self._lib.espeak_Info(None)
        return raw.decode(errors="replace").split()[0] if raw else "unknown"

    def phonemize(self, text: str, voice: str, tie: bool = False) -> str:
        if voice != self._voice:
            if self._lib.espeak_SetVoiceByName(voice.encode()) != 0:
                raise ValueError(f"espeak-ng has no voice {voice!r}")
            self._voice = voice
        buf = ctypes.c_char_p(text.encode("utf-8"))
        pieces: list[str] = []
        mode = self._IPA | (0x80 | (ord("^") << 8) if tie else 0)
        while buf.value:
            out = self._lib.espeak_TextToPhonemes(ctypes.byref(buf), self._UTF8, mode)
            if out:
                pieces.append(out.decode("utf-8", errors="replace"))
        return " ".join(" ".join(pieces).split())

    def clauses(self, text: str, voice: str) -> list[tuple[str, str]]:
        """Clause-level output: ``(phonemes, terminator)`` per clause.

        The terminator (one of ``. , ; : ! ?`` or ``""``) is recovered from the
        stretch of input the engine consumed for the clause. This is how the
        Piper-shaped reference stream is rebuilt for evaluation.
        """
        if voice != self._voice:
            if self._lib.espeak_SetVoiceByName(voice.encode()) != 0:
                raise ValueError(f"espeak-ng has no voice {voice!r}")
            self._voice = voice
        raw = text.encode("utf-8")
        buf = ctypes.c_char_p(raw)
        out: list[tuple[str, str]] = []
        while buf.value:
            before = len(buf.value)
            res = self._lib.espeak_TextToPhonemes(ctypes.byref(buf), self._UTF8, self._IPA)
            after = len(buf.value) if buf.value else 0
            consumed = raw[len(raw) - before : len(raw) - after].decode("utf-8", errors="replace")
            ipa = " ".join(res.decode("utf-8", errors="replace").split()) if res else ""
            if ipa:
                out.append((ipa, _terminator(consumed, after == 0)))
        return out


_TERMINATORS = {".": ".", ",": ",", ";": ";", ":": ":", "!": "!", "?": "?", "\u0964": ".", "\u0965": ".", "\u2026": "."}
_CLOSERS = "\"')]}\u201d\u2019\u00bb "


def _terminator(consumed: str, at_end: bool) -> str:
    """The clause terminator inside the consumed input (the engine reads one
    character of look-ahead past the white space that follows it)."""
    s = consumed.rstrip()
    if not at_end:
        m = len(s)
        # drop the look-ahead character
        if m >= 2 and s[m - 2].isspace():
            s = s[: m - 1].rstrip()
    s = s.rstrip(_CLOSERS)
    if not s:
        return ""
    last = s[-1]
    if last in "!?":
        # runs of ?/! collapse; the first one decides
        k = len(s)
        while k > 0 and s[k - 1] in "!?":
            k -= 1
        return _TERMINATORS[s[k]]
    return _TERMINATORS.get(last, "")


class ExecutableOracle(Oracle):
    def __init__(self, exe: str) -> None:
        self.exe = exe

    def version(self) -> str:
        out = subprocess.run([self.exe, "--version"], capture_output=True, check=False).stdout.decode()
        return out.split()[3] if len(out.split()) > 3 else out.strip()

    def phonemize(self, text: str, voice: str, tie: bool = False) -> str:
        args = [self.exe, "-q", "--ipa", "-v", voice]
        if tie:
            args.append("--tie=^")
        out = subprocess.run([*args, "--", text], capture_output=True, check=True).stdout
        return " ".join(out.decode("utf-8", errors="replace").split())


def find_oracle(prefer: str = "auto") -> Oracle:
    """Return an oracle or raise ``RuntimeError`` when espeak-ng is not installed."""
    if prefer in ("auto", "library"):
        try:
            import espeakng_loader  # type: ignore[import-not-found]

            return LibraryOracle(espeakng_loader.get_library_path(), espeakng_loader.get_data_path())
        except Exception:
            pass
        found = ctypes.util.find_library("espeak-ng")
        if found:
            try:
                return LibraryOracle(found, None)
            except OSError:
                pass
    if prefer in ("auto", "executable"):
        exe = shutil.which("espeak-ng")
        if exe:
            return ExecutableOracle(exe)
    raise RuntimeError(
        "espeak-ng not found. Install it yourself (apt install espeak-ng, or pip install espeakng-loader); "
        "uccharan does not ship it."
    )


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("-v", "--voice", default="en-us")
    ap.add_argument("--backend", choices=["auto", "library", "executable"], default="auto")
    ap.add_argument("--tie", action="store_true", help="join multi-letter phonemes with ^")
    ap.add_argument("--version", action="store_true")
    ap.add_argument("text", nargs="*")
    args = ap.parse_args()
    oracle = find_oracle(args.backend)
    if args.version:
        print(oracle.version())
        return 0
    lines = args.text if args.text else (line.rstrip("\n") for line in sys.stdin)
    for line in lines:
        print(oracle.phonemize(line, args.voice, args.tie) if line.strip() else "")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
