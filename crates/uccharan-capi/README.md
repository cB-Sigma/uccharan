# uccharan-capi

**Phonemiser, text normaliser, transliterator and pronunciation engine** for text-to-speech and
speech recognition: 39 languages (English, Hindi, nineteen more South Asian languages and
seventeen European ones), in Rust, Python, JavaScript and C. Apache-2.0, no GPL code or data.
([what it does](https://github.com/cB-Sigma/uccharan#readme) ·
[measurements](https://github.com/cB-Sigma/uccharan/blob/main/docs/evaluation.md))

C ABI for [uccharan](https://github.com/cB-Sigma/uccharan): a shared and a static library plus
`include/uccharan.h`.

```c
#include "uccharan.h"

UccharanPhonemizer *p = uccharan_new();
char *ipa = uccharan_phonemize(p, "Hello, world.", "en-us", "espeak-ipa", 0);   /* "həlˈoʊ, wˈɜːld." */
char *cps = uccharan_phonemize(p, "नमस्ते", "hi", "sherpa-onnx", 0);              /* "110 601 …" */
char *many = uccharan_phonemize_lines(p, "one\ntwo\nthree", "en-us", "piper", 0, 0); /* all cores */
char *spoken = uccharan_normalize(p, "Rs. 250 on 3/4/2024", "en-us", NULL);     /* JSON: text and spans */
uccharan_string_free(ipa); uccharan_string_free(cps); uccharan_string_free(many); uccharan_string_free(spoken);
uccharan_free(p);
```

Also in the header: `uccharan_transliterate` (romanised text into native script),
`uccharan_phonemize_wordlists` and `uccharan_wordlist_convert` (pronunciation word lists),
`uccharan_sound_index_new` and `uccharan_match_terms` (sound-alike matching); structured results
are JSON strings.

Build: `cargo build --release -p uccharan-capi` → `target/release/libuccharan.{so,dylib,a}` /
`uccharan.dll`. A handle may be shared between threads. Every returned string must be released with
`uccharan_string_free`. Flags: `UCCHARAN_IMPROVED`, `UCCHARAN_HINGLISH`,
`UCCHARAN_NORMALIZE_ESPEAK`, `UCCHARAN_NORMALIZE_OFF`, `UCCHARAN_NO_PUNCTUATION`.

Every language pack is compiled in by default. Build with `--no-default-features --features
embed-en-us,embed-hi` (or no features at all) for a smaller library, and load the `.ucd` packs
attached to each GitHub release with `uccharan_load`. The transliteration packs (`xlit-*.ucd`)
and Wiktionary packs (`wikipron-*.ucd`) are CC BY-SA 4.0 and never compiled in; load one with
`uccharan_load_pack`.

## Licence

Licence: Apache-2.0. Includes data from the CMU Pronouncing Dictionary (BSD-2-Clause) and misaki (Apache-2.0); see NOTICE.
The optional `wikipron-*` and `xlit-*` packs are CC BY-SA 4.0; they are separate downloads and are not part of this package.
