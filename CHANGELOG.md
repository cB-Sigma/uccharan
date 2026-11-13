# Changelog

All notable changes are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow semantic versioning.

## [0.2.0] - unreleased

Smaller, with modular language packs, and better at names.

### Added

- **Indian names in English text** (improved mode): a character-trigram classifier (trained on a
  hand-written list of Indian names against CMUdict's words) picks out unknown capitalised words
  that look Indian; they are read through uccharan's romanised-Hindi and Hindi rules and mapped
  to English phones. A names gold set of 380 names of many origins, with a held-out half, and
  `tools/eval/names_eval.py`, which reports accuracy per origin. Held-out names: 89.0 % against
  espeak-ng's 61.6 %; Indian names 77.1 % against 43.8 %.
- **Language packs.** The Python wheel carries en-US and Hindi only (1.1 MB instead of 5.5 MB);
  en-GB and the Kokoro English data are fetched from the GitHub release on first use, checked
  against SHA-256 sums that ship inside the wheel, and cached (`uccharan.load_pack`,
  `UCCHARAN_DATA_DIR`, `UCCHARAN_OFFLINE`). The npm package checks the packs it fetches the same
  way. Rust: `Phonemizer::load_pack(&self, bytes)` loads a pack into a shared phonemizer. The C
  library gained the same `embed-*` features as the Rust crate.
- `tools/bench/sizes.py`: sizes of the packs and the code next to espeak-ng's files, raw and
  with gzip, xz and zstd.
- The release workflow attaches the `.ucd` packs and their SHA-256 sums to the GitHub release.
- Benchmarks time each sentence once per fresh process after a warm-up on other text, with the
  two libraries taking turns (`tools/bench/compare.py`), so that uccharan's cache cannot flatter
  it.

### Changed

- **Data format 2** (packs from 0.1.0 are not readable): the lexicon is entropy-coded and leaves
  out every entry that the letter-to-sound tables reproduce exactly (the build checks that every
  word still comes out identical, and a compact set remembers which words were left out); the
  letter-to-sound tables are hash tables with 16-bit fingerprints. en-US data: 2.0 MB → 0.73 MB,
  en-GB 2.0 MB → 0.71 MB, Kokoro English 2.9 / 3.1 MB → 1.9 / 2.1 MB. Every word comes out as
  before the compression.
- A small per-thread cache of recent lexicon answers (4 096 words), a fast path for plain words
  in normalisation, and fenced lexicon blocks: faster than 0.1.0 despite the compressed data.
- Compatible mode, closer to espeak-ng (both accents): *couldn't / wouldn't / shouldn't / mustn't
  have* are joined as espeak-ng joins them; *beyond* has its /j/; *realize(d/s)*, *realization*,
  *strength(s)*, *length(s)* take the forms espeak-ng uses; *de-* before a vowel (*deodorant*) and
  possessives of names in *-y* (*Mary's*) keep a full /i/. Held-out en-US PER 1.19 % → 1.16 %,
  en-GB 1.41 % → 1.39 %.

## [0.1.0]

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
