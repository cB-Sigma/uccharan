//! Word tables of the data-driven languages (`data/sources/<lang>/words.tsv`:
//! word, then its phonemes in compatible mode).

use std::path::Path;
use uccharan::data::{Builder, Data};

pub fn build(root: &Path, lang: &str, source: &str) -> Vec<u8> {
    let text = std::fs::read_to_string(root.join("data/sources").join(lang).join("words.tsv"))
        .unwrap_or_default();
    let rows: Vec<(String, String)> = text
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_once('\t'))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let mut b = Builder::new();
    b.meta("lang", lang);
    b.meta("format", "1");
    b.meta("source", source);
    if !text.lines().any(|l| l.starts_with("#model")) {
        // A list of the words the letter rules read differently.
        for (k, v) in &rows {
            b.entry(k, v);
        }
        return b.build().expect("build");
    }
    // A full word list: a letter-to-sound model learns it, and the table
    // keeps the words that the run-time reading without a table entry
    // (spelled word, model, elision) does not give back.
    let latin = uccharan::Lang::parse(lang)
        .and_then(uccharan::Lang::latin)
        .expect("a Latin-script language");
    let trainable = |w: &str| w == w.to_lowercase() && !w.contains(['\'', '’']);
    let train: Vec<(String, String)> = rows
        .iter()
        .filter(|(w, _)| trainable(w))
        .map(|(w, r)| (w.clone(), plain(r)))
        .collect();
    let (letters, model) = crate::g2p::train(&train, 1, &crate::g2p::SHAPES, Some(latin));
    b.lts(model.outputs, model.tables);
    b.meta("letters", &letters);
    b.meta("stress", "latent");
    let differs = |d: &Data, w: &str, r: &str| latin.token(w, true, Some(d)).1 != r;
    let first = Data::from_vec(b.build().expect("build")).expect("image");
    let mut kept = 0usize;
    for (w, r) in &rows {
        if w.contains('\'') {
            continue;
        }
        if !trainable(w) || differs(&first, w, r) {
            b.entry(w, r);
            kept += 1;
        }
    }
    let second = Data::from_vec(b.build().expect("build")).expect("image");
    for (w, r) in rows.iter().filter(|(w, _)| w.contains('\'')) {
        if differs(&second, w, r) {
            b.entry(w, r);
            kept += 1;
        }
    }
    eprintln!("{lang}: {} words, {kept} table entries", rows.len());
    if let Ok(dir) = std::env::var("LEXGEN_DUMP") {
        let d = Data::from_vec(b.build().expect("build")).expect("image");
        let mut out = String::new();
        let mut found = String::new();
        for (w, _) in &rows {
            if d.lookup(w, &mut found) {
                out.push_str(w);
                out.push('\n');
            }
        }
        let _ = std::fs::write(Path::new(&dir).join(format!("{lang}_entries.txt")), out);
    }
    b.build().expect("build")
}

/// A reading without the marks of a latent liaison consonant.
pub fn plain(r: &str) -> String {
    r.chars()
        .filter(|c| !matches!(c, 'ᶻ' | 'ᵗ' | 'ⁿ' | 'ʳ' | 'ᵖ'))
        .collect()
}
