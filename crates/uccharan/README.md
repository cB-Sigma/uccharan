# uccharan

**Phonemiser, text normaliser, transliterator and pronunciation engine** for text-to-speech and
speech recognition: 39 languages (English, Hindi, nineteen more South Asian languages and
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

`Phonemizer` is `Sync`; share one instance between threads. The language data (about 0.7 MB per
English accent, 3 KB for Hindi, 46 KB to 0.5 MB for each of Urdu, Sindhi and the European word
tables, and 2 MB per accent of misaki's dictionaries for the Kokoro profile) is compiled in
through the default `embed-*` features; turn them off and use `Phonemizer::load`, or
`Phonemizer::load_pack` on a shared phonemizer, to supply the `.ucd` packs (attached to each
GitHub release) at run time.

`translit::transliterate` works by rule without data and far better with a language's
transliteration pack (`xlit-hi.ucd`, …), loaded with `Phonemizer::load_pack`. Those packs and the
Wiktionary packs (`wikipron-*.ucd`) are CC BY-SA 4.0: they are release assets, not part of this
crate.

Measured agreement with the reference, speed and the end-to-end results are in the repository:
<https://github.com/cB-Sigma/uccharan>.

## Licence

Licence: Apache-2.0. Includes data from the CMU Pronouncing Dictionary (BSD-2-Clause) and misaki (Apache-2.0); see NOTICE.
The optional `wikipron-*` and `xlit-*` packs are CC BY-SA 4.0; they are separate downloads and are not part of this package.
