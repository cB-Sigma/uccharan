# Changelog

All notable changes are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow semantic versioning.

## [0.5.0] - 2026-11-14

Language data on demand: the word tables of sixteen Latin-script languages, Urdu and Sindhi
are now packs that are downloaded and checked the first time a language is used, which makes the
wheel, the crate and the C library much smaller; the tables themselves are in the new crate
`uccharan-data`. Roman numerals such as IV and IX were read one step too high in every
Latin-script language; that is fixed. Capitals, Roman numerals and single letters now follow each
language's reading, which lowers the error rate in all seventeen Latin-script languages.

### Fixed

- **Roman numerals with a subtracted letter** (`IV`, `IX`, `XL`, `XIV` …) were read one step too
  high in every Latin-script language (`Carlos IV` as "cinco"). Spellings that are not numerals
  (`IIII`, `IL`) are no longer read as numbers.
- **Spanish and Catalan single letters** (`plan B`, `vitamina C`) are said by their names instead
  of as a bare consonant.

### Changed

- **Capitals, Roman numerals and Catalan sandhi in compatible mode** (all seventeen Latin-script
  languages; held-out halves scored again, see docs/evaluation.md): a token in capitals is spelled
  when it has two or three letters or no vowel and read as a word otherwise (`NATO`, `COVID`);
  Finnish, Hungarian and Czech stress the first letter of a spelled abbreviation; Turkish capital
  I is the dotless letter. Roman numerals follow each voice: cardinals (Spanish, Portuguese,
  Catalan, Czech, Romanian), "römisch" + cardinal (German), cardinal + "romain" (French), ordinals
  (Italian, Danish), letters (Dutch, Polish, Turkish, Swedish, Hungarian, Finnish, Norwegian);
  improved mode reads them as numbers everywhere. Catalan voices final s, ʃ and z before voiced
  sounds and before h-words and reads the conjunction *i* as a glide before a vowel; Finnish tens
  after a hundred carry secondary stress. Held-out PER: Spanish 0.74 → 0.55 %, Italian 0.66 →
  0.53 %, Catalan 1.86 → 1.51 %, Polish 1.33 → 1.19 %, German 1.01 → 0.95 %, Dutch 1.77 → 1.69 %,
  French 0.85 → 0.79 %, Portuguese 0.67 → 0.61 % (Brazil) and 0.89 → 0.84 % (Portugal), Turkish
  0.72 → 0.68 %, Czech 1.40 → 1.32 %, Swedish 1.51 → 1.44 %, Romanian 1.00 → 0.97 %, Hungarian
  1.44 → 1.37 %, Finnish 1.64 → 1.57 %, Norwegian 1.11 → 1.03 %, Danish 2.26 → 2.22 %.
  New language-file options: `roman`, `roman-skip`, `roman-before`, `roman-after`, `ordN`,
  `romanN`, `Nh` number forms, `caps-words`, `caps-entries`, `spell-stress`, `letter-names`,
  `dotless-i`, `vowel-mark = h`.
- **Numbers written in groups of three digits with spaces** (`10 000`, `1 250 000`) are read as
  one number in German, Dutch, French, Polish, Swedish, Romanian, Hungarian, Finnish and
  Norwegian. French makes no liaison before words with h aspiré (`les haricots`) or before
  *huit* and *onze*.

