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
