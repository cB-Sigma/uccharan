//! Santali in Ol Chiki: habits of the written text, and romanised input.
//!
//! Some Ol Chiki text types a full stop for the mark ᱹ (gaahlaa ttuddaag),
//! which it resembles (ᱟ.ᱲᱟ for ᱟᱹᱲᱟ), and a hyphen for ᱼ (phaarkaa: ᱢᱮᱱᱟᱜ-ᱟ for
//! ᱢᱮᱱᱟᱜᱼᱟ); [`prepare`] writes the marks back before the text is read.
//!
//! Romanised Santali follows the Santali Latin alphabet (Bodding 1930;
//! Wikipedia "Santali Latin alphabet"): a ạ e i o u (o and o̠ = ᱚ, ō ô ǒ = ᱳ),
//! k' c' t' p' for the checked stops, ṭ ḍ ṇ ṛ, ñ ṅ, h after a consonant for
//! aspiration, a tilde for a nasal vowel, and its casual ASCII form, which
//! leaves out the marks. It is not romanised Hindi, so it has rules of its
//! own here, written in Ol Chiki by [`transliterate`].

use crate::norm::Item;
use crate::text::{script_of, Script};

fn ol_vowel(c: char) -> bool {
    matches!(c, 'ᱚ' | 'ᱟ' | 'ᱤ' | 'ᱩ' | 'ᱮ' | 'ᱳ')
}

fn ol_letter(c: char) -> bool {
    ('\u{1c5a}'..='\u{1c77}').contains(&c)
}

fn glottal(c: char) -> bool {
    matches!(c, 'ᱜ' | 'ᱡ' | 'ᱫ' | 'ᱵ')
}

/// `s` with a full stop between an Ol Chiki vowel letter (ᱟ ᱮ ᱚ) and
/// another Ol Chiki letter written as ᱹ, and a hyphen between a glottal
/// letter and an Ol Chiki vowel letter written as ᱼ; `None` when there is
/// neither.
pub fn prepare(s: &str) -> Option<String> {
    let cs: Vec<char> = s.chars().collect();
    let mut hit = false;
    let mut out = String::with_capacity(s.len() + 4);
    for (i, &c) in cs.iter().enumerate() {
        let prev = i.checked_sub(1).map(|k| cs[k]);
        let next = cs.get(i + 1).copied();
        if c == '.'
            && prev.is_some_and(|p| matches!(p, 'ᱟ' | 'ᱮ' | 'ᱚ'))
            && next.is_some_and(ol_letter)
        {
            out.push('ᱹ');
            hit = true;
        } else if c == '-' && prev.is_some_and(glottal) && next.is_some_and(ol_vowel) {
            out.push('ᱼ');
            hit = true;
        } else {
            out.push(c);
        }
    }
    hit.then_some(out)
}

/// Common romanised Santali words and their Ol Chiki spelling (sorted): the
/// words that signal a Santali clause and the short words that are also
/// English words. Spellings as in Ol Chiki text (IndicCorp v2).
const ROMAN_SAT: &[(&str, &str)] = &[
    ("ale", "ᱟᱞᱮ"),
    ("am", "ᱟᱢ"),
    ("ape", "ᱟᱯᱮ"),
    ("ar", "ᱟᱨ"),
    ("bang", "ᱵᱟᱝ"),
    ("chiki", "ᱪᱤᱠᱤ"),
    ("ciki", "ᱪᱤᱠᱤ"),
    ("disom", "ᱫᱤᱥᱚᱢ"),
    ("do", "ᱫᱚ"),
    ("hor", "ᱦᱚᱲ"),
    ("inj", "ᱤᱧ"),
    ("johar", "ᱡᱚᱦᱟᱨ"),
    ("kana", "ᱠᱟᱱᱟ"),
    ("ko", "ᱠᱚ"),
    ("menak'a", "ᱢᱮᱱᱟᱜᱼᱟ"),
    ("menaka", "ᱢᱮᱱᱟᱜᱼᱟ"),
    ("mit", "ᱢᱤᱫ"),
    ("noa", "ᱱᱚᱣᱟ"),
    ("ol", "ᱚᱞ"),
    ("ona", "ᱚᱱᱟ"),
    ("parsi", "ᱯᱟᱹᱨᱥᱤ"),
    ("re", "ᱨᱮ"),
    ("santal", "ᱥᱟᱱᱛᱟᱲ"),
    ("santali", "ᱥᱟᱱᱛᱟᱲᱤ"),
    ("uni", "ᱩᱱᱤ"),
];

