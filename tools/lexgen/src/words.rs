//! Word tables of the data-driven languages (`data/sources/<lang>/words.tsv`:
//! word, then its phonemes in compatible mode).

use std::path::Path;
use uccharan::data::Builder;

pub fn build(root: &Path, lang: &str, source: &str) -> Vec<u8> {
    let mut b = Builder::new();
    let text = std::fs::read_to_string(root.join("data/sources").join(lang).join("words.tsv"))
        .unwrap_or_default();
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut it = line.split('\t');
        if let (Some(k), Some(v)) = (it.next(), it.next()) {
            b.entry(k, v);
        }
    }
    b.meta("lang", lang);
    b.meta("format", "1");
    b.meta("source", source);
    b.build().expect("build")
}
