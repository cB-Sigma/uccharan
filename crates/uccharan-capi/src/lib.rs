//! C ABI for uccharan.
//!
//! Every function is safe to call from any thread; a `UccharanPhonemizer` may
//! be shared between threads once created (loading data requires exclusive
//! access). Strings are UTF-8 and NUL-terminated. Strings returned by the
//! library must be released with [`uccharan_string_free`].
//!
//! This crate is the only place in the project that uses `unsafe`: it is
//! required to cross the FFI boundary (raw pointers in, raw pointers out).

use std::ffi::{c_char, CStr, CString};
use uccharan_core::profile::{self, Profile};
use uccharan_core::{Lang, Mode, Normalize, Options, Phonemizer};

/// Opaque phonemizer handle.
pub struct UccharanPhonemizer {
    inner: Phonemizer,
}

/// Use the improved (not reference-compatible) behaviour.
pub const UCCHARAN_IMPROVED: u32 = 1;
/// Detect romanised Hindi in Hindi requests (experimental).
pub const UCCHARAN_HINGLISH: u32 = 2;
/// Read numbers and symbols the way the reference phonemizer does.
pub const UCCHARAN_NORMALIZE_ESPEAK: u32 = 4;
/// Do not expand numbers and symbols.
pub const UCCHARAN_NORMALIZE_OFF: u32 = 8;
/// Leave clause punctuation out of `espeak-ipa` output.
pub const UCCHARAN_NO_PUNCTUATION: u32 = 16;

/// Library version, e.g. "0.1.0". The pointer is static; do not free it.
#[no_mangle]
pub extern "C" fn uccharan_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr().cast()
}

/// Create a phonemizer with the built-in language data. Never returns NULL.
#[no_mangle]
pub extern "C" fn uccharan_new() -> *mut UccharanPhonemizer {
    Box::into_raw(Box::new(UccharanPhonemizer {
        inner: Phonemizer::new(),
    }))
}

/// Destroy a phonemizer. NULL is ignored.
///
/// # Safety
/// `p` must be NULL or a pointer returned by [`uccharan_new`] that has not
/// been freed.
#[no_mangle]
pub unsafe extern "C" fn uccharan_free(p: *mut UccharanPhonemizer) {
    if !p.is_null() {
        // SAFETY: per the contract, `p` came from Box::into_raw in uccharan_new.
        drop(unsafe { Box::from_raw(p) });
    }
}

/// Load (or replace) the data image of a language ("en-us", "en-gb", "hi").
/// Returns 0 on success, -1 for a bad argument, -2 for a malformed image.
///
/// # Safety
/// `p` must be a live phonemizer not used concurrently; `lang` a valid C
/// string; `data` must point to `len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn uccharan_load(
    p: *mut UccharanPhonemizer,
    lang: *const c_char,
    data: *const u8,
    len: usize,
) -> i32 {
    if p.is_null() || lang.is_null() || data.is_null() {
        return -1;
    }
    // SAFETY: pointers were checked for NULL and the caller guarantees validity.
    let (p, lang, bytes) = unsafe {
        (
            &mut *p,
            CStr::from_ptr(lang),
            std::slice::from_raw_parts(data, len),
        )
    };
    let Some(lang) = lang.to_str().ok().and_then(Lang::parse) else {
        return -1;
    };
    match p.inner.load(lang, bytes.to_vec()) {
        Ok(()) => 0,
        Err(_) => -2,
    }
}

/// Load any `.ucd` pack, for example an optional `wikipron-en-us.ucd`
/// (CC BY-SA 4.0 data, used by the improved mode). The pack names its
/// language itself. Returns 0 on success, -1 for a bad argument, -2 for a
/// malformed or unknown pack.
///
/// # Safety
/// `p` must be a live phonemizer; `data` must point to `len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn uccharan_load_pack(
    p: *mut UccharanPhonemizer,
    data: *const u8,
    len: usize,
) -> i32 {
    if p.is_null() || data.is_null() {
        return -1;
    }
    // SAFETY: pointers were checked for NULL and the caller guarantees validity.
    let (p, bytes) = unsafe { (&*p, std::slice::from_raw_parts(data, len)) };
    match p.inner.load_pack(bytes.to_vec()) {
        Ok(_) => 0,
        Err(_) => -2,
    }
}

