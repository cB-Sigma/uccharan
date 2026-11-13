# Architecture

uccharan is one Rust crate with no runtime dependencies, plus thin bindings.

```
text ──► segment ──► normalise ──► route per word ──► words ──► clause pass ──► render
         text.rs     norm.rs       lib.rs             en/, hi/   en/, hi/        profile.rs
```

| Stage | Module | What it does |
| --- | --- | --- |
| Segment | `text.rs` | Splits text into clauses and chunks, keeps the clause terminator (`. , ; : ! ?`, danda), decides whether a dot ends a sentence or belongs to an abbreviation, classifies characters by script. No regular expressions; a hand-written scanner. |
| Normalise | `norm.rs` | Turns numbers, money, dates, times, units, ordinals, ranges, fractions, telephone-style digit strings, abbreviations, initialisms and symbols into words, in English or Hindi. Three styles: `Natural`, `EspeakLike` (the reference's own readings, used when measuring agreement), `Off`. |
| Route | `lib.rs` | Each word goes to the front end of its script: Devanagari to Hindi, Latin to English (the request's accent, or British English inside Hindi text, as the reference does). With the `hinglish` option, romanised Hindi words are first rewritten in Devanagari (`hi/roman.rs`). |
| English words | `en/mod.rs` | Lexicon lookup → morphology (plural, possessive, -ed, -ing, -ly, -er, -est, -ness …) → compound splitting → letter-to-sound tables → spelling out. Homographs are chosen by a part-of-speech hint from the previous word (compatible) or by clause-wide cues (`en/improve.rs`, improved). |
| English clause pass | `en/mod.rs`, `en/fw.rs` | Function words get their weak or strong form and stress level from a hand-written table; a few pairs fuse into one word as the reference prints them ("of the" → `ʌvðə`); "the" before a vowel, flapped "it/at", linking r. |
| Hindi words | `hi/mod.rs` | Devanagari → underlying sounds (conjuncts, nukta letters, anusvara, chandrabindu, visarga), gemination, weight-based stress, inherent-vowel deletion, allophones, then symbols. Two deletion algorithms: the compatible one reproduces the reference's stress-driven behaviour; the improved one is the standard right-to-left V C _ C V rule with morpheme seams. |
| Kokoro English | `kokoro.rs` | A port of misaki's English G2P (lookup in misaki's gold and silver dictionaries, stress, suffixes, numbers, sub-word splitting) with a rule-based tagger instead of spaCy. Used for the `kokoro` profile for English; misaki 0.9.4's dictionaries are compiled in (`kokoro-en-*.ucd`, `embed-kokoro` feature) and can be replaced with `Phonemizer::load_misaki`. |
| Render | `profile.rs` | Pure functions from the analysed sentences to each consumer's shape: one IPA string, Piper sentence strings and ids, sherpa-onnx codepoint lists, Kokoro tokens and ids, Kitten and Matcha ids. |

## The analysed form

`Phonemizer::analyze` returns `Vec<Sentence>`; a sentence has clauses; a clause has words and a
terminator; a word carries its source text, its phonemes in espeak-style IPA (stress marks placed
immediately before the stressed vowel), the language whose rules produced it, and whether it is
glued to the previous word. Every profile is rendered from this one structure, so adding a profile
never touches the linguistic code.

## Language data

`crates/uccharan/data/*.ucd` are small read-only images (`data.rs`):

- a lexicon: a front-coded, block-indexed sorted string table used in place (no parsing at load
  time). A lookup is a binary search over block heads followed by a scan of at most 16 entries, with
  no allocation beyond the output string;
- letter-to-sound tables for English: for each letter and a window of neighbouring letters, the
  phoneme string that letter most often produced in the lexicon. Eight window shapes from 4+4
  letters down to the bare letter; an entry is stored only when the narrower windows would predict
  something else. Counted from the CMUdict-derived lexicon, not learned from another phonemizer;
- metadata (language, format version, source).

The images are built by `tools/lexgen` from `data/sources` and are reproducible byte for byte.
English is about 2.1 MB per accent; Hindi is 4 KB because Hindi is almost entirely rules.

By default the three images are compiled into the library (`embed-*` cargo features). Without
those features, or in WebAssembly, the application loads them at run time with
`Phonemizer::load`.

## Threads and memory

`Phonemizer` is immutable after construction and `Sync`: any number of threads can call it through
a shared reference with no locks and no per-thread state. `render_batch` splits a slice of texts
over scoped threads. There is no global state anywhere, which is the main structural difference
from the reference library, whose API is a single global voice.

## Bindings

| Binding | Where | Notes |
| --- | --- | --- |
| C | `crates/uccharan-capi` | `cdylib` + `staticlib`, header written by cbindgen (`include/uccharan.h`). The only `unsafe` in the project. |
| Python | `bindings/python` | PyO3, abi3 (one wheel per platform for Python ≥ 3.9), releases the GIL in batch calls. Typed wrapper in `python/uccharan/__init__.py`; `uccharan.compat.EspeakBackend` mimics the common uses of `phonemizer`'s class. |
| WebAssembly | `bindings/wasm` | wasm-bindgen; the core carries no language data, the JavaScript wrapper fetches `data/<lang>.ucd` on first use. Works in browsers and in Node ≥ 18. |

## What is deliberately not here

- No neural model. Out-of-lexicon English words go through the letter-to-sound tables. A small
  neural fallback is on the roadmap as a separate, optional artefact.
- No regular-expression engine, no ICU, no allocator tricks: the WebAssembly core is about 410 KiB.
- No SSML, no prosody prediction, no audio.
