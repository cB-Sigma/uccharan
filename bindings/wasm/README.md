# uccharan (JavaScript / WebAssembly)

**Phonemiser, text normaliser, transliterator and pronunciation engine** for text-to-speech and
speech recognition: 44 languages (English, Hindi, twenty-four more South Asian languages and
seventeen European ones), in Rust, Python, JavaScript and C. Apache-2.0, no GPL code or data.
([what it does](https://github.com/cB-Sigma/uccharan#readme) ·
[measurements](https://github.com/cB-Sigma/uccharan/blob/main/docs/evaluation.md))

In the browser and in Node: phonemes in the shapes Piper, Kokoro, Kitten TTS, Matcha-TTS and
sherpa-onnx voices expect, the spoken form of numbers, dates and money for voices that read text,
romanised Indian-language text into native script, pronunciation word lists, and sound-alike
matching of a recogniser's output against your vocabulary.

```sh
npm install uccharan
```

```js
import { phonemize, kokoroIds, normalize, loadTranslit, transliterate, createSoundIndex } from "uccharan";

await phonemize("Hello, world.");                                     // "həlˈoʊ, wˈɜːld."
await phonemize("नमस्ते दुनिया", { lang: "hi" });                       // "nəmˈʌsteː dˈʊnɪjˌaː"
await phonemize("Hi there. Bye!", { profile: "piper" });              // ["hˈaɪ ðˈɛɹ.", "bˈaɪ!"]
await phonemize("Hi there.", { profile: "sherpa-onnx" });             // [[104, 712, …]]
await phonemize("I read it yesterday.", { improved: true });          // "aɪ ɹˈɛd ɪt jˈɛstɚdˌeɪ."
kokoroIds(await phonemize("नमस्ते", { lang: "hi", profile: "kokoro" })); // [0, …, 0]

(await normalize("₹5 करोड़ का सौदा 10:30 बजे", { lang: "hi" })).text;   // "पांच करोड़ रुपये का सौदा साढ़े दस बजे"

await loadTranslit("hi", bytesOfXlitHiUcd);                           // CC BY-SA 4.0 pack from the release page
(await transliterate("aap kaise ho", { lang: "hi" })).text;           // "आप कैसे हो"

const names = await createSoundIndex(["Kothari", "Paytm"]);
names.apply("send it to Coat Harry", { lang: "en-us" }).text;         // "send it to Kothari"
```

### Compatible or improved mode

**Compatible mode** (the default) reproduces espeak-ng's phonemes, mistakes included, because that
is what existing Piper, Kokoro, Kitten TTS, Matcha-TTS and sherpa-onnx voices were trained on: use
it to drive those voices. **Improved mode** (`{ improved: true }`) reads text as it is actually said:
homographs in context, names, Hindi schwa deletion and nasals, Bhojpuri, Maithili, Magahi, Dogri
and Bodo read as themselves, and written number words in Tamil, Kannada, Telugu and Malayalam. Use
it to train new voices, and always drive a voice with the mode it was trained with.
[What changes, and how to choose](https://github.com/cB-Sigma/uccharan#compatible-or-improved-choosing-a-mode).

The WebAssembly core is about 1.5 MiB (about 560 KiB gzipped). Language data is fetched the first
time a language is used (`data/en-us.ucd` and `data/en-gb.ucd` about 0.7 MB each, `data/hi.ucd`
3 KB, the Urdu, Sindhi and European word tables 46 KB to 0.66 MB each; `data/kokoro-en-*.ucd`,
misaki's dictionaries for the Kokoro profile, about 2 MB each) and checked against the SHA-256
sums the package was built with; point `init({ dataBaseUrl })` elsewhere to host it yourself.
After `await init()` and `await loadLanguage(...)`, `phonemizeSync` runs without awaiting.

The transliteration packs (`xlit-hi.ucd`, …) and the Wiktionary packs (`wikipron-*.ucd`) are
CC BY-SA 4.0 and are not in this package: download them from the
[release page](https://github.com/cB-Sigma/uccharan/releases) and pass the bytes to
`loadTranslit` / `loadWikipron`, or place the files in your data directory.

Live demo: <https://cb-sigma.github.io/uccharan/>. Documentation and measurements:
<https://github.com/cB-Sigma/uccharan>.

## Licence

Licence: Apache-2.0. Includes data from the CMU Pronouncing Dictionary (BSD-2-Clause) and misaki (Apache-2.0); see NOTICE.
The optional `wikipron-*` and `xlit-*` packs are CC BY-SA 4.0; they are separate downloads and are not part of this package.
