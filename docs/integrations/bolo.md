# bolo

[bolo](https://github.com/cb-Sigma/bolo) is uccharan's sibling project: a speech server that pulls
models and serves them over familiar APIs. Its Piper voices run on sherpa-onnx, and their phonemes
currently come from an optional, separately installed espeak-ng component.

uccharan is meant to replace that component for English and Hindi voices:

```python
import uccharan

def phoneme_codepoints(text: str, espeak_voice: str) -> list[list[int]]:
    """What the Piper runtime needs for GenerationConfig.phoneme_codepoints."""
    return uccharan.phonemize(text, espeak_voice, "sherpa-onnx")
```

With that in place a Piper voice for `en-us`, `en-gb-x-rp` or `hi` needs no GPL component at all.
Voices in other languages would keep using the optional component until uccharan covers them.

Status: **not wired into bolo yet.** The two voices used for uccharan's end-to-end measurements
(`piper-en-us-amy-low`, `piper-hi-in-rohan-medium`) are the same model files bolo's catalogue
serves, and they were driven through the same sherpa-onnx call, but no bolo release ships the
adapter and it has not been tested inside bolo.
