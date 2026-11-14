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
`devtest` our held-out set — evaluation input only, never shipped). Add the espeak voice to
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
every letter 0x80 code points), read the language specification, compare a few dozen specified
words with the reference, then iterate on the dev set with
`python tools/eval/run_eval.py --lang <tag> --set dev --show 20`.

## 5. Gates before a language ships

- held-out agreement measured and written into the README language table, honestly, including
  where it falls short;
- unit tests with words from the specification (`crates/uccharan/tests/indic_languages.rs`);
- `cargo test`, the Python tests and the WebAssembly tests pass;
- speed measured against the same espeak-ng voice (`tools/bench/compare.py --langs <tag>`);
- every data source recorded in `DATA_LICENSES.md` (number words from CLDR: Unicode License v3,
  already in `NOTICE`); share-alike or non-commercial lexicons go into a separate downloadable pack,
  never into the core.
