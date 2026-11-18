# uccharan

**Phonemiser, text normaliser, transliterator and pronunciation engine** for text-to-speech and
speech recognition: 42 languages (English, Hindi, twenty-two more South Asian languages and
seventeen European ones), in Rust, Python, JavaScript and C. Apache-2.0, no GPL code or data.
([what it does](https://github.com/cB-Sigma/uccharan#readme) ·
[measurements](https://github.com/cB-Sigma/uccharan/blob/main/docs/evaluation.md))

The crate turns text into the phoneme streams that espeak-trained voices expect (Piper, Kokoro,
Kitten TTS, Matcha-TTS, sherpa-onnx), gives voices that read text the spoken form of numbers,
dates and money (`normalize_api`), writes romanised Indian-language text in native script
(`translit`), applies pronunciation word lists (`wordlist`) and matches a recogniser's output
against a vocabulary by sound (`soundalike`).

```rust
use uccharan::{profile, Lang, Mode, Options, Phonemizer};

let p = Phonemizer::new();
assert_eq!(p.phonemize("Hello, world.", &Options::new(Lang::EnUs)), "həlˈoʊ, wˈɜːld.");

// One list of Unicode codepoints per sentence, for sherpa-onnx's GenerationConfig.phoneme_codepoints.
let cps = profile::sherpa_codepoints(&p.analyze("नमस्ते दुनिया।", &Options::new(Lang::Hi)));

// Improved mode: homographs in context, names, Hindi schwa deletion and nasals.
let mut o = Options::new(Lang::EnUs);
o.mode = Mode::Improved;
assert!(p.phonemize("I read it yesterday.", &o).contains("ɹˈɛd"));

// The spoken form of a text, for voices that read text.
use uccharan::normalize_api::{normalize, NormOptions};
let n = normalize(&p, "₹5 करोड़ का सौदा 10:30 बजे", Lang::Hi, &NormOptions::default());
assert_eq!(n.text, "पांच करोड़ रुपये का सौदा साढ़े दस बजे");

// Names a speech recogniser misheard, put back by sound.
use uccharan::soundalike::{ApplyOptions, IndexOptions, SoundIndex};
let index = SoundIndex::from_terms(&p, &["Kothari", "Paytm"], &IndexOptions::default());
let fixed = index.apply(&p, "send it to Coat Harry", Lang::EnUs, &ApplyOptions::default());
assert_eq!(fixed.text, "send it to Kothari");
```

`Phonemizer` is `Sync`; share one instance between threads.

### Language data

English (0.7 MB per accent), Hindi (3 KB) and misaki's dictionaries for the Kokoro profile (2 MB
per accent) are compiled in through the default features. The languages read by rules alone
(Spanish, the Indic scripts) need no data. The other languages have a **word table** (46 KB to
0.66 MB each: `de`, `nl`, `fr`, `it`, `pt-br`, `pt`, `pl`, `tr`, `cs`, `sv`, `ca`, `ro`, `hu`, `fi`,
`nb`, `da`, `ur`, `sd`), which is not in this crate. Choose how your program gets them:

| | |
| --- | --- |
| `features = ["bundle"]` | every word table compiled in (from the `uccharan-data` crate, 3.5 MB); `embed-de`, `embed-fr` … compile in single ones |
| `features = ["download"]` | a table is fetched from the GitHub release of this version the first time its language is used, checked against the SHA-256 sum compiled into the crate, and cached in `~/.cache/uccharan/<version>/`; `UCCHARAN_OFFLINE=1` forbids it |
| neither | the table is read from the directory named by `UCCHARAN_DATA_DIR`, or from the cache; the `.ucd` files are attached to each [GitHub release](https://github.com/cB-Sigma/uccharan/releases) |

A language is never read without its table by accident. `Phonemizer::ensure(lang)` loads what a
language needs and returns an error that names the missing pack, its URL and the directories
looked in; call it once per language at start-up. Without that call, `phonemize` loads the table
at first use and panics with the same message when it cannot. `Phonemizer::missing_packs(lang)`
lists what is not loaded, `Phonemizer::load_pack(bytes)` takes a pack you read yourself, and
`Phonemizer::new().rules_fallback(true)` accepts the letter rules alone (far less accurate).

`translit::transliterate` works by rule without data and far better with a language's
transliteration pack (`xlit-hi.ucd`, …), loaded with `Phonemizer::load_pack`. Those packs and the
Wiktionary packs (`wikipron-*.ucd`) are CC BY-SA 4.0: they are release assets, not part of this
crate.

Measured agreement with the reference, speed and the end-to-end results are in the repository:
<https://github.com/cB-Sigma/uccharan>.

## Licence

Licence: Apache-2.0. Includes data from the CMU Pronouncing Dictionary (BSD-2-Clause) and misaki (Apache-2.0); see NOTICE.
The optional `wikipron-*` and `xlit-*` packs are CC BY-SA 4.0; they are separate downloads and are not part of this package.