- **Word tables are fetched when a language is first used** instead of being part of every
  package. English, Hindi and the languages read by rules alone stay built in; the word tables
  of German, Dutch, French, Italian, Portuguese (both), Polish, Turkish, Czech, Swedish, Catalan,
  Romanian, Hungarian, Finnish, Norwegian, Danish, Urdu and Sindhi (46 KB to 0.5 MB each) are
  packs attached to the GitHub release, checked against SHA-256 sums built into the package and
  cached in `~/.cache/uccharan/<version>/`. Sizes: wheel 5.6 → 2.0 MB, crate 8.4 → 4.9 MB
  compressed, C library 11.6 → 7.6 MB; the npm package already loaded each language's file on
  first use.
  - A language is never read without its table: Python raises `PackError`, the C calls return
    NULL and the new `uccharan_last_error()` gives the reason (`uccharan_ensure()` checks ahead),
    Rust's new `Phonemizer::ensure` returns the error (`phonemize` panics with the same message
    when `ensure` was not called and the table cannot be loaded), and `phonemizeSync` in
    JavaScript throws. Every message names the pack, its URL and where the file is looked for.
  - Offline: `python -m uccharan.packs --dir DIR de fr …` (or `--all`) fetches packs for
    `UCCHARAN_DATA_DIR`, which may now name several directories; `UCCHARAN_OFFLINE=1` forbids
    downloads.
  - Rust: `features = ["bundle"]` compiles every table in (they live in the new `uccharan-data`
    crate; `embed-<tag>` features still select single tables), `features = ["download"]` fetches
    a missing table on first use; without either, tables are read from `UCCHARAN_DATA_DIR` or
    the cache. New module `uccharan::packs`, `Phonemizer::missing_packs`,
    `Phonemizer::rules_fallback`. `Phonemizer::has` is now false for a language whose table is
    not loaded, and `Phonemizer::empty()` reads such a language with its letter rules.
  - Command line: `uccharan --packs` lists the packs, `uccharan --download NAME…` fetches them
    (with the `download` feature). The C library builds with `--features bundle` or `download`
    as well.

- **Hungarian**: a list of 2.4 k words with only secondary stress at the end of a clause, and
  number words with one stress per group of three digits. Held-out PER 2.08 → 1.44 % on
  FLORES-200 (1.02 % without digits; second scoring of that half); words in the table 1.8 % wrong,
  other listed words 0.2 %, words outside the word list 3.8 % (1121 words).
- **Sindhi**, words outside the word list: `sd.ucd` now holds a model that corrects the letter
  rules' reading of a word (trained on the readings of the 61 k-word list) and the words that
  rules and model together get wrong (6.9 k entries, 120 KB; was 18 k entries, 209 KB). Held-out
  PER 1.52 → 1.30 % (third scoring of that half, see docs/evaluation.md); words outside the list
  29.8 → 21.9 % wrong. The same model made Urdu's unlisted words worse on the development
  sentences (45.7 → 49.6 % wrong), so Urdu keeps its table of exceptions.

## [0.4.0] - 2026-11-14

### Added

