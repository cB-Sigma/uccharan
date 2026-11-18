//! Bodo (Boro): the apostrophe of the Devanagari spelling, and romanised
//! input.
//!
//! Bodo Devanagari writes an apostrophe after a consonant whose inherent
//! vowel is said where the reader would otherwise drop it (बर’ "Boro",
//! कर’ना, लाथिख’); improved mode reads it as that vowel's sign.
//!
//! Romanised Bodo follows the older Latin orthography of Bodo (w for the
//! vowel ɯ, o for the inherent vowel, ng for ŋ: "Gaswi subung anw udangwi
//! man sanman arw mwnthai lana jwnwm layw", the first article of the
//! Universal Declaration of Human Rights). It is not romanised Hindi, so
//! it has rules of its own here.

use crate::norm::Item;
use crate::text::{script_of, Script};

/// `s` with every apostrophe (`'`, `’`, `ʼ`) that follows a Devanagari
/// consonant written as the vowel sign ॉ, which the Bodo language file reads
/// as the inherent vowel; `None` when there is none.
pub fn apostrophe_vowel(s: &str) -> Option<String> {
    let consonant =
        |c: char| matches!(c, '\u{0915}'..='\u{0939}' | '\u{0958}'..='\u{095f}' | '\u{093c}');
    let mut prev = ' ';
    let mut hit = false;
    let mut out = String::with_capacity(s.len() + 4);
    for c in s.chars() {
        if matches!(c, '\'' | '’' | 'ʼ') && consonant(prev) {
            out.push('\u{0949}');
            hit = true;
        } else {
            out.push(c);
        }
        prev = c;
    }
    hit.then_some(out)
}

/// Common romanised Bodo words and their Devanagari spelling (sorted): the
/// words of the Latin sample above, function words and the words that are
/// also English words. Spellings as in Bodo Devanagari text (IndicCorp v2).
const ROMAN_BRX: &[(&str, &str)] = &[
    ("ang", "आं"),
    ("anw", "आनो"),
    ("arw", "आरो"),
    ("assam", "आसाम"),
    ("be", "बे"),
    ("bi", "बि"),
    ("bima", "बिमा"),
    ("bipha", "बिफा"),
    ("bodo", "बड’"),
    ("boro", "बर’"),
    ("dong", "दं"),
    ("dwi", "दै"),
    ("gaswi", "गासै"),
    ("gaswibw", "गासैबो"),
    ("ja", "जा"),
    ("jwng", "जों"),
    ("jwnwm", "जोनोम"),
    ("khalam", "खालाम"),
    ("lana", "लाना"),
    ("layw", "लायो"),
    ("man", "मान"),
    ("mwjang", "मोजां"),
    ("mwnthai", "मोनथाय"),
    ("na", "ना"),
    ("nonga", "नङा"),
    ("nwng", "नों"),
    ("raw", "राव"),
    ("sanman", "सनमान"),
    ("sase", "सासे"),
    ("se", "से"),
    ("subung", "सुबुं"),
    ("udangwi", "उदांयै"),
];

