//! Romanised Hindi ("Hinglish" typed in Latin letters). **Experimental.**
//!
//! There is no compatibility target here: the reference phonemizer reads
//! romanised Hindi as English. The approach is deliberately simple:
//!
//! 1. per-token language guess from two lexicons (a hand-written list of
//!    common romanisations, and the English lexicon) plus the neighbours;
//! 2. tokens judged Hindi are rewritten in Devanagari — from the list, or by a
//!    rule-based transliteration for unlisted words — and then go through the
//!    normal Hindi path.

use super::Hindi;
use crate::en::English;
use crate::norm::Item;
use crate::text::{script_of, Script};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Guess {
    /// In the romanised list and not an English word.
    Hindi,
    /// In both lists ("to", "me", "the", "hi", "main").
    Ambiguous,
    /// English word only.
    English,
    /// In neither list.
    Unknown,
    /// Not a Latin-script word.
    Other,
}

/// Rewrite Latin items that look like romanised Hindi as Devanagari.
pub fn rewrite(items: &mut [Item], hi: &Hindi<'_>, en: Option<&English<'_>>) {
    let guesses: Vec<Guess> = items
        .iter()
        .map(|it| {
            if it.spell
                || it.generated
                || !it
                    .text
                    .chars()
                    .all(|c| script_of(c) == Script::Latin || c == '\'')
            {
                return if it.text.chars().any(|c| script_of(c) == Script::Devanagari) {
                    Guess::Hindi
                } else {
                    Guess::Other
                };
            }
            let w = it.text.to_lowercase();
            let listed = hi.lookup(&format!("~{w}")).is_some();
            let english = en.is_some_and(|e| e.in_lexicon(&w));
            match (listed, english) {
                (true, false) => Guess::Hindi,
                (true, true) => Guess::Ambiguous,
                (false, true) => Guess::English,
                (false, false) => Guess::Unknown,
            }
        })
        .collect();
    let latin = guesses
        .iter()
        .filter(|g| !matches!(g, Guess::Other))
        .count();
    let sure_hindi = guesses.iter().filter(|g| **g == Guess::Hindi).count();
    let sure_english = guesses.iter().filter(|g| **g == Guess::English).count();
    if latin == 0 || sure_hindi == 0 {
        return;
    }
    // The clause leans Hindi when unambiguous Hindi words are at least as common as
    // unambiguous English ones.
    let hindi_clause = sure_hindi >= sure_english;
    for i in 0..items.len() {
        let near_hindi =
            (i > 0 && guesses[i - 1] == Guess::Hindi) || guesses.get(i + 1) == Some(&Guess::Hindi);
        let take = match guesses[i] {
            Guess::Hindi => !items[i]
                .text
                .chars()
                .any(|c| script_of(c) == Script::Devanagari),
            Guess::Ambiguous => near_hindi || hindi_clause,
            Guess::Unknown => near_hindi && hindi_clause,
            _ => false,
        };
        if !take {
            continue;
        }
        let w = items[i].text.to_lowercase();
        let deva = hi
            .lookup(&format!("~{w}"))
            .unwrap_or_else(|| transliterate(&w));
        if !deva.is_empty() {
            items[i].text = deva;
        }
    }
}

/// Rule-based romanised Hindi → Devanagari for words outside the list.
/// Follows the informal conventions of chat romanisation (aa/ee/oo for long
/// vowels, kh/gh/ch/chh/th/dh/ph/bh/sh for aspirates and sibilants).
pub fn transliterate(word: &str) -> String {
    const CONS: &[(&str, &str)] = &[
        ("chh", "छ"),
        ("ksh", "क्ष"),
        ("shr", "श्र"),
        ("gy", "ज्ञ"),
        ("kh", "ख"),
        ("gh", "घ"),
        ("ch", "च"),
        ("jh", "झ"),
        ("th", "थ"),
        ("dh", "ध"),
        ("ph", "फ"),
        ("bh", "भ"),
        ("sh", "श"),
        ("ng", "ंग"),
        ("k", "क"),
        ("g", "ग"),
        ("c", "क"),
        ("j", "ज"),
        ("t", "त"),
        ("d", "द"),
        ("n", "न"),
        ("p", "प"),
        ("f", "फ़"),
        ("b", "ब"),
        ("m", "म"),
        ("y", "य"),
        ("r", "र"),
        ("l", "ल"),
        ("v", "व"),
        ("w", "व"),
        ("s", "स"),
        ("h", "ह"),
        ("z", "ज़"),
        ("q", "क़"),
        ("x", "क्स"),
    ];
    // (roman, independent letter, vowel sign)
    const VOWELS: &[(&str, &str, &str)] = &[
        ("aa", "आ", "ा"),
        ("ai", "ऐ", "ै"),
        ("au", "औ", "ौ"),
        ("ee", "ई", "ी"),
        ("ii", "ई", "ी"),
        ("oo", "ऊ", "ू"),
        ("uu", "ऊ", "ू"),
        ("ou", "औ", "ौ"),
        ("ei", "ए", "े"),
        ("a", "अ", ""),
        ("i", "इ", "ि"),
        ("u", "उ", "ु"),
        ("e", "ए", "े"),
        ("o", "ओ", "ो"),
    ];
    let w: String = word
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .collect::<String>()
        .to_lowercase();
    let b = w.as_str();
    let mut out = String::new();
    let mut i = 0;
    let mut prev_cons = false;
    while i < b.len() {
        let rest = &b[i..];
        if let Some((r, d)) = CONS.iter().find(|(r, _)| rest.starts_with(r)) {
            if prev_cons {
                out.push('\u{094d}');
            }
            out.push_str(d);
            i += r.len();
            prev_cons = true;
            continue;
        }
        if let Some((r, indep, sign)) = VOWELS.iter().find(|(r, _, _)| rest.starts_with(r)) {
            let at_end = i + r.len() == b.len();
            if prev_cons {
                // A final short "a" or "i"/"u"/"e"/"o" is long in practice: "ladka" → लड़का.
                match (*r, at_end) {
                    ("a", true) => out.push('ा'),
                    ("i", true) => out.push('ी'),
                    ("u", true) => out.push('ू'),
                    _ => out.push_str(sign),
                }
            } else {
                out.push_str(if *r == "a" && at_end && !out.is_empty() {
                    "आ"
                } else {
                    indep
                });
            }
            i += r.len();
            prev_cons = false;
            // nasal vowel: "n" at the end after a long vowel ("kahan", "nahin")
            if &b[i..] == "n" && matches!(*r, "aa" | "ai" | "ee" | "oo" | "o" | "e") {
                out.push('ं');
                i += 1;
            }
            continue;
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transliteration_rules() {
        assert_eq!(transliterate("ladka"), "लद्का");
        assert_eq!(transliterate("khushi"), "खुशी");
        assert_eq!(transliterate("pyaar"), "प्यार");
        assert_eq!(transliterate("bhaag"), "भाग");
        assert_eq!(transliterate("zindagi"), "ज़िन्दगी");
    }
}
