/* uccharan C API, written by cbindgen from crates/uccharan-capi/src/lib.rs; do not edit.
 * SPDX-License-Identifier: Apache-2.0 */

#ifndef UCCHARAN_H
#define UCCHARAN_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

// Use the improved (not reference-compatible) behaviour.
#define UCCHARAN_IMPROVED 1

// Detect romanised Hindi in Hindi requests (experimental).
#define UCCHARAN_HINGLISH 2

// Read numbers and symbols the way the reference phonemizer does.
#define UCCHARAN_NORMALIZE_ESPEAK 4

// Do not expand numbers and symbols.
#define UCCHARAN_NORMALIZE_OFF 8

// Leave clause punctuation out of `espeak-ipa` output.
#define UCCHARAN_NO_PUNCTUATION 16

// Opaque phonemizer handle.
typedef struct UccharanPhonemizer UccharanPhonemizer;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

// Library version, e.g. "0.1.0". The pointer is static; do not free it.
const char *uccharan_version(void);

// Create a phonemizer with the built-in language data. Never returns NULL.
struct UccharanPhonemizer *uccharan_new(void);

// Destroy a phonemizer. NULL is ignored.
//
// # Safety
// `p` must be NULL or a pointer returned by [`uccharan_new`] that has not
// been freed.
void uccharan_free(struct UccharanPhonemizer *p);

// Load (or replace) the data image of a language ("en-us", "en-gb", "hi").
// Returns 0 on success, -1 for a bad argument, -2 for a malformed image.
//
// # Safety
// `p` must be a live phonemizer not used concurrently; `lang` a valid C
// string; `data` must point to `len` readable bytes.
int32_t uccharan_load(struct UccharanPhonemizer *p,
                      const char *lang,
                      const uint8_t *data,
                      uintptr_t len);

// Load any `.ucd` pack, for example an optional `wikipron-en-us.ucd`
// (CC BY-SA 4.0 data, used by the improved mode). The pack names its
// language itself. Returns 0 on success, -1 for a bad argument, -2 for a
// malformed or unknown pack.
//
// # Safety
// `p` must be a live phonemizer; `data` must point to `len` readable bytes.
int32_t uccharan_load_pack(struct UccharanPhonemizer *p, const uint8_t *data, uintptr_t len);

// Phonemize `text`.
//
// `lang` is "en-us", "en-gb" or "hi" (espeak voice names are accepted).
// `profile` is "espeak-ipa", "piper", "kokoro", "kitten", "matcha" or
// "sherpa-onnx"; for "piper" the result has one sentence per line, for
// "sherpa-onnx" decimal codepoints separated by spaces with sentences
// separated by " | ". `flags` is a combination of the `UCCHARAN_*` constants.
//
// Returns a newly allocated string, or NULL when an argument is invalid.
//
// # Safety
// `p` must be a live phonemizer; `text`, `lang` and `profile` valid C strings.
char *uccharan_phonemize(const struct UccharanPhonemizer *p,
                         const char *text,
                         const char *lang,
                         const char *profile,
                         uint32_t flags);

// Phonemize many utterances at once on several threads.
//
// `text` holds one utterance per line (`\n`); the result holds one line per
// input line, in order. For "piper" the sentences of an utterance are joined
// by a space. `threads` is the number of worker threads; 0 uses every
// available core. Other arguments are as for [`uccharan_phonemize`].
//
// Returns a newly allocated string, or NULL when an argument is invalid.
//
// # Safety
// `p` must be a live phonemizer; `text`, `lang` and `profile` valid C strings.
char *uccharan_phonemize_lines(const struct UccharanPhonemizer *p,
                               const char *text,
                               const char *lang,
                               const char *profile,
                               uint32_t flags,
                               uint32_t threads);

// Load misaki's English dictionaries for the Kokoro English front end:
// `gold` and `silver` are the UTF-8 text of `us_gold.json` / `us_silver.json`
// (or the `gb_` files with `lang` "en-gb"). Returns 0 on success, -1 for a
// bad argument, -2 for unreadable data or when they are already loaded.
//
// # Safety
// `p` must be a live phonemizer; `lang`, `gold` and `silver` valid C strings.
int32_t uccharan_load_misaki(const struct UccharanPhonemizer *p,
                             const char *lang,
                             const char *gold,
                             const char *silver);

// Release a string returned by this library. NULL is ignored.
//
// # Safety
// `s` must be NULL or a pointer returned by [`uccharan_phonemize`] or
// [`uccharan_phonemize_lines`] that has not been freed.
void uccharan_string_free(char *s);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* UCCHARAN_H */
