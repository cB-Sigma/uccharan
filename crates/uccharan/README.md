# uccharan

**5× faster than espeak-ng. Just as accurate. Free to ship.**
A drop-in phonemiser for Piper, Kokoro and sherpa-onnx voices — English and Hindi, in Rust, Python,
C and the browser. Apache-2.0. ([measurements](https://github.com/cB-Sigma/uccharan/blob/main/docs/evaluation.md))

Permissively licensed text-to-phoneme conversion for neural text-to-speech: English (US and
British RP), Hindi and code-mixed Hindi + English, producing the phoneme streams that
espeak-trained voices expect (Piper, Kokoro, Kitten TTS, Matcha-TTS, sherpa-onnx), with no GPL
code or data.

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
```

`Phonemizer` is `Sync`; share one instance between threads. The language data (about 0.7 MB per
English accent, 3 KB for Hindi, and 2 MB per accent of misaki's dictionaries for the Kokoro
profile) is compiled in through the default `embed-*` features; turn them
off and use `Phonemizer::load`, or `Phonemizer::load_pack` on a shared phonemizer, to supply the
`.ucd` packs (attached to each GitHub release) at run time.

Measured agreement with the reference, speed and the end-to-end results are in the repository:
<https://github.com/cB-Sigma/uccharan>.

## Licence

Licence: Apache-2.0. Includes data from the CMU Pronouncing Dictionary (BSD-2-Clause) and misaki (Apache-2.0); see NOTICE.