- **Italian** (`it`): compatible with the espeak-ng voice `it`; word table `it.ucd` (9.2 k entries and a letter-to-sound model from 104 k words, 188 KB, feature `embed-it`), number words, letter names and clause rules. Held-out PER 0.66 % on FLORES-200 (0.48 % without digits); words in the table 0.5 % wrong, other listed words 0.1 %, words outside the word list 32.2 % (503 words); 4.6× the speed of espeak-ng.
- **Portuguese (Brazil)** (`pt-br`): compatible with the espeak-ng voice `pt-br`; word table `pt-br.ucd` (5.1 k entries and a letter-to-sound model from 82.5 k words, 171 KB, feature `embed-pt-br`), number words, letter names and clause rules. Held-out PER 0.67 % on FLORES-200 (0.46 % without digits); words in the table 0.7 % wrong, other listed words 0.8 %, words outside the word list 42.0 % (490 words); 5.1× the speed of espeak-ng.
- **Portuguese (Portugal)** (`pt`): compatible with the espeak-ng voice `pt`; word table `pt.ucd` (6.9 k entries and a letter-to-sound model from 81.6 k words, 215 KB, feature `embed-pt`), number words, letter names and clause rules. Held-out PER 0.89 % on FLORES-200 (0.64 % without digits); words in the table 1.3 % wrong, other listed words 1.8 %, words outside the word list 50.2 % (484 words); 5.2× the speed of espeak-ng.
- **Polish** (`pl`): compatible with the espeak-ng voice `pl`; word table `pl.ucd` (18.8 k entries and a letter-to-sound model from 127 k words, 239 KB, feature `embed-pl`), number words, letter names and clause rules. Held-out PER 1.33 % on FLORES-200 (1.05 % without digits); words in the table 3.6 % wrong, other listed words 2.7 %, words outside the word list 30.7 % (1058 words); 3.1× the speed of espeak-ng.
- **Turkish** (`tr`): compatible with the espeak-ng voice `tr`; word table `tr.ucd` (16.3 k entries and a letter-to-sound model from 151 k words, 230 KB, feature `embed-tr`), number words, letter names and clause rules. Held-out PER 0.72 % on FLORES-200 (0.51 % without digits); words in the table 0.1 % wrong, other listed words 0.0 %, words outside the word list 22.3 % (1087 words); 2.7× the speed of espeak-ng.
- **Czech** (`cs`): compatible with the espeak-ng voice `cs`; word table `cs.ucd` (4.9 k entries and a letter-to-sound model from 133 k words, 78 KB, feature `embed-cs`), number words, letter names and clause rules. Held-out PER 1.40 % on FLORES-200 (1.13 % without digits); words in the table 0.9 % wrong, other listed words 2.9 %, words outside the word list 11.4 % (501 words); 1.8× the speed of espeak-ng.
- **Swedish** (`sv`): compatible with the espeak-ng voice `sv`; word table `sv.ucd` (7.0 k entries and a letter-to-sound model from 94.8 k words, 171 KB, feature `embed-sv`), number words, letter names and clause rules. Held-out PER 1.51 % on FLORES-200 (0.88 % without digits); words in the table 0.1 % wrong, other listed words 0.0 %, words outside the word list 32.7 % (1161 words); 3.7× the speed of espeak-ng.
- **Catalan** (`ca`): compatible with the espeak-ng voice `ca`; word table `ca.ucd` (8.1 k entries and a letter-to-sound model from 71.0 k words, 195 KB, feature `embed-ca`), number words, letter names and clause rules. Held-out PER 1.86 % on FLORES-200 (1.53 % without digits); words in the table 7.3 % wrong, other listed words 4.4 %, words outside the word list 41.8 % (553 words); 4.9× the speed of espeak-ng.
- **Romanian** (`ro`): compatible with the espeak-ng voice `ro`; word table `ro.ucd` (14.4 k entries and a letter-to-sound model from 92.4 k words, 308 KB, feature `embed-ro`), number words, letter names and clause rules. Held-out PER 1.00 % on FLORES-200 (0.89 % without digits); words in the table 0.1 % wrong, other listed words 0.0 %, words outside the word list 49.1 % (629 words); 3.3× the speed of espeak-ng.
- **Hungarian** (`hu`): compatible with the espeak-ng voice `hu`; word table `hu.ucd` (3.2 k entries and a letter-to-sound model from 152 k words, 46 KB, feature `embed-hu`), number words, letter names and clause rules. Held-out PER 2.08 % on FLORES-200 (1.61 % without digits); words in the table 2.7 % wrong, other listed words 6.6 %, words outside the word list 3.8 % (1119 words); 13.1× the speed of espeak-ng.
- **Finnish** (`fi`): compatible with the espeak-ng voice `fi`; word table `fi.ucd` (5.7 k entries and a letter-to-sound model from 153 k words, 137 KB, feature `embed-fi`), number words, letter names and clause rules. Held-out PER 1.64 % on FLORES-200 (1.13 % without digits); words in the table 5.7 % wrong, other listed words 4.5 %, words outside the word list 24.6 % (1938 words); 1.7× the speed of espeak-ng.
- **Norwegian Bokmål** (`nb`): compatible with the espeak-ng voice `nb`; word table `nb.ucd` (5.2 k entries and a letter-to-sound model from 85.8 k words, 100 KB, feature `embed-nb`), number words, letter names and clause rules. Held-out PER 1.11 % on FLORES-200 (0.52 % without digits); words in the table 1.3 % wrong, other listed words 0.0 %, words outside the word list 15.9 % (1119 words); 2.3× the speed of espeak-ng.
- **Danish** (`da`): compatible with the espeak-ng voice `da`; word table `da.ucd` (26.6 k entries and a letter-to-sound model from 81.4 k words, 490 KB, feature `embed-da`), number words, letter names and clause rules. Held-out PER 2.26 % on FLORES-200 (1.81 % without digits); words in the table 0.7 % wrong, other listed words 0.4 %, words outside the word list 68.2 % (1025 words); 27.1× the speed of espeak-ng.

