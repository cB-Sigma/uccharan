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

// Opaque handle of a vocabulary prepared for sound-alike matching.
typedef struct UccharanSoundIndex UccharanSoundIndex;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

// Why the last call on this thread returned NULL or an error code because
// of a language pack: a message that names the pack, where it is published
// and where the file is looked for. An empty string when no such error has
// happened. The pointer is valid until the next library call on the same
// thread; do not free it.
const char *uccharan_last_error(void);

// Make a language ready before its first use. English and Hindi are built
// in; the word tables of the other languages are packs (`de.ucd`, `fr.ucd`
// …, attached to the release). A pack that is not loaded is read from the
// directory named by `UCCHARAN_DATA_DIR`, from the cache directory, or,
// when the library was built with the `download` feature and
// `UCCHARAN_OFFLINE` is not `1`, from the release, and checked against its
// SHA-256 sum. The phonemizing calls do the same at a language's first use.
//
// Returns 0 when the language is ready, -1 for a bad argument and -2 when a
// pack is missing or damaged ([`uccharan_last_error`] says which).
//
// # Safety
// `p` must be a live phonemizer; `lang` a valid C string.
int32_t uccharan_ensure(const struct UccharanPhonemizer *p, const char *lang);

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
// Returns a newly allocated string, or NULL when an argument is invalid or the
// language pack is missing (see [`uccharan_last_error`]).
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
// Returns a newly allocated string, or NULL when an argument is invalid or the
// language pack is missing (see [`uccharan_last_error`]).
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

// Normalise-only mode: the spoken form of `text` for a voice that reads
// text, as JSON `{"text", "sentences", "spans"}` where every range gives
// `byte`, `char` (code point) and `utf16` offsets (see docs/normalize.md).
//
// `options_json` may be NULL or a JSON object with the members `mode`,
// `digits`, `en_scale`, `latin`, `urls`, `respell` and `wordlists` (an
// array of word lists as TSV, PLS or JSON text, highest priority first).
//
// Returns a newly allocated string, or NULL when an argument is invalid
// (unknown language, malformed options or word list).
//
// # Safety
// `p` must be a live phonemizer; `text` and `lang` valid C strings;
// `options_json` NULL or a valid C string.
char *uccharan_normalize(const struct UccharanPhonemizer *p,
                         const char *text,
                         const char *lang,
                         const char *options_json);

// Romanised text into the script of `lang` (`aap kaise ho` → `आप कैसे हो`),
// as JSON `{"text", "spans"}` where every span has a `kind` (`indic`,
// `english` or `other`), `src` and `out` ranges in `byte`, `char` (code
// point) and `utf16` offsets, and the alternatives of a word (see
// docs/transliteration.md). Load the language's pack (`xlit-hi.ucd`, …)
// with [`uccharan_load_pack`] first; without it a rule converter answers.
//
// `options_json` may be NULL or a JSON object with the members `english`
// (`keep`, `native` or `convert`), `alternatives` and `context`.
//
// Returns a newly allocated string, or NULL when an argument is invalid
// (unknown language, malformed options).
//
// # Safety
// `p` must be a live phonemizer; `text` and `lang` valid C strings;
// `options_json` NULL or a valid C string.
char *uccharan_transliterate(const struct UccharanPhonemizer *p,
                             const char *text,
                             const char *lang,
                             const char *options_json);

// As [`uccharan_phonemize`], with pronunciation word lists applied:
// `wordlists_json` is a JSON array of word lists (TSV, PLS or JSON text),
// highest priority first. A matched entry's IPA replaces the word's
// reading (in the profile's phone set); an entry with only a spoken form
// is phonemised from it.
//
// Returns a newly allocated string, or NULL when an argument is invalid.
//
// # Safety
// As [`uccharan_phonemize`]; `wordlists_json` must be a valid C string.
char *uccharan_phonemize_wordlists(const struct UccharanPhonemizer *p,
                                   const char *text,
                                   const char *lang,
                                   const char *profile,
                                   uint32_t flags,
                                   const char *wordlists_json);

// Convert a word list (TSV, PLS or JSON text) to `to`: "tsv", "json" or
// "pls" (`lang` picks the PLS language and may be NULL for the entries of
// any language).
//
// Returns a newly allocated string, or NULL for invalid input.
//
// # Safety
// `text` and `to` must be valid C strings; `lang` NULL or a valid C string.
char *uccharan_wordlist_convert(const char *text, const char *to, const char *lang);

// Build a sound-alike index over a vocabulary: a word list (TSV, PLS or
// JSON text), a JSON array of terms (strings, or objects with `word` and
// optional `lang`, `ipa`, `say`), or text with one term per line.
// `options` is NULL or a JSON object (`langs`: the languages terms without
// phonemes are read in; `ordinary_words`: more words to take for ordinary
// speech).
//
// Returns NULL for invalid input. Release the index with
// [`uccharan_sound_index_free`]. An index may be shared between threads.
//
// # Safety
// `p` must be a live phonemizer; `vocabulary` a valid C string; `options`
// NULL or a valid C string.
struct UccharanSoundIndex *uccharan_sound_index_new(const struct UccharanPhonemizer *p,
                                                    const char *vocabulary,
                                                    const char *options);

// Destroy a sound-alike index. NULL is ignored.
//
// # Safety
// `index` must be NULL or a pointer returned by
// [`uccharan_sound_index_new`] that has not been freed.
void uccharan_sound_index_free(struct UccharanSoundIndex *index);

// Correct a transcript against a sound-alike index. The result is JSON:
// `{"text", "changes": [{"src", "out", "from", "to", "index", "score",
// "case_only"}], "candidates": [{"src", "text", "term", "index", "score",
// "exact", "kind", "words"}]}` with `byte`, `char` and `utf16` offsets in
// every range. `text` is the transcript with every candidate applied that
// passes the options; `candidates` lists all places that sound like a
// term, applied or not. `options` is NULL or a JSON object: `threshold`,
// `uncertain_threshold`, `ordinary_threshold`, `confidence` (one number
// per whitespace-separated word), `max_confidence`, `fix_case`,
// `max_words`, `min_score`, `all`. With `threshold` 2 and `fix_case`
// false nothing is changed and only the candidates are reported.
//
// Returns a newly allocated string, or NULL for invalid input.
//
// # Safety
// `p` must be a live phonemizer and `index` a live index; `text` and
// `lang` valid C strings; `options` NULL or a valid C string.
char *uccharan_match_terms(const struct UccharanPhonemizer *p,
                           const struct UccharanSoundIndex *index,
                           const char *text,
                           const char *lang,
                           const char *options);

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
