# sherpa-onnx

sherpa-onnx 2.0 removed its built-in espeak-ng and accepts phonemes from the caller through
`GenerationConfig.phoneme_codepoints`: one list of Unicode codepoints per sentence. That is
uccharan's `sherpa-onnx` profile.

```python
import sherpa_onnx
import uccharan

tts = sherpa_onnx.OfflineTts(sherpa_onnx.OfflineTtsConfig(
    model=sherpa_onnx.OfflineTtsModelConfig(
        vits=sherpa_onnx.OfflineTtsVitsModelConfig(
            model="en_US-amy-low.onnx", tokens="tokens.txt"),
        num_threads=2)))

text = "The quick brown fox jumps over the lazy dog."
config = sherpa_onnx.GenerationConfig()
config.phoneme_codepoints = uccharan.phonemize(text, tts.lang, "sherpa-onnx")
audio = tts.generate(text, config)          # audio.samples, audio.sample_rate
```

`tts.lang` is the espeak voice name stored in the model's metadata (`en-us`, `en-gb-x-rp`, `hi`);
uccharan accepts those names directly.

Notes:

- sherpa-onnx adds the model's own BOS, EOS and padding; pass the bare codepoints.
- Codepoints that are not in the model's `tokens.txt` are skipped with a log line. Voices with the
  older 130-symbol Piper table lack the syllabic mark U+0329; that is the only such symbol
  uccharan emits for them, and the reference emits it too.
- Keep the default `compatible` mode for existing voices.

This is the path that was tested end to end: 500 held-out sentences per language through
`en_US-amy-low` and `hi_IN-rohan-medium` with sherpa-onnx 2.0.1; see
[../evaluation.md](../evaluation.md). To repeat it: `tools/eval/e2e_tts_asr.py`.

From JavaScript (sherpa-onnx's WebAssembly or Node builds) the same lists come from
`await phonemize(text, { lang, profile: "sherpa-onnx" })`. That combination was not tested.