/// Phonemize `text`.
///
/// `lang` is "en-us", "en-gb" or "hi" (espeak voice names are accepted).
/// `profile` is "espeak-ipa", "piper", "kokoro", "kitten", "matcha" or
/// "sherpa-onnx"; for "piper" the result has one sentence per line, for
/// "sherpa-onnx" decimal codepoints separated by spaces with sentences
/// separated by " | ". `flags` is a combination of the `UCCHARAN_*` constants.
///
/// Returns a newly allocated string, or NULL when an argument is invalid.
///
/// # Safety
/// `p` must be a live phonemizer; `text`, `lang` and `profile` valid C strings.
#[no_mangle]
pub unsafe extern "C" fn uccharan_phonemize(
    p: *const UccharanPhonemizer,
    text: *const c_char,
    lang: *const c_char,
    profile: *const c_char,
    flags: u32,
) -> *mut c_char {
    if p.is_null() || text.is_null() || lang.is_null() || profile.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: pointers were checked for NULL and the caller guarantees validity.
    let (p, text, lang, prof) = unsafe {
        (
            &*p,
            CStr::from_ptr(text),
            CStr::from_ptr(lang),
            CStr::from_ptr(profile),
        )
    };
    let (Ok(text), Some(lang), Some(prof)) = (
        text.to_str(),
        lang.to_str().ok().and_then(Lang::parse),
        prof.to_str().ok().and_then(Profile::parse),
    ) else {
        return std::ptr::null_mut();
    };
    let opt = options(lang, flags);
    let out = match prof {
        Profile::Piper => profile::piper_sentences(&p.inner.analyze(text, &opt)).join("\n"),
        other => p
            .inner
            .render(text, &opt, other, flags & UCCHARAN_NO_PUNCTUATION == 0),
    };
    CString::new(out).map_or(std::ptr::null_mut(), CString::into_raw)
}

fn options(lang: Lang, flags: u32) -> Options {
    let mut opt = Options::new(lang);
    if flags & UCCHARAN_IMPROVED != 0 {
        opt.mode = Mode::Improved;
    }
    opt.hinglish = flags & UCCHARAN_HINGLISH != 0;
    if flags & UCCHARAN_NORMALIZE_ESPEAK != 0 {
        opt.normalize = Normalize::EspeakLike;
    }
    if flags & UCCHARAN_NORMALIZE_OFF != 0 {
        opt.normalize = Normalize::Off;
    }
    opt
}

/// Phonemize many utterances at once on several threads.
///
/// `text` holds one utterance per line (`\n`); the result holds one line per
/// input line, in order. For "piper" the sentences of an utterance are joined
/// by a space. `threads` is the number of worker threads; 0 uses every
/// available core. Other arguments are as for [`uccharan_phonemize`].
///
/// Returns a newly allocated string, or NULL when an argument is invalid.
///
/// # Safety
/// `p` must be a live phonemizer; `text`, `lang` and `profile` valid C strings.
#[no_mangle]
pub unsafe extern "C" fn uccharan_phonemize_lines(
    p: *const UccharanPhonemizer,
    text: *const c_char,
    lang: *const c_char,
    profile: *const c_char,
    flags: u32,
    threads: u32,
) -> *mut c_char {
    if p.is_null() || text.is_null() || lang.is_null() || profile.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: pointers were checked for NULL and the caller guarantees validity.
    let (p, text, lang, prof) = unsafe {
        (
            &*p,
            CStr::from_ptr(text),
            CStr::from_ptr(lang),
            CStr::from_ptr(profile),
        )
    };
    let (Ok(text), Some(lang), Some(prof)) = (
        text.to_str(),
        lang.to_str().ok().and_then(Lang::parse),
        prof.to_str().ok().and_then(Profile::parse),
    ) else {
        return std::ptr::null_mut();
    };
    let opt = options(lang, flags);
    let lines: Vec<&str> = text.split('\n').collect();
    let out = p
        .inner
        .render_batch(
            &lines,
            &opt,
            prof,
            flags & UCCHARAN_NO_PUNCTUATION == 0,
            threads as usize,
        )
        .join("\n");
    CString::new(out).map_or(std::ptr::null_mut(), CString::into_raw)
}

