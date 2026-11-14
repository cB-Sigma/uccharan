//! Optional pronunciation packs built from WikiPron (Wiktionary text,
//! CC BY-SA 4.0): `wikipron-en-us.ucd`, `wikipron-en-gb.ucd`,
//! `wikipron-hi.ucd`. They are not part of the Apache-2.0 core; users load
//! them with [`crate::Phonemizer::load_pack`]. They are consulted in the
//! improved mode only, for words and names outside uccharan's own lexicon.
//!
//! Pack entries are already in uccharan's symbols but carry no stress
//! (WikiPron removes it), and a `.` separates vowels of different
//! syllables. [`with_stress`] puts the stress of uccharan's own prediction
//! for the word onto the pack's segments.

use crate::Lang;

fn is_vowel_char(c: char) -> bool {
    matches!(
        c,
        'a' | 'e'
            | 'i'
            | 'o'
            | 'u'
            | 'æ'
            | 'ɑ'
            | 'ɒ'
            | 'ɔ'
            | 'ə'
            | 'ɛ'
            | 'ɜ'
            | 'ɪ'
            | 'ʊ'
            | 'ʌ'
            | 'ɐ'
            | 'ɚ'
            | 'ᵻ'
            | 'ɘ'
            | 'ɵ'
            | 'ʉ'
    )
}

fn is_mod(c: char) -> bool {
    matches!(c, 'ː' | '\u{303}' | '\u{329}' | '\u{32f}')
}

/// Nuclei of a pronunciation as `(start, end)` byte ranges, and the stress
/// mark found right before each (`0`, `1` primary, `2` secondary).
fn nuclei(s: &str) -> Vec<(usize, usize, u8)> {
    let mut out: Vec<(usize, usize, u8)> = Vec::new();
    let mut open = false;
    let mut pending = 0u8;
    for (i, c) in s.char_indices() {
        if is_vowel_char(c) {
            if open {
                out.last_mut().unwrap().1 = i + c.len_utf8();
            } else {
                out.push((i, i + c.len_utf8(), pending));
                open = true;
            }
            pending = 0;
        } else if open && is_mod(c) {
            out.last_mut().unwrap().1 = i + c.len_utf8();
        } else {
            open = false;
            match c {
                'ˈ' => pending = 1,
                'ˌ' => pending = 2,
                _ => {}
            }
        }
    }
    out
}

fn reduced(v: &str) -> bool {
    matches!(v, "ə" | "ɚ" | "ɐ" | "ᵻ")
}

/// Put the stress pattern of `predicted` (uccharan's own reading of the
/// word) onto a pack entry and finish the vowels whose quality depends on
/// stress.
pub fn with_stress(stored: &str, predicted: &str, lang: Lang) -> String {
    let w = nuclei(stored);
    if w.is_empty() {
        return stored.replace('.', "");
    }
    let p = nuclei(predicted);
    let mut levels = vec![0u8; w.len()];
    if p.len() == w.len() {
        for (k, n) in p.iter().enumerate() {
            levels[k] = n.2;
        }
    }
    if !levels.contains(&1) {
        levels.iter_mut().for_each(|l| *l = 0);
        let primary = p.iter().position(|n| n.2 == 1);
        let mut idx = match primary {
            Some(pi) if p.len() > 1 && w.len() > 1 => {
                if lang == Lang::Hi {
                    // Hindi stress sits near the end: keep the distance from it.
                    (w.len() - 1).saturating_sub(p.len() - 1 - pi)
                } else {
                    ((pi * (w.len() - 1)) as f32 / (p.len() - 1) as f32).round() as usize
                }
            }
            _ => 0,
        };
        idx = idx.min(w.len() - 1);
        let text = |k: usize| &stored[w[k].0..w[k].1];
        if reduced(text(idx)) {
            // Move to the nearest full vowel.
            let mut best: Option<usize> = None;
            for d in 1..w.len() {
                for k in [idx.wrapping_sub(d), idx + d] {
                    if k < w.len() && !reduced(text(k)) && best.is_none() {
                        best = Some(k);
                    }
                }
                if best.is_some() {
                    break;
                }
            }
            idx = best.unwrap_or(idx);
        }
        levels[idx] = 1;
    }
    let mut out = String::with_capacity(stored.len() + 4);
    let mut at = 0usize;
    let last = w.len() - 1;
    for (k, &(a, b, _)) in w.iter().enumerate() {
        out.push_str(&stored[at..a].replace('.', ""));
        match levels[k] {
            1 => out.push('ˈ'),
            2 => out.push('ˌ'),
            _ => {}
        }
        let v = &stored[a..b];
        let stressed = levels[k] != 0;
        let word_final = b == stored.len();
        let v: &str = match (lang, v) {
            (Lang::EnUs | Lang::EnGb, "i") if stressed => "iː",
            (Lang::EnGb, "i") => "ɪ",
            (Lang::EnUs | Lang::EnGb, "ə") if stressed => "ʌ",
            (Lang::Hi, "ə") if levels[k] == 1 => "ʌ",
            (Lang::EnUs, "ɚ") if stressed => "ɜː",
            (Lang::EnGb, "ə") if word_final && k == last && k > 0 => "ɐ",
            (_, v) => v,
        };
        out.push_str(v);
        at = b;
    }
    out.push_str(&stored[at..].replace('.', ""));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfers_stress() {
        assert_eq!(
            with_stress("naɪ.ɹoʊbi", "naɪɹˈoʊbi", Lang::EnUs),
            "naɪɹˈoʊbi"
        );
        // Different syllable counts: nearest relative position, full vowel.
        assert_eq!(with_stress("kwɑːmeɪ", "kwˈɑːm", Lang::EnUs), "kwˈɑːmeɪ");
        assert_eq!(with_stress("bənænə", "bənˈænə", Lang::EnUs), "bənˈænə");
        assert_eq!(with_stress("kɪtaːb", "kɪtˈaːb", Lang::Hi), "kɪtˈaːb");
        assert_eq!(with_stress("ki.a", "", Lang::EnUs), "kˈiːa");
    }
}
