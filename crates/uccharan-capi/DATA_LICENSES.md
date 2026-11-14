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
| `crates/uccharan/data/ur.ucd`, `sd.ucd` | Urdu and Sindhi word tables (47 k and 18 k entries), compiled from `data/sources/{ur,sd}/words.tsv` | Derived from running espeak-ng 1.52.0 on the 120,000 most frequent Urdu and 61,000 most frequent Sindhi words of AI4Bharat Sangraha (`verified/urd`, `verified/snd`, CC BY 4.0), read alone and in sentences; only the words uccharan's letter rules read differently are kept | Apache-2.0 |
| `crates/uccharan/data/fr.ucd` | French word table and letter-to-sound model, compiled from `data/sources/fr/words.tsv` | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba French sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `fra_news_2023_300K` (CC BY 4.0) that occur at least twice (100 k words), read alone and in sentences; the file holds a letter-to-sound model trained on those readings and the words that the model and uccharan's letter rules read differently | Apache-2.0 |
| `crates/uccharan/data/it.ucd` | Italian word table and letter-to-sound model, compiled from `data/sources/it/words.tsv` | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Italian sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `ita_news_2023_300K` (CC BY 4.0) that occur at least twice (104 k words), read alone and in sentences; the file holds a letter-to-sound model trained on those readings and the words that the model and uccharan's letter rules read differently | Apache-2.0 |
| `crates/uccharan/data/pt-br.ucd` | Portuguese (Brazil) word table and letter-to-sound model, compiled from `data/sources/pt-br/words.tsv` | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Portuguese sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `por-br_newscrawl_2011_300K` (CC BY 4.0) that occur at least twice (82 k words), read alone and in sentences; the file holds a letter-to-sound model trained on those readings and the words that the model and uccharan's letter rules read differently | Apache-2.0 |
| `crates/uccharan/data/pt.ucd` | Portuguese (Portugal) word table and letter-to-sound model, compiled from `data/sources/pt/words.tsv` | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Portuguese sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `por-pt_newscrawl_2011_300K` (CC BY 4.0) that occur at least twice (82 k words), read alone and in sentences; the file holds a letter-to-sound model trained on those readings and the words that the model and uccharan's letter rules read differently | Apache-2.0 |
| `crates/uccharan/data/pl.ucd` | Polish word table and letter-to-sound model, compiled from `data/sources/pl/words.tsv` | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Polish sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `pol_news_2023_300K` (CC BY 4.0) that occur at least twice (127 k words), read alone and in sentences; the file holds a letter-to-sound model trained on those readings and the words that the model and uccharan's letter rules read differently | Apache-2.0 |
| `crates/uccharan/data/tr.ucd` | Turkish word table and letter-to-sound model, compiled from `data/sources/tr/words.tsv` | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Turkish sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `tur_news_2023_300K` (CC BY 4.0) that occur at least twice (151 k words), read alone and in sentences; the file holds a letter-to-sound model trained on those readings and the words that the model and uccharan's letter rules read differently | Apache-2.0 |
| `crates/uccharan/data/cs.ucd` | Czech word table and letter-to-sound model, compiled from `data/sources/cs/words.tsv` | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Czech sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `ces_news_2023_300K` (CC BY 4.0) that occur at least twice (133 k words), read alone and in sentences; the file holds a letter-to-sound model trained on those readings and the words that the model and uccharan's letter rules read differently | Apache-2.0 |
| `crates/uccharan/data/sv.ucd` | Swedish word table and letter-to-sound model, compiled from `data/sources/sv/words.tsv` | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Swedish sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `swe_news_2023_300K` (CC BY 4.0) that occur at least twice (95 k words), read alone and in sentences; the file holds a letter-to-sound model trained on those readings and the words that the model and uccharan's letter rules read differently | Apache-2.0 |
| `crates/uccharan/data/ca.ucd` | Catalan word table and letter-to-sound model, compiled from `data/sources/ca/words.tsv` | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Catalan sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `cat_news_2022_300K` (CC BY 4.0) that occur at least twice (71 k words), read alone and in sentences; the file holds a letter-to-sound model trained on those readings and the words that the model and uccharan's letter rules read differently | Apache-2.0 |
| `crates/uccharan/data/ro.ucd` | Romanian word table and letter-to-sound model, compiled from `data/sources/ro/words.tsv` | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Romanian sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `ron_news_2022_300K` (CC BY 4.0) that occur at least twice (92 k words), read alone and in sentences; the file holds a letter-to-sound model trained on those readings and the words that the model and uccharan's letter rules read differently | Apache-2.0 |
| `crates/uccharan/data/hu.ucd` | Hungarian word table and letter-to-sound model, compiled from `data/sources/hu/words.tsv` | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Hungarian sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `hun_news_2023_300K` (CC BY 4.0) that occur at least twice (152 k words), read alone and in sentences; the file holds a letter-to-sound model trained on those readings and the words that the model and uccharan's letter rules read differently | Apache-2.0 |
| `crates/uccharan/data/fi.ucd` | Finnish word table and letter-to-sound model, compiled from `data/sources/fi/words.tsv` | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Finnish sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `fin_news_2022_300K` (CC BY 4.0) that occur at least twice (153 k words), read alone and in sentences; the file holds a letter-to-sound model trained on those readings and the words that the model and uccharan's letter rules read differently | Apache-2.0 |
| `crates/uccharan/data/nb.ucd` | Norwegian Bokmål word table and letter-to-sound model, compiled from `data/sources/nb/words.tsv` | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Norwegian Bokmål sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `nor_news_2023_300K` (CC BY 4.0) that occur at least twice (86 k words), read alone and in sentences; the file holds a letter-to-sound model trained on those readings and the words that the model and uccharan's letter rules read differently | Apache-2.0 |
| `crates/uccharan/data/da.ucd` | Danish word table and letter-to-sound model, compiled from `data/sources/da/words.tsv` | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Danish sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `dan_news_2022_300K` (CC BY 4.0) that occur at least twice (81 k words), read alone and in sentences; the file holds a letter-to-sound model trained on those readings and the words that the model and uccharan's letter rules read differently | Apache-2.0 |
| `crates/uccharan/data/nl.ucd` | Dutch word table and letter-to-sound model, compiled from `data/sources/nl/words.tsv` | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Dutch sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `nld_news_2022_300K` (CC BY 4.0) that occur at least twice (80 k words), read alone and in sentences; the file holds a letter-to-sound model trained on those readings and the words that the model and uccharan's letter rules read differently | Apache-2.0 |
| `crates/uccharan/data/de.ucd` | German word table and letter-to-sound model, compiled from `data/sources/de/words.tsv` | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba German sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `deu_news_2023_300K` (CC BY 4.0) that occur at least twice (139 k words), read alone and in sentences; the file holds a letter-to-sound model trained on those readings and the words that the model and uccharan's letter rules read differently | Apache-2.0 |
| `crates/uccharan/tests/data/xlit-toy.ucd` (crate only) | A tiny transliteration pack for the tests: 25 hand-written word pairs and counts | Built by `cargo run --release -p uccharan-lexgen -- xlit-toy` from `data/sources/translit/toy/` (written for this project) | Apache-2.0 |
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
| `[words]` lines starting with `@` in `crates/uccharan/lang/{pa,or,kok,si,ur,sd}.lang`, the `[spell]` sections of `or.lang` and `kok.lang`, and `HI_NUM_REFERENCE` in `crates/uccharan/src/hi/mod.rs` | Number words 0–99 (and scale words) and letter names as espeak-ng reads them | Derived from running espeak-ng 1.52.0 on the numbers 0–99, a few scale numbers, and single letters | Apache-2.0 |
| `data/sources/ur/words.tsv`, `data/sources/sd/words.tsv` | Sources of `ur.ucd` and `sd.ucd` | Derived from running espeak-ng 1.52.0 on the 120,000 most frequent Urdu and 61,000 most frequent Sindhi words of AI4Bharat Sangraha (`verified/urd`, `verified/snd`, CC BY 4.0), read alone and in sentences | Apache-2.0 |
| `data/sources/fr/words.tsv`; the `[unstressed]` list and number-word readings in `crates/uccharan/lang/fr.lang` | Source of `fr.ucd`; words that lose their stress inside a clause; readings of the number words | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba French sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `fra_news_2023_300K` (CC BY 4.0) that occur at least twice (100 k words), read alone and in sentences, and on numbers | Apache-2.0 |
| `data/sources/it/words.tsv`; the `[unstressed]` list, number-word readings and letter names in `crates/uccharan/lang/it.lang` | Source of `it.ucd`; words that lose their stress inside a clause; readings of the number words and of the letters | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Italian sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `ita_news_2023_300K` (CC BY 4.0) that occur at least twice (104 k words), read alone and in sentences, and on numbers and single letters | Apache-2.0 |
| `data/sources/pt-br/words.tsv`; the `[unstressed]` list, number-word readings and letter names in `crates/uccharan/lang/pt-br.lang` | Source of `pt-br.ucd`; words that lose their stress inside a clause; readings of the number words and of the letters | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Portuguese sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `por-br_newscrawl_2011_300K` (CC BY 4.0) that occur at least twice (82 k words), read alone and in sentences, and on numbers and single letters | Apache-2.0 |
| `data/sources/pt/words.tsv`; the `[unstressed]` list, number-word readings and letter names in `crates/uccharan/lang/pt.lang` | Source of `pt.ucd`; words that lose their stress inside a clause; readings of the number words and of the letters | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Portuguese sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `por-pt_newscrawl_2011_300K` (CC BY 4.0) that occur at least twice (82 k words), read alone and in sentences, and on numbers and single letters | Apache-2.0 |
| `data/sources/pl/words.tsv`; the `[unstressed]` list, number-word readings and letter names in `crates/uccharan/lang/pl.lang` | Source of `pl.ucd`; words that lose their stress inside a clause; readings of the number words and of the letters | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Polish sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `pol_news_2023_300K` (CC BY 4.0) that occur at least twice (127 k words), read alone and in sentences, and on numbers and single letters | Apache-2.0 |
| `data/sources/tr/words.tsv`; the `[unstressed]` list, number-word readings and letter names in `crates/uccharan/lang/tr.lang` | Source of `tr.ucd`; words that lose their stress inside a clause; readings of the number words and of the letters | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Turkish sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `tur_news_2023_300K` (CC BY 4.0) that occur at least twice (151 k words), read alone and in sentences, and on numbers and single letters | Apache-2.0 |
| `data/sources/cs/words.tsv`; the `[unstressed]` list, number-word readings and letter names in `crates/uccharan/lang/cs.lang` | Source of `cs.ucd`; words that lose their stress inside a clause; readings of the number words and of the letters | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Czech sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `ces_news_2023_300K` (CC BY 4.0) that occur at least twice (133 k words), read alone and in sentences, and on numbers and single letters | Apache-2.0 |
| `data/sources/sv/words.tsv`; the `[unstressed]` list, number-word readings and letter names in `crates/uccharan/lang/sv.lang` | Source of `sv.ucd`; words that lose their stress inside a clause; readings of the number words and of the letters | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Swedish sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `swe_news_2023_300K` (CC BY 4.0) that occur at least twice (95 k words), read alone and in sentences, and on numbers and single letters | Apache-2.0 |
| `data/sources/ca/words.tsv`; the `[unstressed]` list, number-word readings and letter names in `crates/uccharan/lang/ca.lang` | Source of `ca.ucd`; words that lose their stress inside a clause; readings of the number words and of the letters | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Catalan sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `cat_news_2022_300K` (CC BY 4.0) that occur at least twice (71 k words), read alone and in sentences, and on numbers and single letters | Apache-2.0 |
| `data/sources/ro/words.tsv`; the `[unstressed]` list, number-word readings and letter names in `crates/uccharan/lang/ro.lang` | Source of `ro.ucd`; words that lose their stress inside a clause; readings of the number words and of the letters | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Romanian sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `ron_news_2022_300K` (CC BY 4.0) that occur at least twice (92 k words), read alone and in sentences, and on numbers and single letters | Apache-2.0 |
| `data/sources/hu/words.tsv`; the `[unstressed]` list, number-word readings and letter names in `crates/uccharan/lang/hu.lang` | Source of `hu.ucd`; words that lose their stress inside a clause; readings of the number words and of the letters | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Hungarian sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `hun_news_2023_300K` (CC BY 4.0) that occur at least twice (152 k words), read alone and in sentences, and on numbers and single letters | Apache-2.0 |
| `data/sources/fi/words.tsv`; the `[unstressed]` list, number-word readings and letter names in `crates/uccharan/lang/fi.lang` | Source of `fi.ucd`; words that lose their stress inside a clause; readings of the number words and of the letters | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Finnish sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `fin_news_2022_300K` (CC BY 4.0) that occur at least twice (153 k words), read alone and in sentences, and on numbers and single letters | Apache-2.0 |
| `data/sources/nb/words.tsv`; the `[unstressed]` list, number-word readings and letter names in `crates/uccharan/lang/nb.lang` | Source of `nb.ucd`; words that lose their stress inside a clause; readings of the number words and of the letters | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Norwegian Bokmål sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `nor_news_2023_300K` (CC BY 4.0) that occur at least twice (86 k words), read alone and in sentences, and on numbers and single letters | Apache-2.0 |
| `data/sources/da/words.tsv`; the `[unstressed]` list, number-word readings and letter names in `crates/uccharan/lang/da.lang` | Source of `da.ucd`; words that lose their stress inside a clause; readings of the number words and of the letters | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Danish sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `dan_news_2022_300K` (CC BY 4.0) that occur at least twice (81 k words), read alone and in sentences, and on numbers and single letters | Apache-2.0 |
| `data/sources/nl/words.tsv`; the `[unstressed]` list and number-word readings in `crates/uccharan/lang/nl.lang` | Source of `nl.ucd`; words that lose their stress inside a clause; readings of the number words | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba Dutch sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `nld_news_2022_300K` (CC BY 4.0) that occur at least twice (80 k words), read alone and in sentences, and on numbers | Apache-2.0 |
| `data/sources/de/words.tsv`; the `[unstressed]` list and number-word readings in `crates/uccharan/lang/de.lang` | Source of `de.ucd`; words that lose their stress inside a clause; readings of the number words | Derived from running espeak-ng 1.52.0 on the words of the Tatoeba German sentences (CC BY 2.0 FR) and of the Leipzig Corpora Collection sample `deu_news_2023_300K` (CC BY 4.0) that occur at least twice (139 k words), read alone and in sentences, and on numbers | Apache-2.0 |
| `crates/uccharan/lang/numbers/*.txt` | Spell-out rule sets (`%spellout-numbering`, `%spellout-cardinal`) of Bengali, Gujarati, Kannada, Malayalam, Marathi, Nepali, Odia, Punjabi, Tamil, Telugu and Urdu for the written number words of normalise-only mode; percent words added by hand; Nepali long vowels, a Malayalam space and Telugu exact plurals adjusted in `tools/cldr_rbnf.py` | Unicode CLDR `common/rbnf/<lang>.xml`, https://github.com/unicode-org/cldr | Unicode License v3 (text in `NOTICE`) |
| `crates/uccharan/lang/sa.lang` `[numbers]` | Sanskrit cardinal number words 0–99 | Written by hand for this project (classical forms) | Apache-2.0 |
| `tools/eval/gold/african_names.src`, `african_names.tsv` (evaluation only, not shipped) | 388 African names with hand-written English-context pronunciations | Written by hand for this project from published grammars (sources in the file header) | Apache-2.0 |
| `tools/eval/gold/{north_african,egyptian,se_african}_names.{src,tsv}` (evaluation only, not shipped) | 181 North African, 167 Egyptian and 184 Chichewa, Shona, Tsonga and Tigrinya names with hand-written English-context pronunciations | Written by hand for this project from published grammars (sources in the file headers) | Apache-2.0 |
| `tools/eval/data/<lang>_dev.tsv`, `<lang>_heldout.tsv` (evaluation only, not shipped) | Sentences for measuring agreement | Tatoeba (CC BY 2.0 FR) or, where Tatoeba is small, the FLORES-200 `dev`/`devtest` splits; for Konkani, sentences from AI4Bharat Sangraha (`verified/gom`) | CC BY 2.0 FR; FLORES-200 CC BY-SA 4.0; Sangraha CC BY 4.0 |

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

