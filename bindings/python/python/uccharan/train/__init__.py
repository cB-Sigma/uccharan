"""Tools for people who train voices and check pronunciation.

* :func:`label`: a dataset manifest to training-ready phonemes (Piper,
  Matcha-TTS, icefall-style), a symbol table and ``voice.json``.
* :func:`select_script`: a compact reading script that covers the phonemes
  and diphones of a language.
* :func:`coverage`: what a dataset covers, what it lacks, and its outliers.
* :func:`score`: phoneme error of a recogniser's output against a
  reference, word by word.
* :func:`mfa_dictionary`: a pronunciation dictionary for forced aligners.

The same tools are on the command line as ``uccharan-train``. See
docs/training.md.
"""

from __future__ import annotations

from ._phones import split_phones, split_sentences
from .coverage import CoverageReport, Outlier, coverage, read_alignments
from .label import Labelled, LabelledUtterance, UtteranceWarning, label
from .manifest import Utterance, from_rows, read_manifest, read_texts
from .mfa import mfa_dictionary
from .score import Score, WordResult, score, score_many
from .script import Script, Selected, select_script
from .symbols import SymbolTable

__all__ = [
    "CoverageReport",
    "Labelled",
    "LabelledUtterance",
    "Outlier",
    "Score",
    "Script",
    "Selected",
    "SymbolTable",
    "Utterance",
    "UtteranceWarning",
    "WordResult",
    "coverage",
    "from_rows",
    "label",
    "mfa_dictionary",
    "read_alignments",
    "read_manifest",
    "read_texts",
    "score",
    "score_many",
    "select_script",
    "split_phones",
    "split_sentences",
]