/// Listed words that mark a clause as Santali on their own.
const SIGNAL: &[&str] = &[
    "ale", "ape", "chiki", "ciki", "disom", "hor", "inj", "johar", "kana", "menak'a", "menaka",
    "noa", "ona", "parsi", "santal", "santali", "uni",
];

/// The Ol Chiki letter of a romanised consonant (one or two Latin letters,
/// aspiration apart) and the number of letters it takes.
fn consonant(cs: &[char]) -> Option<(&'static str, usize)> {
    let two: String = cs.iter().take(2).collect();
    Some(match two.as_str() {
        // ng before a vowel is ŋɡ (sengel ᱥᱮᱝᱜᱮᱞ), elsewhere ŋ.
        "ng" if cs.get(2).is_some_and(|c| vowel(*c).is_some()) => ("ᱝᱜ", 2),
        "ng" => ("ᱝ", 2),
        "ny" => ("ᱧ", 2),
        "sh" => ("ᱥ", 2),
        _ => (
            match cs[0] {
                'k' | 'q' => "ᱠ",
                'g' => "ᱜ",
                'c' => "ᱪ",
                'j' | 'z' => "ᱡ",
                't' => "ᱛ",
                'd' => "ᱫ",
                'ṭ' => "ᱴ",
                'ḍ' => "ᱰ",
                'ṇ' => "ᱬ",
                'n' => "ᱱ",
                'p' => "ᱯ",
                'b' => "ᱵ",
                'm' => "ᱢ",
                'y' => "ᱭ",
                'r' => "ᱨ",
                'ṛ' => "ᱲ",
                'l' => "ᱞ",
                'w' | 'v' => "ᱣ",
                's' | 'ś' | 'ṣ' => "ᱥ",
                'h' => "ᱦ",
                'ñ' | 'ń' => "ᱧ",
                'ṅ' | 'ŋ' => "ᱝ",
                'f' => "ᱯᱷ",
                'x' => "ᱠᱥ",
                _ => return None,
            },
            1,
        ),
    })
}

/// The Ol Chiki vowel of a romanised vowel letter and whether it is nasal.
fn vowel(c: char) -> Option<(&'static str, bool)> {
    Some(match c {
        'a' | 'á' | 'à' => ("ᱟ", false),
        'ã' => ("ᱟ", true),
        'ạ' | 'ä' | 'â' => ("ᱟᱹ", false),
        'e' | 'é' => ("ᱮ", false),
        'ẽ' => ("ᱮ", true),
        'ẹ' | 'è' | 'ê' | 'ɛ' => ("ᱮᱹ", false),
        'i' | 'í' | 'ī' => ("ᱤ", false),
        'ĩ' => ("ᱤ", true),
        'u' | 'ú' | 'ū' => ("ᱩ", false),
        'ũ' => ("ᱩ", true),
        'o' | 'ọ' | 'ò' | 'ɔ' => ("ᱚ", false),
        'õ' => ("ᱚ", true),
        'ō' | 'ô' | 'ǒ' | 'ó' => ("ᱳ", false),
        _ => return None,
    })
}

