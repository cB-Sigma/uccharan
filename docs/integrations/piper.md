# Piper

Piper voices whose config says `"phoneme_type": "espeak"` (or says nothing: older voices) take
phoneme ids built from espeak-ng phonemes. uccharan produces those ids from the voice's own
`phoneme_id_map`.

```python
import json
import numpy as np
import onnxruntime as ort
import uccharan

config = json.load(open("hi_IN-rohan-medium.onnx.json"))
session = ort.InferenceSession("hi_IN-rohan-medium.onnx")

for sentence in uccharan.phonemize("नमस्ते दुनिया। आप कैसे हैं?", config["espeak"]["voice"], "piper"):
    ids, missing = uccharan.piper_ids(sentence, config)
    scales = np.array([config["inference"]["noise_scale"], config["inference"]["length_scale"],
                       config["inference"]["noise_w"]], dtype=np.float32)
    audio = session.run(None, {"input": np.array([ids], dtype=np.int64),
                               "input_lengths": np.array([len(ids)], dtype=np.int64),
                               "scales": scales})[0]
```

- One string per sentence, clause punctuation kept, as Piper's own front end produces.
- Ids are framed `[BOS, PAD, p, PAD, …, EOS]`. `missing` lists symbols the voice's map does not
  have; Piper skips those.
- Voices trained with other phoneme types (`text`, `pinyin`, Japanese, Hebrew …) and voices for
  languages other than `en-us`, `en-gb-x-rp`/`en` and `hi` are out of scope.

What was checked: the id framing against an independent implementation and two real voice configs
(`tools/eval/conformance.py`), and synthesis through sherpa-onnx's Piper runtime. Running a voice
directly with onnxruntime as above was not tested here, and neither was the `piper` command-line
program (piper1-gpl embeds espeak-ng and has no hook for an external phonemizer).
