# Data files and their licences

Every data file in this repository, where it came from, and what licence it
carries.

## Shipped in the packages (crate, wheel, npm package, C library)

| File | What it is | Source | Licence |
| --- | --- | --- | --- |
| `crates/uccharan/data/en-us.ucd` | English (General American) lexicon, 126 k entries, and letter-to-sound tables | Built by `tools/lexgen` from CMUdict plus the hand-written files below | BSD-2-Clause (CMUdict) + Apache-2.0 |
| `crates/uccharan/data/en-gb.ucd` | English (Received Pronunciation) lexicon and letter-to-sound tables | The same CMUdict entries passed through this project's accent rules, plus the hand-written files below | BSD-2-Clause (CMUdict) + Apache-2.0 |
| `crates/uccharan/data/hi.ucd` | Hindi exceptions and the romanised-Hindi word list | Built from the hand-written files below | Apache-2.0 |
| `crates/uccharan/data/kokoro-en-us.ucd`, `kokoro-en-gb.ucd` | misaki's dictionaries, compiled (Kokoro English) | see "misaki's English dictionaries" below | Apache-2.0 |
| `bindings/wasm/data/*.ucd` | Copies of the files above for the npm package | — | as above |

The `.ucd` files are reproducible: `cargo run --release -p uccharan-lexgen`
rebuilds them byte for byte and `… -- check` fails when they are stale.

## Sources of the shipped data (in the repository)

| File | What it is | Source | Licence |
| --- | --- | --- | --- |
| `data/sources/cmudict/cmudict.dict` | The CMU Pronouncing Dictionary | https://github.com/cmusphinx/cmudict (file `cmudict.dict`) | BSD-2-Clause-style; text in `data/sources/cmudict/LICENSE` and in `NOTICE` |
| `data/sources/en/overrides.tsv` | About a hundred everyday words whose first CMUdict variant is a casual or minority form | Written by hand for this project | Apache-2.0 |
| `data/sources/en/names.tsv` | Names, brands and loanwords for the improved mode | Written by hand for this project | Apache-2.0 |
| `data/sources/hi/exceptions.tsv` | Hindi words that do not follow the schwa-deletion rules | Written by hand for this project | Apache-2.0 |
| `data/sources/hi/romanized.tsv` | Common romanised spellings of Hindi words → Devanagari | Written by hand for this project | Apache-2.0 |

Tables inside the source code (function words and weak forms, homograph cues,
number words, units, abbreviations, Devanagari letter values, Hindi suffixes)
were written by hand for this project and are Apache-2.0.

The letter-to-sound tables are counted from CMUdict's own entries (after this
project's conversion to the target notation). No statistical model in this
repository was trained on the output of another phonemizer.

## misaki's English dictionaries (Kokoro English)

| File | What it is | Source | Licence |
| --- | --- | --- | --- |
| `data/sources/misaki/us_gold.json`, `us_silver.json`, `gb_gold.json`, `gb_silver.json` | misaki's US and British English dictionaries, unchanged | the `misaki/data` directory of the **misaki 0.9.4** wheel on PyPI (hexgrad/misaki); SHA-256 sums in `data/sources/misaki/README.md` | Apache-2.0 (upstream licence statement; `data/sources/misaki/LICENSE`) |
| `crates/uccharan/data/kokoro-en-us.ucd`, `kokoro-en-gb.ucd` (and the copies in the npm package) | the same entries compiled by `tools/lexgen` for the `kokoro` profile | as above | Apache-2.0 |

**Provenance caveat.** misaki's repository and package are Apache-2.0, but upstream does not
document where the dictionary entries originally come from (hexgrad/misaki issue #51, closed, and
issues #107 and #113, open on 2026-11-13, ask exactly that, including whether any entries were
generated with espeak-ng or taken from Wiktionary). This project relies on the upstream Apache-2.0
licence statement. If you need certainty about the dictionaries' origin, build without the
`embed-kokoro` feature (Rust) and do not use the `kokoro` profile for English, or load your own
dictionaries with `load_misaki`.

misaki's code is also Apache-2.0 and was ported with attribution (see `NOTICE`).

## Symbol tables reproduced for conformance

| Where | What | Source | Licence |
| --- | --- | --- | --- |
| `crates/uccharan/src/profile.rs` (`KOKORO_VOCAB`) | Symbol → id table of Kokoro-82M | `config.json` of hexgrad/Kokoro-82M | Apache-2.0 |
| `crates/uccharan/src/profile.rs` (StyleTTS2-style table) | Symbol order used by Kitten TTS 0.8 and Matcha-TTS | KittenML/KittenTTS (Apache-2.0), shivammehta25/Matcha-TTS (MIT) | Apache-2.0 / MIT |

Piper `phoneme_id_map`s are not stored; they are read from the voice's own
`.onnx.json` at run time.

## Evaluation data (repository only, never packaged)

| File | What it is | Source | Licence |
| --- | --- | --- | --- |
| `tools/eval/data/en_dev.tsv`, `en_heldout.tsv` | English sentences (development sample and held-out set) | Tatoeba sentence export, https://tatoeba.org | CC BY 2.0 FR (attribution: Tatoeba contributors; the first column is the Tatoeba sentence id) |
| `tools/eval/data/hi_dev.tsv`, `hi_heldout.tsv` | Hindi sentences | Tatoeba sentence export | CC BY 2.0 FR |
| `tools/eval/data/en_prose.tsv` | Sentences from "The Adventures of Sherlock Holmes" | Project Gutenberg #1661 | Public domain in the USA |
| `tools/eval/data/en_words_*.txt`, `hi_words_heldout.txt` | Word lists derived from the files above and from CMUdict's word list | as above | as above |
| `tools/eval/gold/**/*.tsv` | Hand-curated gold sets | Written by hand for this project; expected forms follow CMUdict variants and standard descriptions of English and Hindi phonology | Apache-2.0 |
| `tools/eval/results/*.json`, `tools/bench/results/*.json` | Aggregate measurements from runs on the development machine | Produced by the tools in this repository | Apache-2.0 |

misaki (for the Kokoro comparison) and espeak-ng (for the compatibility
comparison) are run as references at evaluation time; their output is not
stored apart from aggregate numbers and a few example lines.

The evaluation tools can compare against an espeak-ng that the user installed.
Its output is produced at run time and is not stored in the repository, apart
from a few short example strings inside result files and tests.

## Not used

WikiPron and other Wiktionary-derived data (CC BY-SA), `ipa-dict` British
English (GPL-derived), Britfone, Unisyn, and any data set generated with
espeak-ng are not used, neither as data nor to fit tables.

## Brand assets

`docs/assets/brand` is original work for this project (Apache-2.0). The
wordmark's letter outlines come from Inter and Hind, both under the SIL Open
Font License 1.1; no font file is distributed.
