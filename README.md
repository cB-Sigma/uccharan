<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/assets/brand/banner-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="docs/assets/brand/banner-light.png">
    <img alt="uccharan · उच्चारण — text to phonemes, without the GPL" src="docs/assets/brand/banner-light.png" width="800">
  </picture>
</p>

<p align="center">
  <a href="https://github.com/cB-Sigma/uccharan/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/cB-Sigma/uccharan/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="Licence: Apache-2.0" src="https://img.shields.io/badge/licence-Apache--2.0-1f62b3"></a>
  <a href="https://pypi.org/project/uccharan/"><img alt="PyPI" src="https://img.shields.io/pypi/v/uccharan?color=1f62b3"></a>
  <a href="https://crates.io/crates/uccharan"><img alt="crates.io" src="https://img.shields.io/crates/v/uccharan?color=1f62b3"></a>
  <a href="https://www.npmjs.com/package/uccharan"><img alt="npm" src="https://img.shields.io/npm/v/uccharan?color=1f62b3"></a>
</p>

<p align="center">
  <a href="https://cb-sigma.github.io/uccharan/"><img alt="Try the live demo" src="docs/assets/brand/button-live-demo.png" width="257"></a>
</p>

**uccharan** (उच्चारण, "pronunciation") turns text into the phoneme streams that neural
text-to-speech voices expect — for **English (US and British), Hindi and code-mixed Hindi +
English** — under the Apache-2.0 licence, with no GPL code or data. It is written in Rust and runs
from Python, JavaScript (WebAssembly, in the browser and Node), Rust and C.

Most open voices (Piper, Kokoro, Kitten TTS, Matcha-TTS, StyleTTS2) were trained on phonemes from
espeak-ng, which is GPL-licensed. sherpa-onnx 2.0 removed espeak-ng for that reason and now expects
the caller to bring phonemes. uccharan is that phonemizer:

- **Compatible mode matches espeak-ng** closely enough that existing voices work unchanged:
  1.19 % phoneme disagreement on held-out American English, 0.12 % on Hindi, and the same
  speech-recognition word error rate when real voices speak its output ([numbers](#measured-compatibility)).
- **Improved mode is more correct than espeak-ng**: homographs in context, names, numbers, Hindi
  schwa deletion and nasals — 80.5 % against espeak-ng's 51.6 % on a hand-made test set written
  after the rules were frozen ([numbers](#correctness-improved-mode)).
- **Both are faster**: 5× lower latency and 5× higher throughput than espeak-ng for English,
  2× for Hindi, 8–10× faster start-up, and it scales across threads ([numbers](#speed)).
- **Kokoro English**: a port of misaki, the front end Kokoro was trained on, with its dictionaries
  built in, matching it on 94.9 % of sentences at 76× its speed.

<p align="center">
  <img alt="The live demo: typing English and Hindi, switching between compatible and improved mode and between output profiles" src="docs/assets/demo/demo.gif" width="760">
</p>

## Quick start

**Python** (3.9+, Linux, macOS, Windows)

```sh
pip install uccharan
```

```python
import uccharan

uccharan.phonemize("Hello, world.")                         # 'həlˈoʊ, wˈɜːld.'
uccharan.phonemize("नमस्ते दुनिया", lang="hi")                 # 'nəmˈʌsteː dˈʊnɪjˌaː'
uccharan.phonemize("I read it yesterday.", improved=True)   # 'aɪ ɹˈɛd ɪt jˈɛstɚdˌeɪ.'

# sherpa-onnx >= 2.0: one list of codepoints per sentence
config.phoneme_codepoints = uccharan.phonemize(text, tts.lang, "sherpa-onnx")
```

**JavaScript** (browser or Node 18+)

```sh
npm install uccharan
```

```js
import { phonemize } from "uccharan";
await phonemize("Hello, world.");                          // "həlˈoʊ, wˈɜːld."
await phonemize("Hi there. Bye!", { profile: "piper" });   // ["hˈaɪ ðˈɛɹ.", "bˈaɪ!"]
```

**Rust**

```sh
cargo add uccharan
```

```rust
use uccharan::{Lang, Options, Phonemizer};
let p = Phonemizer::new();
assert_eq!(p.phonemize("Hello, world.", &Options::new(Lang::EnUs)), "həlˈoʊ, wˈɜːld.");
```

**C** (`libuccharan` + `uccharan.h` from the release page, or `cargo build --release -p uccharan-capi`)

```c
#include "uccharan.h"
UccharanPhonemizer *p = uccharan_new();
char *ipa = uccharan_phonemize(p, "Hello, world.", "en-us", "espeak-ipa", 0);
uccharan_string_free(ipa);
uccharan_free(p);
```

**Command line**: `uccharan -l hi -p piper "नमस्ते दुनिया।"`. Code written for `phonemizer` can use
`from uccharan.compat import EspeakBackend`.

## Output profiles

| Profile | Gives you | For |
| --- | --- | --- |
| `espeak-ipa` | IPA string, words separated by spaces | anything that used `phonemizer`'s espeak backend |
| `piper` | one string per sentence; `piper_ids()` maps it through a voice's `phoneme_id_map` | Piper voices |
| `sherpa-onnx` | one codepoint list per sentence | sherpa-onnx ≥ 2.0 `phoneme_codepoints` |
| `kokoro` | Kokoro tokens; `kokoro_ids()` | Kokoro-82M (English via the built-in port of misaki) |
| `kitten`, `matcha` | phoneme string; `kitten_ids()`, `matcha_ids()` | Kitten TTS 0.8, Matcha-TTS |

Details: [docs/profiles.md](docs/profiles.md).

## Measured compatibility

All numbers are from runs on the development machine (4 shared vCPUs, no GPU) against espeak-ng
1.52.0 through its library API, on held-out sentences that were never used to write rules.
Method and full breakdowns: [docs/evaluation.md](docs/evaluation.md).

| Language | Profile | Phoneme agreement (held-out) | Symbols outside the voice's table | End-to-end WER, espeak-ng → uccharan | Drop-in? |
| --- | --- | --- | --- | --- | --- |
| en-US | Piper / sherpa-onnx | PER 1.16 %, 76.1 % sentences identical | none¹ | `amy-low` 3.17 % → 2.97 %; `lessac-medium` 4.30 % → 4.21 % | **yes** |
| en-GB (RP) | Piper / sherpa-onnx | PER 1.38 %, 72.0 % identical | none | `alba-medium` 7.36 % → 7.47 % | yes, one voice tested |
| Hindi | Piper / sherpa-onnx | PER 0.12 %, 98.8 % identical | none | `hi_IN-rohan-medium` 8.95 % → 8.82 % | yes, one voice tested |
| en-US | `espeak-ipa` (phonemizer shape) | PER 1.19 % | — | — | yes |
| Hindi | Kokoro | 98.8 % of sentences identical to espeak-ng + Kokoro's mapping | none | Kokoro sid 32: 10.35 % → 10.25 % | yes, one voice tested |
| en-US | Kokoro | 94.9 % of sentences identical to **misaki** (99.96 % of dictionary words) | none | Kokoro sid 3: 2.95 % (misaki) → 2.91 % | **yes** |
| en-GB | Kokoro | 92.3 % identical to misaki | none | Kokoro sid 21: 2.08 % (misaki) → 2.12 % | yes, one voice tested |
| en-US | Kitten TTS 0.8 | ids identical to the `phonemizer` front end for 75.1 % of texts | none | not run | untested end to end |
| en-US | Matcha-TTS | table and framing unit-tested | — | not run | untested |

End-to-end: 500 held-out sentences per Piper voice (300 per Kokoro voice) synthesised with
sherpa-onnx 2.0.1 from each front end's phonemes and transcribed by Whisper (base for English, a
Hindi tiny fine-tune for Hindi); in every row the 95 % interval of the difference contains zero.
"Drop-in" requires PER ≤ 5 %, no out-of-table symbols and an end-to-end WER difference within
+0.5 (English) / +1.0 (Hindi).
¹ Old 130-symbol Piper voices lack the syllabic mark U+0329, which espeak-ng emits as well.

## Correctness (improved mode)

Hand-written gold sets ([tools/eval/gold](tools/eval/gold)); expected pronunciations come from
CMUdict variants and standard phonology, never from a phonemizer. These are the sets written
*after* improved mode was frozen:

| Set | espeak-ng 1.52 | uccharan compatible | **uccharan improved** |
| --- | ---: | ---: | ---: |
| English homographs in context (40) | 62.5 % | 60.0 % | **82.5 %** |
| English proper nouns (20) | **60.0 %** | 45.0 % | 55.0 % |
| English numbers, dates, units (20) | 35.0 % | 30.0 % | **85.0 %** |
| Hindi schwa deletion, nasals, nukta (36) | 61.1 % | 61.1 % | **97.2 %** |
| Romanised Hindi, experimental (12) | 0 % | 0 % | **58.3 %** |
| **All (128)** | 51.6 % | 47.7 % | **80.5 %** |

espeak-ng still does better on unseen proper nouns. On the development gold sets (348 items, used
to build improved mode) improved mode scores 99.7 % against espeak-ng's 50.3 %; that number only
shows the rules do what they were written to do. What improved mode changes:
[docs/improved-mode.md](docs/improved-mode.md).

## Speed

Same machine, same 3 000 held-out sentences, one call per sentence, both libraries loaded into one
process ([method](docs/evaluation.md#5-speed-start-up-memory-and-size)):

| | English (en-US) uccharan | espeak-ng | Hindi uccharan | espeak-ng |
| --- | ---: | ---: | ---: | ---: |
| Latency per sentence, p50 | **10.4 µs** | 53.2 µs | **6.9 µs** | 14.8 µs |
| Latency per sentence, p99 | **37 µs** | 230 µs | **27 µs** | 49 µs |
| Throughput, one thread | **677 k words/s** | 128 k | **783 k words/s** | 366 k |
| Throughput, 4 workers² | **1.71 M words/s** | 202 k | **1.74 M words/s** | 609 k |
| Start-up + first sentence | **0.55 ms** | 4.3 ms | **0.46 ms** | 4.9 ms |
| Memory after start-up | **2.2 MB** | 3.0 MB | **1.4 MB** | 2.9 MB |

² uccharan: one handle shared by 4 threads; espeak-ng has global state, so 4 processes. The
measurement machine delivered at most about 2.15 cores' worth of work (control measurement).

Kokoro English against misaki: 26 µs against 1.96 ms per sentence (76×), 0.6 ms against 1.45 s
start-up, 2.5 MB against 204 MB of memory.

Where uccharan does **not** win: its English lexicon makes the library larger (11.2 MB with all
data built in — 2 MB per English accent, 3 MB per accent of misaki dictionaries for Kokoro, 4 KB of
Hindi — against about 1.5 MB for espeak-ng's English), and memory after a long English run is
about the same (3.8 MB against 3.6 MB). Scaling beyond two cores could not be measured on the
4-vCPU shared machine. The WebAssembly core is
414 KiB (155 KiB gzipped); language data loads on demand.

## How it compares

| | Licence (code / data) | espeak-compatible output | Hindi | WASM | Notes |
| --- | --- | --- | --- | --- | --- |
| **uccharan** | Apache-2.0 / BSD-2 (CMUdict), Apache-2.0 (misaki dictionaries, own tables) | yes, measured | yes | yes | this project |
| espeak-ng | GPL-3.0 | is the reference | yes | GPL builds | 100+ languages |
| `phonemizer`, piper1-gpl | GPL-3.0 | wraps espeak-ng | yes | no | |
| phonemizer.js (npm) | labelled Apache-2.0, bundles an espeak-ng WASM build (GPL) | wraps espeak-ng | yes | yes | |
| misaki | Apache-2.0 / dictionaries of undocumented origin | no (Kokoro phones) | via espeak-ng | no | English OOV via espeak-ng |
| misaki-rs, loqui-g2p | MIT / misaki data | no (Kokoro phones) | no | not verified | English only |
| piper-plus-g2p | MIT | no, own phoneme set | no | yes | not compatible with upstream Piper voices |
| gruut | MIT / mixed | no | no | no | dormant |
| Epitran | MIT | no stress, broad IPA | table, no schwa deletion | no | |
| OpenPhonemizer | BSD-3-Clause-Clear; training set CC-BY-SA, generated with espeak | mimics espeak (self-reported 93.5/100) | no | no | unmaintained |
| goruut | MIT / mixed incl. share-alike | no | yes | no | Go server |

Facts as checked on 2026-11-13 from each project's repository and package metadata; see
[docs/evaluation.md](docs/evaluation.md) for uccharan's own numbers. If something here is out of
date, please open an issue.

## Licence and provenance

Code: Apache-2.0. Data: the English lexicon is derived from the CMU Pronouncing Dictionary
(BSD-2-Clause, notice in [NOTICE](NOTICE)); the Kokoro English dictionaries are misaki's
(Apache-2.0); every other table was written by hand for this project. [DATA_LICENSES.md](DATA_LICENSES.md) lists every file.

uccharan contains no espeak-ng code, rules or dictionary data. The Kokoro English front end is a port
of misaki's Apache-2.0 code and ships misaki 0.9.4's English dictionaries unchanged under their
upstream Apache-2.0 licence statement; upstream does not document where the dictionary entries
originally come from (details in [DATA_LICENSES.md](DATA_LICENSES.md)).

## What is not tested

- Multi-core scaling beyond about two cores (the shared measurement machine offered no more).
- en-GB and Hindi end to end on more than one Piper voice each, and Kokoro on more than one speaker
  per language.
- Kitten TTS and Matcha-TTS end to end (only symbol tables and framing are checked).
- The `piper` command-line program (it embeds espeak-ng and has no hook) and running Piper voices
  directly with onnxruntime.
- Windows and macOS builds (built by the release workflow, not run here).
- A human listening test.
- Integration inside [bolo](https://github.com/cB-Sigma/bolo).
- Side-by-side speed against espeak-ng's WebAssembly build.

## Roadmap

- **More Indic languages**, starting with Hindi's Devanagari relatives: Marathi, Nepali, Maithili,
  Bhojpuri — each with held-out sets and the same gates ([how](docs/adding-a-language.md)).
- **An optional GPL data pack**, kept in a separate repository and package, for people who can
  accept the GPL and want espeak-ng's own dictionaries for languages uccharan does not cover yet.
- **A small neural fallback** for out-of-lexicon words (int8, under 1.5 MB per language, trained
  only on permissive lexicons), as a separate optional artefact.
- More voices end to end, including Kitten TTS through sherpa-onnx.

## Documentation

[Architecture](docs/architecture.md) · [Profiles](docs/profiles.md) ·
[Improved mode](docs/improved-mode.md) · [Evaluation](docs/evaluation.md) ·
[Adding a language](docs/adding-a-language.md) ·
Integrations: [sherpa-onnx](docs/integrations/sherpa-onnx.md), [Piper](docs/integrations/piper.md),
[Kokoro](docs/integrations/kokoro.md), [bolo](docs/integrations/bolo.md) ·
[Contributing](CONTRIBUTING.md) · [Changelog](CHANGELOG.md) · [Security](SECURITY.md)

uccharan is a sibling of [bolo](https://github.com/cB-Sigma/bolo). Made by Abhinav Gautam.
