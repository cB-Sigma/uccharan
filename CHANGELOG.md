# Changelog

All notable changes are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow semantic versioning.

## [0.2.0] - 2026-11-14

Sixteen more South Asian languages; smaller, with modular language packs; and better at names.

### Added

- **African names in English text** (improved mode): names spelled in African-language
  orthographies are recognised by spelling cues and read with their own letter values and
  penultimate stress, mapped to English phonemes (`uccharan::african`). A new hand-written gold
  set of 388 African names from 37 languages with sources per language
  (`tools/eval/gold/african_names.src`, built by `tools/eval/build_african_names.py`;
  `names_eval.py --gold`). Held-out: 70.8 % against espeak-ng's 45.6 % (41.0 % before); the
  general names set rises to 91.9 % (African 7/8 against espeak-ng's 4/8). Compatible mode is
  unchanged.
- **Egyptian, Chichewa, Shona, Tsonga and Tigrinya names** (improved mode): a list of 1 508
  Arabic given names from Wikidata (CC0) read with Arabic stress even where CMUdict has them with
  English stress; Egyptian spelling conventions; Ethiopian name elements; more Bantu onsets and
  clusters, Tsonga *x* and *rh*. Two new hand-written gold sets (167 Egyptian names; 224 Chichewa,
  Shona, Tsonga and Tigrinya names, the Tigrinya development half grown to 59). Held-out (second scoring): Egyptian 69.4 % against espeak-ng's 56.5 %;
  Chichewa/Shona/Tsonga/Tigrinya 87.5/73.1/73.7/70.4 % against 41.7/42.3/42.1/63.0 %. A list of
  Eritrean and Ethiopian name words from Wikidata (CC0) for Horn of Africa names.

- **Bhojpuri and Maithili** (`bho`, `mai`): compatible mode reproduces espeak-ng's Hindi-voice
  reading (no espeak-ng voice exists; held-out PER 0.80 % and 0.75 % against `hi` on FLORES-200),
  improved mode reads them with their own phonology (ऐ औ = əi əu, इ उ after a vowel, avagraha and
  its Latin-*s* spelling, flaps, nasal vowels, past -ल- forms, unstressed copula, Bhojpuri number
  words), Kaithi script support (`uccharan::kaithi`) and an experimental romanised-Bhojpuri word
  list. New hand-written gold sets (Bhojpuri 437 words and 153 sentences; Maithili 410 words, 134 with
  Wiktionary pronunciations, and 151 sentences; `tools/eval/build_bho_gold.py`,
  `tools/eval/bho_eval.py`). Held-out words: Bhojpuri 96.4 % against 62.9 % for the Hindi-voice
  fallback, Maithili 93.4 % against 68.6 %; sentence PER 0.43 % and 0.29 % against 5.54 % and
  5.66 %. 3.0–3.3× faster than espeak-ng's `hi` voice on the same text.
- **Bengali, Marathi, Telugu, Kannada, Malayalam, Nepali, Tamil, Gujarati and Assamese** (`bn`,
  `mr`, `te`, `kn`, `ml`, `ne`, `ta`, `gu`, `as`), compatible with the espeak-ng voices of the same names (held-out PER 0.37 %,
  0.27 %, 0.87 %, 1.58 %, 2.01 %, 2.67 %, 4.09 %, 2.41 %, 7.21 %), with improved-mode fixes for known reference
  quirks, every output profile, number reading, and routing of each script to its own language
  inside mixed text (with `(bn)`-style flags in the `espeak-ipa` profile). 2–37× faster than
  espeak-ng on the same sentences.
- **Punjabi, Odia, Sinhala, Konkani and Sanskrit** (`pa`, `or`, `si`, `kok`, `sa`) on the shared
  Brahmic engine. Held-out PER against espeak-ng 1.52.0: Punjabi 0.41 %, Odia 0.50 %, Sinhala
  0.86 % (FLORES-200), Konkani 0.20 % (Sangraha); Sanskrit has no espeak-ng voice, so compatible
  mode is the Hindi voice's reading (0.95 % against `hi`) and improved mode reads it as Sanskrit
  (no schwa deletion, classical stress rule, visarga echo, Sanskrit number words). 2.3–5.1× faster
  than espeak-ng. Engine additions: letter names with their own stress, spelled-out words triggered
  by a letter sequence, explicit letter kinds for scripts outside the common Indic layout, a word
  break after ZWJ conjuncts, more anusvara and deletion options, a number-word table per language.
- **A data-driven language engine**: ordered context-sensitive rewrite rules
  (`uccharan::rules`), a language-file format (`uccharan::langdef`: letters, rule passes per
  mode, exceptions, unstressed words, number words with scale and combining forms) and one
  decoder for all Brahmic scripts (`uccharan::indic`: stress schemes, inherent-vowel deletion
  schemes, anusvara/visarga/geminate options). A new language is a `languages!` row plus a
  `.lang` file. `tools/eval/build_lang_sets.py` builds held-out sets per language (Tatoeba, or
  FLORES-200 for evaluation only); `tools/cldr_numbers.py` drafts number words from Unicode CLDR.

- **Optional WikiPron packs (CC BY-SA 4.0, not Apache-2.0)**: `wikipron-en-us`, `wikipron-en-gb`
  and `wikipron-hi`, built by `uccharan-lexgen wikipron` from Wiktionary pronunciations
  (via WikiPron) and rewritten into uccharan's symbols. They are separate release assets, never
  inside a package and never fetched unless asked for; each names its licence in its metadata.
  The improved mode uses them for words and names outside uccharan's own lexicon, taking the
  stress from uccharan's own reading (WikiPron's transcriptions carry none) and, where Wiktionary
  lists several pronunciations, the one closest to uccharan's. Compatible mode does not use them.
  Loading: `Phonemizer::load_pack` / `has_wikipron` (Rust), `uccharan.load_pack("wikipron-en-us")`
  (Python), `loadWikipron` (JavaScript), `uccharan_load_pack` (C), `--pack FILE` (command line).
  Held-out names with the pack: 88.4 % (89.0 % without); unseen gold sets 82.8 % (82.0 %).
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

### Fixed

- Tamil compatible mode: a hand-written list of Sanskrit and foreign loan stems that keep a voiced
  initial stop (பூமி, தேசிய, கவனம், பிரிட்டிஷ், …), compound boundaries before படு and after
  முன், the பிர-/கிர- cluster limited to k and p, and a word-initial glide ʲ merged into a preceding
  i or j (also Malayalam, Telugu). Held-out PER 4.41 % → 4.09 % (Malayalam 2.04 % → 2.01 %).
- Hindi compatible mode: before a hyphen, a word of two or more syllables ending in …CəC keeps its
  final inherent vowel as the reference does (अलग-अलग), and a vowel letter अ inside a word is ə
  unless stressed (जंतुअन). Hindi development PER 0.15 % → 0.14 %.
- A doubled *k* outside the English lexicon was dropped by the letter-to-sound model (*Mekki*
  read as *mˈɪi*); it is now read like *ck*.

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
