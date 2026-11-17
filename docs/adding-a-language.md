# Adding a language

Most languages are a **language file** (`crates/uccharan/lang/<tag>.lang`) read by a shared
engine, plus one line that registers it. English and Hindi have their own front ends; Bengali,
Marathi, Telugu, Kannada, Malayalam, Nepali and Tamil are language files. This is the path a new
language takes.

## 1. Decide what "compatible" means

Find the voices you want to serve and read their configuration: which espeak voice name they were
trained with (`espeak.voice` in a Piper `.onnx.json`), which symbols are in their table, which
espeak-ng version. Install that espeak-ng yourself and pin the version in your notes.

## 2. Build the evaluation sets first

```sh
python tools/eval/build_lang_sets.py <tag> --flores /path/to/flores200_dataset
```

writes `tools/eval/data/<tag>_dev.tsv` and `<tag>_heldout.tsv` (add the language to `LANGS` and
`FLORES` in the script). Sentences come from Tatoeba (CC BY 2.0 FR), split by a hash of the
sentence id; when Tatoeba is small, from FLORES-200 (CC BY-SA 4.0; `dev` becomes our dev set,
`devtest` our held-out set — evaluation input only, never shipped); when neither has the language,
from AI4Bharat's IndicCorp v2 monolingual text (CC0; add it to `INDICCORP` with a pattern that
marks lines written in Hindi, as for Dogri). Add the espeak voice to
`VOICES` in `tools/eval/common.py` and `tools/bench/compare.py`. Tune only on the dev set; run the
held-out set to report.

## 3. Register the language

One row in the `languages!` table in `crates/uccharan/src/lib.rs`:

```rust
/// Kannada (espeak voice `kn`).
Kn "kn" ["kn-in", "kan"] Brahmic(0x0c80), Some(include_str!("../lang/kn.lang"));
```

(variant, canonical tag, other accepted names, script, language file). Add the tag to the language
lists of the Python package (`bindings/python/python/uccharan/__init__.py`, `compat.py`) and the
npm package (`bindings/wasm/index.js`).

## 4. Write the language file

```text
[meta]                 ; engine options
tag = kn
block = 0c80           ; first code point of the script's Unicode block
stress = initial       ; initial | penult | weight | initial-light-heavy
final-secondary = no
deletion = none        ; none | final | medial (inherent-vowel deletion)
inherent = ɐ ɐ         ; inherent vowel unstressed, stressed
anusvara = homorganic  ; split | velar | homorganic | n | m
improved-numbers = written ; improved mode reads numbers as the written words of
                       ; lang/numbers/<tag>.txt (si, kok), not as the number keys

[letters]              ; one letter per line: letter, sounds (a sign may be "ɾ ɪ"; "+j" is a glide after the vowel)
ಕ k
ೃ ɹ u
ಋ ɹ u stressed          ; "stressed": attracts primary stress
X ː consonant          ; marker letters written by [pre] rules need an explicit class

[classes]
V = a e i o u

[pre]                  ; rewrite rules on letters, before reading
ಜ್ಞ -> ಜ್ಞ

[post]                 ; rewrite rules on phonemes (stress marks included), after reading
k -> ɡ / {V} _ {V}
[post.compat]          ; reference-only habits
[post.improved]        ; corrections for the improved mode

[words]                ; exceptions: word, phonemes (or "compatible | improved"); "@word" = the word as read from digits
[unstressed]           ; function words: word, level inside a clause, level at its end (0, s, p)
[numbers]              ; 0–99 words (draft them with tools/cldr_numbers.py from Unicode CLDR), then
scale100 ನೂರು           ; scale words; "scale1000+" = form when more follows, "2x" = form before a scale,
grouping indian        ; "100+" = form of an exact value when more follows, join-tens, glue-hundred,
point ದಶಮಾಂಶ            ; point, minus, percent, one-before-scale
```

Rewrite rules (`uccharan::rules`) are written `match -> output / left _ right`, with classes
`{Name}`, negated classes `{!Name}`, `#` for a word boundary, `.` for any character, `*` for
repetition, several environments separated by `|`, and `{Class} -> 0` to delete a class member.
At each position the longest match wins, then file order; environments look at the input of the
pass. The full option list is in `crates/uccharan/src/indic.rs` and `langdef.rs`.