/// Romanised Bodo in Devanagari by rule. Consonants: kh th ph (ख थ फ),
/// k t p, g d b, ng (ं at the end of a syllable, ङ before a vowel), m n,
/// s z j (स ज ज), r l h y; vowels: a i u e (आ इ उ ए), w (ओ, the vowel ɯ),
/// wi wu (ऐ औ), o (the inherent vowel: no sign in the first syllable,
/// the apostrophe after it). Consonants without a vowel between them are
/// joined with a virama; a word-final consonant is written bare.
pub fn transliterate(word: &str) -> String {
    let cs: Vec<char> = word.to_lowercase().chars().collect();
    let vowel = |c: Option<&char>| c.is_some_and(|c| "aeiouw".contains(*c));
    let mut out = String::new();
    // What the last unit was: none yet, a consonant, a vowel.
    #[derive(PartialEq, Clone, Copy)]
    enum Last {
        Start,
        Cons,
        Vowel,
    }
    let mut last = Last::Start;
    let mut seen_vowel = false;
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        let next = cs.get(i + 1).copied();
        // Vowels (w is a vowel unless a, e or o follows it).
        let w_cons = c == 'w' && matches!(next, Some('a' | 'e' | 'o'));
        if vowel(Some(&c)) && !w_cons {
            let (indep, sign, len): (&str, &str, usize) = match (c, next) {
                ('w', Some('i')) => ("ऐ", "ै", 2),
                ('w', Some('u')) => ("औ", "ौ", 2),
                ('w', _) => ("ओ", "ो", 1),
                ('a', Some('a')) => ("आ", "ा", 2),
                ('a', _) => ("आ", "ा", 1),
                ('i', Some('i')) | ('e', Some('e')) => ("ई", "ी", 2),
                ('i', _) => ("इ", "ि", 1),
                ('u', Some('u')) | ('o', Some('o')) => ("ऊ", "ू", 2),
                ('u', _) => ("उ", "ु", 1),
                ('e', _) => ("ए", "े", 1),
                // o after a or e is the second half of a diphthong (दाव).
                ('o', _) if last == Last::Vowel => ("व", "व", 1),
                ('o', _) => ("अ", "", 1),
                _ => ("", "", 1),
            };
            match last {
                Last::Cons => {
                    if c == 'o' && len == 1 && seen_vowel {
                        // The inherent vowel after the first syllable: the
                        // apostrophe keeps it from being dropped.
                        out.push('’');
                    } else {
                        out.push_str(sign);
                    }
                }
                _ => out.push_str(indep),
            }
            last = Last::Vowel;
            seen_vowel = true;
            i += len;
            continue;
        }
        // Consonants, longest spelling first.
        let pair: String = cs[i..(i + 2).min(cs.len())].iter().collect();
        let (letter, len): (&str, usize) = match pair.as_str() {
            "kh" => ("ख", 2),
            "th" => ("थ", 2),
            "ph" => ("फ", 2),
            "gh" => ("घ", 2),
            "jh" => ("झ", 2),
            "dh" => ("ध", 2),
            "bh" => ("भ", 2),
            "sh" => ("श", 2),
            "ch" => ("च", 2),
            "ng" => ("ङ", 2),
            _ => (
                match c {
                    'k' | 'c' | 'q' => "क",
                    'g' => "ग",
                    't' => "त",
                    'd' => "द",
                    'p' => "प",
                    'f' => "फ",
                    'b' => "ब",
                    'v' => "भ",
                    'm' => "म",
                    'n' => "न",
                    's' => "स",
                    'z' | 'j' => "ज",
                    'r' => "र",
                    'l' => "ल",
                    'h' => "ह",
                    'y' => "य",
                    'w' => "व",
                    'x' => "क्स",
                    _ => "",
                },
                1,
            ),
        };
        if letter.is_empty() {
            i += 1;
            continue;
        }
        let after = cs.get(i + len);
        if letter == "ङ" && last == Last::Vowel && !vowel(after) {
            // ŋ closing a syllable: the anusvara.
            out.push('ं');
            i += len;
            continue;
        }
        if last == Last::Cons {
            out.push('्');
        }
        out.push_str(letter);
        last = Last::Cons;
        i += len;
    }
    out
}

/// Rewrite the Latin words of a clause that are romanised Bodo in
/// Devanagari: listed words always, other words unless `english` knows
/// them, and only in a clause with at least one listed word or a word that
/// writes the vowel w before a consonant (jwnwm, arw).
pub fn rewrite_roman(items: &mut [Item], english: impl Fn(&str) -> bool) {
    let latin = |it: &Item| {
        !it.spell
            && !it.generated
            && it.text.chars().any(|c| c.is_ascii_alphabetic())
            && it.text.chars().all(|c| script_of(c) == Script::Latin)
    };
    let bodo_w = |w: &str| {
        let cs: Vec<char> = w.chars().collect();
        cs.iter().enumerate().any(|(k, &c)| {
            c == 'w'
                && k > 0
                && !"aeiou".contains(cs[k - 1])
                && cs.get(k + 1).is_none_or(|n| !"aeio".contains(*n))
        })
    };
    let listed = |w: &str| ROMAN_BRX.binary_search_by(|p| p.0.cmp(w)).ok();
    let signal = items.iter().any(|it| {
        let w = it.text.to_lowercase();
        latin(it) && (listed(&w).is_some() || bodo_w(&w))
    });
    if !signal {
        return;
    }
    for it in items.iter_mut().filter(|it| latin(it)) {
        let w = it.text.to_lowercase();
        if let Some(k) = listed(&w) {
            it.text = ROMAN_BRX[k].1.to_string();
        } else if !english(&w) || bodo_w(&w) {
            let d = transliterate(&w);
            if !d.is_empty() {
                it.text = d;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_is_sorted() {
        assert!(ROMAN_BRX.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn apostrophe() {
        assert_eq!(apostrophe_vowel("बर’ आरो कर'ना").unwrap(), "बरॉ आरो करॉना");
        assert_eq!(apostrophe_vowel("आं ’बे’"), None);
    }

    #[test]
    fn rules() {
        for (r, d) in [
            ("boro", "बर’"),
            ("dokhona", "दख’ना"),
            ("kokrajhar", "कक्राझार"),
            ("lathikho", "लाथिख’"),
            ("subung", "सुबुं"),
            ("jwnwm", "जोनोम"),
            ("gaswi", "गासै"),
            ("mwswu", "मोसौ"),
            ("mwnthai", "मोन्थाइ"),
            ("dao", "दाव"),
            ("sni", "स्नि"),
            ("bithanga", "बिथाङा"),
        ] {
            assert_eq!(transliterate(r), d, "{r}");
        }
    }

    #[test]
    fn rewrites_a_clause() {
        let mut items = vec![
            Item::word("Ang"),
            Item::word("office"),
            Item::word("thangw"),
        ];
        for it in &mut items {
            // Words of the text, not of a number reading.
            it.generated = false;
        }
        rewrite_roman(&mut items, |w| w == "office");
        let t: Vec<&str> = items.iter().map(|i| i.text.as_str()).collect();
        assert_eq!(t, ["आं", "office", "थाङो"]);
    }
}
