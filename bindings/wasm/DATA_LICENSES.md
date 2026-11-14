# Data files and their licences

Every data file in this repository, where it came from, and what licence it
carries.

## Shipped in the packages (crate, wheel, npm package, C library)

| File | What it is | Source | Licence |
| --- | --- | --- | --- |
| `crates/uccharan/data/en-us.ucd` | English (General American): the CMUdict-derived entries that the letter-to-sound tables do not reproduce (52 k of 126 k), a set of the left-out words, letter-to-sound tables, and the name-origin classifier | Built by `tools/lexgen` from CMUdict plus the hand-written files below | BSD-2-Clause (CMUdict) + Apache-2.0 |
| `crates/uccharan/data/en-gb.ucd` | English (Received Pronunciation): the same kinds of tables | The same CMUdict entries passed through this project's accent rules, plus the hand-written files below | BSD-2-Clause (CMUdict) + Apache-2.0 |
| `crates/uccharan/data/hi.ucd` | Hindi exceptions and the romanised-Hindi word list | Built from the hand-written files below | Apache-2.0 |
| `crates/uccharan/data/kokoro-en-us.ucd`, `kokoro-en-gb.ucd` | misaki's dictionaries, compiled (Kokoro English) | see "misaki's English dictionaries" below | Apache-2.0 |
| `bindings/wasm/data/*.ucd` | Copies of the files above for the npm package | — | as above |
| `*.ucd` assets of each GitHub release | The same files, which the Python package downloads on first use (checked by SHA-256) | — | as above |

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
| `data/sources/names/indian_names.txt` | About 350 common Indian given names and surnames in their usual Latin spelling (no pronunciations) | Written by hand for this project | Apache-2.0 |
| `crates/uccharan/src/en/arabic_names.txt` | 1 508 Arabic given names in English transliteration (no pronunciations), used by the improved mode to read them with Arabic stress | Labels of Wikidata given-name items whose language is Arabic or whose native label is Arabic, queried 2026-11-14 and filtered by `tools/arabic_names.py` | CC0-1.0 (Wikidata) |
| `crates/uccharan/src/en/horn_names.txt` | 3 828 words of the names of Eritrean and Ethiopian people (no pronunciations), used by the improved mode to read them as names from the Horn of Africa | English labels of Wikidata people with Eritrean or Ethiopian citizenship, queried 2026-11-14 and filtered by `tools/arabic_names.py` | CC0-1.0 (Wikidata) |
| `tools/eval/gold/names.tsv` (evaluation only, not shipped in packages) | 380 names in short sentences with expected pronunciations | Hand-written IPA, and CMUdict variants where the name is in CMUdict | Apache-2.0; CMUdict parts BSD-2-Clause |

| `crates/uccharan/lang/*.lang` (Bengali, Marathi, Telugu, Nepali, …) | Letter values, rewrite rules, exceptions and unstressed words of the data-driven languages, written by hand for this project from the language specifications; the cardinal number words 0–99 come from Unicode CLDR's rule-based number formats (`common/rbnf/<lang>.xml`, drafted with `tools/cldr_numbers.py` and adjusted by hand) | Written for this project; CLDR | Apache-2.0; number words Unicode License v3 (`data/sources/cldr/LICENSE`, `NOTICE`) |
| `tools/eval/gold/african_names.src`, `african_names.tsv` (evaluation only, not shipped) | 388 African names with hand-written English-context pronunciations | Written by hand for this project from published grammars (sources in the file header) | Apache-2.0 |
| `tools/eval/gold/{north_african,egyptian,se_african}_names.{src,tsv}` (evaluation only, not shipped) | 181 North African, 167 Egyptian and 184 Chichewa, Shona, Tsonga and Tigrinya names with hand-written English-context pronunciations | Written by hand for this project from published grammars (sources in the file headers) | Apache-2.0 |
| `tools/eval/data/<lang>_dev.tsv`, `<lang>_heldout.tsv` (evaluation only, not shipped) | Sentences for measuring agreement | Tatoeba (CC BY 2.0 FR) or, where Tatoeba is small, the FLORES-200 `dev`/`devtest` splits | CC BY 2.0 FR; FLORES-200 CC BY-SA 4.0 |

Tables inside the source code (function words and weak forms, homograph cues,
number words, units, abbreviations, Devanagari letter values, Hindi suffixes)
were written by hand for this project and are Apache-2.0.

