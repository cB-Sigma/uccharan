//! Bhojpuri (`bho`) and Maithili (`mai`) in Devanagari (and Kaithi, see
//! [`crate::kaithi`]).
//!
//! The reference phonemizer has no voice for either language; Bhojpuri and
//! Maithili text given to it is read with the Hindi voice. Compatible mode
//! therefore reproduces that Hindi reading exactly. Improved mode starts from
//! uccharan's improved Hindi reading and applies what the languages' grammars
//! describe differently (Shukla 1981, *Bhojpuri Grammar*; Tiwari 1960, *The
//! Origin and Development of Bhojpuri*; Grierson, *Linguistic Survey of
//! India* V.2, 1903; Jha 1958, *The Formation of the Maithili Language*;
//! Yadav 1996, *A Reference Grammar of Maithili*; Masica 1991):
//!
//! * a vowel letter इ उ after another vowel is a vowel of its own (hiatus),
//!   not the second half of a Hindi-style diphthong: गइल ɡəil, रउआ ɾəuaː;
//! * ऐ औ are the diphthongs əi əu (Hindi ɛː ɔː);
//! * the avagraha ऽ after a word marks a pronounced final short vowel
//!   (imperatives and the 2nd person: चलऽ cələ, देखऽ deːkʰə);
//! * ड़ ढ़ are flaps ɽ ɽʱ;
//! * Maithili keeps a final short i u written ि ु (Hindi drops or
//!   lengthens them) and a word-final inherent vowel after a conjunct;
//! * the copula, auxiliaries and pronouns are unstressed;
//! * numbers keep the Hindi reading except for the words that differ.

use crate::hi::Hindi;
use crate::norm::Item;
use crate::{Lang, WordOut};

/// Words without stress in improved mode (copula, auxiliaries, pronouns,
/// postpositions, particles).
const UNSTRESSED_BHO: &[&str] = &[
    "आ",
    "उ",
    "ऊ",
    "ई",
    "ए",
    "ओ",
    "क",
    "का",
    "कि",
    "की",
    "के",
    "को",
    "खातिर",
    "जे",
    "जौन",
    "त",
    "तऽ",
    "तू",
    "तोहार",
    "न",
    "ना",
    "नू",
    "पर",
    "बा",
    "बाटे",
    "बाड़न",
    "बाड़ी",
    "बाड़ू",
    "बाड़े",
    "बानी",
    "भी",
    "में",
    "रहल",
    "रहे",
    "ले",
    "से",
    "हम",
    "हमार",
    "हवे",
    "ह",
    "हऽ",
    "हउवे",
    "हो",
];

const UNSTRESSED_MAI: &[&str] = &[
    "आ",
    "ई",
    "ओ",
    "क",
    "कि",
    "के",
    "केँ",
    "को",
    "छथि",
    "छल",
    "छलाह",
    "छलै",
    "छी",
    "छै",
    "छैक",
    "जे",
    "त",
    "तँ",
    "तोहर",
    "नहि",
    "पर",
    "मे",
    "में",
    "सँ",
    "से",
    "हम",
    "हमर",
    "अछि",
    "ओ",
    "भेल",
    "छथिन",
    "अहाँ",
];

/// Number words read differently from Hindi (improved mode).
const NUMBERS_BHO: &[(&str, &str)] = &[
    ("दो", "दू"),
    ("छह", "छव"),
    ("छः", "छव"),
    ("ग्यारह", "एगारह"),
    ("पहला", "पहिला"),
    ("पहली", "पहिला"),
    ("दूसरा", "दूसर"),
    ("दूसरी", "दूसर"),
    ("तीसरा", "तीसर"),
    ("तीसरी", "तीसर"),
    ("चौथा", "चउथा"),
    ("चौथी", "चउथा"),
];

const NUMBERS_MAI: &[(&str, &str)] = &[
    ("दो", "दू"),
    ("छह", "छह"),
    ("ग्यारह", "एगारह"),
    ("पहला", "पहिल"),
    ("पहली", "पहिल"),
    ("दूसरा", "दोसर"),
    ("दूसरी", "दोसर"),
    ("तीसरा", "तेसर"),
    ("तीसरी", "तेसर"),
    ("चौथा", "चारिम"),
    ("चौथी", "चारिम"),
];

fn is_vowel_ipa(c: char) -> bool {
    "aeiouəɐɑɛɪɔʊʌæɜ".contains(c)
}

/// Endings of the Bhojpuri past (-ल-) forms.
const PAST_ENDINGS: &[&str] = &[
    "ल",
    "ला",
    "ली",
    "ले",
    "लें",
    "लीं",
    "लन",
    "लस",
    "लू",
    "लसि",
    "लनि",
    "लऽ",
    "नी",
    "लाह",
    "लक",
    "लथि",
    "लहुँ",
    "लीह",
];

