//! Python extension module `uccharan._native`. The typed public API lives in
//! `python/uccharan/__init__.py`.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::sync::OnceLock;
use uccharan::profile::{self, PhonemeIdMap, Profile};
use uccharan::{Lang, Mode, Normalize, Options, Phonemizer};

mod train;
mod translit;

fn engine() -> &'static Phonemizer {
    static ENGINE: OnceLock<Phonemizer> = OnceLock::new();
    // The Python layer fetches a language's pack before its first use and
    // raises when it cannot; the engine itself never stops the process.
    ENGINE.get_or_init(|| Phonemizer::new().rules_fallback(true))
}

fn options(lang: &str, normalize: &str, improved: bool, hinglish: bool) -> PyResult<Options> {
    let lang =
        Lang::parse(lang).ok_or_else(|| PyValueError::new_err(Lang::unknown_message(lang)))?;
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
    let mut out: Vec<String> = e.missing_packs(l).into_iter().map(String::from).collect();
    if profile == "kokoro" && l.is_english() && !e.has_misaki(l) {
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

// ---------------------------------------------------------------------------
// Normalise-only mode and pronunciation word lists.

/// A parsed pronunciation word list (TSV, PLS or JSON text).
#[pyclass(name = "WordList", frozen, module = "uccharan._native")]
struct PyWordList {
    inner: std::sync::Arc<uccharan::wordlist::WordList>,
}

#[pymethods]
impl PyWordList {
    #[new]
    fn new(text: &str) -> PyResult<Self> {
        let w = uccharan::wordlist::WordList::parse_any(text)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(PyWordList {
            inner: std::sync::Arc::new(w),
        })
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn to_tsv(&self) -> String {
        self.inner.to_tsv()
    }

    fn to_json(&self) -> String {
        self.inner.to_json()
    }

    #[pyo3(signature = (lang=None))]
    fn to_pls(&self, lang: Option<&str>) -> PyResult<String> {
        let l = match lang {
            None | Some("*") => None,
            Some(t) => Some(
                Lang::parse(t)
                    .ok_or_else(|| PyValueError::new_err(format!("unknown language {t:?}")))?,
            ),
        };
        Ok(self.inner.to_pls(l))
    }

    fn languages(&self) -> Vec<String> {
        self.inner
            .languages()
            .into_iter()
            .map(|l| l.map_or("*", Lang::tag).to_string())
            .collect()
    }

    fn warnings(&self) -> Vec<(usize, String)> {
        self.inner.warnings.clone()
    }
}

fn lists_of(lists: &[PyRef<'_, PyWordList>]) -> Vec<std::sync::Arc<uccharan::wordlist::WordList>> {
    lists.iter().map(|l| l.inner.clone()).collect()
}

/// Spoken form of `text` with spans, as JSON (see the Python wrapper).
#[pyfunction]
#[pyo3(signature = (text, lang, options, lists))]
fn normalize(
    py: Python<'_>,
    text: &str,
    lang: &str,
    options: &str,
    lists: Vec<PyRef<'_, PyWordList>>,
) -> PyResult<String> {
    let l = Lang::parse(lang)
        .ok_or_else(|| PyValueError::new_err(format!("unknown language {lang:?}")))?;
    let mut o =
        uccharan::normalize_api::options_from_json(options).map_err(PyValueError::new_err)?;
    o.wordlists.extend(lists_of(&lists));
    Ok(py.detach(|| uccharan::normalize_api::normalize(engine(), text, l, &o).to_json(text)))
}

/// `render` with word lists applied (entries' IPA for phoneme voices).
#[pyfunction]
#[pyo3(signature = (text, lang, profile, normalize, improved, hinglish, punctuation, flags, lists))]
#[allow(clippy::too_many_arguments)]
fn render_wordlists(
    py: Python<'_>,
    text: &str,
    lang: &str,
    profile: &str,
    normalize: &str,
    improved: bool,
    hinglish: bool,
    punctuation: bool,
    flags: bool,
    lists: Vec<PyRef<'_, PyWordList>>,
) -> PyResult<String> {
    let mut opt = options(lang, normalize, improved, hinglish)?;
    opt.keep_language_flags = flags;
    let prof = parse_profile(profile)?;
    let lists = lists_of(&lists);
    Ok(py.detach(|| {
        uccharan::normalize_api::render_with_wordlists(
            engine(),
            text,
            &opt,
            prof,
            punctuation,
            &lists,
        )
    }))
}

/// Respell IPA for a text voice: `(text, readback, distance)` or None.
#[pyfunction]
#[pyo3(signature = (ipa, lang, source=None, style="native"))]
fn respell(
    ipa: &str,
    lang: &str,
    source: Option<&str>,
    style: &str,
) -> PyResult<Option<(String, String, f32)>> {
    let l = Lang::parse(lang)
        .ok_or_else(|| PyValueError::new_err(format!("unknown language {lang:?}")))?;
    let src = source.and_then(Lang::parse);
    let st = uccharan::respell::Style::parse(style)
        .ok_or_else(|| PyValueError::new_err(format!("unknown respell style {style:?}")))?;
    Ok(uccharan::respell::respell(engine(), ipa, src, l, st)
        .map(|r| (r.text, r.readback, r.distance)))
}

// ---------------------------------------------------------------------------
// Sound-alike matching.

/// A vocabulary prepared for sound-alike matching (see the Python wrapper).
#[pyclass(name = "SoundIndex", frozen, module = "uccharan._native")]
struct PySoundIndex {
    inner: uccharan::soundalike::SoundIndex,
}

#[pymethods]
impl PySoundIndex {
    #[new]
    #[pyo3(signature = (vocabulary, options=""))]
    fn new(py: Python<'_>, vocabulary: &str, options: &str) -> PyResult<Self> {
        py.detach(|| uccharan::soundalike::SoundIndex::from_text(engine(), vocabulary, options))
            .map(|inner| PySoundIndex { inner })
            .map_err(PyValueError::new_err)
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn terms(&self) -> Vec<String> {
        self.inner.terms().iter().map(|t| t.word.clone()).collect()
    }

    fn sounds(&self, index: usize) -> Vec<String> {
        self.inner.sounds(index)
    }

    fn skipped(&self) -> Vec<(usize, String)> {
        self.inner.skipped().to_vec()
    }

    /// Candidates as JSON.
    fn find(&self, py: Python<'_>, text: &str, lang: &str, options: &str) -> PyResult<String> {
        let l = Lang::parse(lang)
            .ok_or_else(|| PyValueError::new_err(format!("unknown language {lang:?}")))?;
        py.detach(|| self.inner.find_json(engine(), text, l, options))
            .map_err(PyValueError::new_err)
    }

    /// The corrected text with its changes, as JSON.
    fn apply(&self, py: Python<'_>, text: &str, lang: &str, options: &str) -> PyResult<String> {
        let l = Lang::parse(lang)
            .ok_or_else(|| PyValueError::new_err(format!("unknown language {lang:?}")))?;
        py.detach(|| self.inner.apply_json(engine(), text, l, options))
            .map_err(PyValueError::new_err)
    }
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
    m.add_class::<PyWordList>()?;
    m.add_function(wrap_pyfunction!(normalize, m)?)?;
    m.add_function(wrap_pyfunction!(render_wordlists, m)?)?;
    m.add_function(wrap_pyfunction!(respell, m)?)?;
    m.add_class::<PySoundIndex>()?;
    train::register(m)?;
    translit::register(m)?;
    Ok(())
}