- **Romanised text into Indian scripts** (`uccharan::translit`): `aap kaise ho` → `आप कैसे हो`,
  `mujhe kal office jana hai` → `मुझे कल office जाना है`. Each Latin word gets candidates from a
  lexicon of attested spellings and from an n-gram transliteration model, ranked by the model,
  by word frequency and by how alike the romanised word and the candidate sound (the
  sound-alike matcher's phoneme distance); one pass over the sentence decides per word between
  English and the language, with word-pair statistics for cases like मैं / में. English words
  stay in Latin letters by default, loanwords typed in English spelling included; `english`
  = `native` respells them in the script and `convert` takes every word for a romanised one.
  The result has spans with byte, code-point and UTF-16 offsets, a kind (`indic`, `english`,
  `other`) and the alternatives of each word. Rust (`translit::transliterate`,
  `normalize_romanised`), Python (`uccharan.transliterate`), JavaScript (`transliterate`,
  `loadTranslit`), C (`uccharan_transliterate`) and `uccharan -p native` / `-p native-json`.
  The data is in nine optional packs under **CC BY-SA 4.0** (`xlit-hi`, `xlit-mr`, `xlit-bn`,
  `xlit-ta`, `xlit-te`, `xlit-gu`, `xlit-pa`, `xlit-kn`, `xlit-ml`; 3 to 9 MB each, release
  assets, never fetched automatically), built by `lexgen xlit` from the Dakshina and
  Aksharantar word pairs; the library contains none of it and converts by rule without a
  pack. With the Hindi pack loaded, the `hinglish` option of `phonemize` uses it. Hindi has
  hand-written chat-spelling and homograph lists checked on development sentences; the lists
  of the other eight languages are hand-written and not yet reviewed by native speakers.
  See docs/transliteration.md and docs/evaluation.md.

### Changed

- **Faster word-table models.** The letter-to-sound model's run-time was rewritten: the hashes of
  a letter's contexts are computed together and shared by the sound and stress tables, a bit set
  built on first use (one bit per table entry present, about 0.3 MB of memory per language in
  use) answers for the contexts a table does not have, buckets are searched by bisection, and no
  text is checked per letter. A word of running text is remembered once instead of three times,
  and each word table has a key filter (`BLM1`, one byte per entry: files grow by 4 to 10 %) so
  that a word the table does not list costs no block decode. Output is unchanged on every
  development and held-out sentence of every language. Throughput against espeak-ng 1.52.0, one
  thread: German 2.0 → 5.8×, Dutch 1.4 → 3.2×, French 3.0 → 5.9×; with the
  earlier run-time Turkish, Czech, Finnish and Norwegian were slower than espeak-ng
  (0.9×, 0.7×, 0.7×, 0.9×) and are now 2.7×, 1.8×, 1.7× and 2.3×.
- `lexgen build` and `lexgen check` build the word tables side by side (`LEXGEN_JOBS`) and take
  about two minutes instead of twenty; `check` remembers under `target/lexgen-check` which tables
  it found up to date with unchanged inputs (`LEXGEN_NO_CACHE=1` checks afresh).
- The README and the package descriptions describe everything uccharan does: phonemiser, text
  normaliser, transliterator and pronunciation engine. The per-language notes moved to
  docs/languages.md and the comparison with other phonemisers to docs/comparison.md.
- The test suite also runs on Windows and macOS on every change.

### Fixed

- Python: `normalize(..., latin="transliterate")` and `transliterate(..., english="native")`
  load the British English data when the text has Latin-script words, as `phonemize` does.
  Without it English words were respelled from the American reading alone (`computer` came out
  as कम्प्यूरर instead of कम्प्यूटर) unless something else had loaded that data first.

## [0.3.0] - 2026-11-14

### Added

- **Tools for training voices and checking pronunciation** (`uccharan.train` in the Python
  package, and the `uccharan-train` command): `label` turns a dataset manifest (LJSpeech-style
  `metadata.csv`, Piper CSV, JSON lines, Hugging Face-style rows) into training-ready phonemes for
  Piper (`--data.phoneme_type text` CSV, phoneme-id CSV, `phonemes.json`), Matcha-TTS filelists
  and icefall-style recipes (`tokens.txt`, token lists), with a symbol table, a `voice.json` that
  records the uccharan version, mode, profile and language, single-code-point symbols throughout,
  a report of symbols outside a target table (`--symbols`, `--replace`, `--extend`), word lists,
  and per-utterance warnings (unknown characters, unread or long numbers, mixed scripts);
  `select` picks a compact reading script that covers every phone and as many diphones as
  possible (greedy set cover with length, digit, script and rare-symbol filters) and writes its
  coverage statistics; `coverage` reports phone and diphone counts, what is missing against a
  reference corpus, the duration per phone from TextGrid, JSON-lines or CTM alignments, and
  outlier utterances (too long, too many out-of-vocabulary words, wrong script, speaking rate);
  `score` computes the phoneme error of a recogniser's text or a phone recogniser's output
  against a reference with the sound-alike matcher's weighted distance, with a score per word and
  the mispronounced, missing and extra words; `mfa-dict` writes a Montreal Forced Aligner
  dictionary for a word list or corpus. In Rust, `uccharan::soundalike::score::align_words` gives
  the weighted alignment per reference word. See docs/training.md and examples/train.
- **Sound-alike matching** (`uccharan::soundalike`): correct speech-recognition output against a
  vocabulary of names, products and jargon by comparing phonemes ("Coat Harry" → "Kothari",
  "pay tm" → "Paytm", "कोठारी" → "Kothari"). A vocabulary index stores each term as its IPA, the
  reading of its spoken form, or uccharan's readings of its spelling (English in both modes,
  romanised Hindi, inner capitals split, initialisms spelled out); spans of one to three transcript
  words are compared with a weighted phoneme edit distance (voicing, aspiration, vowel length and
  retroflex / dental pairs cheap, schwa and stress nearly free, normalised by length) behind a
  consonant-skeleton pre-filter. Candidates carry byte, code-point and UTF-16 offsets, a score,
  whether the term is already there, and the kind of span (unknown word, uncertain, ordinary
  words); `apply` returns the corrected text with every change, with separate thresholds per kind
  and an optional per-word confidence gate. Rust, Python (`uccharan.SoundIndex`,
  `uccharan.match_terms`), JavaScript (`createSoundIndex`, `matchTerms`), C
  (`uccharan_sound_index_new`, `uccharan_match_terms`) and `uccharan --match-terms FILE`. A new
  evaluation (`tools/eval/soundalike`): 651 hand-written terms in English, Hindi and Hinglish
  sentences, spoken by synthetic voices and transcribed by small recognisers, plus hand-written
  mishearings and sentences without a term. Held-out, at the default thresholds: 43.9 % of
  the terms the recognisers got wrong are put back (64.2 % in English sentences) with 97.8 % of
  the corrections right, and 2 of 775 sentences without a term are changed; character edit
  distance puts back 8.8 %, Metaphone codes 18.7 % while changing 38 % of the sentences without a
  term. 7 800 transcripts a second against 1 000 terms, 1 500 against 10 000, 300 against 50 000.