/// Romanised Santali in Ol Chiki by rule. Vowels a ạ e ẹ i o ō u (ᱟ ᱟᱹ ᱮ ᱮᱹ
/// ᱤ ᱚ ᱳ ᱩ; a doubled vowel is one), a tilde writes ᱸ; consonants with their
/// letters (c ᱪ, j ᱡ, y ᱭ, w v ᱣ, ṭ ḍ ṇ ṛ, ñ ny ᱧ, ṅ ng ᱝ), h after a stop is
/// ᱷ. The checked stops: k' c' t' p', and k c t p at the end of a word (in
/// Santali words a final stop is checked), are the glottal letters ᱜ ᱡ ᱫ ᱵ,
/// with ᱼ before a vowel; g j d b at the end of a word or before a voiceless
/// consonant, where ᱜ ᱡ ᱫ ᱵ would be read checked, take ᱽ.
pub fn transliterate(word: &str) -> String {
    let cs: Vec<char> = word.to_lowercase().chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let voiceless = |c: Option<&char>| c.is_some_and(|c| "kctpsh".contains(*c) || *c == 'ṭ');
    let is_vowel = |c: Option<&char>| c.is_some_and(|c| vowel(*c).is_some());
    while i < cs.len() {
        let c = cs[i];
        if let Some((v, nasal)) = vowel(c) {
            out.push_str(v);
            let mut j = i + 1;
            // A doubled vowel is one vowel.
            if cs.get(j) == Some(&c) {
                j += 1;
            }
            if nasal || cs.get(j) == Some(&'\u{303}') {
                out.push('ᱸ');
                if cs.get(j) == Some(&'\u{303}') {
                    j += 1;
                }
            }
            i = j;
            continue;
        }
        if matches!(c, '\'' | '’' | 'ʼ' | '\u{303}' | '\u{320}' | '\u{331}') {
            i += 1;
            continue;
        }
        let Some((letter, len)) = consonant(&cs[i..]) else {
            i += 1;
            continue;
        };
        let after = cs.get(i + len);
        let apostrophe = matches!(after, Some('\'' | '’' | 'ʼ'));
        let end = i + len == cs.len() || (apostrophe && i + len + 1 == cs.len());
        let checked_letter = match letter {
            "ᱠ" => Some("ᱜ"),
            "ᱪ" => Some("ᱡ"),
            "ᱛ" => Some("ᱫ"),
            "ᱯ" => Some("ᱵ"),
            _ => None,
        };
        if let Some(g) = checked_letter.filter(|_| apostrophe || end) {
            out.push_str(g);
            if is_vowel(cs.get(i + len + usize::from(apostrophe))) {
                out.push('ᱼ');
            }
            i += len + usize::from(apostrophe);
            continue;
        }
        out.push_str(letter);
        let mut j = i + len;
        // h after a stop or ṛ: aspiration.
        if cs.get(j) == Some(&'h')
            && matches!(
                letter,
                "ᱠ" | "ᱜ" | "ᱪ" | "ᱡ" | "ᱛ" | "ᱫ" | "ᱴ" | "ᱰ" | "ᱯ" | "ᱵ" | "ᱲ"
            )
        {
            out.push('ᱷ');
            j += 1;
        }
        let voiced_glottal = matches!(letter, "ᱜ" | "ᱡ" | "ᱫ" | "ᱵ") && !out.ends_with('ᱷ');
        if voiced_glottal && (j == cs.len() || voiceless(cs.get(j))) {
            out.push('ᱽ');
        }
        i = j;
    }
    out
}

