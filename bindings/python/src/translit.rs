//! Functions behind `uccharan.transliterate`: romanised text into native script.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use uccharan::Lang;

fn lang_of(lang: &str) -> PyResult<Lang> {
    Lang::parse(lang).ok_or_else(|| PyValueError::new_err(format!("unknown language {lang:?}")))
}

/// Converted text with spans, as JSON; options as JSON (see the Python wrapper).
#[pyfunction]
fn transliterate(py: Python<'_>, text: &str, lang: &str, options: &str) -> PyResult<String> {
    let l = lang_of(lang)?;
    py.detach(|| uccharan::translit::transliterate_json(super::engine(), text, l, options))
        .map_err(PyValueError::new_err)
}

/// True when the transliteration pack of `lang` is loaded.
#[pyfunction]
fn has_translit(lang: &str) -> PyResult<bool> {
    Ok(super::engine().has_translit(lang_of(lang)?))
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(transliterate, m)?)?;
    m.add_function(wrap_pyfunction!(has_translit, m)?)?;
    Ok(())
}