The letter-to-sound tables are counted from CMUdict's own entries (after this
project's conversion to the target notation). The name-origin classifier (the
`NAMC` section of the English packs: character-trigram counts) is trained on
`data/sources/names/indian_names.txt` against CMUdict's words. No statistical
model in this repository was trained on the output of another phonemizer.

## Optional share-alike packs (not in the packages; downloaded on request)

These packs are **not** Apache-2.0 and are not inside the crate, the wheel,
the npm package or the C library. They are separate `.ucd` assets of each
GitHub release, loaded only when a user asks for them
(`uccharan.load_pack("wikipron-en-us")`, `loadWikipron("en-us")` in
JavaScript, `Phonemizer::load_pack` in Rust, `uccharan_load_pack` in C,
`--pack FILE` on the command line). Each pack names its licence in its
metadata (`license=CC-BY-SA-4.0`) and the release carries the licence text
next to it (`wikipron-LICENSE-CC-BY-SA-4.0.txt`).

| File | What it is | Source | Licence |
| --- | --- | --- | --- |
| `data/packs/wikipron-en-us.ucd` (80.8 k words), `wikipron-en-gb.ucd` (81.3 k), `wikipron-hi.ucd` (30.4 k) | Wiktionary's broad pronunciations, rewritten into uccharan's symbols; used by the improved mode for words outside uccharan's own lexicon | Built by `cargo run --release -p uccharan-lexgen -- wikipron` from the files below | **CC BY-SA 4.0** (Wiktionary contributors, via WikiPron) |
| `data/sources/wikipron/eng_latn_us_broad.tsv`, `eng_latn_uk_broad.tsv`, `hin_deva_broad.tsv` | WikiPron scrapes, unchanged; commit and SHA-256 sums in `data/sources/wikipron/README.md` | https://github.com/CUNY-CL/wikipron (`data/scrape/tsv`) | **CC BY-SA 4.0** (Wiktionary text; WikiPron's code is Apache-2.0); licence text in `data/sources/wikipron/LICENSE-CC-BY-SA-4.0.txt` |

If you redistribute a WikiPron pack, or a dataset or model built with one,
the share-alike terms of CC BY-SA 4.0 apply to it. Phoneme output that
uccharan produces with a pack loaded is generated from it word by word; treat
it as you would treat Wiktionary text if that matters for your use.

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
| `tools/eval/data/{bn,mr,...,bho,mai}_{dev,heldout}.tsv` (FLORES-based ones) | Sentences of FLORES-200 dev / devtest where Tatoeba has too few | FLORES-200, https://github.com/facebookresearch/flores | CC BY-SA 4.0 (evaluation only) |
| `tools/eval/gold/bho/*.src`, `gold.tsv` | Bhojpuri gold set: 437 words and 153 sentences with hand-written pronunciations; per-item word source (FLORES-200 bho_Deva, CC BY-SA 4.0; Grierson's *Linguistic Survey of India* V.2, 1903, public domain; the spec's examples) and date in the file | Written by hand for this project | Apache-2.0 (words attested in FLORES are facts of the language, used for evaluation only) |
| `tools/eval/gold/mai/*.src`, `gold.tsv` | Maithili gold set: 410 words and 151 sentences; 134 words with pronunciations from WikiPron's `mai_deva_narrow` list, the rest hand-written; per-item source and date in the file | WikiPron (https://github.com/CUNY-CL/wikipron) and written by hand | Wiktionary items CC BY-SA 4.0 (evaluation only); the rest Apache-2.0 |
| `tools/eval/results/*.json`, `tools/bench/results/*.json` | Aggregate measurements from runs on the development machine | Produced by the tools in this repository | Apache-2.0 |

misaki (for the Kokoro comparison) and espeak-ng (for the compatibility
comparison) are run as references at evaluation time; their output is not
stored apart from aggregate numbers and a few example lines.

The evaluation tools can compare against an espeak-ng that the user installed.
Its output is produced at run time and is not stored in the repository, apart
from a few short example strings inside result files and tests.

## Not used

`ipa-dict` British English (GPL-derived), Britfone and Unisyn are not used.
espeak-ng's own files (source, rules, dictionaries, phoneme tables) are never
used as data or copied. (WikiPron data is used only in the separate CC BY-SA packs
above.)

## Brand assets

`docs/assets/brand` is original work for this project (Apache-2.0). The
wordmark's letter outlines come from Inter and Hind, both under the SIL Open
Font License 1.1; no font file is distributed.
