//! WebAssembly exports. The ergonomic API (lazy data loading, typed options)
//! is the JavaScript wrapper in `index.js`; these functions are its backend.

#![allow(clippy::too_many_arguments)] // flat argument lists keep the foreign-language signatures simple

use std::cell::RefCell;
use uccharan::profile::{self, Profile};
use uccharan::{Lang, Mode, Normalize, Options, Phonemizer};
use wasm_bindgen::prelude::*;

thread_local! {
    static ENGINE: RefCell<Phonemizer> = RefCell::new(Phonemizer::empty());
}

fn lang(s: &str) -> Result<Lang, JsError> {
    Lang::parse(s).ok_or_else(|| {
        JsError::new(&format!(
            "unknown language \"{s}\" (expected en-us, en-gb or hi)"
        ))
    })
}

fn options(
    lang_tag: &str,
    normalize: &str,
    improved: bool,
    hinglish: bool,
) -> Result<Options, JsError> {
    let mut opt = Options::new(lang(lang_tag)?);
    opt.normalize = match normalize {
        "natural" => Normalize::Natural,
        "espeak" | "espeak-like" => Normalize::EspeakLike,
        "off" => Normalize::Off,
        other => return Err(JsError::new(&format!("unknown normalize mode \"{other}\""))),
    };
    if improved {
        opt.mode = Mode::Improved;
    }
    opt.hinglish = hinglish;
    Ok(opt)
}

/// Library version.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Load the data image of a language.
#[wasm_bindgen]
pub fn load(lang_tag: &str, bytes: Vec<u8>) -> Result<(), JsError> {
    let l = lang(lang_tag)?;
    ENGINE
        .with(|e| e.borrow_mut().load(l, bytes))
        .map_err(|e| JsError::new(&e.to_string()))
}

/// Load any `.ucd` pack (for example an optional `wikipron-*` pack).
/// Returns the pack's name.
#[wasm_bindgen(js_name = loadPack)]
pub fn load_pack(bytes: Vec<u8>) -> Result<String, JsError> {
    ENGINE
        .with(|e| e.borrow().load_pack(bytes))
        .map_err(|e| JsError::new(&e))
}

/// True when the language's data has been loaded.
#[wasm_bindgen]
pub fn has(lang_tag: &str) -> bool {
    Lang::parse(lang_tag).is_some_and(|l| ENGINE.with(|e| e.borrow().has(l)))
}

/// One string in the given profile.
#[wasm_bindgen]
pub fn render(
    text: &str,
    lang_tag: &str,
    profile_name: &str,
    normalize: &str,
    improved: bool,
    hinglish: bool,
    punctuation: bool,
    flags: bool,
) -> Result<String, JsError> {
    let opt = options(lang_tag, normalize, improved, hinglish)?;
    let prof = Profile::parse(profile_name)
        .ok_or_else(|| JsError::new(&format!("unknown profile \"{profile_name}\"")))?;
    let mut opt = opt;
    opt.keep_language_flags = flags;
    Ok(ENGINE.with(|e| e.borrow().render(text, &opt, prof, punctuation)))
}

/// Load misaki's dictionaries (the text of its gold and silver JSON files)
/// for the Kokoro English front end.
#[wasm_bindgen(js_name = loadMisaki)]
pub fn load_misaki(lang_tag: &str, gold: &str, silver: &str) -> Result<(), JsError> {
    let l = lang(lang_tag)?;
    ENGINE
        .with(|e| e.borrow().load_misaki(l, gold, silver))
        .map_err(|e| JsError::new(&e))
}

/// Load a compiled Kokoro English data file (`kokoro-en-us.ucd` /
/// `kokoro-en-gb.ucd`). Returns the language tag it is for.
#[wasm_bindgen(js_name = loadKokoro)]
pub fn load_kokoro(bytes: Vec<u8>) -> Result<String, JsError> {
    ENGINE
        .with(|e| e.borrow().load_kokoro(bytes))
        .map(|l| l.tag().to_string())
        .map_err(|e| JsError::new(&e))
}

/// True when misaki's dictionaries are loaded for the language.
#[wasm_bindgen(js_name = hasMisaki)]
pub fn has_misaki(lang_tag: &str) -> bool {
    Lang::parse(lang_tag).is_some_and(|l| ENGINE.with(|e| e.borrow().has_misaki(l)))
}

/// Piper-shaped sentence strings.
#[wasm_bindgen]
pub fn sentences(
    text: &str,
    lang_tag: &str,
    normalize: &str,
    improved: bool,
    hinglish: bool,
) -> Result<Vec<String>, JsError> {
    let opt = options(lang_tag, normalize, improved, hinglish)?;
    Ok(ENGINE.with(|e| profile::piper_sentences(&e.borrow().analyze(text, &opt))))
}

/// Kokoro input ids for a token string.
#[wasm_bindgen(js_name = kokoroIds)]
pub fn kokoro_ids(tokens: &str) -> Vec<i32> {
    profile::kokoro_ids(tokens)
        .into_iter()
        .map(|x| x as i32)
        .collect()
}

/// Kitten TTS 0.8 input ids for a phoneme string.
#[wasm_bindgen(js_name = kittenIds)]
pub fn kitten_ids(phonemes: &str) -> Vec<i32> {
    profile::kitten_ids(phonemes)
        .into_iter()
        .map(|x| x as i32)
        .collect()
}

/// Matcha-TTS input ids for a phoneme string.
#[wasm_bindgen(js_name = matchaIds)]
pub fn matcha_ids(phonemes: &str) -> Vec<i32> {
    profile::matcha_ids(phonemes)
        .into_iter()
        .map(|x| x as i32)
        .collect()
}