/// The stem of a past form whose stem is a bare CəC syllable pair (रहलन,
/// देखलस, पढ़ला): the inherent vowel before ल is said.
fn past_stem(w: &str) -> Option<&str> {
    // Plural pronouns in -नी.
    if matches!(w, "हमनी" | "तोहनी" | "ओहनी" | "एहनी" | "उहनी" | "इहनी")
    {
        return None;
    }
    for e in PAST_ENDINGS {
        if let Some(stem) = w.strip_suffix(e) {
            let cs: Vec<char> = stem.chars().filter(|&c| c != '़').collect();
            let cons = |c: char| ('क'..='ह').contains(&c);
            // Two or three letters, all consonants or a long vowel sign after the first.
            let sign = |c: char| matches!(c, 'ा' | 'े' | 'ो' | 'ी' | 'ू' | 'ि' | 'ु');
            let ok = match cs.as_slice() {
                [a, b] => cons(*a) && cons(*b),
                [a, v, b] => cons(*a) && (sign(*v) || cons(*v)) && cons(*b),
                [a, v, b, c] => cons(*a) && sign(*v) && cons(*b) && cons(*c),
                _ => false,
            };
            if ok {
                return Some(stem);
            }
        }
    }
    None
}

/// Improved reading of one Devanagari word.
pub fn word(hi: &Hindi<'_>, w: &str, lang: Lang) -> String {
    let maithili = lang == Lang::Mai;
    let (stem, avagraha) = match w.strip_suffix('ऽ') {
        Some(s) if !s.is_empty() => (s, true),
        _ => (w, false),
    };
    // Maithili short e and o (ऎ ॆ ऒ ॊ), as in Wiktionary's spellings.
    let short_eo = stem.contains(['ऎ', 'ॆ', 'ऒ', 'ॊ']);
    let respelled;
    let stem = if short_eo {
        respelled = stem
            .replace('ऎ', "ए")
            .replace('ॆ', "े")
            .replace('ऒ', "ओ")
            .replace('ॊ', "ो");
        respelled.as_str()
    } else {
        stem
    };
    let mut ipa = match past_stem(stem) {
        Some(st) if hi.lookup(stem).is_none() => hi.word_seam(stem, st),
        _ => hi.word(stem, false),
    };
    if ipa.is_empty() {
        return ipa;
    }
    // इ उ after a vowel: hiatus, not a diphthong.
    let has_iu = stem.chars().skip(1).any(|c| matches!(c, 'इ' | 'उ'));
    if has_iu {
        let mut out = String::with_capacity(ipa.len());
        let mut prev_vowel = false;
        for c in ipa.chars() {
            let c2 = match c {
                'ɪ' if prev_vowel => 'i',
                'ʊ' if prev_vowel => 'u',
                _ => c,
            };
            out.push(c2);
            if c2 == 'ː' || c2 == 'ˈ' || c2 == 'ˌ' {
                continue;
            }
            prev_vowel = is_vowel_ipa(c2);
        }
        ipa = out;
    }
    if short_eo {
        ipa = ipa.replacen("eː", "e", 1).replacen("oː", "o", 1);
    }
    if stem.contains(['ऐ', 'ै']) {
        ipa = ipa.replace("ɛː", "əi");
    }
    if stem.contains(['औ', 'ौ']) {
        ipa = ipa.replace("ɔː", "əu");
    }
    if maithili {
        // A final short i or u written ि ु is said (Hindi lengthens or drops it).
        let last = stem.chars().last();
        if last == Some('ि') && !ipa.ends_with('ɪ') && !ipa.ends_with('i') {
            ipa.push('ɪ');
        } else if last == Some('ु') && !ipa.ends_with('ʊ') && !ipa.ends_with('u') {
            ipa.push('ʊ');
        }
    }
    if avagraha {
        // The written final short vowel is said.
        let ends_vowel = ipa
            .chars()
            .last()
            .is_some_and(|c| is_vowel_ipa(c) || c == 'ː');
        if !ends_vowel {
            ipa.push('ə');
        }
    }
    ipa
}

/// Rewrite Hindi number words left by the number reader (improved mode).
pub fn adapt_numbers(items: &mut [Item], lang: Lang) {
    let table = if lang == Lang::Mai {
        NUMBERS_MAI
    } else {
        NUMBERS_BHO
    };
    for it in items {
        if let Some((_, to)) = table.iter().find(|(from, _)| *from == it.text) {
            it.text = (*to).to_string();
        }
    }
}

/// Phonemes for a run of Devanagari items of a Bhojpuri or Maithili request.
pub fn span(
    hi: &Hindi<'_>,
    items: &[Item],
    clause_final: bool,
    compat: bool,
    lang: Lang,
) -> Vec<WordOut> {
    let mut out = hi.span(items, clause_final, compat);
    for w in &mut out {
        w.lang = lang;
    }
    if compat {
        return out;
    }
    let unstressed = if lang == Lang::Mai {
        UNSTRESSED_MAI
    } else {
        UNSTRESSED_BHO
    };
    for (w, it) in out.iter_mut().zip(items) {
        let mut ipa = word(hi, &it.text, lang);
        if unstressed.contains(&it.text.as_str()) {
            ipa = crate::ipa::strip_stress(&ipa);
        }
        w.ipa = ipa;
    }
    out
}