For a new Brahmic language, start from a related file (Kannada was derived from Telugu by moving
every letter 0x80 code points), read a description of the language, compare a few dozen specified
words with the reference, then iterate on the dev set with
`python tools/eval/run_eval.py --lang <tag> --set dev --show 20`.

### Latin-script languages

A Latin-script language (Spanish, German, Czech, …) is registered with the script `Latin` and read
by `latin.rs`. Its file has the same sections, with letter rules instead of a letter table, and
these `[meta]` options:

```text
[meta]
tag = cs
vowels = a e i o u ɪ ʊ      ; the phonemes that count as syllables for the stress rule
unstressable = ə            ; vowels that never take stress
stress = initial            ; initial | penult (default) | antepenult | final | spanish
secondary = auto            ; alternate | auto (every other free syllable) | none
stress-options = final-no-secondary   ; also: initial-secondary, two-syllable-secondary,
                            ; final-vowel-unstressed, final-long
suffix = mente mˈente       ; a suffix read apart from its stem
elision = yes               ; l', qu', … are read with the next word
spell-vowels = aeiouyáéíóúůýě   ; a word without one of these letters is spelled out
spell-close-e = no
sandhi-block = h            ; no clause rule reaches into a word that starts with one of these
                            ; letters, nor into a number written in digits
vowel-mark = yes            ; clause rules can tell a word written with a vowel first
                            ; (`h`: also a word written with h and a vowel)
caps-words = de het een     ; small words that stay words in capitals; other capitals of two
                            ; or three letters are spelled (`*`: the [unstressed] words)
caps-entries = yes          ; use the table's entries for two- and three-letter capitals
spell-stress = first        ; a spelled abbreviation has its main stress on the first letter
letter-names = yes          ; a single consonant letter is said by its name
dotless-i = yes             ; capital I is the dotless ı (Turkish)
fuse = na do pro o bez      ; these words are said as one word with the next, stressed again
                            ; as a whole (Czech prepositions)
promote-before-final = a az ; these words take primary stress before a last word of the
                            ; clause that is `word p s` in [unstressed] (Hungarian)
vowel-mark-except = u       ; a word written with one of these letters first is marked `⁂⁑`
                            ; instead of `⁑` (Catalan: no glide before u)
clitic-verbs = ont sont     ; inside a clause these verbs lose their stress after one of the
clitic-subjects = ils elles ; … these words (French)

[pre]                       ; letters to phonemes; rules may write ˈ ˌ ° before a vowel
[post]                      ; on phonemes, after the stress rule
[clause]                    ; on the words of a clause joined by a boundary sign: what a word
                            ; does to its neighbour (liaison, voicing, final s before a vowel)
[spell]                     ; letter names for acronyms and words without vowels
[aspirated]                 ; words (`*`: beginnings of words) before which no clause rule
                            ; links (French h aspiré)
[h-muet.compat]             ; compatible mode: every word written with h first is treated as
                            ; aspirated unless it is listed here (the French reference voice)
[context.compat]            ; `WORD PREV|PREV READING`: compatible-mode reading of a word after
                            ; one of these words (`*a`: a word ending in a)
[words.improved]            ; `word reading`: improved-mode readings, before the word table
[spelling.improved]         ; `LETTERS FROM TO`: improved mode replaces FROM by TO in the reading
                            ; of a word written with LETTERS (French `gn nj ɲ`)
[numbers]                   ; as above, plus:
decimal comma               ; the comma is the decimal sign
swap-tens und               ; units before tens (German)
year-hundreds yes           ; 1100–1999 counted in hundreds (Dutch)
many1000 tysięcy            ; the scale word after counts not ending in 2, 3, 4 (Slavic)
and-below-hundred e         ; a word between a thousand and a rest below 100 (Portuguese)
tail-secondary no           ; words of a number after its first lose secondary stress
tail-stress no              ; … or all stress
group-stress 10000          ; from this value on, the first word of each group of three
                            ; digits keeps its stress (Hungarian)
group-stress-spaced 1000    ; the same for numbers written with spaces between the groups
stress last                 ; only the last word of a number keeps its stress (French)
fraction number             ; digits after the decimal sign: zeros as digits, then one
                            ; number of up to five digits; `fraction two`: up to two;
                            ; `fraction romanian`: up to four, no leading zero;
                            ; `fraction suffix`: one number and the word of
                            ; `fraction-suffixN` (N digits: Hungarian "öt tized");
                            ; `fraction suffix-zero`: the suffix only after zeros (Italian)
fraction-suffix3 millesimi  ; see `fraction suffix`
dot-word punkt#             ; compatible mode: the word said for a full stop between digits
                            ; that does not group thousands ("2.5")
dot-groups no               ; compatible mode: a full stop never groups thousands (Czech,
                            ; Hungarian "1.000" one, dot-word, zero zero zero)
minus-compat no             ; compatible mode: the minus sign is not read (as the voice)
percent-before සියයට        ; the word for % said before the number (Sinhala voice)
ordinal-dot SET [compat]    ; "3." before a lower-case word or a month name, or after a
                            ; word of `ordinal-dot-after`, is an ordinal (before another
                            ; capitalised word it ends the sentence: "iPhone 15. Am …"
                            ; is a cardinal), its words from the CLDR rule
                            ; set SET of lang/numbers/ord-<tag>.txt (tools/cldr_ordinals.py);
                            ; `compat`: the reference reads it so too, else only improved
                            ; and normalise-only mode; `ordN` keys give compatible readings
ordinal-dot-after SET am im ; the rule set after these words ("am 3." dritten)
ordinal-suffix er SET [compat] ; a number with this ending is an ordinal ("1er", "2ª",
                            ; "3:e"); `compat-said`: the reference also says the ending
written-drop-s cents vingts ; normalise-only mode: these written words lose their s before
                            ; another number word that is not a written-plural scale word
roman off                   ; Roman numerals are not numbers in compatible mode (they are
                            ; spelled like other capitals); `roman max 3999` raises the
                            ; default limit of 49
roman-skip VI               ; spellings that are words, not numerals
roman-before römisch#       ; a word said with the numeral in compatible mode (German);
roman-after romain          ; … or after it (French)
ord2 secondo#               ; the ordinal a Roman numeral of this value is read as (Italian)
roman6 si#                  ; a compatible-mode reading of one Roman numeral (French)
20h kaksikymmentä-h#        ; the form of a rest after a hundred (Finnish)
group space-improved        ; "1 500" is one number in improved mode only (`group space`:
                            ; in both modes, compatible mode across a plain space only)
currency € ¤euro#           ; the word said for a currency sign: where the sign stands in
                            ; compatible mode, after the amount in improved mode; its
                            ; reading goes in [words] (`¤euro# ˈeʊɾo`)
