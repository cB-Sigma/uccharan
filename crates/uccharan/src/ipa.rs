//! Small helpers over IPA strings in the espeak-style notation.

/// Primary stress mark.
pub const PRIMARY: char = 'ˈ';
/// Secondary stress mark.
pub const SECONDARY: char = 'ˌ';

/// True for a vowel letter of the notation (base letters only).
pub fn is_vowel(c: char) -> bool {
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
            | 'ɚ'
            | 'ɛ'
            | 'ɜ'
            | 'ɪ'
            | 'ʊ'
            | 'ʌ'
            | 'ᵻ'
            | 'ɐ'
            | 'ɝ'
    )
}

/// True for a stress mark.
pub fn is_stress(c: char) -> bool {
    c == PRIMARY || c == SECONDARY
}

/// Remove all stress marks.
pub fn strip_stress(s: &str) -> String {
    s.chars().filter(|c| !is_stress(*c)).collect()
}

/// True when the string starts with a vowel (ignoring a stress mark).
pub fn starts_with_vowel(s: &str) -> bool {
    s.chars().find(|c| !is_stress(*c)).is_some_and(is_vowel)
}

/// Byte index at which a stress mark for the first syllable belongs: just
/// before the first vowel or syllabic consonant.
fn first_nucleus(s: &str) -> Option<usize> {
    s.char_indices().find(|(_, c)| is_vowel(*c)).map(|(i, _)| i)
}

/// Set the word's stress level: `b'p'` primary, `b's'` secondary, `b'0'` none.
/// Words with several marks keep their internal pattern; only the strongest
/// mark is changed.
pub fn with_stress(s: &str, level: u8) -> String {
    match level {
        b'0' => strip_stress(s),
        b's' => {
            if s.contains(PRIMARY) {
                s.replace(PRIMARY, "ˌ")
            } else if s.contains(SECONDARY) {
                s.to_string()
            } else {
                insert_mark(s, SECONDARY)
            }
        }
        _ => {
            if s.contains(PRIMARY) {
                s.to_string()
            } else if s.contains(SECONDARY) {
                s.replacen(SECONDARY, "ˈ", 1)
            } else {
                insert_mark(s, PRIMARY)
            }
        }
    }
}

fn insert_mark(s: &str, mark: char) -> String {
    match first_nucleus(s) {
        Some(i) => {
            let mut out = String::with_capacity(s.len() + 2);
            out.push_str(&s[..i]);
            out.push(mark);
            out.push_str(&s[i..]);
            out
        }
        None => s.to_string(),
    }
}

/// Syllable nuclei of a word: `(byte index of the nucleus start, stress level, text)`
/// where level is 0, 1 (primary) or 2 (secondary).
pub fn nuclei(s: &str) -> Vec<(usize, u8, &str)> {
    let mut out = Vec::new();
    let cs: Vec<(usize, char)> = s.char_indices().collect();
    let mut k = 0;
    let mut pending = 0u8;
    while k < cs.len() {
        let (i, c) = cs[k];
        if c == PRIMARY {
            pending = 1;
        } else if c == SECONDARY {
            pending = 2;
        } else if is_vowel(c) {
            let mut j = k + 1;
            while j < cs.len() && (is_vowel(cs[j].1) || cs[j].1 == 'ː' || cs[j].1 == '\u{303}') {
                // A new nucleus starts at a hiatus such as "iɪ" or "iə"+"ɪ"; keep known diphthongs together.
                let cur = &s[i..cs[j].0];
                if is_vowel(cs[j].1) && !is_diphthong_prefix(cur, cs[j].1) {
                    break;
                }
                j += 1;
            }
            let end = cs.get(j).map_or(s.len(), |x| x.0);
            out.push((i, pending, &s[i..end]));
            pending = 0;
            k = j;
            continue;
        } else if c == '\u{329}' {
            // syllabic consonant
            let start = cs[k - 1].0;
            out.push((start, pending, &s[start..i + c.len_utf8()]));
            pending = 0;
        }
        k += 1;
    }
    out
}

fn is_diphthong_prefix(cur: &str, next: char) -> bool {
    matches!(
        (cur, next),
        ("a", 'ɪ')
            | ("a", 'ʊ')
            | ("e", 'ɪ')
            | ("o", 'ʊ')
            | ("ɔ", 'ɪ')
            | ("ə", 'ʊ')
            | ("i", 'ə')
            | ("e", 'ə')
            | ("ʊ", 'ə')
            | ("aɪ", 'ə')
            | ("aʊ", 'ə')
            | ("aɪ", 'ɚ')
            | ("aʊ", 'ɚ')
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stress_levels() {
        assert_eq!(with_stress("kˈæn", b'0'), "kæn");
        assert_eq!(with_stress("kæn", b'p'), "kˈæn");
        assert_eq!(with_stress("ɐbˈaʊt", b's'), "ɐbˌaʊt");
        assert_eq!(with_stress("stɹiːt", b'p'), "stɹˈiːt");
        assert!(starts_with_vowel("ˈæpəl"));
        assert!(!starts_with_vowel("kˈɑːɹ"));
    }

    #[test]
    fn nuclei_split() {
        let n = nuclei("hˈæpɪnəs");
        assert_eq!(n.len(), 3);
        assert_eq!(n[0].1, 1);
        assert_eq!(nuclei("stˈʌdiɪŋ").len(), 3);
        assert_eq!(nuclei("fˈaɪɚ").len(), 1);
        assert_eq!(nuclei("bˈʌʔn̩").len(), 2);
    }
}
