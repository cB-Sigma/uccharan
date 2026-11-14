# Training voices and checking pronunciation

`uccharan.train` is for the people who build voices: it turns a dataset into training-ready
phonemes, picks what to record, reports what a dataset covers, and scores pronunciation against a
reference. It is part of the Python package (`pip install uccharan`) and has a command-line
program, `uccharan-train`. Nothing here reads audio, and nothing is downloaded.

| Tool | Python | Command line |
|---|---|---|
| Dataset labelling | `train.label(...)`, `Labelled.write(dir)` | `uccharan-train label` |
| Reading-script selection | `train.select_script(...)` | `uccharan-train select` |
| Coverage report | `train.coverage(...)` | `uccharan-train coverage` |
| Pronunciation scoring | `train.score(...)`, `train.score_many(...)` | `uccharan-train score` |
| Aligner dictionary | `train.mfa_dictionary(...)` | `uccharan-train mfa-dict` |

A complete small example (ten Hindi and ten Bhojpuri sentences) is in
[`examples/train`](../examples/train).

## Why label with uccharan

A voice learns the phoneme symbols it is trained on, and has to be given the same symbols when it
speaks. Piper and sherpa-onnx 2.x both accept phonemes from outside (Piper's
`--data.phoneme_type text` for training, sherpa-onnx's `phoneme_codepoints` at run time), so the
phonemiser that labels the training data and the one inside the application can be the same
library. Three rules follow, and the labelling tool enforces them:

1. **One code point per symbol.** Piper splits the phoneme text with
   `list(unicodedata.normalize("NFD", text))`, and sherpa-onnx takes a list of code points. The
   labels are NFD text in which every code point is one symbol (`ã` is `a` + U+0303, `bʰ` is `b` +
   `ʰ`); nothing longer is ever produced, and a symbol table with longer entries (the vowel clusters
   of some Piper configs) is reported.
2. **A fixed symbol table.** The table is built from the data, or you give the table the voice must
   stay inside (a base checkpoint's config, for a warm start whose phoneme embeddings should keep
   their meaning). Symbols outside it are listed with their counts and code points, per dataset
   and per utterance. `--replace ʱ=ɦ` rewrites a symbol first, which is how breathy stops are
   written for a table that has `ɦ` but not `ʱ`; `--extend` gives the outsiders new ids instead.
3. **Record how the phonemes were made.** `voice.json` holds the uccharan version, the mode
   (compatible or improved), the profile, the language, the normalisation, the replacements and
   the word lists (name, size, SHA-256). A voice must be driven with the mode it learned; when
   uccharan's rules for a language change, label again and continue training.

## Dataset labelling

```sh
uccharan-train label metadata.csv -l hi --improved -o labels/
uccharan-train label manifest.jsonl -l hi --symbols base_voice.onnx.json --replace ʱ=ɦ --strict -o labels/
```

```python
from uccharan import train

rows = train.read_manifest("metadata.csv")              # or train.from_rows(dataset) for Hugging Face rows
done = train.label(rows, "hi", improved=True, wordlists=["names.tsv"])
done.utterances[0].phonemes                              # 'nəmˈʌsteː dˈʊnɪjˌaː.'
done.utterances[2].warnings                              # [unknown-character: 😀, mixed-script: Latin (5 letters)]
done.outside                                             # symbols the target table lacks, with counts
done.write("labels/", validation=50)
```

**Manifests.** `metadata.csv` in the LJSpeech layout (`id|text` or `id|text|normalised text`; the
second column is used, since uccharan normalises the text itself; `--text-column` picks another),
Piper's layout (`--layout piper`: `file|text` or `file|speaker|text`), JSON lines, or mappings in
the shape of Hugging Face `datasets` rows (`train.from_rows`): the audio is `audio_filepath`,
`audio` (a path or a mapping with `path`), `path` or `file`; the text is `text`, `sentence` or
`transcript`; `language`/`lang`, `speaker`, `id` and `duration` are read when present. A row with
its own language is read in that language, so one manifest can mix Hindi and Bhojpuri.

**What happens to each utterance.** The text is normalised (numbers, money, dates, symbols: see
[normalize.md](normalize.md)) and phonemised in the chosen language, mode and profile, with your
[word lists](wordlists.md) applied first. Profiles: `piper` and `sherpa-onnx` (sentences with their
clause punctuation, as those voices are fed at run time; the default), `espeak-ipa` and `matcha`.
`--no-stress` drops the stress marks.

**Warnings**, per utterance, in `labels.jsonl` and `report.json` (the command prints the first
twenty with the manifest line):

| Code | Meaning |
|---|---|
| `unknown-character` | characters uccharan dropped: emoji, symbols, letters of a script it does not read |
| `unread-number` | a number whose spoken form is empty or still has digits |
| `long-number` | eight or more digits in one number: check whether it is a phone number, an id or a quantity |
| `mixed-script` | letters of another script than the language's (Latin words in Hindi text, for example) |
| `outside-symbols` | phoneme symbols that are not in the target table |
| `no-phonemes` | nothing to say; the utterance is left out of the training files |
| `reserved-character` | a `\|` or `"` was removed from the phonemes (both would break the `\|`-separated files) |

**Files written.**

| File | For | Layout |
|---|---|---|
| `piper/metadata.csv` | Piper, `--data.phoneme_type text` | `file\|phonemes`, or `file\|speaker\|phonemes` with more than one speaker |
| `piper/metadata_phoneme_ids.csv` | Piper, `--data.data_type phoneme_ids` | `file\|[speaker\|]text\|ids`, ids framed `BOS PAD s PAD … EOS` |
| `piper/phonemes.json` | Piper, `--data.phonemes_path` | `{"symbol": [id]}`; `_` 0 (pad), `^` 1, `$` 2, space 3, then the symbols in code point order |
| `matcha/filelist.txt`, `train.txt`, `val.txt` | Matcha-TTS | `path\|phonemes`, or `path\|speaker number\|phonemes`; `--validation N` holds N utterances out |
| `matcha/tokens.txt`, `icefall/tokens.txt` | icefall recipes, sherpa-onnx | `symbol id` per line, in id order |
| `icefall/utterances.jsonl` | icefall-style recipes | one object per utterance: `id`, `audio`, `text`, `normalized_text`, `tokens` (list of symbols), `token_ids` |
| `labels.jsonl` | the other tools | everything about each utterance, warnings included |
| `voice.json` | the voice's metadata | phonemiser version, mode, profile, language(s), symbol table, speakers |
| `report.json` | you | symbol counts, unused and outside symbols, warnings per utterance |

A Piper run on these labels:

```sh
python3 -m piper.train fit --data.voice_name hi-anita --data.csv_path labels/piper/metadata.csv \
  --data.audio_dir wavs/ --data.phoneme_type text --data.phonemes_path labels/piper/phonemes.json \
  --model.sample_rate 22050 --data.cache_dir cache/ --data.config_path config.json
```

At run time the same ids come from `uccharan.piper_ids(sentence, config)` with the config Piper
wrote, or from sherpa-onnx with `uccharan.phonemize(text, lang, "sherpa-onnx", improved=...)` and
the voice's `tokens.txt`.

**What was checked against the upstream projects** (their sources and documentation as of
November 2026; no model was trained with these files):

- Piper (`OHF-Voice/piper1-gpl`, `docs/TRAINING.md` and `src/piper/train/vits/dataset.py`): the
  CSV layouts for `phoneme_type text` and `data_type phoneme_ids`, the speaker column, the NFD
  split into code points, the `BOS PAD … EOS` framing, and the JSON map read by
  `--data.phonemes_path` (an id or a list of ids per symbol). Piper reads the CSV with Python's
  `csv` module, where a field that starts with `"` is a quoted field; this is why `"` never appears
  in the phonemes. With `phoneme_ids`, set `--data.num_symbols` above the largest id.
- Matcha-TTS (`shivammehta25/Matcha-TTS`, `matcha/data/text_mel_datamodule.py` and
  `matcha/text/symbols.py`): the filelist layout and the fixed symbol table, which is built in
  (`--symbols matcha` reports what falls outside it: combining marks such as the nasalisation
  tilde do). Upstream's text cleaners phonemise with espeak-ng; a pre-phonemised filelist needs a
  cleaner that returns the text unchanged, and a symbol list that matches `tokens.txt` if the
  built-in table is not enough. That change to the recipe is yours and was not run here.
- icefall (`k2-fsa/icefall`, `egs/ljspeech/TTS/vits/tokenizer.py` and
  `local/prepare_tokens_ljspeech.py`, shared by the Matcha recipe): `tokens.txt` (`symbol id`; a
  line with only an id is the space; `_`, `^`, `$` and the space must exist) and a list of
  single-symbol tokens per utterance, which the recipe stores on each cut. `utterances.jsonl` is
  not a lhotse manifest: copy `tokens` onto your cuts by `id` in place of the recipe's own
  phonemisation step.

## Reading-script selection

For a new voice you choose what the speaker reads. From a text corpus, `select` picks a compact
set of sentences that covers every phone of the corpus and as many diphones (pairs of
neighbouring phones) as possible.

```sh
uccharan-train select corpus.txt -l bho --improved -n 500 -o script/
```

```python
script = train.select_script(sentences, "bho", improved=True, max_sentences=500)
script.sentences[0].text, script.sentences[0].new_phones
script.stats["diphones"]              # {'covered': …, 'total': …, 'missing': […]}
script.write("script/")               # script.txt, script.tsv, coverage.json
```

- **Units.** A phone is a base symbol with the marks that follow it (`aː`, `bʰ`, `ã`); stress is
  ignored unless `--stress`. Diphones cross word boundaries but not punctuation.
- **Selection.** Greedy set cover: each step takes the sentence that adds the most uncovered
  phones, then the most uncovered diphones, the shorter sentence on a tie, until nothing is gained
  or `--max-sentences` is reached. `--prefer length` divides the gain by the sentence's length,
  which gives the least text to read for the same coverage instead of the fewest sentences.
  Greedy cover is not the smallest possible set, but it is within a logarithmic factor of it.
- **Filters.** `--min-words 3`, `--max-words 20`, `--max-chars`; no digits unless
  `--allow-digits`; no letters of another script unless `--allow-mixed-script`; nothing uccharan
  drops as unknown; no sentence with a phone that occurs in fewer than `--min-symbol-sentences`
  sentences (default 2: a phone seen once in a corpus is usually a foreign word or a typing error;
  set it to 1 for a small, clean corpus); with `--symbols`, only sentences inside that table.
- **Output.** `script.txt` (one sentence per line), `script.tsv` (phonemes, the phones each
  sentence adds, running coverage) and `coverage.json`: candidates, rejections per reason, phones
  and diphones covered, total and missing, phones that occur only in rejected sentences, and the
  count of every phone in the script.

Running text is split into sentences at `. ! ? । ॥ ۔ …`; `--lines` takes one sentence per line.
On 2 000 Hindi sentences the selection takes under half a second and covers all 75 phones and all
1 701 diphones of the candidates with 483 sentences; the first 50 sentences with `--prefer length`
already cover every phone.

## Coverage report

```sh
uccharan-train coverage labels/labels.jsonl -l hi --improved --reference corpus.txt \
    --alignments aligned/ --json coverage.json
```

```python
report = train.coverage(done, "hi", improved=True, reference=train.read_texts("corpus.txt"),
                        alignments=train.read_alignments("aligned/"))
report.phones, report.diphones, report.missing_diphones[:10], report.durations["aː"], report.outliers
print(report.summary())
```

The dataset is a manifest or the `labels.jsonl` that `label` wrote.

- **Counts** of every phone and diphone, and the phones seen fewer than `--min-count` times.
- **What is missing** is judged against `--reference`, a text corpus of the language read the same
  way: the phones and diphones the corpus has and the dataset lacks, most frequent first, with
  coverage by type and by frequency. Without a reference, `--symbols` lists the symbols of a table
  that the dataset never uses.
- **Duration per phone**, when phone alignments are given: count, total, mean, median, minimum and
  maximum. Alignments are a directory of Praat TextGrid files (long format, as the Montreal Forced
  Aligner writes them; the `phones` tier), JSON lines (`{"id": …, "phones": [{"phone", "start",
  "end"}]}`) or a CTM file. Silence labels are skipped.
- **Outliers**: too long (phones or seconds; by default more than three interquartile ranges above
  the third quartile, or `--max-phones` / `--max-seconds`), too short (under three phones), a
  speaking rate far from the rest (when the manifest has durations), wrong script (fewer than half
  the letters in the language's script), and too many out-of-vocabulary words (more than
  `--max-oov`, default 30 %). A word is out of vocabulary when it is not in `--vocabulary`; without
  that list, English words are checked against the built-in lexicon, and words of the other
  languages, which are read by rule, only count when they are in another script.

## Pronunciation scoring

```sh
uccharan-train score -l hi --reference "मेरा घोड़ा बहुत तेज़ है" --heard "मेरा कीड़ा बहुत तेज है"
# score 91%, phoneme error 16.7%
#   घोड़ा [ɡʰˈoːɽaː]: heard as “कीड़ा” 73% (gʰ→k, oː→iː)
uccharan-train score -l en-us --reference "She sells sea shells" --heard-phonemes "s iː s ɛ l z s iː ʃ ɛ l"
uccharan-train score -l hi --pairs roundtrip.jsonl --json scores.json
```

```python
s = train.score("मेरा घोड़ा बहुत तेज़ है", "मेरा कीड़ा बहुत तेज है", lang="hi")
s.error_rate, s.weighted_error, s.score
[(w.word, w.status, w.score, w.errors) for w in s.words]
s.mispronounced, s.extra, s.feedback()
train.score_many(pairs, lang="hi")       # pooled error rate and the words most often wrong
```

The reference text and what was heard are both turned into phonemes and aligned with the weighted
phoneme distance of the [sound-alike matcher](soundalike.md): voicing, aspiration, vowel length
and dental / retroflex pairs are cheap, a weak vowel or an `h` costs little to drop, an unrelated
sound costs a whole phone. What was heard is a recogniser's transcript (`--heard`, read in
`--heard-lang`, so a recogniser that writes Hindi in Latin letters can still be compared), or an
IPA string from a phone recogniser (`--heard-phonemes`; spaces and stress are ignored).

