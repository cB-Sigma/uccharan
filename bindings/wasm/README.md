# uccharan (JavaScript / WebAssembly)

**5× faster than espeak-ng. Just as accurate. Free to ship.**
A drop-in phonemiser for Piper, Kokoro and sherpa-onnx voices — English and Hindi, in Rust, Python,
C and the browser. Apache-2.0. ([measurements](https://github.com/cB-Sigma/uccharan/blob/main/docs/evaluation.md))

Permissively licensed text-to-phoneme conversion for neural TTS in the browser and in Node:
English (US, British RP), Hindi and Hinglish, in the shapes Piper, Kokoro, Kitten TTS, Matcha-TTS
and sherpa-onnx voices expect. No GPL code or data.

```sh
npm install uccharan
```

```js
import { phonemize, kokoroIds } from "uccharan";

await phonemize("Hello, world.");                                     // "həlˈoʊ, wˈɜːld."
await phonemize("नमस्ते दुनिया", { lang: "hi" });                       // "nəmˈʌsteː dˈʊnɪjˌaː"
await phonemize("Hi there. Bye!", { profile: "piper" });              // ["hˈaɪ ðˈɛɹ.", "bˈaɪ!"]
await phonemize("Hi there.", { profile: "sherpa-onnx" });             // [[104, 712, …]]
await phonemize("I read it yesterday.", { improved: true });          // "aɪ ɹˈɛd ɪt jˈɛstɚdˌeɪ."
kokoroIds(await phonemize("नमस्ते", { lang: "hi", profile: "kokoro" })); // [0, …, 0]
```

The WebAssembly core is about 440 KiB (165 KiB gzipped). Language data is fetched the first time a language is used
(`data/en-us.ucd` and `data/en-gb.ucd` about 0.7 MB each, `data/hi.ucd` 3 KB; `data/kokoro-en-*.ucd`, misaki's dictionaries for the Kokoro profile,
about 2 MB each) and checked against the SHA-256 sums the package was built with; point
`init({ dataBaseUrl })` elsewhere to host it yourself. After `await init()` and
`await loadLanguage(...)`, `phonemizeSync` runs without awaiting.

Live demo: <https://cb-sigma.github.io/uccharan/>. Documentation and measurements:
<https://github.com/cB-Sigma/uccharan>.

## Licence

Licence: Apache-2.0. Includes data from the CMU Pronouncing Dictionary (BSD-2-Clause) and misaki (Apache-2.0); see NOTICE.
