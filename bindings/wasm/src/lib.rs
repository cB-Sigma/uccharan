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

// ---------------------------------------------------------------------------
// Normalise-only mode and pronunciation word lists.

fn parse_lists(
    lists: &[String],
) -> Result<Vec<std::sync::Arc<uccharan::wordlist::WordList>>, JsError> {
    lists
        .iter()
        .enumerate()
        .map(|(i, t)| {
            uccharan::wordlist::WordList::parse_any(t)
                .map(std::sync::Arc::new)
                .map_err(|e| JsError::new(&format!("wordlists[{i}]: {e}")))
        })
        .collect()
}

/// Spoken form with spans, as JSON; options as JSON (word lists as text
/// in its `wordlists` member).
#[wasm_bindgen(js_name = normalizeJson)]
pub fn normalize_json(text: &str, lang_tag: &str, options: &str) -> Result<String, JsError> {
    let l = lang(lang_tag)?;
    ENGINE
        .with(|e| uccharan::normalize_api::normalize_json(&e.borrow(), text, l, options))
        .map_err(|e| JsError::new(&e))
}

/// `render` with word lists (TSV, PLS or JSON texts) applied.
#[wasm_bindgen(js_name = renderWordlists)]
pub fn render_wordlists(
    text: &str,
    lang_tag: &str,
    profile_name: &str,
    normalize: &str,
    improved: bool,
    hinglish: bool,
    punctuation: bool,
    flags: bool,
    lists: Vec<String>,
) -> Result<String, JsError> {
    let mut opt = options(lang_tag, normalize, improved, hinglish)?;
    opt.keep_language_flags = flags;
    let prof = Profile::parse(profile_name)
        .ok_or_else(|| JsError::new(&format!("unknown profile \"{profile_name}\"")))?;
    let lists = parse_lists(&lists)?;
    Ok(ENGINE.with(|e| {
        uccharan::normalize_api::render_with_wordlists(
            &e.borrow(),
            text,
            &opt,
            prof,
            punctuation,
            &lists,
        )
    }))
}

/// Respell IPA for a text voice: JSON `{"text","readback","distance"}`, or
/// an empty string when the language has no writer.
#[wasm_bindgen]
pub fn respell(ipa: &str, lang_tag: &str, source: &str, style: &str) -> Result<String, JsError> {
    let l = lang(lang_tag)?;
    let st = uccharan::respell::Style::parse(style)
        .ok_or_else(|| JsError::new(&format!("unknown respell style \"{style}\"")))?;
    let r =
        ENGINE.with(|e| uccharan::respell::respell(&e.borrow(), ipa, Lang::parse(source), l, st));
    Ok(r.map_or_else(String::new, |r| {
        let mut o = String::from("{\"text\":");
        uccharan::minijson::push_str(&mut o, &r.text);
        o.push_str(",\"readback\":");
        uccharan::minijson::push_str(&mut o, &r.readback);
        o.push_str(&format!(",\"distance\":{}}}", r.distance));
        o
    }))
}

/// Convert a word list (TSV, PLS or JSON text) to `tsv`, `json` or `pls`
/// (`lang` picks the PLS language; empty for entries of any language).
#[wasm_bindgen(js_name = convertWordlist)]
pub fn convert_wordlist(text: &str, to: &str, lang_tag: &str) -> Result<String, JsError> {
    let w =
        uccharan::wordlist::WordList::parse_any(text).map_err(|e| JsError::new(&e.to_string()))?;
    match to {
        "tsv" => Ok(w.to_tsv()),
        "json" => Ok(w.to_json()),
        "pls" => Ok(w.to_pls(Lang::parse(lang_tag))),
        other => Err(JsError::new(&format!(
            "unknown word-list format \"{other}\""
        ))),
    }
}

// ---------------------------------------------------------------------------
// Sound-alike matching.

/// A vocabulary prepared for sound-alike matching: a word list (TSV, PLS
/// or JSON), a JSON array of terms, or one term per line; `options` is a
/// JSON object (`langs`) or empty.
#[wasm_bindgen]
pub struct SoundIndex {
    inner: uccharan::soundalike::SoundIndex,
}

#[wasm_bindgen]
impl SoundIndex {
    /// Build the index.
    #[wasm_bindgen(constructor)]
    pub fn new(vocabulary: &str, options: &str) -> Result<SoundIndex, JsError> {
        ENGINE
            .with(|e| uccharan::soundalike::SoundIndex::from_text(&e.borrow(), vocabulary, options))
            .map(|inner| SoundIndex { inner })
            .map_err(|e| JsError::new(&e))
    }

    /// Number of terms.
    #[wasm_bindgen(getter)]
    pub fn length(&self) -> usize {
        self.inner.len()
    }

    /// Candidates as JSON; options as JSON.
    #[wasm_bindgen(js_name = findJson)]
    pub fn find_json(&self, text: &str, lang_tag: &str, options: &str) -> Result<String, JsError> {
        let l = lang(lang_tag)?;
        ENGINE
            .with(|e| self.inner.find_json(&e.borrow(), text, l, options))
            .map_err(|e| JsError::new(&e))
    }

    /// The corrected text with its changes, as JSON; options as JSON.
    #[wasm_bindgen(js_name = applyJson)]
    pub fn apply_json(&self, text: &str, lang_tag: &str, options: &str) -> Result<String, JsError> {
        let l = lang(lang_tag)?;
        ENGINE
            .with(|e| self.inner.apply_json(&e.borrow(), text, l, options))
            .map_err(|e| JsError::new(&e))
    }
}
