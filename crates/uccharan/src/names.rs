//! Improved mode: personal names of Indian origin written in Latin script
//! inside English text ("Venkatesh", "Chaudhary", "Srinivasan").
//!
//! English letter-to-sound rules read these badly (the reference reads
//! "Gaurav" as *gav*, "Raghavan" with English vowels). When a capitalised word
//! is outside the English lexicon and looks like an Indian name, it is
//! transliterated to Devanagari with the romanised-Hindi rules, pronounced by
//! the Hindi front end, and the result is mapped to the nearest English
//! phonemes, which is how such names are usually said in Indian and
//! international English.

use crate::hi;

/// Score of a word under the name-origin model: the mean log-odds (×4) of
/// its letter trigrams for "romanised Indian name" against "English word".
/// The model is a sorted table of `(3 trigram bytes, i8 log-odds)` built by
/// `tools/lexgen` from a hand-written list of Indian names and CMUdict's word
/// list; trigrams it does not hold count as English-like (−4).
pub fn indic_score(model: &[u8], word: &str) -> i32 {
    let w = word.to_ascii_lowercase();
    if model.is_empty() || w.len() < 3 || !w.bytes().all(|b| b.is_ascii_lowercase()) {
        return i32::MIN;
    }
    let padded: Vec<u8> = b"^^"
        .iter()
        .copied()
        .chain(w.bytes())
        .chain(*b"$")
        .collect();
    let n = model.len() / 4;
    let (mut sum, mut k) = (0i32, 0i32);
    for t in padded.windows(3) {
        let (mut lo, mut hi) = (0usize, n);
        let mut v = -4i32;
        while lo < hi {
            let mid = (lo + hi) / 2;
            match model[mid * 4..mid * 4 + 3].cmp(t) {
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
                std::cmp::Ordering::Equal => {
                    v = model[mid * 4 + 3] as i8 as i32;
                    break;
                }
            }
        }
        sum += v;
        k += 1;
    }
    sum * 10 / k.max(1)
}

/// Mean log-odds (×40) above which a capitalised word outside the English
/// lexicon is treated as an Indian name.
pub const INDIC_THRESHOLD: i32 = 8;

/// Build the model table from Indian names and English words.
pub fn train_model(indian: &[String], english: &[String]) -> Vec<u8> {
    use std::collections::HashMap;
    let count = |words: &[String]| {
        let mut m: HashMap<[u8; 3], u32> = HashMap::new();
        let mut total = 0u32;
        for w in words {
            let padded: Vec<u8> = b"^^"
                .iter()
                .copied()
                .chain(w.to_ascii_lowercase().bytes())
                .chain(*b"$")
                .collect();
            for t in padded.windows(3) {
                *m.entry([t[0], t[1], t[2]]).or_insert(0) += 1;
                total += 1;
            }
        }
        (m, total.max(1))
    };
    let (ci, ni) = count(indian);
    let (ce, ne) = count(english);
    let mut out: Vec<([u8; 3], i8)> = ci
        .iter()
        .map(|(t, &a)| {
            let b = *ce.get(t).unwrap_or(&0);
            let lo = ((a as f64 + 0.5) / ni as f64).ln() - ((b as f64 + 0.5) / ne as f64).ln();
            (*t, (lo * 4.0).round().clamp(-127.0, 127.0) as i8)
        })
        .collect();
    out.sort();
    out.into_iter()
        .flat_map(|(t, v)| [t[0], t[1], t[2], v as u8])
        .collect()
}

/// Hindi phonemes (improved-mode notation) → English phonemes in the
/// espeak-style notation of the English front end.
pub fn hindi_to_english(ipa: &str) -> String {
    let mut s: String = ipa.replace(['\u{303}', 'ʰ', 'ʱ'], "");
    for (a, b) in [
        ("ʈ", "t"),
        ("ɖ", "d"),
        ("ɳ", "n"),
        ("ɽ", "ɹ"),
        ("ʂ", "ʃ"),
        ("ɲ", "n"),
        ("ʋ", "v"),
        ("c", "tʃ"),
        ("ɟ", "dʒ"),
        ("ɾ", "ɹ"),
        ("r", "ɹ"),
        ("x", "k"),
        ("ɣ", "ɡ"),
        ("q", "k"),
        ("aː", "ɑː"),
        ("eː", "eɪ"),
        ("oː", "oʊ"),
        ("ɛː", "ɛ"),
        ("ɔː", "ɔː"),
        ("iː", "iː"),
        ("uː", "uː"),
    ] {
        s = s.replace(a, b);
    }
    const V: &str = "aeiouɑæəɛɪʊʌɔ";
    let cs: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    for (i, &c) in cs.iter().enumerate() {
        let prev = out.chars().last();
        match c {
            // English has no geminates: one consonant for two.
            'ː' if prev.is_some_and(|p| !V.contains(p)) => {}
            _ if prev == Some(c) && !V.contains(c) && c != 'ː' => {}
            // v after a consonant is the glide of "Ashwin", "Bhardwaj".
            'v' if prev.is_some_and(|p| "sʃdtkɡ".contains(p)) => out.push('w'),
            // A stressed short a: [ɑː] in an open syllable ("Yadav"), [ʌ] in a closed one.
            'ə' | 'ʌ' if prev == Some('ˈ') => {
                let open = cs.get(i + 1).is_some_and(|n| !V.contains(*n))
                    && cs
                        .get(i + 2)
                        .is_some_and(|n| V.contains(*n) || *n == 'ˈ' || *n == 'ˌ');
                // "ar" before a consonant is the long [ɑːɹ] of "Siddharth", "Bhardwaj"
                let open = open
                    || (cs.get(i + 1) == Some(&'ɹ')
                        && cs.get(i + 2).is_some_and(|n| !V.contains(*n)));
                out.push_str(if open { "ɑː" } else { "ʌ" });
            }
            _ => out.push(c),
        }
    }
    // A final long a is short in English names ("Ananya" -> …jə).
    if out.ends_with("ɑː") && out.chars().filter(|c| "aeiouɑæəɛɪʊʌɔ".contains(*c)).count() > 2
    {
        out.truncate(out.len() - "ɑː".len());
        out.push('ə');
    }
    out
}

/// Spelling habits of romanised names that the romanised-Hindi rules do not
/// cover ("-ay" is a diphthong, not a + y).
fn respell(word: &str) -> String {
    let mut w = word.to_ascii_lowercase();
    if let Some(stem) = w.strip_suffix("ay") {
        w = format!("{stem}e");
    }
    // a final consonant + y is a vowel ("Murthy")
    if w.len() > 2 && w.ends_with('y') && !w[..w.len() - 1].ends_with(|c: char| "aeiou".contains(c))
    {
        w.pop();
        w.push('i');
    }
    w.replace("ao", "aa")
}

/// English phonemes for an Indian name, or `None` when the word does not
/// look like one.
pub fn indian_name(model: &[u8], word: &str, hindi: &hi::Hindi<'_>) -> Option<String> {
    if indic_score(model, word) < INDIC_THRESHOLD {
        return None;
    }
    let deva = hi::roman::transliterate(&respell(word));
    let ipa = hindi.name(&deva);
    (!ipa.is_empty()).then(|| hindi_to_english(&ipa))
}
