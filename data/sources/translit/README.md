# Sources of the transliteration packs

`cargo run --release -p uccharan-lexgen -- xlit <lang>` builds
`data/packs/xlit-<lang>.ucd` from the directory `<lang>/` here. The small
files are in the repository; the large inputs are fetched and prepared by the
scripts in `tools/eval/translit/` and are not checked in.

| File | In the repository | What it is |
| --- | --- | --- |
| `<lang>/recipe.txt` | yes | n-gram order, cut-offs, licence, and the weights fitted on development data |
| `<lang>/chat.tsv` | yes | hand-written chat spellings (`nhi` नहीं, `kr` कर) |
| `<lang>/homographs.tsv` | yes | hand-written spellings that are both an English word and a native word (`to` तो) |
| `en_freq.tsv` | yes | English word counts from the Tatoeba English sentences (CC BY 2.0 FR) |
| `toy/` | yes | everything for the tiny pack of the tests (`lexgen xlit-toy`) |
| `<lang>/train.tsv` | no | word pairs: native word, romanised word, source, count |
| `<lang>/freq.tsv` | no | native word counts (first line `#total`) |
| `<lang>/bigrams.tsv` | no | counts of adjacent native words |

The Hindi lists were written and checked against development sentences. The
lists of the other eight languages were written by hand without review by a
native speaker; corrections are welcome.

## Getting the large inputs

They take 25 to 280 MB per language. Any directory will do; pass it with
`--data DIR` (or `UCCHARAN_XLIT_DATA=DIR`), laid out as `DIR/<lang>/train.tsv`.

1. Word pairs. Extract the lexicons of the Dakshina dataset v1.0
   (https://github.com/google-research-datasets/dakshina, CC BY-SA 4.0) to
   `DIR/raw/dakshina/dakshina_dataset_v1.0/<lang>/lexicons/`, then run
   `python3 tools/eval/translit/prep.py <lang>` with `XLIT_DATA=DIR`. It
   downloads the language's Aksharantar archive
   (https://huggingface.co/datasets/ai4bharat/Aksharantar), writes
   `train.tsv`, and writes the development and held-out word sets
   (`dev_dak.tsv`, `test_dak.tsv`, `dev_ak.tsv`, `test_ak.tsv`). A training
   pair is left out when its native word or its romanised word occurs in a
   development or held-out file.
2. Counts. `python3 tools/eval/translit/corpus.py <lang> SENTENCES...` with
   `XLIT_DATA=DIR` counts words and adjacent word pairs in sentence files of
   the Leipzig Corpora Collection (https://wortschatz.uni-leipzig.de/en/download):
   `hin_mixed_2019_1M` for Hindi (plus the Tatoeba Hindi sentences with
   `--extra FILE:3`), `mar_news_2020_300K` and `mar_wikipedia_2021_300K`,
   `ben_news_2020_300K` and `ben_wikipedia_2021_300K`,
   `{tam,tel,kan,mal}_wikipedia_2021_300K`, `{guj,pan}_wikipedia_2021_100K`.

`cargo run --release -p uccharan-lexgen -- xlit-check` rebuilds every pack
whose inputs are present and fails when a pack is stale; packs without inputs
on the machine are reported and skipped.

## Licences

The packs are CC BY-SA 4.0 (`LICENSE-CC-BY-SA-4.0.txt`, `NOTICE.txt`; both are
attached to each release next to the packs). See `DATA_LICENSES.md` for each
source.

## Other languages on the Hindi model

Bhojpuri, Maithili and Magahi are typed with the romanisation habits of Hindi
and written in Devanagari, and there are no public word pairs for them. A pack
for one of them is the Hindi `train.tsv` with that language's own `freq.tsv`,
`bigrams.tsv`, `chat.tsv` and `homographs.tsv`, and `lang=<tag>` in its
`recipe.txt`: the letters-to-Devanagari model is shared, and the ranking then
prefers the words of the language. See `docs/transliteration.md`.