- **Written number words in normalise-only mode** for Bengali, Gujarati, Kannada, Malayalam,
  Marathi, Nepali, Odia, Punjabi, Tamil, Telugu and Urdu: `normalize()` now spells numbers with
  the standard orthography of Unicode CLDR's spell-out rules (new `uccharan::rbnf`, rule files in
  `crates/uccharan/lang/numbers/`, written by `tools/cldr_rbnf.py`) instead of the phonemiser's
  voice-oriented number words: Tamil இருபத்து ஐந்து and இரண்டாயிரத்து இருபத்து நான்கு, Nepali तीन, Marathi
  टक्के, Bengali শতাংশ. Phonemisation is unchanged. Gold sets extended (bn, mr, ne, ta) and added
  (te, kn, ml, gu, pa, or, ur; `tools/eval/gold/norm`); held-out exact match after the change:
  bn 14/14, mr 14/14, ne 14/14, ta 14/14 (the first ten rows of each were scored for 0.2.0),
  te 7/7, kn 7/7, ml 7/7, gu 7/7, pa 7/7, ur 7/7, or 6/7.
- **Spanish** (`es`) on a new Latin-script engine (`latin.rs`; language files with letter rules,
  a stress module, clause-level rules across word boundaries, `[spell]` letter names, Roman
  numerals, `+`-joined number words): held-out PER 0.74 % against espeak-ng 1.52.0 `es` on
  FLORES-200 (0.45 % without digits), 2.3× faster. Improved mode: e for the voice's ɛ, ai ei oi
  au eu, d after l and n.