/// Rewrite the Latin words of a clause that are romanised Santali in Ol
/// Chiki: listed words always, other words unless `english` knows them, and
/// only in a clause with a word that marks it as Santali (a listed word of
/// [`SIGNAL`], or a word written with a letter of the Santali Latin alphabet
/// that English does not use: ạ ṛ ṭ ḍ ṇ ñ ṅ, or an apostrophe after k c t p).
pub fn rewrite_roman(items: &mut [Item], english: impl Fn(&str) -> bool) {
    let latin = |it: &Item| {
        !it.spell
            && !it.generated
            && it.text.chars().any(char::is_alphabetic)
            && it
                .text
                .chars()
                .all(|c| script_of(c) == Script::Latin || matches!(c, '\'' | '’'))
    };
    let marked = |w: &str| {
        w.chars().any(|c| "ạṛṭḍṇñṅ".contains(c))
            || ["k'", "c'", "t'", "p'", "k’", "c’", "t’", "p’"]
                .iter()
                .any(|m| w.contains(m))
    };
    let listed = |w: &str| ROMAN_SAT.binary_search_by(|p| p.0.cmp(w)).ok();
    let signal = items.iter().any(|it| {
        let w = it.text.to_lowercase();
        latin(it) && (SIGNAL.contains(&w.as_str()) || marked(&w))
    });
    if !signal {
        return;
    }
    for it in items.iter_mut().filter(|it| latin(it)) {
        let w = it.text.to_lowercase();
        if let Some(k) = listed(&w) {
            it.text = ROMAN_SAT[k].1.to_string();
        } else if !english(&w) || marked(&w) {
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
    fn lists_are_sorted() {
        assert!(ROMAN_SAT.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(SIGNAL.iter().all(|s| ROMAN_SAT.iter().any(|p| p.0 == *s)));
    }

    #[test]
    fn written_marks() {
        assert_eq!(prepare("ᱟ.ᱲᱟ ᱢᱮᱱᱟᱜ-ᱟ ᱾").unwrap(), "ᱟᱹᱲᱟ ᱢᱮᱱᱟᱜᱼᱟ ᱾");
        assert_eq!(prepare("ᱫᱚ. ᱟᱨ ᱥᱟᱶ-ᱛᱮ"), None);
    }

    #[test]
    fn rules() {
        for (r, o) in [
            ("hor", "ᱦᱚᱨ"),
            ("hoṛ", "ᱦᱚᱲ"),
            ("mit'", "ᱢᱤᱫ"),
            ("mit", "ᱢᱤᱫ"),
            ("menak'a", "ᱢᱮᱱᱟᱜᱼᱟ"),
            ("pạrsi", "ᱯᱟᱹᱨᱥᱤ"),
            ("santal", "ᱥᱟᱱᱛᱟᱞ"),
            ("dharti", "ᱫᱷᱟᱨᱛᱤ"),
            ("kathae", "ᱠᱟᱛᱷᱟᱮ"),
            ("raj", "ᱨᱟᱡᱽ"),
            ("cet'", "ᱪᱮᱫ"),
            ("cetan", "ᱪᱮᱛᱟᱱ"),
            ("jãhã", "ᱡᱟᱸᱦᱟᱸ"),
            ("iñ", "ᱤᱧ"),
            ("baṅ", "ᱵᱟᱝ"),
            ("sengel", "ᱥᱮᱝᱜᱮᱞ"),
            ("bang", "ᱵᱟᱝ"),
        ] {
            assert_eq!(transliterate(r), o, "{r}");
        }
    }

    #[test]
    fn rewrites_a_clause() {
        let mut items = vec![
            Item::word("Johar"),
            Item::word("office"),
            Item::word("disom"),
            Item::word("re"),
        ];
        for it in &mut items {
            // Words of the text, not of a number reading.
            it.generated = false;
        }
        rewrite_roman(&mut items, |w| w == "office");
        let t: Vec<&str> = items.iter().map(|i| i.text.as_str()).collect();
        assert_eq!(t, ["ᱡᱚᱦᱟᱨ", "office", "ᱫᱤᱥᱚᱢ", "ᱨᱮ"]);
        let mut english = vec![Item::word("do"), Item::word("re")];
        for it in &mut english {
            it.generated = false;
        }
        rewrite_roman(&mut english, |_| false);
        assert_eq!(english[0].text, "do");
    }
}
