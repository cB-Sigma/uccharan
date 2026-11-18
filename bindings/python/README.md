# uccharan (Python)

**Phonemiser, text normaliser, transliterator and pronunciation engine** for text-to-speech and
speech recognition: 44 languages (English, Hindi, twenty-four more South Asian languages and
seventeen European ones), in Rust, Python, JavaScript and C. Apache-2.0, no GPL code or data.
([what it does](https://github.com/cB-Sigma/uccharan#readme) ·
[measurements](https://github.com/cB-Sigma/uccharan/blob/main/docs/evaluation.md))

- **Phonemes** in the shapes Piper, Kokoro, Kitten TTS, Matcha-TTS and sherpa-onnx voices expect,
  compatible with the espeak-ng output those voices were trained on; an improved mode for voices
  you train yourself.
- **Normalise-only mode**: the spoken form of numbers, money, dates and URLs for voices that read
  text, with spans aligned to the input.
- **Transliteration** of romanised Hindi, Marathi, Bengali, Tamil, Telugu, Gujarati, Punjabi,
  Kannada and Malayalam into native script.
- **Pronunciation word lists**, **sound-alike matching** of a recogniser's output against your
  vocabulary, and **tools for training voices**.

```sh
uv add uccharan          # or: uv pip install uccharan
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

uccharan.normalize("Rs. 250 on 3/4/2024", "en-us").text      # 'two hundred fifty rupees on March fourth twenty twenty four'
uccharan.normalize("₹5 करोड़ का सौदा 10:30 बजे", "hi").text     # 'पांच करोड़ रुपये का सौदा साढ़े दस बजे'

uccharan.load_pack("xlit-hi")                                # CC BY-SA 4.0 pack, downloaded once
uccharan.transliterate("mujhe kal office jana hai", "hi").text   # 'मुझे कल office जाना है'

names = uccharan.SoundIndex(["Kothari", "Paytm"])
names.apply("send it to Coat Harry", "en-us").text           # 'send it to Kothari'
```

### Compatible or improved mode

**Compatible mode** (the default) reproduces espeak-ng's phonemes, mistakes included, because that
is what existing Piper, Kokoro, Kitten TTS, Matcha-TTS and sherpa-onnx voices were trained on: use
it to drive those voices. **Improved mode** (`improved=True`; `--improved` on the command line) reads text as it is actually said:
homographs in context, names, Hindi schwa deletion and nasals, Bhojpuri, Maithili, Magahi, Dogri
and Bodo read as themselves, and written number words in Tamil, Kannada, Telugu and Malayalam. Use
it to train new voices, and always drive a voice with the mode it was trained with. Numbers and dates are read as speech by default in both modes (`normalize="natural"`); `normalize="espeak"` copies espeak-ng number for number.
[What changes, and how to choose](https://github.com/cB-Sigma/uccharan#compatible-or-improved-choosing-a-mode).

For code written against `phonemizer`:

```python
from uccharan.compat import EspeakBackend
backend = EspeakBackend("en-us", preserve_punctuation=True, with_stress=True)
backend.phonemize(["Hello, world."], strip=True)             # ['həlˈoʊ, wˈɜːld.']
```

A command-line program is installed as `uccharan` (`uccharan -l hi -p piper "नमस्ते"`).
It reads standard input as UTF-8, or as UTF-16 or UTF-32 with a byte order mark (as Notepad and
PowerShell write files), and always writes UTF-8.

On Windows, cmd and PowerShell 7 work as they are. Windows PowerShell 5.1 needs UTF-8 set for
the pipe in both directions:

```powershell
[Console]::OutputEncoding = [Text.Encoding]::UTF8   # to read uccharan's output in a pipe
$OutputEncoding = [Text.Encoding]::UTF8             # to pipe text into uccharan
"नमस्ते" | uccharan -l hi
```

Guides: [normalise-only mode](https://github.com/cB-Sigma/uccharan/blob/main/docs/normalize.md), [word lists](https://github.com/cB-Sigma/uccharan/blob/main/docs/wordlists.md),
[sound-alike matching](https://github.com/cB-Sigma/uccharan/blob/main/docs/soundalike.md), [transliteration](https://github.com/cB-Sigma/uccharan/blob/main/docs/transliteration.md),
[languages](https://github.com/cB-Sigma/uccharan/blob/main/docs/languages.md).

For training voices and checking pronunciation, `uccharan.train` and the `uccharan-train` command
label a dataset (training-ready phonemes for Piper, Matcha-TTS and icefall-style recipes, a symbol
table and a `voice.json`), choose a reading script, report phoneme and diphone coverage, and score
a recogniser's output against a reference word by word
([guide](https://github.com/cB-Sigma/uccharan/blob/main/docs/training.md)).

English for Kokoro uses a port of misaki with misaki 0.9.4's dictionaries; `load_misaki(dir)`
loads another copy.

The wheel (about 2 MB) carries American English, Hindi (which Bhojpuri, Maithili and Magahi use too) and the languages read by rules alone.
British English, the word tables of the other languages (`de`, `fr`, `it`, `ur` …, 46 KB to
0.66 MB each) and the Kokoro English data are language packs: downloaded from the GitHub release
of the same version the first time their language is used, checked against SHA-256 sums inside
the wheel, and cached in `~/.cache/uccharan`. When a pack cannot be had, the call raises
`uccharan.packs.PackError` naming the pack and its URL; a language is never read without its
word table.

For a machine without network access, fetch the packs elsewhere and point `UCCHARAN_DATA_DIR` at
them:

```sh
python -m uccharan.packs --dir /opt/uccharan-data de fr ur    # or --all; no arguments lists the packs
UCCHARAN_DATA_DIR=/opt/uccharan-data UCCHARAN_OFFLINE=1 python app.py
```

`UCCHARAN_OFFLINE=1` forbids downloads; `uccharan.load_pack(path)` loads a file directly. A wheel
with every word table compiled in is built from the source distribution with
`pip install uccharan --no-binary uccharan --config-settings=build-args="--features bundle"`. The transliteration packs (`xlit-hi`, …) and the Wiktionary packs
(`wikipron-en-us`, …) are CC BY-SA 4.0 and are fetched only when you call `load_pack` with their
name.

Documentation, measured agreement, speed and end-to-end results:
<https://github.com/cB-Sigma/uccharan>.

## Licence

Licence: Apache-2.0. Includes data from the CMU Pronouncing Dictionary (BSD-2-Clause) and misaki (Apache-2.0); see NOTICE.
The optional `wikipron-*` and `xlit-*` packs are CC BY-SA 4.0; they are separate downloads and are not part of this package.
