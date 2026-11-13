//! Hindi data: a small exceptions lexicon and the romanised-Hindi word list,
//! both hand-written for this project (`data/sources/hi/*.tsv`).

use std::path::Path;
use uccharan::data::Builder;

pub fn build(root: &Path) -> Vec<u8> {
    let mut b = Builder::new();
    for (file, prefix) in [("exceptions.tsv", ""), ("romanized.tsv", "~")] {
        let text =
            std::fs::read_to_string(root.join("data/sources/hi").join(file)).unwrap_or_default();
        for line in text.lines() {
            let line = line.trim_end();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut it = line.split('\t');
            if let (Some(k), Some(v)) = (it.next(), it.next()) {
                // exceptions.tsv: "compatible<TAB>improved" is stored as "compatible|improved".
                match it.next().filter(|x| !x.is_empty() && prefix.is_empty()) {
                    Some(improved) => b.entry(&format!("{prefix}{k}"), &format!("{v}|{improved}")),
                    None => b.entry(&format!("{prefix}{k}"), v),
                };
            }
        }
    }
    b.meta("lang", "hi");
    b.meta("format", "1");
    b.meta("source", "hand-written for uccharan (Apache-2.0)");
    b.build().expect("build")
}
