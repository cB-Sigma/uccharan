# uccharan (Python)

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

English for Kokoro uses a port of misaki with misaki 0.9.4's dictionaries built in (Apache-2.0;
see DATA_LICENSES.md).

Documentation, measured agreement, speed and end-to-end results:
<https://github.com/cb-Sigma/uccharan>. Licence: Apache-2.0; English data derived from CMUdict
(BSD-2-Clause).
