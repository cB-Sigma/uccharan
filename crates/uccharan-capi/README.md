# uccharan-capi

**5× faster than espeak-ng. Just as accurate. Free to ship.**
A drop-in phonemiser for Piper, Kokoro and sherpa-onnx voices — English and Hindi, in Rust, Python,
C and the browser. Apache-2.0. ([measurements](https://github.com/cB-Sigma/uccharan/blob/main/docs/evaluation.md))

C ABI for [uccharan](https://github.com/cB-Sigma/uccharan): a shared and a static library plus
`include/uccharan.h`.

```c
#include "uccharan.h"

UccharanPhonemizer *p = uccharan_new();
char *ipa = uccharan_phonemize(p, "Hello, world.", "en-us", "espeak-ipa", 0);   /* "həlˈoʊ, wˈɜːld." */
char *cps = uccharan_phonemize(p, "नमस्ते", "hi", "sherpa-onnx", 0);              /* "110 601 …" */
char *many = uccharan_phonemize_lines(p, "one\ntwo\nthree", "en-us", "piper", 0, 0); /* all cores */
uccharan_string_free(ipa); uccharan_string_free(cps); uccharan_string_free(many);
uccharan_free(p);
```

Build: `cargo build --release -p uccharan-capi` → `target/release/libuccharan.{so,dylib,a}` /
`uccharan.dll`. A handle may be shared between threads. Every returned string must be released with
`uccharan_string_free`. Flags: `UCCHARAN_IMPROVED`, `UCCHARAN_HINGLISH`,
`UCCHARAN_NORMALIZE_ESPEAK`, `UCCHARAN_NORMALIZE_OFF`, `UCCHARAN_NO_PUNCTUATION`.

Every language pack is compiled in by default. Build with `--no-default-features --features
embed-en-us,embed-hi` (or no features at all) for a smaller library, and load the `.ucd` packs
attached to each GitHub release with `uccharan_load`.

## Licence

Licence: Apache-2.0. Includes data from the CMU Pronouncing Dictionary (BSD-2-Clause) and misaki (Apache-2.0); see NOTICE.