currency-plural € euros     ; improved and normalise-only mode: the currency word after an
                            ; amount that is not one (`plural one-zero`: 0 and 1 are one,
                            ; French; `plural slavic` / `plural czech`: `currency-plural`
                            ; for 2-4, `currency-many $ dolarów` for the rest)
currency-singular $ dollaro ; … the word after one, where it is not the `currency` word
currency-count * 1 un       ; … the words for this amount before a currency word (`*`:
                            ; every currency; `currency-count £ 1 une`)
currency-of de              ; … a word between amounts ending in 00 or 20-99 and the
                            ; currency word (Romanian "douăzeci de euro")
time h colon dot            ; improved and normalise-only mode: written forms read as clock
                            ; times: h "10h30" "10 h 30" "10h", colon "10:30", dot "10.30",
                            ; dot-context "10.30" after a `time-context` word or before a
                            ; `time-after` word (German "um 10.30", "10.30 Uhr")
time-hour heure heures      ; the hour word (one, more), said with the h form, with
                            ; `time-hour-always yes`, and on the full hour ("10:00")
time-and e                  ; the word between the hour word and the minutes (Portuguese)
time-hours1 une             ; the words for an hour count (feminine forms)
time-context um gegen       ; see `time … dot-context`
time-after Uhr              ; a word written after a time, said as its hour word
time-hour-minutes बजियै      ; the hour word said when minutes follow, in place of
                            ; the `time-hour` word (Dogri दस बजियै त्रीह् मिंट)