- `error_rate` is the plain phoneme error rate (substitutions + deletions + insertions over the
  reference phones); `weighted_error` counts each error at its weight; `score` is one minus that.
- Every reference word gets a score, the heard words it was aligned with, and its errors in the
  matcher's broad notation: `gʰ→k` (heard as), `-h` (not heard), `+ə` (extra). A word is
  `missing` when nothing was heard for it and `mispronounced` when its score is under
  `--threshold`. A heard word that stands for no reference word is listed in `extra` and is not
  blamed on its neighbours.
- **Threshold.** In a word of four or five phones a wrong vowel length scores about 0.97, a lost
  aspiration 0.96, a voicing error 0.9, an unrelated consonant 0.8 or less. The default 0.85
  passes what recognisers commonly confuse and is meant for checking synthetic speech (text →
  voice → recogniser → score: words that keep failing point at the voice's pronunciation or at the
  phonemes it was given). For language-learning feedback use 0.95 or more, and prefer a phone
  recogniser: a word recogniser corrects a learner's slips towards real words.
- Numbers, money and symbols are compared as spoken (`₹500` against “पाँच सौ रुपये”). Word lists
  apply to both sides. Both sides are read in improved mode unless `--compatible`.

Limits: the score measures agreement between two phoneme strings. A recogniser's own errors count
as mispronunciations, and two spellings that uccharan reads differently (`पाँच` and `पांच` in
improved mode) differ here too. It was tested on text pairs, not yet against human judgements of
pronunciation.

## Pronunciation dictionary for forced aligners

```sh
uccharan-train mfa-dict corpus.txt -l bho --improved -o bho.dict
```

writes a dictionary in the Montreal Forced Aligner format (checked against its documentation: the
word, a tab, the phones separated by spaces; one line per pronunciation; lower case):

```text
आज	aː ɟ
कइसन	k ʌ i s ə n
```

The words come from a word list or a corpus. Each word is read on its own, so readings that depend
on neighbouring words are not included; `--both-modes` lists the compatible and the improved
reading when they differ, `--stress` keeps stress marks on the phones, and words in another script
are left out unless `--foreign`. The dictionary was not yet used in an aligner run.