### Transliteration packs (romanised text into native script)

`uccharan.load_pack("xlit-hi")`, `loadTranslit("hi")` in JavaScript,
`Phonemizer::load_pack` in Rust, `uccharan_load_pack` in C, `--pack FILE` on
the command line. The release carries the licence text and the attributions
next to the packs (`xlit-LICENSE-CC-BY-SA-4.0.txt`, `xlit-NOTICE.txt`). The
library itself contains none of this data: without a pack it converts by rule.

| File | What it is | Source | Licence |
| --- | --- | --- | --- |
| `data/packs/xlit-hi.ucd`, `xlit-mr.ucd`, `xlit-bn.ucd`, `xlit-ta.ucd`, `xlit-te.ucd`, `xlit-gu.ucd`, `xlit-pa.ucd`, `xlit-kn.ucd`, `xlit-ml.ucd` | Per language: an n-gram transliteration model, a lexicon of romanised spellings, word and word-pair counts, English word counts, letter models and the fitted weights | Built by `cargo run --release -p uccharan-lexgen -- xlit <lang>` from the sources below (`data/sources/translit/README.md`) | **CC BY-SA 4.0** (the Dakshina word pairs and the Wikipedia-derived counts are share-alike) |
| Dakshina dataset v1.0, word lexicons `<lang>.translit.sampled.train.tsv` (not in the repository; fetched by `tools/eval/translit/prep.py`) | Romanised spellings of native words with attestation counts: model and lexicon | Google Research, https://github.com/google-research-datasets/dakshina | **CC BY-SA 4.0** |
| Aksharantar, `<lang>_train.json` (not in the repository; fetched by `prep.py`) | Word pairs: model and lexicon | AI4Bharat, https://huggingface.co/datasets/ai4bharat/Aksharantar | As stated on the dataset card: manually collected rows CC BY; rows mined from Samanantar and IndicCorp, and rows from existing sources, CC0. Rows whose source is Dakshina are **CC BY-SA 4.0** |
| Leipzig Corpora Collection samples `hin_mixed_2019_1M`, `mar_news_2020_300K`, `ben_news_2020_300K`, `{mar,ben,tam,tel,kan,mal}_wikipedia_2021_300K`, `{guj,pan}_wikipedia_2021_100K` (not in the repository) | Native word and word-pair counts | https://wortschatz.uni-leipzig.de/en/download | CC BY 4.0; the Wikipedia samples carry Wikipedia text (**CC BY-SA 4.0**) |
| Tatoeba Hindi sentences (not in the repository) | Added to the Hindi counts | Tatoeba sentence export, https://tatoeba.org | CC BY 2.0 FR |
| `data/sources/translit/en_freq.tsv` | English word counts (19 k words): how English a Latin word is | Counted from the Tatoeba English sentence export | CC BY 2.0 FR (attribution: Tatoeba contributors) |
| `data/sources/translit/<lang>/chat.tsv`, `homographs.tsv`, `recipe.txt` | Chat spellings, homographs of English words, and the build recipe with the fitted weights; Hindi also uses `data/sources/hi/romanized.tsv` | Written for this project | Apache-2.0 |
| `data/sources/translit/LICENSE-CC-BY-SA-4.0.txt`, `NOTICE.txt` | Licence text and attributions shipped next to the packs | Creative Commons; this project | — |