/// Load misaki's English dictionaries for the Kokoro English front end:
/// `gold` and `silver` are the UTF-8 text of `us_gold.json` / `us_silver.json`
/// (or the `gb_` files with `lang` "en-gb"). Returns 0 on success, -1 for a
/// bad argument, -2 for unreadable data or when they are already loaded.
///
/// # Safety
/// `p` must be a live phonemizer; `lang`, `gold` and `silver` valid C strings.
#[no_mangle]
pub unsafe extern "C" fn uccharan_load_misaki(
    p: *const UccharanPhonemizer,
    lang: *const c_char,
    gold: *const c_char,
    silver: *const c_char,
) -> i32 {
    if p.is_null() || lang.is_null() || gold.is_null() || silver.is_null() {
        return -1;
    }
    // SAFETY: pointers were checked for NULL and the caller guarantees validity.
    let (p, lang, gold, silver) = unsafe {
        (
            &*p,
            CStr::from_ptr(lang),
            CStr::from_ptr(gold),
            CStr::from_ptr(silver),
        )
    };
    let (Some(lang), Ok(gold), Ok(silver)) = (
        lang.to_str().ok().and_then(Lang::parse),
        gold.to_str(),
        silver.to_str(),
    ) else {
        return -1;
    };
    match p.inner.load_misaki(lang, gold, silver) {
        Ok(()) => 0,
        Err(_) => -2,
    }
}

/// Normalise-only mode: the spoken form of `text` for a voice that reads
/// text, as JSON `{"text", "sentences", "spans"}` where every range gives
/// `byte`, `char` (code point) and `utf16` offsets (see docs/normalize.md).
///
/// `options_json` may be NULL or a JSON object with the members `mode`,
/// `digits`, `en_scale`, `latin`, `urls`, `respell` and `wordlists` (an
/// array of word lists as TSV, PLS or JSON text, highest priority first).
///
/// Returns a newly allocated string, or NULL when an argument is invalid
/// (unknown language, malformed options or word list).
///
/// # Safety
/// `p` must be a live phonemizer; `text` and `lang` valid C strings;
/// `options_json` NULL or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn uccharan_normalize(
    p: *const UccharanPhonemizer,
    text: *const c_char,
    lang: *const c_char,
    options_json: *const c_char,
) -> *mut c_char {
    if p.is_null() || text.is_null() || lang.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: pointers were checked for NULL and the caller guarantees validity.
    let (p, text, lang) = unsafe { (&*p, CStr::from_ptr(text), CStr::from_ptr(lang)) };
    let opts = if options_json.is_null() {
        Some("")
    } else {
        // SAFETY: non-NULL and valid per the contract.
        unsafe { CStr::from_ptr(options_json) }.to_str().ok()
    };
    let (Ok(text), Some(lang), Some(opts)) = (
        text.to_str(),
        lang.to_str().ok().and_then(Lang::parse),
        opts,
    ) else {
        return std::ptr::null_mut();
    };
    match uccharan_core::normalize_api::normalize_json(&p.inner, text, lang, opts) {
        Ok(out) => CString::new(out).map_or(std::ptr::null_mut(), CString::into_raw),
        Err(_) => std::ptr::null_mut(),
    }
}