- **German** (`de`): on the Latin-script engine with German letter rules (vowel length from the spelling, unstressed prefixes, stressed particles and loan suffixes, final devoicing, r allophones), number words with units before tens and the decimal comma, and a word table (`de.ucd`, 16 k entries, 357 KB, feature `embed-de`) made by running espeak-ng 1.52.0 on a 139 k-word list. Held-out PER 1.01 % on FLORES-200 (0.82 % without digits); words of the word list 0.1 % wrong, words outside it 46.7 % (811 of 17,344 scored words); 2.4× faster. Improved mode writes ɐ for -er and aː for the voice's ɑː.
- **Dutch** (`nl`): Dutch letter rules (vowel length by open and closed syllable, unstressed prefixes and endings, -lijk, final devoicing, g and ch), stop and s assimilation across words inside a clause, number words (one-word tens, years counted in hundreds, decimal comma) and a word table (`nl.ucd`, 12 k entries, 231 KB, feature `embed-nl`; 80 k-word list). Held-out PER 1.77 % on FLORES-200 (1.39 % without digits); words of the word list 1.8 % wrong, words outside it 54.7 % (803 of 19,173 scored words); 1.8× faster. Improved mode marks i, y and u long before r.
- **French** (`fr`): French letter rules (silent final letters, nasal vowels, digraphs, schwa in open syllables, stress on the last full syllable), elided words read with the next word (`l'école`), liaison consonants kept as latent marks in the word table and said before a vowel inside a clause (`les amis` → `le-z amˈi`), the voice's clitic hyphen (`ʒə- nə- sˈɛ pˈa`), one-word number readings with stress on the last word, and a word table (`fr.ucd`, 12 k entries, 170 KB, feature `embed-fr`; 100 k-word list). Held-out PER 0.85 % on FLORES-200 (0.65 % without digits); words of the word list 1.6 % wrong, words outside it 32.4 % (330 of 21,347 scored words); 2.8× faster. Improved mode writes ɥ before i.
- **A letter-to-sound model in the word tables** of German, Dutch and French: back-off tables over
  letter contexts, trained on each language's full word list, with stress marks chosen so that the
  language's own stress rule gives the reference pattern. It reads the words a table does not
  list, and the tables keep only the words the model and the rules get wrong, which makes them
  half the size or less (German 748 → 357 KB). Words held out of the model's training list:
  German 62.1 → 69.0 % right, Dutch 48.6 → 59.4 %, French 83.7 → 86.4 %. The held-out figures
  above include the model (before it: German 1.29 %, Dutch 1.91 %, French 0.98 %; see
  docs/evaluation.md).

### Changed

- **Urdu and Sindhi**, words outside the tables: consonant pairs said without a vowel (نٹ, رڈ, ہ before
  a consonant, …), final ہ after a consonant as a weak vowel that keeps the short vowel before it,
  izafat kasra as eː, ی before و as eː, unstressed iː short, Sindhi on the Hindi-type deletion. The
  tables now hold only the words the rules get wrong, taken from 120 k Urdu and 61 k Sindhi corpus
  words (`ur.ucd` 47 k entries, 495 KB; `sd.ucd` 18 k entries, 192 KB). Held-out PER Urdu 2.18 →
  1.44 %, Sindhi 2.79 → 1.52 %. New letter attribute `weak` in language files.

### Fixed

- Python: `bho`, `mai` and `mag` raised "unknown pack" on first use (they are read with the
  Hindi data, which the wheel carries); they now work without a pack.
- Command line (the `uccharan` program, `python -m uccharan` and `uccharan-train`): standard input
  and output are UTF-8 on every platform; on Windows, phonemes could not be written to a console
  or pipe that used a legacy code page.

## [0.2.0] - 2026-11-14

Nineteen more South Asian languages; smaller, with modular language packs; and better at names.

### Added