If you redistribute a transliteration pack, or a model built with one, the
share-alike terms of CC BY-SA 4.0 apply to it.

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
| `tools/eval/data/{bn,mr,...,bho,mai,mag,es,de,nl,fr,it,pt-br,pt,pl,tr,cs,sv,ca,ro,hu,fi,nb,da}_{dev,heldout}.tsv` (FLORES-based ones) | Sentences of FLORES-200 dev / devtest where Tatoeba has too few | FLORES-200, https://github.com/facebookresearch/flores | CC BY-SA 4.0 (evaluation only) |
| `tools/eval/gold/bho/*.src`, `gold.tsv` | Bhojpuri gold set: 437 words and 153 sentences with hand-written pronunciations; per-item word source (FLORES-200 bho_Deva, CC BY-SA 4.0; Grierson's *Linguistic Survey of India* V.2, 1903, public domain; the language description's examples) and date in the file | Written by hand for this project | Apache-2.0 (words attested in FLORES are facts of the language, used for evaluation only) |
| `tools/eval/gold/mai/*.src`, `gold.tsv` | Maithili gold set: 410 words and 151 sentences; 134 words with pronunciations from WikiPron's `mai_deva_narrow` list, the rest hand-written; per-item source and date in the file | WikiPron (https://github.com/CUNY-CL/wikipron) and written by hand | Wiktionary items CC BY-SA 4.0 (evaluation only); the rest Apache-2.0 |
| `tools/eval/gold/mag/*.src`, `gold.tsv` | Magahi gold set: 512 words and 152 sentences; 300 words from Wiktionary entries (lemma and romanisation via kaikki.org), the rest attested in FLORES-200 mag_Deva or grammar descriptions; pronunciations hand-written or converted from the romanisation; per-item source, licence and date in `gold.tsv` | Wiktionary (https://kaikki.org/dictionary/Magahi/), FLORES-200 and written by hand | Wiktionary and FLORES items CC BY-SA 4.0 (evaluation only); the rest Apache-2.0 |
| `tools/eval/soundalike/terms/*.tsv` | Term sets of the sound-alike evaluation: 310 Indian personal names with their Devanagari forms, 173 brand and product names, 168 technical and domain terms (97 with a spoken form), 259 mishearings | Written by hand for this project; brand and product names are test items only and belong to their owners | Apache-2.0 |
| `tools/eval/soundalike/data/items.jsonl`, `confusions.jsonl` | Sentences with the terms (hand-written templates), sentences without a term (from the Tatoeba and Sherlock Holmes files above, each with its id; 60 hand-written romanised Hindi ones), and what small speech recognisers (Whisper tiny and base, Moonshine tiny) wrote when synthetic voices (Piper `en_US-amy-low` and `hi_IN-rohan-medium`, Kokoro v1.0) read them | Written by `tools/eval/soundalike/build_items.py` and `make_confusions.py` | Apache-2.0; the Tatoeba sentences CC BY 2.0 FR (attribution: Tatoeba contributors; the `source` field is the sentence id) |
| `tools/eval/gold/hinglish/dev.tsv`, `dev-chat.tsv`, `test.tsv` | Romanised Hindi chat sentences with English mixed in, and their references (130 development sentences, 164 held-out) | Written for this project | Apache-2.0 |
| Dakshina dataset v1.0: `<lang>.translit.sampled.{dev,test}.tsv` and `<lang>.romanized.rejoined.{dev,test}.*` (not in the repository) | Development and held-out word pairs and romanised sentences for transliteration | https://github.com/google-research-datasets/dakshina | CC BY-SA 4.0 (evaluation only) |
| Aksharantar `<lang>_valid.json`, `<lang>_test.json` (not in the repository) | Development and held-out word pairs for transliteration | https://huggingface.co/datasets/ai4bharat/Aksharantar | CC BY / CC0 as on the dataset card (evaluation only) |
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