/// As [`uccharan_phonemize`], with pronunciation word lists applied:
/// `wordlists_json` is a JSON array of word lists (TSV, PLS or JSON text),
/// highest priority first. A matched entry's IPA replaces the word's
/// reading (in the profile's phone set); an entry with only a spoken form
/// is phonemised from it.
///
/// Returns a newly allocated string, or NULL when an argument is invalid.
///
/// # Safety
/// As [`uccharan_phonemize`]; `wordlists_json` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn uccharan_phonemize_wordlists(
    p: *const UccharanPhonemizer,
    text: *const c_char,
    lang: *const c_char,
    profile: *const c_char,
    flags: u32,
    wordlists_json: *const c_char,
) -> *mut c_char {
    if p.is_null()
        || text.is_null()
        || lang.is_null()
        || profile.is_null()
        || wordlists_json.is_null()
    {
        return std::ptr::null_mut();
    }
    // SAFETY: pointers were checked for NULL and the caller guarantees validity.
    let (p, text, lang, prof, lists) = unsafe {
        (
            &*p,
            CStr::from_ptr(text),
            CStr::from_ptr(lang),
            CStr::from_ptr(profile),
            CStr::from_ptr(wordlists_json),
        )
    };
    let (Ok(text), Some(lang), Some(prof), Ok(lists)) = (
        text.to_str(),
        lang.to_str().ok().and_then(Lang::parse),
        prof.to_str().ok().and_then(Profile::parse),
        lists.to_str(),
    ) else {
        return std::ptr::null_mut();
    };
    let wrapped = format!("{{\"wordlists\":{lists}}}");
    let Ok(o) = uccharan_core::normalize_api::options_from_json(&wrapped) else {
        return std::ptr::null_mut();
    };
    let opt = options(lang, flags);
    let out = uccharan_core::normalize_api::render_with_wordlists(
        &p.inner,
        text,
        &opt,
        prof,
        flags & UCCHARAN_NO_PUNCTUATION == 0,
        &o.wordlists,
    );
    CString::new(out).map_or(std::ptr::null_mut(), CString::into_raw)
}

/// Convert a word list (TSV, PLS or JSON text) to `to`: "tsv", "json" or
/// "pls" (`lang` picks the PLS language and may be NULL for the entries of
/// any language).
///
/// Returns a newly allocated string, or NULL for invalid input.
///
/// # Safety
/// `text` and `to` must be valid C strings; `lang` NULL or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn uccharan_wordlist_convert(
    text: *const c_char,
    to: *const c_char,
    lang: *const c_char,
) -> *mut c_char {
    if text.is_null() || to.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: pointers were checked for NULL and the caller guarantees validity.
    let (text, to) = unsafe { (CStr::from_ptr(text), CStr::from_ptr(to)) };
    let lang = if lang.is_null() {
        None
    } else {
        // SAFETY: non-NULL and valid per the contract.
        unsafe { CStr::from_ptr(lang) }
            .to_str()
            .ok()
            .and_then(Lang::parse)
    };
    let (Ok(text), Ok(to)) = (text.to_str(), to.to_str()) else {
        return std::ptr::null_mut();
    };
    let Ok(w) = uccharan_core::wordlist::WordList::parse_any(text) else {
        return std::ptr::null_mut();
    };
    let out = match to {
        "tsv" => w.to_tsv(),
        "json" => w.to_json(),
        "pls" => w.to_pls(lang),
        _ => return std::ptr::null_mut(),
    };
    CString::new(out).map_or(std::ptr::null_mut(), CString::into_raw)
}

/// Opaque handle of a vocabulary prepared for sound-alike matching.
pub struct UccharanSoundIndex {
    inner: uccharan_core::soundalike::SoundIndex,
}