/// Bhojpuri and Maithili texts often write the avagraha as a Latin `s`
/// right after a Devanagari letter (`हs`, `सs`); restore it.
pub fn avagraha_s(text: &str) -> Option<String> {
    let cs: Vec<char> = text.chars().collect();
    let hit = |i: usize| {
        cs[i] == 's'
            && i > 0
            && ('\u{0900}'..='\u{097f}').contains(&cs[i - 1])
            && cs.get(i + 1).is_none_or(|c| !c.is_alphanumeric())
    };
    if !(0..cs.len()).any(hit) {
        return None;
    }
    Some(
        (0..cs.len())
            .map(|i| if hit(i) { 'ऽ' } else { cs[i] })
            .collect(),
    )
}

/// Common romanised Bhojpuri spellings (as typed in chat and on social
/// media) and their Devanagari, for the experimental romanised path.
const ROMAN_BHO: &[(&str, &str)] = &[
    ("aail", "आइल"),
    ("aawat", "आवत"),
    ("aur", "अउर"),
    ("auri", "अउरी"),
    ("ba", "बा"),
    ("baa", "बा"),
    ("baade", "बाड़े"),
    ("baadi", "बाड़ी"),
    ("baani", "बानी"),
    ("baate", "बाटे"),
    ("babuji", "बाबूजी"),
    ("bade", "बाड़े"),
    ("badi", "बाड़ी"),
    ("bahut", "बहुत"),
    ("bani", "बानी"),
    ("bate", "बाटे"),
    ("bhail", "भइल"),
    ("bhaiya", "भइया"),
    ("bhojpuri", "भोजपुरी"),
    ("bolela", "बोलेला"),
    ("boleni", "बोलेनी"),
    ("chal", "चल"),
    ("chala", "चलऽ"),
    ("dekha", "देखऽ"),
    ("dugo", "दूगो"),
    ("ego", "एगो"),
    ("ehi", "एही"),
    ("ekar", "एकर"),
    ("gaanv", "गाँव"),
    ("gail", "गइल"),
    ("gaon", "गाँव"),
    ("gayil", "गइल"),
    ("ghar", "घर"),
    ("ha", "ह"),
    ("ham", "हम"),
    ("hamaar", "हमार"),
    ("hamar", "हमार"),
    ("hamni", "हमनी"),
    ("hawe", "हवे"),
    ("hum", "हम"),
    ("ihan", "इहाँ"),
    ("jaat", "जात"),
    ("jaib", "जाइब"),
    ("jaun", "जउन"),
    ("ji", "जी"),
    ("ka", "का"),
    ("kaa", "का"),
    ("kahan", "कहाँ"),
    ("kahat", "कहत"),
    ("kahe", "काहे"),
    ("kail", "कइल"),
    ("kaisan", "कइसन"),
    ("kaise", "कइसे"),
    ("karab", "करब"),
    ("karat", "करत"),
    ("kauno", "कउनो"),
    ("ke", "के"),
    ("khana", "खाना"),
    ("kuch", "कुछ"),
    ("kuchh", "कुछ"),
    ("laika", "लइका"),
    ("laiki", "लइकी"),
    ("mai", "माई"),
    ("me", "में"),
    ("na", "ना"),
    ("naikhe", "नइखे"),
    ("neek", "नीक"),
    ("nik", "नीक"),
    ("ohi", "ओही"),
    ("okar", "ओकर"),
    ("parnam", "परनाम"),
    ("rahal", "रहल"),
    ("rahe", "रहे"),
    ("raua", "रउआ"),
    ("rauaa", "रउआ"),
    ("rauwa", "रउवा"),
    ("sabh", "सभ"),
    ("sabhe", "सभे"),
    ("se", "से"),
    ("theek", "ठीक"),
    ("thik", "ठीक"),
    ("tohaar", "तोहार"),
    ("tohar", "तोहार"),
    ("tu", "तू"),
    ("uhan", "उहाँ"),
];

/// Romanised Bhojpuri: rewrite listed Latin words as Devanagari (so that the
/// romanised-Hindi classifier then sees a Devanagari-leaning clause).
pub fn rewrite_roman(items: &mut [Item]) {
    for it in items {
        let w = it.text.to_lowercase();
        if let Ok(k) = ROMAN_BHO.binary_search_by(|p| p.0.cmp(w.as_str())) {
            it.text = ROMAN_BHO[k].1.to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bhojpuri_words() {
        let hi = Hindi::new(None);
        let w = |s: &str| crate::ipa::strip_stress(&word(&hi, s, Lang::Bho));
        assert_eq!(w("गइल"), "ɡʌil");
        assert!(w("चलऽ").ends_with('ə'));
        assert!(w("ऐसन").starts_with("əi"));
        assert!(w("बाड़े").contains('ɽ'));
    }
}
