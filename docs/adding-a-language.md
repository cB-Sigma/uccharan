# Adding a language

The next languages planned are Hindi's Devanagari relatives: Marathi, Nepali, Maithili and
Bhojpuri. This is the path a new language takes.

## 1. Decide what "compatible" means

Find the voices you want to serve and read their configuration: which espeak voice name they were
trained with (`espeak.voice` in a Piper `.onnx.json`), which symbols are in their table, which
espeak-ng version. Install that espeak-ng yourself and pin the version in your notes.

## 2. Build the evaluation sets first

Extend `tools/eval/build_sets.py`: a development sample and a held-out set of sentences from a
redistributable source (Tatoeba is CC BY), disjoint by sentence id, plus word lists. Decide now
that you will never look at held-out disagreements while writing rules.

## 3. Write the front end

Add `crates/uccharan/src/<lang>/mod.rs` with a `span(items, clause_final, compat) -> Vec<WordOut>`
function, a variant in `Lang`, and a routing rule in `Phonemizer::clause_words`.

For a Devanagari language most of `hi/mod.rs` carries over; what differs is exactly what you must
probe and write:

- the letter inventory and its values (Marathi `ळ`, `ऱ्`, the two values of `च ज झ`; Nepali's
  vowel lengths and `क्ष`, `ज्ञ`);
- inherent-vowel deletion: Marathi and Nepali keep more final vowels than Hindi; Maithili and
  Bhojpuri have their own patterns;
- anusvara and chandrabindu;
- stress, if the target notation marks it;
- number words and abbreviations in `norm.rs`.

Work from descriptions of the language and from the development set's disagreement list
(`python tools/eval/run_eval.py --lang … --set dev --show 40`). Each systematic disagreement
becomes one hand-written rule with a comment saying what it does. If the reference is wrong about
the language, put the correct behaviour behind `improved` and keep `compatible` faithful.

## 4. Data

If the language needs a lexicon, add its sources under `data/sources/<lang>/`, teach
`tools/lexgen` to build `<lang>.ucd`, and add every file to `DATA_LICENSES.md` with its source and
licence. Share-alike data (Wiktionary, WikiPron) cannot be shipped; it may be used as an
evaluation reference that stays out of the repository.

## 5. Profiles

Check every symbol your front end can emit against the voices' tables with
`tools/eval/conformance.py`. A symbol the voice does not know is silently dropped by the model's
tokenizer, which deletes sound.

## 6. Gates

A language is described as a drop-in replacement for a profile only when, on the held-out set:

- sentence-level phoneme error rate against the reference is at most 5 %;
- no emitted symbol is outside the voice's table (other than symbols the reference itself emits
  outside it);
- an end-to-end run through at least one real voice (`tools/eval/e2e_tts_asr.py`) shows a word
  error rate within +0.5 (English-like scripts) or +1.0 (others) of the reference's.

Publish the numbers either way, in `docs/evaluation.md` and the README table.