/// Build a sound-alike index over a vocabulary: a word list (TSV, PLS or
/// JSON text), a JSON array of terms (strings, or objects with `word` and
/// optional `lang`, `ipa`, `say`), or text with one term per line.
/// `options` is NULL or a JSON object (`langs`: the languages terms without
/// phonemes are read in; `ordinary_words`: more words to take for ordinary
/// speech).
///
/// Returns NULL for invalid input. Release the index with
/// [`uccharan_sound_index_free`]. An index may be shared between threads.
///
/// # Safety
/// `p` must be a live phonemizer; `vocabulary` a valid C string; `options`
/// NULL or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn uccharan_sound_index_new(
    p: *const UccharanPhonemizer,
    vocabulary: *const c_char,
    options: *const c_char,
) -> *mut UccharanSoundIndex {
    if p.is_null() || vocabulary.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: pointers were checked for NULL and the caller guarantees validity.
    let (p, vocabulary) = unsafe { (&*p, CStr::from_ptr(vocabulary)) };
    let options = if options.is_null() {
        Ok("")
    } else {
        // SAFETY: non-NULL and valid per the contract.
        unsafe { CStr::from_ptr(options) }.to_str()
    };
    let (Ok(vocabulary), Ok(options)) = (vocabulary.to_str(), options) else {
        return std::ptr::null_mut();
    };
    match uccharan_core::soundalike::SoundIndex::from_text(&p.inner, vocabulary, options) {
        Ok(inner) => Box::into_raw(Box::new(UccharanSoundIndex { inner })),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Destroy a sound-alike index. NULL is ignored.
///
/// # Safety
/// `index` must be NULL or a pointer returned by
/// [`uccharan_sound_index_new`] that has not been freed.
#[no_mangle]
pub unsafe extern "C" fn uccharan_sound_index_free(index: *mut UccharanSoundIndex) {
    if !index.is_null() {
        // SAFETY: per the contract, `index` came from Box::into_raw.
        drop(unsafe { Box::from_raw(index) });
    }
}

/// Correct a transcript against a sound-alike index. The result is JSON:
/// `{"text", "changes": [{"src", "out", "from", "to", "index", "score",
/// "case_only"}], "candidates": [{"src", "text", "term", "index", "score",
/// "exact", "kind", "words"}]}` with `byte`, `char` and `utf16` offsets in
/// every range. `text` is the transcript with every candidate applied that
/// passes the options; `candidates` lists all places that sound like a
/// term, applied or not. `options` is NULL or a JSON object: `threshold`,
/// `uncertain_threshold`, `ordinary_threshold`, `confidence` (one number
/// per whitespace-separated word), `max_confidence`, `fix_case`,
/// `max_words`, `min_score`, `all`. With `threshold` 2 and `fix_case`
/// false nothing is changed and only the candidates are reported.
///
/// Returns a newly allocated string, or NULL for invalid input.
///
/// # Safety
/// `p` must be a live phonemizer and `index` a live index; `text` and
/// `lang` valid C strings; `options` NULL or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn uccharan_match_terms(
    p: *const UccharanPhonemizer,
    index: *const UccharanSoundIndex,
    text: *const c_char,
    lang: *const c_char,
    options: *const c_char,
) -> *mut c_char {
    if p.is_null() || index.is_null() || text.is_null() || lang.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: pointers were checked for NULL and the caller guarantees validity.
    let (p, index, text, lang) =
        unsafe { (&*p, &*index, CStr::from_ptr(text), CStr::from_ptr(lang)) };
    let options = if options.is_null() {
        Ok("")
    } else {
        // SAFETY: non-NULL and valid per the contract.
        unsafe { CStr::from_ptr(options) }.to_str()
    };
    let (Ok(text), Ok(lang), Ok(options)) = (text.to_str(), lang.to_str(), options) else {
        return std::ptr::null_mut();
    };
    let Some(lang) = Lang::parse(lang) else {
        return std::ptr::null_mut();
    };
    match index.inner.apply_json(&p.inner, text, lang, options) {
        Ok(s) => CString::new(s).map_or(std::ptr::null_mut(), CString::into_raw),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Release a string returned by this library. NULL is ignored.
///
/// # Safety
/// `s` must be NULL or a pointer returned by [`uccharan_phonemize`] or
/// [`uccharan_phonemize_lines`] that has not been freed.
#[no_mangle]
pub unsafe extern "C" fn uccharan_string_free(s: *mut c_char) {
    if !s.is_null() {
        // SAFETY: per the contract, `s` came from CString::into_raw.
        drop(unsafe { CString::from_raw(s) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_through_the_abi() {
        let p = uccharan_new();
        let text = CString::new("Hello, world.").unwrap();
        let lang = CString::new("en-us").unwrap();
        let prof = CString::new("espeak-ipa").unwrap();
        // SAFETY: all pointers are valid for the duration of the calls.
        unsafe {
            let s = uccharan_phonemize(p, text.as_ptr(), lang.as_ptr(), prof.as_ptr(), 0);
            assert_eq!(CStr::from_ptr(s).to_str().unwrap(), "həlˈoʊ, wˈɜːld.");
            uccharan_string_free(s);
            let bad = CString::new("xx").unwrap();
            assert!(uccharan_phonemize(p, text.as_ptr(), bad.as_ptr(), prof.as_ptr(), 0).is_null());
            assert!(uccharan_phonemize(
                std::ptr::null(),
                text.as_ptr(),
                lang.as_ptr(),
                prof.as_ptr(),
                0
            )
            .is_null());
            assert_eq!(uccharan_load(p, lang.as_ptr(), [1u8, 2, 3].as_ptr(), 3), -2);
            uccharan_free(p);
            uccharan_free(std::ptr::null_mut());
            uccharan_string_free(std::ptr::null_mut());
        }
    }
}

#[cfg(test)]
mod textnorm_tests {
    use super::*;

    #[test]
    fn normalize_and_word_lists() {
        let p = uccharan_new();
        let c = |s: &str| CString::new(s).unwrap();
        let (text, lang) = (c("Pay ₹500."), c("en-us"));
        // SAFETY: all pointers are valid for the duration of the calls.
        unsafe {
            let s = uccharan_normalize(p, text.as_ptr(), lang.as_ptr(), std::ptr::null());
            let out = CStr::from_ptr(s).to_str().unwrap().to_string();
            assert!(
                out.starts_with("{\"text\":\"Pay five hundred rupees.\""),
                "{out}"
            );
            uccharan_string_free(s);
            let bad = c("{\"latin\":\"x\"}");
            assert!(uccharan_normalize(p, text.as_ptr(), lang.as_ptr(), bad.as_ptr()).is_null());
            let lists = c("[\"Kokoro\\ten\\tkəkˈɔːɹoʊ\"]");
            let (k, prof) = (c("Kokoro"), c("espeak-ipa"));
            let s = uccharan_phonemize_wordlists(
                p,
                k.as_ptr(),
                lang.as_ptr(),
                prof.as_ptr(),
                0,
                lists.as_ptr(),
            );
            assert_eq!(CStr::from_ptr(s).to_str().unwrap(), "kəkˈɔːɹoʊ");
            uccharan_string_free(s);
            let tsv = c("Kokoro\ten\tkəkˈɔːɹoʊ\n");
            let s = uccharan_wordlist_convert(tsv.as_ptr(), c("pls").as_ptr(), lang.as_ptr());
            assert!(CStr::from_ptr(s)
                .to_str()
                .unwrap()
                .contains("<grapheme>Kokoro</grapheme>"));
            uccharan_string_free(s);
            uccharan_free(p);
        }
    }
}

#[cfg(test)]
mod soundalike_tests {
    use super::*;

    #[test]
    fn match_terms_through_the_abi() {
        let p = uccharan_new();
        let c = |s: &str| CString::new(s).unwrap();
        let (vocab, text, lang) = (c("Kothari\nPaytm\n"), c("call Coat Harry"), c("en-us"));
        // SAFETY: all pointers are valid for the duration of the calls.
        unsafe {
            let ix = uccharan_sound_index_new(p, vocab.as_ptr(), std::ptr::null());
            assert!(!ix.is_null());
            let s = uccharan_match_terms(p, ix, text.as_ptr(), lang.as_ptr(), std::ptr::null());
            let out = CStr::from_ptr(s).to_str().unwrap().to_string();
            assert!(
                out.starts_with(
                    "{\"text\":\"call Kothari\",\"changes\":[{\"src\":{\"byte\":[5,15]"
                ),
                "{out}"
            );
            uccharan_string_free(s);
            let none = c("{\"threshold\": 2, \"fix_case\": false}");
            let s = uccharan_match_terms(p, ix, text.as_ptr(), lang.as_ptr(), none.as_ptr());
            let out = CStr::from_ptr(s).to_str().unwrap().to_string();
            assert!(
                out.starts_with("{\"text\":\"call Coat Harry\",\"changes\":[],\"candidates\":[{"),
                "{out}"
            );
            uccharan_string_free(s);
            let bad = c("{\"nope\": 1}");
            assert!(
                uccharan_match_terms(p, ix, text.as_ptr(), lang.as_ptr(), bad.as_ptr()).is_null()
            );
            assert!(uccharan_sound_index_new(p, c("[1]").as_ptr(), std::ptr::null()).is_null());
            uccharan_sound_index_free(ix);
            uccharan_sound_index_free(std::ptr::null_mut());
            uccharan_free(p);
        }
    }
}
