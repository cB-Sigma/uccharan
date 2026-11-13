//! Python extension module `uccharan._native`. The typed public API lives in
//! `python/uccharan/__init__.py`.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::sync::OnceLock;
use uccharan::profile::{self, PhonemeIdMap, Profile};
use uccharan::{Lang, Mode, Normalize, Options, Phonemizer};

fn engine() -> &'static Phonemizer {
    static ENGINE: OnceLock<Phonemizer> = OnceLock::new();
    ENGINE.get_or_init(Phonemizer::new)
}

fn options(lang: &str, normalize: &str, improved: bool, hinglish: bool) -> PyResult<Options> {
    let lang = Lang::parse(lang).ok_or_else(|| {
        PyValueError::new_err(format!(
            "unknown language {lang:?} (expected en-us, en-gb or hi)"
        ))
    })?;
    let mut opt = Options::new(lang);
    opt.normalize = match normalize {
        "natural" => Normalize::Natural,
        "espeak" | "espeak-like" => Normalize::EspeakLike,
        "off" => Normalize::Off,
        other => {
            return Err(PyValueError::new_err(format!(
                "unknown normalize mode {other:?} (expected natural, espeak or off)"
            )))
        }
    };
    if improved {
        opt.mode = Mode::Improved;
    }
    opt.hinglish = hinglish;
    Ok(opt)
}

fn parse_profile(name: &str) -> PyResult<Profile> {
    Profile::parse(name).ok_or_else(|| {
        PyValueError::new_err(format!("unknown profile {name:?} (expected espeak-ipa, piper, kokoro, kitten, matcha or sherpa-onnx)"))
    })
}

/// One phoneme string for the text in the given profile.
#[pyfunction]
#[pyo3(signature = (text, lang, profile, normalize, improved, hinglish, punctuation, flags))]
#[allow(clippy::too_many_arguments)]
fn render(
    py: Python<'_>,
    text: &str,
    lang: &str,
    profile: &str,
    normalize: &str,
    improved: bool,
    hinglish: bool,
    punctuation: bool,
    flags: bool,
) -> PyResult<String> {
    let opt = options(lang, normalize, improved, hinglish)?;
    let prof = parse_profile(profile)?;
    Ok(py.detach(|| {
        let mut opt = opt;
        opt.keep_language_flags = flags;
        engine().render(text, &opt, prof, punctuation)
    }))
}

