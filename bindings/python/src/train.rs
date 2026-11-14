//! Functions behind `uccharan.train`: pronunciation scoring.

use pyo3::prelude::*;
use uccharan::soundalike::score;
use uccharan::Lang;

/// One alignment step: `(kind, reference phone, heard phone, cost)`.
type Step = (&'static str, String, String, f32);
/// One reference word: `(cost, weight, phones, heard start, heard end, steps)`.
type Word = (f32, f32, usize, usize, usize, Vec<Step>);
/// `(words, cost, weight, reference phones, heard phones, substitutions,
/// deletions, insertions, extra heard words)`.
type Aligned = (
    Vec<Word>,
    f32,
    f32,
    usize,
    usize,
    usize,
    usize,
    usize,
    Vec<usize>,
);

/// Align the IPA of reference words with the IPA of heard words, with the
/// sound-alike matcher's weighted distance.
#[pyfunction]
#[pyo3(signature = (reference, heard, reference_english=false, heard_english=false))]
fn align_words(
    py: Python<'_>,
    reference: Vec<String>,
    heard: Vec<String>,
    reference_english: bool,
    heard_english: bool,
) -> Aligned {
    py.detach(|| {
        let r: Vec<&str> = reference.iter().map(String::as_str).collect();
        let h: Vec<&str> = heard.iter().map(String::as_str).collect();
        let a = score::align_words(&r, &h, reference_english, heard_english);
        let words = a
            .words
            .into_iter()
            .map(|w| {
                let ops = w
                    .ops
                    .into_iter()
                    .map(|o| (o.kind.name(), o.reference, o.heard, o.cost))
                    .collect();
                (w.cost, w.weight, w.phones, w.heard.start, w.heard.end, ops)
            })
            .collect();
        (
            words,
            a.cost,
            a.weight,
            a.reference_phones,
            a.heard_phones,
            a.substitutions,
            a.deletions,
            a.insertions,
            a.extra,
        )
    })
}

/// The phones of an IPA string in the matcher's broad notation.
#[pyfunction]
#[pyo3(signature = (ipa, english=false))]
fn broad_phones(ipa: &str, english: bool) -> Vec<String> {
    score::broad_phones(ipa, english)
}

/// Whether the English lexicon lists the word; `None` for the other
/// languages, which are read by rules and tables of exceptions (and when
/// the English data is not loaded).
#[pyfunction]
fn in_lexicon(word: &str, lang: &str) -> Option<bool> {
    let l = Lang::parse(lang).filter(|l| l.is_english())?;
    let d = super::engine().data(l)?;
    let lower = word.to_lowercase();
    let mut s = String::new();
    Some(d.lookup(&lower, &mut s) || d.known(&lower))
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(align_words, m)?)?;
    m.add_function(wrap_pyfunction!(broad_phones, m)?)?;
    m.add_function(wrap_pyfunction!(in_lexicon, m)?)?;
    Ok(())
}
