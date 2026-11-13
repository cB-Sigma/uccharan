# Changelog

All notable changes are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow semantic versioning.

## [0.1.0] - unreleased

First version.

### Added

- Rust core (`uccharan`): text segmentation, number and symbol normalisation, English (General
  American and Received Pronunciation) from a CMUdict-derived lexicon with morphology,
  letter-to-sound tables and clause-level weak forms; Hindi from Devanagari rules with schwa
  deletion; code-mixed Hindi and English routed word by word; experimental romanised Hindi.
- Two behaviours: `compatible` (matches the phoneme streams espeak-trained voices expect) and
  `improved` (homographs in context, a names and brands list, context-aware abbreviations, natural
  numbers, morpheme-aware Hindi schwa deletion, homorganic nasals, plain-IPA Hindi symbols).
- Output profiles: espeak-style IPA, Piper (sentence strings and ids from a voice's
  `phoneme_id_map`), sherpa-onnx codepoints, Kokoro tokens and ids, Kitten TTS and Matcha-TTS ids.
- Kokoro English front end: a port of misaki's English G2P (Apache-2.0) with a rule-based
  tagger in place of spaCy and uccharan's own rules for words outside misaki's dictionaries.
  misaki 0.9.4's dictionaries are built in; another copy can be loaded with `load_misaki`,
  `--misaki`, `loadMisaki` or `uccharan_load_misaki`.
- Bindings: C ABI with a generated header, Python (abi3 wheels, typed, a small stand-in for
  `phonemizer`'s `EspeakBackend`), WebAssembly (npm package for browsers and Node).
- Command-line program `uccharan` (with `--ids`, `--threads`, `--misaki`).
- Batch API on native threads (`render_batch`, `uccharan_phonemize_lines`).
- Evaluation tools: agreement with a reference phonemizer, hand-curated gold sets, per-profile
  conformance checks against real model files, an end-to-end synthesis and recognition check, and
  speed comparisons. Results of the runs behind the published numbers are in `tools/eval/results`
  and `tools/bench/results`.
- Live demo (static site, WebAssembly).
