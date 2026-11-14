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
    // Most words of running text are not in the table: a key filter
    // answers for them without decoding a block.
    b.key_filter(8);
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
    let Some(latin) = uccharan::Lang::parse(lang).and_then(uccharan::Lang::latin) else {
        // `#model N`: a context wider than three letters must be seen N times.
        let min_count = text
            .lines()
            .find_map(|l| l.strip_prefix("#model"))
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(1);
        return build_corrector(lang, &rows, min_count, b);
    };
    let trainable = |w: &str| w == w.to_lowercase() && !w.contains(['\'', '’']);
    let train: Vec<(String, String)> = rows
        .iter()
        .filter(|(w, _)| trainable(w))
        .map(|(w, r)| (w.clone(), plain(r).replace("ˈʔ", "ʔˈ").replace("ˌʔ", "ʔˌ")))
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
    dump_entries(lang, &rows, &mut b);
    b.build().expect("build")
}

/// A reading without the marks of a latent liaison consonant.
pub fn plain(r: &str) -> String {
    r.chars()
        .filter(|c| !matches!(c, 'ᶻ' | 'ᵗ' | 'ⁿ' | 'ʳ' | 'ᵖ'))
        .collect()
}

/// A word table for a language read by letter-decoding rules (Urdu,
/// Sindhi): the model learns to correct the rules' reading of a word, and
/// the table keeps the words that model and rules together still get wrong.
fn build_corrector(
    lang: &str,
    rows: &[(String, String)],
    min_count: u32,
    mut b: Builder,
) -> Vec<u8> {
    let indic = uccharan::Lang::parse(lang)
        .and_then(uccharan::Lang::indic)
        .expect("a language with a language file");
    let train: Vec<(String, String)> = rows
        .iter()
        .map(|(w, r)| (indic.word(w, true), r.clone()))
        .collect();
    let (letters, model) = crate::g2p::train(&train, min_count, &crate::g2p::SHAPES, None);
    b.lts(model.outputs, model.tables);
    b.meta("letters", &letters);
    b.meta("input", "rules");
    let first = Data::from_vec(b.build().expect("build")).expect("image");
    let mut kept = 0usize;
    for (w, r) in rows {
        if indic.read(w, true, false, Some(&first)) != *r {
            b.entry(w, r);
            kept += 1;
        }
    }
    eprintln!("{lang}: {} words, {kept} table entries", rows.len());
    dump_entries(lang, rows, &mut b);
    b.build().expect("build")
}

/// `LEXGEN_DUMP=<dir>`: write the words that became table entries to
/// `<dir>/<lang>_entries.txt`.
fn dump_entries(lang: &str, rows: &[(String, String)], b: &mut Builder) {
    let Ok(dir) = std::env::var("LEXGEN_DUMP") else {
        return;
    };
    let d = Data::from_vec(b.build().expect("build")).expect("image");
    let mut out = String::new();
    let mut found = String::new();
    for (w, _) in rows {
        if d.lookup(w, &mut found) {
            out.push_str(w);
            out.push('\n');
        }
    }
    let _ = std::fs::write(Path::new(&dir).join(format!("{lang}_entries.txt")), out);
}