- **Normalise-only mode** for voices that read text: `normalize()` in Rust, Python, JavaScript
  and C (`uccharan_normalize`), and `-p text` / `-p text-json` on the command line. Returns the
  spoken form, sentence ranges and typed spans aligned to the input (byte, code-point and UTF-16
  offsets). Options: digit language (context, request or fixed), lakh/crore or international
  English, Latin words kept, respelled or spelled, URLs and emails read, kept or dropped. New
  readings: `Rs.`/`INR`/`रु.` amounts, amounts with scale words (`₹5 करोड़`), Hindi clock and
  fractional idioms (साढ़े दस बजे, डेढ़ लाख), `+91` numbers, Indian-English lakh/crore, day-first
  dates in en-US when the first part cannot be a month. Gold sets for en, hi, mr, bn, ne and ta
  (`tools/eval/gold/norm`, `tools/eval/norm_eval.py`); held-out exact match en 45/45, hi 42/42,
  mr 9/10, bn 9/10, ne 8/10, ta 5/10.
- **Pronunciation word lists**: TSV format with word, language, IPA, spoken form and match rule
  (whole word, case, exact substring, regular expression), W3C PLS 1.0 import and export, ordered
  lists with first-list-wins priority. Applied in phonemisation (IPA converted to the profile's
  phone set, including Kokoro) and in normalise-only mode (spoken form, or a respelling chosen by
  reading candidates back with uccharan, for 13 scripts and English).

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

- **Hindi number words in compatible mode** now follow espeak-ng's own readings of 0–99 (a table
  derived from running espeak-ng 1.52.0 on those numbers) and its one-word "एक सौ": held-out PER
  Hindi 0.12 → 0.09 %, Bhojpuri 0.80 → 0.51 %, Maithili 0.75 → 0.46 %, Sanskrit 0.95 → 0.64 %.
- **Magahi** (`mag`): compatible mode is the Hindi voice's reading (held-out PER 0.40 % against
  `hi` on FLORES-200); improved mode adds Magahi rules to the Bhojpuri ones (s for श ष, n for ण,
  -əi verb endings, final short ि ु, nasal vowel for ं after a long vowel, reduplication). Magahi
  gold set (512 words, 152 sentences; Wiktionary, FLORES-200 and grammar sources): held-out words
  98.5 % (Hindi voice 64.4 %), sentence PER 0.71 % (7.67 %).
- **Urdu and Sindhi** (`ur`, `sd`), the first Perso-Arabic-script languages: letter rules on the
  shared engine plus word tables (`ur.ucd`, 40 k words; `sd.ucd`, 20 k words; compiled in by the
  `embed-ur`/`embed-sd` features, fetched on demand in the browser) with espeak-ng 1.52.0's
  readings of frequent corpus words in context. Held-out PER 2.18 % (Urdu) and 2.79 % (Sindhi);
  4.6× and 2.7× faster than espeak-ng. Improved mode reads ص ظ as s z and the Urdu digits ۰–۹.
  Arabic-script punctuation (، ؛ ؟ ۔) ends clauses.
- **Punjabi, Odia, Sinhala, Konkani and Sanskrit** (`pa`, `or`, `si`, `kok`, `sa`) on the shared
  Brahmic engine. Held-out PER against espeak-ng 1.52.0: Punjabi 0.41 %, Odia 0.50 %, Sinhala
  0.86 % (FLORES-200), Konkani 0.20 % (Sangraha); Sanskrit has no espeak-ng voice, so compatible
  mode is the Hindi voice's reading (0.64 % against `hi`) and improved mode reads it as Sanskrit
  (no schwa deletion, classical stress rule, visarga echo, Sanskrit number words). 2.3–5.1× faster
  than espeak-ng. Engine additions: letter names with their own stress, spelled-out words triggered
  by a letter sequence, explicit letter kinds for scripts outside the common Indic layout, a word
  break after ZWJ conjuncts, more anusvara and deletion options, a number-word table per language.
- **Bhojpuri and Maithili** (`bho`, `mai`): compatible mode reproduces espeak-ng's Hindi-voice
  reading (no espeak-ng voice exists; held-out PER 0.51 % and 0.46 % against `hi` on FLORES-200),
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
