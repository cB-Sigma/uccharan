//! Dogri: romanised input. Dogri is typed in Latin letters with the habits
//! of romanised Hindi, so romanised Dogri goes through the same path
//! (`hinglish`, `transliterate` without a pack) after the common Dogri
//! words of this list are written in Devanagari.

use crate::norm::Item;

/// Common romanised Dogri spellings and their Devanagari (sorted). Written
/// for this list; not yet checked by a Dogri speaker or on chat text.
const ROMAN_DOI: &[(&str, &str)] = &[
    ("ae", "ऐ"),
    ("bi", "बी"),
    ("ch", "च"),
    ("da", "दा"),
    ("de", "दे"),
    ("di", "दी"),
    ("dogri", "डोगरी"),
    ("duggar", "डुग्गर"),
    ("eh", "एह्"),
    ("gall", "गल्ल"),
    ("gallan", "गल्लां"),
    ("gi", "गी"),
    ("hun", "हून"),
    ("ithe", "इत्थै"),
    ("jammu", "जम्मू"),
    ("kanne", "कन्नै"),
    ("kiyan", "कि’यां"),
    ("kuthe", "कुत्थै"),
    ("nei", "नेईं"),
    ("nein", "नेईं"),
    ("oh", "ओह्"),
    ("shail", "शैल"),
    ("te", "ते"),
    ("thaman", "थमां"),
    ("thma", "थमां"),
    ("tusen", "तुसें"),
    ("uthe", "उत्थै"),
];

/// Rewrite the listed romanised Dogri words of a clause in Devanagari (the
/// romanised-Hindi rules then read the rest).
pub fn rewrite_roman(items: &mut [Item]) {
    for it in items {
        let w = it.text.to_lowercase();
        if let Ok(k) = ROMAN_DOI.binary_search_by(|p| p.0.cmp(w.as_str())) {
            it.text = ROMAN_DOI[k].1.to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_is_sorted() {
        assert!(ROMAN_DOI.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn rewrites_listed_words() {
        let mut items = vec![
            Item::word("Tusen"),
            Item::word("kiyan"),
            Item::word("office"),
        ];
        rewrite_roman(&mut items);
        let t: Vec<&str> = items.iter().map(|i| i.text.as_str()).collect();
        assert_eq!(t, ["तुसें", "कि’यां", "office"]);
    }
}
