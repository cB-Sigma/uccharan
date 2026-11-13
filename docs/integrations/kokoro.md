# Kokoro

Kokoro-82M takes ids from a 114-symbol vocabulary, wrapped in the pad id 0. Its English voices were
trained on the output of **misaki**; its Hindi voices on espeak-ng's `hi` output passed through a
short published mapping. uccharan has a front end for each.

## English: the Kokoro English front end

`crates/uccharan/src/kokoro.rs` is a port of misaki's English G2P (misaki 0.9.4, Apache-2.0): the
same dictionary lookup (gold, then silver), stress rules, suffix rules, number reading, sub-word
splitting and context rules. Two pieces differ:

- spaCy's tagger is replaced by a small rule-based tagger that decides only what misaki's
  dictionary looks at (about 790 words have different pronunciations by part of speech);
- words outside misaki's dictionaries, which misaki sends to espeak-ng, are read by uccharan's own
  rules and mapped to misaki's phone set.

misaki 0.9.4's US and British dictionaries are **built in** (about 3 MB each; the npm package
fetches them on first use). Upstream does not document their original sources; uccharan relies on
misaki's Apache-2.0 licence statement (see `DATA_LICENSES.md`).

```python
import uccharan

tokens = uccharan.phonemize("I read the book yesterday.", "en-us", "kokoro")   # 'ˌI ɹˈɛd ðə bˈʊk jˈɛstəɹdˌA.'
ids = uccharan.kokoro_ids(tokens)            # [0, …, 0]
```

```sh
uccharan -l en-us -p kokoro "I read the book yesterday."
```

```js
import { phonemize } from "uccharan";
await phonemize("I read the book yesterday.", { profile: "kokoro" });   // fetches data/kokoro-en-us.ucd once
```

To use a different copy of the dictionaries (say, a newer misaki release), load it before the first
Kokoro request: `uccharan.load_misaki(path)` (default: an installed `misaki` package),
`uccharan --misaki DIR`, `loadMisaki(lang, gold, silver)` in JavaScript, or
`uccharan_load_misaki(p, "en-us", gold_json, silver_json)` in C.

Measured against misaki itself (3 000 held-out sentences; [evaluation](../evaluation.md#6-kokoro)):
94.9 % of en-US sentences give the identical token string, words in misaki's dictionaries agree
99.96 % of the time, part-of-speech-dependent words 98.5 %, words misaki sends to espeak-ng 75 %.
Through a real Kokoro voice the speech-recognition word error rate is the same as with misaki.

A build without the dictionaries (Rust: no `embed-kokoro` feature) falls back to converting
uccharan's espeak-style English to Kokoro symbols, a different convention (69.5 % of frequent words
equal misaki's entry).

## Hindi

uccharan's compatible Hindi through the published espeak → Kokoro mapping. On 3 000 held-out
sentences it equals the reference-through-mapping string for 98.8 % of sentences (token error rate
0.13 %); every token is in the vocabulary.

```python
tokens = uccharan.phonemize("नमस्ते दुनिया, आप कैसे हैं?", "hi", "kokoro")
```

## Through sherpa-onnx

sherpa-onnx's Kokoro models take `phoneme_codepoints` in the model's own alphabet:

```python
config.phoneme_codepoints = [[ord(c) for c in uccharan.phonemize(text, "en-us", "kokoro")]]
```

This is the path used for the end-to-end measurements (`tools/eval/kokoro_e2e.py`).