time-minute मिंट             ; a word said after the minutes
apostrophe keep             ; an apostrophe between two letters of an Indic script
                            ; stays in the word (Dogri र’लाया, a tone mark); by
                            ; default it is dropped (Assamese ল'ৰা)
written-hyphen keep         ; normalise-only mode keeps hyphens in number words (French
                            ; vingt-deux); otherwise they become spaces
written-glue tausend        ; … joins these words to the number word before them
written-glue-after hundert  ; … joins the number word after these to them
written-plural million millions ; … the plural of a scale word after a count other than one
[dates]                     ; dates written with digits (improved and normalise-only mode;
                            ; compatible mode keeps the voice's piece-by-piece reading):
                            ; month1 … month12 (CLDR common/main, format context, wide),
                            ; pattern D de M de Y, day cardinal | ordinal SET |
                            ; possessive SET (Hungarian), day1 premier, year cardinal |
                            ; hundreds | ordinal SET (Polish)
[numbers.written]           ; the written form of a number word for normalise-only mode,
                            ; where it is not the word without its marks (`#`, `num`, and
                            ; the variant suffixes `v2`, `2`, `x`, `-h`): `tusen# ett`;
                            ; `*FROM TO` replaces inside a word (`_` is a space, `FROM$`
                            ; only at its end: Spanish `*+ì+ _y_`, Italian `*otre$ otré`);
                            ; a number table of lookup keys (si, sd) gives every key its word
```

Normalise-only mode (`normalize`) writes numbers with the same `[numbers]` rules as the
phonemiser, so a new number word needs its written form checked: the round-trip test in
`crates/uccharan/tests/normalize_api.rs` reads the written words back with the phonemiser.

Letters that neither the letter rules (`[pre]`, `[spell]`) nor the word table's model know,
in names from other languages (Łódź, Dvořák, Erdoğan), are read as their base letters (ł l,
ř r, ğ g, ø o, ß ss), as the reference voices read them.

### A word table with a model

Letter rules alone rarely reach the reference voice's agreement in a language with a large
exceptions dictionary. A language can ship a **word table** (`crates/uccharan-data/data/<tag>.ucd`),
built by `tools/lexgen` from `data/sources/<tag>/words.tsv`:

```text
# comment lines: what the list is and where it comes from
#model
word<TAB>reading in compatible mode
```

- Without a `#model` line the file lists exceptions only: every row becomes a table entry.
- With `#model`, the file is a **full** list of words with their readings (for the existing
  languages: the words of a corpus that occur at least twice, read by the reference voice alone
  and inside sentences). `lexgen` trains a letter-to-sound model on it, stores the model in the
  table and keeps as entries only the words that the run-time reads differently without an entry.
  At run time the order is: `[words]` of the language file, table entry, spelled word, model,
  letter rules. For a Latin-script language the model reads the spelling, and its stress marks
  are the ones the language's stress rule cannot derive.
- An optional `data/sources/<tag>/train.tsv` (same format) adds words that only train the model:
  they get no table entries, so the table does not grow (Danish: rarer corpus words read alone).
- For a language read by a letter table (Sindhi), the model reads what the letter rules give for
  the word and corrects it; `#model 3` asks that a wide context be seen three times. Measure
  first: for Urdu such a model made unlisted words worse, and it is not used.

Add the table to `LATIN_TABLES` (or the word-table list) in `tools/lexgen/src/main.rs`; to the
`uccharan-data` crate (a feature named after the tag in its `Cargo.toml`, the tag in `TAGS` and an
arm in `table()`); to the `embed-<tag>` features and the `bundle` list of `crates/uccharan` and
`crates/uccharan-capi`; to the lists in `bindings/wasm/build.mjs` and `index.js`; and to
`DATA_LICENSES.md` with its copies, as a row that
says what the list was derived from. Then:

```sh
LEXGEN_ONLY=<tag> cargo run --release -p uccharan-lexgen -- build   # one table
cargo run --release -p uccharan-lexgen -- check                     # all tables are up to date
python3 tools/packs.py                                              # new SHA-256 sums; tests load tables by them
cargo run --release -p uccharan-lexgen -- g2p-eval <tag> words.tsv 1  # every tenth word held out
```

`check` builds the tables on all cores and remembers, under `target/lexgen-check`, the tables it
found up to date with unchanged inputs (`LEXGEN_NO_CACHE=1` to check afresh). Report held-out
results in three groups: words in the table, other listed words, words outside the list.

## 5. Gates before a language ships

- held-out agreement measured and written into the README language table, honestly, including
  where it falls short;
- unit tests with words from the language's description (`crates/uccharan/tests/indic_languages.rs`,
  `latin_languages.rs`);
- `cargo test`, the Python tests and the WebAssembly tests pass;
- speed measured against the same espeak-ng voice (`tools/bench/compare.py --langs <tag>`);
- every data source recorded in `DATA_LICENSES.md` (number words from CLDR: Unicode License v3,
  already in `NOTICE`); share-alike or non-commercial lexicons go into a separate downloadable pack,
  never into the core.
