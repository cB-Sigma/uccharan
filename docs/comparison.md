# How it compares with other phonemisers

The table covers the phonemiser only; the other parts of uccharan (normalise-only mode, word
lists, sound-alike matching, transliteration, training tools) are outside it.

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
[evaluation.md](evaluation.md) for uccharan's own numbers. If something here is out of
date, please open an issue.