/// Batch version of `render`; runs on all cores with the GIL released.
#[pyfunction]
#[pyo3(signature = (texts, lang, profile, normalize, improved, hinglish, punctuation))]
#[allow(clippy::too_many_arguments)]
fn render_batch(
    py: Python<'_>,
    texts: Vec<String>,
    lang: &str,
    profile: &str,
    normalize: &str,
    improved: bool,
    hinglish: bool,
    punctuation: bool,
) -> PyResult<Vec<String>> {
    let opt = options(lang, normalize, improved, hinglish)?;
    let prof = parse_profile(profile)?;
    Ok(py.detach(|| {
        let threads = std::thread::available_parallelism()
            .map_or(1, |n| n.get())
            .min(texts.len().max(1));
        let chunk = texts.len().div_ceil(threads.max(1)).max(1);
        let mut out: Vec<String> = Vec::with_capacity(texts.len());
        std::thread::scope(|s| {
            let handles: Vec<_> = texts
                .chunks(chunk)
                .map(|part| {
                    let opt = &opt;
                    s.spawn(move || {
                        part.iter()
                            .map(|t| engine().render(t, opt, prof, punctuation))
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            for h in handles {
                out.extend(h.join().expect("worker thread panicked"));
            }
        });
        out
    }))
}

/// Load misaki's dictionaries (text of the gold and silver JSON files) for
/// the Kokoro English front end.
#[pyfunction]
fn load_misaki(lang: &str, gold: &str, silver: &str) -> PyResult<()> {
    let l = Lang::parse(lang)
        .ok_or_else(|| PyValueError::new_err(format!("unknown language {lang:?}")))?;
    engine()
        .load_misaki(l, gold, silver)
        .map_err(PyValueError::new_err)
}

/// Load a language pack image; returns the pack's language name.
#[pyfunction]
fn load_pack(bytes: Vec<u8>) -> PyResult<String> {
    engine().load_pack(bytes).map_err(PyValueError::new_err)
}

/// Names of the packs that `lang` with `profile` still needs (empty when
/// everything is loaded or built in).
#[pyfunction]
fn missing_packs(lang: &str, profile: &str) -> Vec<String> {
    let Some(l) = Lang::parse(lang) else {
        return Vec::new();
    };
    let e = engine();
    let mut out = Vec::new();
    if !e.has(l) {
        out.push(l.tag().to_string());
    }
    if profile == "kokoro" && l != Lang::Hi && !e.has_misaki(l) {
        out.push(format!("kokoro-{}", l.tag()));
    }
    out
}

/// True when misaki's dictionaries are loaded for the language.
#[pyfunction]
fn has_misaki(lang: &str) -> bool {
    Lang::parse(lang).is_some_and(|l| engine().has_misaki(l))
}

/// Piper-shaped sentence strings (NFD, clause terminators kept).
#[pyfunction]
#[pyo3(signature = (text, lang, normalize, improved, hinglish))]
fn sentences(
    py: Python<'_>,
    text: &str,
    lang: &str,
    normalize: &str,
    improved: bool,
    hinglish: bool,
) -> PyResult<Vec<String>> {
    let opt = options(lang, normalize, improved, hinglish)?;
    Ok(py.detach(|| profile::piper_sentences(&engine().analyze(text, &opt))))
}

/// Unicode codepoints per sentence, for sherpa-onnx `phoneme_codepoints`.
#[pyfunction]
#[pyo3(signature = (text, lang, normalize, improved, hinglish))]
fn codepoints(
    py: Python<'_>,
    text: &str,
    lang: &str,
    normalize: &str,
    improved: bool,
    hinglish: bool,
) -> PyResult<Vec<Vec<u32>>> {
    let opt = options(lang, normalize, improved, hinglish)?;
    Ok(py.detach(|| profile::sherpa_codepoints(&engine().analyze(text, &opt))))
}

/// `(text, ipa, language, glued)` for every word, grouped by clause and sentence.
#[pyfunction]
#[pyo3(signature = (text, lang, normalize, improved, hinglish))]
#[allow(clippy::type_complexity)]
fn analyze(
    text: &str,
    lang: &str,
    normalize: &str,
    improved: bool,
    hinglish: bool,
) -> PyResult<Vec<Vec<(Vec<(String, String, String, bool)>, String)>>> {
    let opt = options(lang, normalize, improved, hinglish)?;
    Ok(engine()
        .analyze(text, &opt)
        .into_iter()
        .map(|s| {
            s.clauses
                .into_iter()
                .map(|c| {
                    let words = c
                        .words
                        .into_iter()
                        .map(|w| (w.text, w.ipa, w.lang.tag().to_string(), w.glue))
                        .collect();
                    (
                        words,
                        c.terminator.as_char().map(String::from).unwrap_or_default(),
                    )
                })
                .collect()
        })
        .collect())
}

/// Piper phoneme ids for one sentence string and a voice config (JSON text).
/// Returns `(ids, symbols missing from the map)`.
#[pyfunction]
fn piper_ids(sentence: &str, config_json: &str) -> PyResult<(Vec<i64>, Vec<String>)> {
    let map = PhonemeIdMap::from_json(config_json)
        .ok_or_else(|| PyValueError::new_err("no phoneme_id_map found in the voice config"))?;
    let (ids, missing) = map.ids(sentence);
    Ok((ids, missing.into_iter().map(String::from).collect()))
}

/// Kokoro input ids `[0, …, 0]` for a Kokoro token string.
#[pyfunction]
fn kokoro_ids(tokens: &str) -> Vec<i64> {
    profile::kokoro_ids(tokens)
}

/// Kitten TTS 0.8 input ids for a phoneme string.
#[pyfunction]
fn kitten_ids(phonemes: &str) -> Vec<i64> {
    profile::kitten_ids(phonemes)
}

/// Matcha-TTS input ids for a phoneme string.
#[pyfunction]
fn matcha_ids(phonemes: &str) -> Vec<i64> {
    profile::matcha_ids(phonemes)
}

/// Symbols of `text` that the profile's fixed table cannot represent.
#[pyfunction]
fn unmapped(text: &str, profile: &str) -> PyResult<Vec<String>> {
    Ok(profile::unmapped(text, parse_profile(profile)?)
        .into_iter()
        .map(String::from)
        .collect())
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_function(wrap_pyfunction!(render, m)?)?;
    m.add_function(wrap_pyfunction!(render_batch, m)?)?;
    m.add_function(wrap_pyfunction!(load_misaki, m)?)?;
    m.add_function(wrap_pyfunction!(has_misaki, m)?)?;
    m.add_function(wrap_pyfunction!(load_pack, m)?)?;
    m.add_function(wrap_pyfunction!(missing_packs, m)?)?;
    m.add_function(wrap_pyfunction!(sentences, m)?)?;
    m.add_function(wrap_pyfunction!(codepoints, m)?)?;
    m.add_function(wrap_pyfunction!(analyze, m)?)?;
    m.add_function(wrap_pyfunction!(piper_ids, m)?)?;
    m.add_function(wrap_pyfunction!(kokoro_ids, m)?)?;
    m.add_function(wrap_pyfunction!(kitten_ids, m)?)?;
    m.add_function(wrap_pyfunction!(matcha_ids, m)?)?;
    m.add_function(wrap_pyfunction!(unmapped, m)?)?;
    Ok(())
}
