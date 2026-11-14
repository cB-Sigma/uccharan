# uccharan (Python)

**5× faster than espeak-ng. Just as accurate. Free to ship.**
A drop-in phonemiser for Piper, Kokoro and sherpa-onnx voices — English, Hindi, nineteen more South Asian languages, Spanish, French, German and Dutch, in Rust, Python,
C and the browser. Apache-2.0. ([measurements](https://github.com/cB-Sigma/uccharan/blob/main/docs/evaluation.md))

Permissively licensed text-to-phoneme conversion for neural TTS: English (US, British RP), Hindi
and Hinglish, in the shapes Piper, Kokoro, Kitten TTS, Matcha-TTS and sherpa-onnx voices expect.
No GPL code or data; no runtime dependencies.

```sh
pip install uccharan
```

```python
import uccharan

uccharan.phonemize("Hello, world.")                          # 'həlˈoʊ, wˈɜːld.'
uccharan.phonemize("नमस्ते दुनिया", lang="hi")                  # 'nəmˈʌsteː dˈʊnɪjˌaː'
uccharan.phonemize("Hi there. Bye!", "en-us", "piper")       # ['hˈaɪ ðˈɛɹ.', 'bˈaɪ!']
uccharan.phonemize("Hi there.", "en-us", "sherpa-onnx")      # [[104, 712, 97, 618, …]]
uccharan.phonemize("I read it yesterday.", improved=True)    # 'aɪ ɹˈɛd ɪt jˈɛstɚdˌeɪ.'
uccharan.phonemize("I read the book.", "en-us", "kokoro")    # 'ˌI ɹˈɛd ðə bˈʊk.'  (misaki conventions, for Kokoro)
uccharan.phonemize_batch(lines, "hi")                        # all cores, GIL released
```

For code written against `phonemizer`:

```python
from uccharan.compat import EspeakBackend
backend = EspeakBackend("en-us", preserve_punctuation=True, with_stress=True)
backend.phonemize(["Hello, world."], strip=True)             # ['həlˈoʊ, wˈɜːld.']
```

A command-line program is installed as `uccharan` (`uccharan -l hi -p piper "नमस्ते"`).

For training voices and checking pronunciation, `uccharan.train` and the `uccharan-train` command
label a dataset (training-ready phonemes for Piper, Matcha-TTS and icefall-style recipes, a symbol
table and a `voice.json`), choose a reading script, report phoneme and diphone coverage, and score
a recogniser's output against a reference word by word
([guide](https://github.com/cB-Sigma/uccharan/blob/main/docs/training.md)).

English for Kokoro uses a port of misaki with misaki 0.9.4's dictionaries; `load_misaki(dir)`
loads another copy.

The wheel (about 3 MB) carries the en-US, Hindi and other South Asian, Spanish, German, Dutch and French language data. British English and the Kokoro English data are
language packs: downloaded from the GitHub release of the same version the first time they are
needed, checked against SHA-256 sums inside the wheel, and cached in `~/.cache/uccharan`. On an
offline machine put the `.ucd` files (from the release page) in a directory named by
`UCCHARAN_DATA_DIR`, set `UCCHARAN_OFFLINE=1` to forbid downloads, or call
`uccharan.load_pack(path)`.

Documentation, measured agreement, speed and end-to-end results:
<https://github.com/cB-Sigma/uccharan>.

## Licence

Licence: Apache-2.0. Includes data from the CMU Pronouncing Dictionary (BSD-2-Clause) and misaki (Apache-2.0); see NOTICE.
