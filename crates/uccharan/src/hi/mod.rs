//! Hindi (Devanagari).
//!
//! Hindi spelling is close to phonemic; the work is in (1) building the
//! underlying sound string with an inherent vowel after every bare consonant,
//! (2) weight-based stress, (3) inherent-vowel ("schwa") deletion, and
//! (4) a few allophonic choices. The compatible mode reproduces the
//! conventions espeak-trained Hindi voices learned; the improved mode fixes
//! the known oddities (see docs/improved-mode.md).

pub mod roman;

use crate::data::Data;
use crate::norm::Item;
use crate::{Lang, WordOut};

/// One underlying sound.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Ph {
    /// IPA symbol(s) without length or nasality.
    s: &'static str,
    vowel: bool,
    /// An inherent vowel (schwa), as opposed to a written vowel.
    schwa: bool,
    long: bool,
    nasal: bool,
    /// 0 none, 1 primary, 2 secondary.
    stress: u8,
    /// The consonant was written with a virama before it (part of a conjunct).
    conjunct: bool,
    /// Independent vowel letter (not a matra).
    independent: bool,
}

impl Ph {
    fn c(s: &'static str) -> Ph {
        Ph {
            s,
            vowel: false,
            schwa: false,
            long: false,
            nasal: false,
            stress: 0,
            conjunct: false,
            independent: false,
        }
    }
    fn v(s: &'static str, long: bool) -> Ph {
        Ph {
            s,
            vowel: true,
            schwa: false,
            long,
            nasal: false,
            stress: 0,
            conjunct: false,
            independent: false,
        }
    }
    fn schwa() -> Ph {
        Ph {
            s: "ə",
            vowel: true,
            schwa: true,
            long: false,
            nasal: false,
            stress: 0,
            conjunct: false,
            independent: false,
        }
    }
}

fn consonant(c: char) -> Option<&'static str> {
    Some(match c {
        'क' => "k",
        'ख' => "kʰ",
        'ग' => "ɡ",
        'घ' => "ɡʰ",
        'ङ' => "ŋ",
        'च' => "c",
        'छ' => "cʰ",
        'ज' => "ɟ",
        'झ' => "ɟʰ",
        'ञ' => "ɲ",
        'ट' => "ʈ",
        'ठ' => "ʈʰ",
        'ड' => "ɖ",
        'ढ' => "ɖʰ",
        'ण' => "ɳ",
        'त' => "t",
        'थ' => "tʰ",
        'द' => "d",
        'ध' => "dʰ",
        'न' => "n",
        'प' => "p",
        'फ' => "pʰ",
        'ब' => "b",
        'भ' => "bʰ",
        'म' => "m",
        'य' => "j",
        'र' => "ɾ",
        'ल' => "l",
        'व' => "ʋ",
        'श' => "ʃ",
        'ष' => "ʂ",
        'स' => "s",
        'ह' => "h",
        'ळ' => "ɭ",
        '\u{0958}' => "q",
        '\u{0959}' => "x",
        '\u{095a}' => "ɣ",
        '\u{095b}' => "z",
        '\u{095c}' => "ɽ",
        '\u{095d}' => "ɽʰ",
        '\u{095e}' => "f",
        '\u{095f}' => "j",
        _ => return None,
    })
}

fn with_nukta(base: &'static str) -> &'static str {
    match base {
        "k" => "q",
        "kʰ" => "x",
        "ɡ" => "ɣ",
        "ɟ" => "z",
        "ɖ" => "ɽ",
        "ɖʰ" => "ɽʰ",
        "pʰ" => "f",
        other => other,
    }
}

/// `(symbol, long)` for vowel letters and signs.
fn vowel(c: char) -> Option<(&'static str, bool, bool)> {
    // (symbol, long, independent)
    Some(match c {
        'अ' => ("ə", false, true),
        'आ' => ("a", true, true),
        'इ' => ("ɪ", false, true),
        'ई' => ("i", true, true),
        'उ' => ("ʊ", false, true),
        'ऊ' => ("u", true, true),
        'ए' => ("e", true, true),
        'ऐ' => ("ɛ", true, true),
        'ओ' => ("o", true, true),
        'औ' => ("ɔ", true, true),
        'ऑ' => ("ɔ", false, true),
        'ऍ' => ("ɛ", false, true),
        'ा' => ("a", true, false),
        'ि' => ("ɪ", false, false),
        'ी' => ("i", true, false),
        'ु' => ("ʊ", false, false),
        'ू' => ("u", true, false),
        'े' => ("e", true, false),
        'ै' => ("ɛ", true, false),
        'ो' => ("o", true, false),
        'ौ' => ("ɔ", true, false),
        'ॉ' => ("ɔ", false, false),
        'ॅ' => ("ɛ", false, false),
        _ => return None,
    })
}

fn homorganic(next: Option<char>) -> &'static str {
    match next {
        Some('क' | 'ख' | 'ग' | 'घ' | '\u{0958}' | '\u{0959}' | '\u{095a}') => "ŋ",
        Some('च' | 'छ' | 'ज' | 'झ' | 'य') => "ɲ",
        Some('ट' | 'ठ' | 'ड' | 'ढ' | '\u{095c}' | '\u{095d}') => "ɳ",
        Some('प' | 'फ' | 'ब' | 'भ' | 'म' | '\u{095e}') => "m",
        _ => "n",
    }
}

/// Underlying sounds of a Devanagari word.
fn parse(word: &str, improved: bool) -> Vec<Ph> {
    let cs: Vec<char> = word
        .chars()
        .filter(|c| !matches!(c, '\u{200c}' | '\u{200d}'))
        .collect();
    let mut out: Vec<Ph> = Vec::with_capacity(cs.len() + 4);
    let mut i = 0;
    let mut after_virama = false;
    while i < cs.len() {
        let c = cs[i];
        if let Some(mut s) = consonant(c) {
            i += 1;
            let mut gy = false;
            if cs.get(i) == Some(&'\u{093c}') {
                s = with_nukta(s);
                i += 1;
            }
            // ज्ञ is read "gy"; क्ष is "ksh" with a palatal sibilant.
            if s == "ɲ" && after_virama && out.last().is_some_and(|p| p.s == "ɟ") {
                out.last_mut().unwrap().s = "ɡ";
                s = "j";
                gy = true;
            }
            if s == "ʂ" && after_virama && out.last().is_some_and(|p| p.s == "k") {
                s = "ʃ";
            }
            let mut p = Ph::c(s);
            p.conjunct = after_virama;
            // `nasal` on a consonant marks the glide of ज्ञ, which has no release vowel.
            p.nasal = gy;
            out.push(p);
            after_virama = false;
            match cs.get(i) {
                Some('\u{094d}') => {
                    after_virama = true;
                    i += 1;
                }
                Some(&m) if vowel(m).is_some_and(|v| !v.2) => {
                    let (vs, long, _) = vowel(m).unwrap();
                    out.push(Ph::v(vs, long));
                    i += 1;
                }
                Some('ृ') => {
                    out.push(Ph::c("ɾ"));
                    out.last_mut().unwrap().conjunct = true;
                    out.push(Ph::v("ɪ", false));
                    i += 1;
                }
                _ => out.push(Ph::schwa()),
            }
            continue;
        }
        after_virama = false;
        if let Some((vs, long, indep)) = vowel(c) {
            let mut p = Ph::v(vs, long && !(indep && vs == "u" && !improved));
            p.independent = indep;
            if vs == "ə" {
                p.schwa = true;
            }
            out.push(p);
            i += 1;
            continue;
        }
        match c {
            'ऋ' => {
                out.push(Ph::c("ɾ"));
                out.push(Ph::v("ɪ", false));
            }
            'ं' | 'ँ' => {
                let next = cs.get(i + 1).copied().filter(|n| consonant(*n).is_some());
                let is_final = i + 1 == cs.len();
                let Some(v) = out.last().filter(|p| p.vowel).cloned() else {
                    i += 1;
                    continue;
                };
                let after_long_a =
                    out.len() >= 2 && out[out.len() - 2].vowel && out[out.len() - 2].s == "a";
                if improved {
                    // Nasal vowel at the end of a word or for chandrabindu; a homorganic
                    // nasal consonant before a stop otherwise.
                    if c == 'ं' && !is_final && next.is_some() {
                        // Before ह the nasal is velar (सिंह).
                        out.push(Ph::c(if next == Some('ह') {
                            "ŋ"
                        } else {
                            homorganic(next)
                        }));
                    } else {
                        out.last_mut().unwrap().nasal = true;
                    }
                } else if c == 'ं' {
                    if v.schwa {
                        out.push(Ph::c(homorganic(next)));
                    } else if v.independent && matches!(v.s, "i" | "ʊ") {
                        // ईं → iːn, उं → ʊ + nasal consonant
                        out.push(Ph::c(homorganic(next)));
                    } else if v.independent && v.s == "o" && is_final && after_long_a {
                        // plural oblique …आओं
                        *out.last_mut().unwrap() = Ph::c("w");
                    } else {
                        out.last_mut().unwrap().nasal = true;
                    }
                } else if v.schwa {
                    out.push(Ph::c("n"));
                } else if is_final {
                    if v.independent && v.s == "e" && after_long_a {
                        *out.last_mut().unwrap() = Ph::c("j");
                    } else {
                        out.last_mut().unwrap().nasal = true;
                    }
                } else if v.independent && v.s == "e" {
                    out.last_mut().unwrap().nasal = true;
                } else {
                    if !v.independent {
                        out.last_mut().unwrap().nasal = true;
                    }
                    out.push(Ph::c("n"));
                }
            }
            'ः' => out.push(Ph::c("h")),
            _ => {}
        }
        i += 1;
    }
    out
}

/// Geminates and the lengthening of voiceless stops before a conjunct y.
fn geminate(ph: &mut Vec<Ph>, improved: bool) {
    let mut i = 0;
    while i + 1 < ph.len() {
        if !ph[i].vowel && !ph[i + 1].vowel && ph[i + 1].conjunct {
            let (a, b) = (ph[i].s, ph[i + 1].s);
            let merged: Option<&'static str> = if improved {
                // One long consonant for every doubled or unaspirated + aspirated pair.
                if a == b || (b.strip_suffix('ʰ') == Some(a)) {
                    Some(b)
                } else {
                    None
                }
            } else {
                match (a, b) {
                    ("k", "k")
                    | ("ɡ", "ɡ")
                    | ("c", "c")
                    | ("ɟ", "ɟ")
                    | ("t", "t")
                    | ("d", "d")
                    | ("p", "p") => Some(b),
                    ("t", "tʰ") | ("d", "dʰ") => Some(b),
                    _ => None,
                }
            };
            if let Some(m) = merged {
                ph[i].s = m;
                ph[i].long = true;
                ph.remove(i + 1);
                continue;
            }
            if !improved {
                // Reference habit: unaspirated + aspirated retroflex/palatal pairs are written twice aspirated.
                if matches!((a, b), ("c", "cʰ") | ("ʈ", "ʈʰ") | ("ɖ", "ɖʰ")) {
                    ph[i].s = b;
                }
                if b == "j" && matches!(a, "k" | "c" | "ʈ" | "t" | "p") && !ph[i].long {
                    ph[i].long = true;
                }
            }
        }
        i += 1;
    }
}

fn vowel_indices(ph: &[Ph]) -> Vec<usize> {
    (0..ph.len()).filter(|&i| ph[i].vowel).collect()
}

/// Weight-based stress on the underlying form.
fn assign_stress(ph: &mut [Ph]) {
    let vs = vowel_indices(ph);
    if vs.is_empty() {
        return;
    }
    let n = vs.len();
    let weight = |k: usize| -> u8 {
        let i = vs[k];
        let mut w = 0u8;
        if ph[i].long || ph[i].nasal {
            w += 1;
        }
        let end = vs.get(k + 1).copied().unwrap_or(ph.len());
        let cons = &ph[i + 1..end];
        let closed =
            cons.len() >= 2 || cons.iter().any(|c| c.long) || (k + 1 == n && !cons.is_empty());
        if closed {
            w += 1;
        }
        w
    };
    let mut primary = 0usize;
    if n > 1 {
        let mut best = 0u8;
        for k in 0..n - 1 {
            let w = weight(k);
            if w > best || (w == best && w > 0) {
                best = w;
                primary = k;
            }
        }
        if weight(n - 1) >= 2 && best < 2 {
            primary = n - 1;
        }
    }
    ph[vs[primary]].stress = 1;
    for k in 0..n {
        if k == primary {
            continue;
        }
        let prev_stressed = k > 0 && ph[vs[k - 1]].stress != 0;
        let next_stressed = k + 1 < n && ph[vs[k + 1]].stress != 0;
        if !prev_stressed && !next_stressed {
            ph[vs[k]].stress = 2;
        }
    }
}

/// Inherent-vowel deletion, left to right.
fn delete_schwas(ph: &mut Vec<Ph>, improved: bool, boundary: Option<usize>) {
    let total_vowels = ph.iter().filter(|p| p.vowel).count();
    let mut i = 0;
    while i < ph.len() {
        if !(ph[i].schwa && ph[i].stress != 1) || (ph[i].independent) {
            i += 1;
            continue;
        }
        let is_final = i + 1 == ph.len();
        if is_final {
            // Word-final: deleted unless it is the only vowel, or it releases a
            // conjunct that ends in a sonorant (मित्र, सत्य).
            let release = i >= 2 && ph[i - 1].conjunct && final_release(&ph[i - 2], &ph[i - 1]);
            let y_glide = i >= 2 && ph[i - 1].s == "j" && !ph[i - 1].conjunct && ph[i - 2].schwa;
            if total_vowels > 1 && !release && !y_glide {
                ph.remove(i);
                continue;
            }
            i += 1;
            continue;
        }
        let prev_h = i > 0 && matches!(ph[i - 1].s, "h" | "ɽʰ");
        if prev_h {
            i += 1;
            continue;
        }
        // Medial rule: V C ə C V[written] → V C C V.
        let vc = i >= 2 && ph[i - 2].vowel && !ph[i - 1].vowel;
        let next_vowel = ph[i + 1..].iter().find(|p| p.vowel);
        let next_is_schwa = next_vowel.is_some_and(|p| p.schwa);
        let last_before_final_consonant = next_vowel.is_none();
        // One consonant, or a single long consonant, must follow before the next vowel for the
        // result to stay pronounceable.
        let following_cons = ph[i + 1..].iter().take_while(|p| !p.vowel).count();
        let _ = following_cons;
        let mut delete = vc && !next_is_schwa && !last_before_final_consonant;
        if improved {
            // Morpheme-aware: also delete before a suffix boundary, never across it.
            if let Some(b) = boundary {
                if i + 1 == b && i >= 2 && ph[i - 2].vowel && !ph[i - 1].vowel {
                    delete = true;
                }
            }
        }
        if delete {
            ph.remove(i);
            continue;
        }
        i += 1;
    }
}

/// Inherent-vowel deletion for the improved mode: the standard account of
/// Hindi (final deletion, then medial deletion in a V C _ C V frame scanning
/// from the end of the word), made aware of morpheme seams and of the
/// learned words that keep the vowel before a final short i or u.
/// Delete only a word-final inherent vowel (names read in English).
fn final_schwa_only(ph: &mut Vec<Ph>) {
    let vowels = ph.iter().filter(|p| p.vowel).count();
    if ph.last().is_some_and(|l| l.schwa && !l.independent) && vowels > 1 {
        ph.pop();
    }
}

fn delete_schwas_standard(ph: &mut Vec<Ph>, boundary: Option<usize>) {
    let total_vowels = ph.iter().filter(|p| p.vowel).count();
    if let Some(last) = ph.last() {
        if last.schwa && !last.independent && total_vowels > 1 {
            let i = ph.len() - 1;
            let release = i >= 2
                && ph[i - 1].conjunct
                && matches!(ph[i - 1].s, "ɾ" | "j" | "ʋ" | "ɳ")
                && !ph[i - 1].nasal;
            let n_release = i >= 2
                && ph[i - 1].conjunct
                && ph[i - 1].s == "n"
                && matches!(ph[i - 2].s, "t" | "ɡ" | "ɡʰ");
            if !release && !n_release {
                ph.pop();
            }
        }
    }
    let mut seam = boundary.filter(|&b| b < ph.len());
    let nasal_stop = |ph: &[Ph], i: usize| {
        i >= 3
            && ph[i - 3].vowel
            && matches!(ph[i - 2].s, "n" | "m" | "ŋ" | "ɲ" | "ɳ")
            && !ph[i - 1].long
            && ph[i - 1]
                .s
                .starts_with(['p', 'b', 't', 'd', 'ʈ', 'ɖ', 'c', 'ɟ', 'k', 'ɡ'])
    };
    let mut i = ph.len();
    while i > 0 {
        i -= 1;
        if !ph[i].schwa || ph[i].independent || i + 1 >= ph.len() {
            continue;
        }
        // The first vowel of the word, and of a second morpheme, stays.
        if !ph[..i].iter().any(|p| p.vowel) {
            continue;
        }
        if let Some(b) = seam {
            if i >= b && !ph[b..i].iter().any(|p| p.vowel) {
                continue;
            }
        }
        let before =
            (i >= 2 && ph[i - 2].vowel && !ph[i - 1].vowel && !ph[i - 1].long) || nasal_stop(ph, i);
        let after = i + 2 < ph.len() && !ph[i + 1].vowel && !ph[i + 1].long && ph[i + 2].vowel;
        let at_seam = seam == Some(i + 1);
        let learned =
            after && i + 3 == ph.len() && matches!(ph[i + 2].s, "ɪ" | "ʊ") && !ph[i + 2].long;
        if before && (after || at_seam) && !learned {
            ph.remove(i);
            if let Some(b) = seam {
                if b > i {
                    seam = Some(b - 1);
                }
            }
        }
    }
}

/// Does a word-final conjunct keep a short release vowel?
fn final_release(c1: &Ph, c2: &Ph) -> bool {
    match c2.s {
        // ज्ञ (gy) ends without a release.
        "j" => !c2.nasal,
        "ɾ" | "l" | "ɳ" => true,
        "n" => c1.s != "ɾ",
        "m" => !matches!(c1.s, "ɾ" | "n"),
        _ => false,
    }
}

fn render(ph: &[Ph], improved: bool) -> String {
    let mut out = String::with_capacity(ph.len() * 3);
    let n = ph.len();
    let single_letter = n == 2 && ph[1].schwa;
    for (i, p) in ph.iter().enumerate() {
        let last = i + 1 == n;
        if p.vowel {
            // Final "y" after an inherent vowel: the glide syllable is written ɛj.
            match p.stress {
                1 => out.push('ˈ'),
                2 => out.push('ˌ'),
                _ => {}
            }
            let sym: &str = if p.schwa
                && p.independent
                && !improved
                && ph.get(i + 1).is_some_and(|q| q.s == "h")
            {
                "ɛ"
            } else if p.schwa && p.independent && !improved && (i == 0 || p.stress == 1) {
                // A vowel letter अ inside a word (जंतुअन) is ə unless stressed.
                "ʌ"
            } else if p.schwa {
                if p.stress == 1 && !single_letter {
                    "ʌ"
                } else {
                    "ə"
                }
            } else {
                p.s
            };
            let sym = match (p.nasal, sym) {
                (true, "ɪ") => "i",
                (true, "ʊ") => "u",
                _ => sym,
            };
            out.push_str(sym);
            if p.nasal {
                out.push('\u{303}');
            }
            let long = match p.s {
                "i" => p.long && !p.nasal && (improved || (p.stress != 0 && !last)),
                "u" => p.long && !p.nasal && (improved || !p.independent),
                "e" => p.long,
                "a" | "ɛ" | "o" | "ɔ" => p.long && !p.nasal,
                _ => false,
            };
            if long {
                out.push('ː');
            }
        } else {
            let sym: &str = match p.s {
                "ɽ" => {
                    if improved {
                        "ɽ"
                    } else {
                        "r."
                    }
                }
                "ɽʰ" => {
                    if improved {
                        "ɽʰ"
                    } else {
                        "r.h"
                    }
                }
                "ɾ" => {
                    // Trill before a sonorant or fricative in a written conjunct.
                    let next = ph.get(i + 1);
                    if !improved
                        && next.is_some_and(|q| {
                            !q.vowel
                                && q.conjunct
                                && matches!(
                                    q.s,
                                    "n" | "m"
                                        | "j"
                                        | "l"
                                        | "ʋ"
                                        | "ʃ"
                                        | "s"
                                        | "h"
                                        | "ɳ"
                                        | "ʂ"
                                        | "z"
                                        | "f"
                                        | "ɾ"
                                        | "x"
                                        | "ɣ"
                                )
                        })
                    {
                        "r"
                    } else {
                        "ɾ"
                    }
                }
                s => s,
            };
            out.push_str(sym);
            if p.long {
                out.push('ː');
            }
        }
    }
    out
}

/// Suffixes that start a new morpheme (improved mode only): the inherent vowel
/// before them is dropped and none is dropped across the seam.
const SUFFIXES: &[&str] = &[
    "पन",
    "दार",
    "वाला",
    "वाली",
    "वाले",
    "कार",
    "गार",
    "बाज़",
    "खाना",
    "शाला",
    "सभा",
    "घर",
    "गाड़ी",
    "पति",
    "वासी",
    "वाद",
    "वादी",
    "दान",
    "नगर",
    "पुर",
    "गढ़",
    "राज",
    "लोक",
    "शास्त्र",
    "विज्ञान",
    "मंत्री",
    "कालीन",
    "जनक",
    "पूर्ण",
    "पूर्वक",
    "शील",
    "हीन",
    "रहित",
    "मय",
    "ता",
    "ती",
    "ते",
    "ना",
    "नी",
    "ने",
    "कर",
];

/// Function words: `(word, medial level, final level)` with `0` none,
/// `s` secondary, `p` primary. Written by hand from observed behaviour.
static FUNCTION_WORDS: &[(&str, u8, u8)] = &[
    ("अगर", b's', b's'),
    ("आप", b's', b's'),
    ("आपका", b's', b's'),
    ("आपकी", b's', b's'),
    ("आपके", b's', b's'),
    ("आपको", b's', b's'),
    ("इनका", b's', b's'),
    ("इन्हें", b's', b's'),
    ("इसका", b's', b's'),
    ("उनका", b's', b's'),
    ("उनकी", b's', b's'),
    ("उनके", b's', b's'),
    ("उन्हे", b's', b's'),
    ("उसका", b's', b's'),
    ("उसकी", b's', b's'),
    ("उसके", b's', b's'),
    ("और", b'0', b'0'),
    ("का", b'0', b'0'),
    ("की", b'0', b'0'),
    ("के", b'0', b'0'),
    ("को", b'0', b'0'),
    ("क्या", b'0', b'0'),
    ("तक", b's', b's'),
    ("तुझे", b's', b's'),
    ("तुम", b's', b's'),
    ("तुम्हारा", b's', b's'),
    ("तुम्हारी", b's', b's'),
    ("तुम्हारे", b's', b's'),
    ("तुम्हें", b's', b's'),
    ("तू", b'0', b'0'),
    ("तेरा", b's', b's'),
    ("था", b'0', b'0'),
    ("थी", b'0', b'0'),
    ("थीं", b'0', b'0'),
    ("थे", b'0', b'0'),
    ("बिना", b's', b's'),
    ("मुझे", b's', b's'),
    ("में", b'0', b'p'),
    ("मेरा", b's', b's'),
    ("मेरी", b's', b's'),
    ("मेरे", b's', b's'),
    ("मैं", b'0', b'0'),
    ("यदि", b's', b's'),
    ("यह", b's', b's'),
    ("ये", b'0', b'0'),
    ("रहा", b's', b's'),
    ("रही", b's', b's'),
    ("रहे", b's', b's'),
    ("वह", b's', b's'),
    ("वे", b'0', b'0'),
    ("वो", b'0', b'0'),
    ("से", b'0', b'0'),
    ("हम", b's', b's'),
    ("हमारा", b's', b's'),
    ("हमारी", b's', b's'),
    ("हमारे", b's', b's'),
    ("हमे", b's', b's'),
    ("हूँ", b'0', b'0'),
    ("है", b'0', b'0'),
    ("हैं", b'0', b'0'),
    ("हो", b'0', b'0'),
];

/// The Hindi front end.
pub struct Hindi<'a> {
    data: Option<&'a Data>,
    wiki: Option<&'a Data>,
}

impl<'a> Hindi<'a> {
    /// Create the front end; the data image holds the exceptions lexicon and
    /// the romanised-Hindi word list and may be absent.
    pub fn new(data: Option<&'a Data>) -> Self {
        Hindi { data, wiki: None }
    }

    /// Use an optional WikiPron pack (improved mode) for words outside the
    /// exceptions lexicon.
    pub fn with_wiki(mut self, wiki: Option<&'a Data>) -> Self {
        self.wiki = wiki;
        self
    }

    pub(crate) fn lookup(&self, key: &str) -> Option<String> {
        let mut s = String::new();
        self.data?.lookup(key, &mut s).then_some(s)
    }

    /// Pronunciation of one Devanagari word with its lexical stress.
    pub fn word(&self, word: &str, compat: bool) -> String {
        self.word_with(word, compat, true)
    }

    /// A romanised name read out in English: every written vowel is kept
    /// except the final inherent one ("Raghavan" keeps the medial a).
    pub fn name(&self, word: &str) -> String {
        self.word_with(word, false, false)
    }

    /// Improved reading of a word whose last morpheme starts at the inherent
    /// vowel after `stem` (Bhojpuri past forms: रह-ल-न ɾəhələn keeps that
    /// vowel, where Hindi-type deletion would drop it).
    pub fn word_seam(&self, word: &str, stem: &str) -> String {
        let mut ph = parse(word, true);
        if ph.is_empty() {
            return String::new();
        }
        geminate(&mut ph, true);
        assign_stress(&mut ph);
        let b = parse(stem, true).len().checked_sub(1);
        delete_schwas_standard(&mut ph, b);
        render(&ph, true)
    }

    fn word_with(&self, word: &str, compat: bool, medial: bool) -> String {
        let improved = !compat;
        if let Some(e) = self.lookup(word) {
            // "compat|improved" when the two differ.
            let mut it = e.split('|');
            let first = it.next().unwrap_or("");
            let second = it.next().unwrap_or(first);
            let chosen = if compat { first } else { second };
            // An empty compatible column means "use the rules".
            if !chosen.is_empty() {
                return chosen.to_string();
            }
        }
        if improved && medial {
            if let Some(w) = self.wiki {
                let mut s = String::new();
                if w.lookup(word, &mut s) {
                    let predicted = Hindi::new(None).word_with(word, false, true);
                    return crate::wiki::with_stress(&s, &predicted, Lang::Hi);
                }
            }
        }
        let mut ph = parse(word, improved);
        if ph.is_empty() {
            return String::new();
        }
        geminate(&mut ph, improved);
        // Final य after an inherent vowel: the last syllable is a glide.
        let n = ph.len();
        let y_final = n >= 4
            && ph[n - 1].schwa
            && ph[n - 2].s == "j"
            && !ph[n - 2].conjunct
            && ph[n - 3].schwa
            && !ph[n - 3].independent;
        assign_stress(&mut ph);
        if y_final && !improved {
            // …əjə → …ɛj: the glide syllable keeps its vowel, written ɛ in front of the j.
            let v = ph.last_mut().unwrap();
            v.s = "ɛ";
            v.schwa = false;
        }
        let boundary = if improved {
            suffix_boundary(word, &ph)
        } else {
            None
        };
        if improved && !medial {
            final_schwa_only(&mut ph);
        } else if improved {
            delete_schwas_standard(&mut ph, boundary);
        } else {
            delete_schwas(&mut ph, false, None);
        }
        if y_final && !improved {
            let n = ph.len();
            ph.swap(n - 1, n - 2);
        }
        render(&ph, improved)
    }

    /// Phonemes for a run of Devanagari items inside one clause.
    pub fn span(&self, items: &[Item], clause_final: bool, compat: bool) -> Vec<WordOut> {
        let n = items.len();
        items
            .iter()
            .enumerate()
            .map(|(i, it)| {
                let mut ipa = self.word(&it.text, compat);
                let mut glue = it.glue;
                // Compatible mode: number words as the reference reads digits.
                if compat && it.generated {
                    if let Some(k) = crate::norm::HI_NUM.iter().position(|w| *w == it.text) {
                        ipa = HI_NUM_REFERENCE[k].to_string();
                    } else if it.text == "सौ" {
                        // "एक सौ" is one word in the reference.
                        glue = i > 0;
                    } else if it.text == "हज़ार" || it.text == "हजार" {
                        ipa = "hˈʌɟaːɾ".to_string();
                    }
                }
                if let Ok(k) = FUNCTION_WORDS.binary_search_by(|f| f.0.cmp(it.text.as_str())) {
                    let f = FUNCTION_WORDS[k];
                    let level = if i + 1 == n && clause_final { f.2 } else { f.1 };
                    ipa = match level {
                        b'0' => crate::ipa::strip_stress(&ipa),
                        b's' => {
                            // The inherent vowel is only written ʌ under primary stress
                            // (and for a word-initial अ).
                            let keep = it.text.starts_with('अ');
                            let t = ipa.replace('ˈ', "ˌ");
                            if keep {
                                t
                            } else {
                                t.replace('ʌ', "ə")
                            }
                        }
                        _ => ipa,
                    };
                }
                // Compatible mode: before a hyphen the reference keeps the final
                // inherent vowel of a word of two or more syllables (अलग-अलग).
                if compat && items.get(i + 1).is_some_and(|x| x.hyphen) {
                    let cs: Vec<char> = it.text.chars().collect();
                    let n = cs.len();
                    let cons = |c: char| ('क'..='ह').contains(&c);
                    // …CəC at the end: the last two letters are bare consonants.
                    let last = if n >= 3 && cons(cs[n - 2]) && cs[n - 1] != 'य' {
                        cs[n - 1]
                    } else {
                        ' '
                    };
                    let vowels: Vec<(usize, char)> = ipa
                        .char_indices()
                        .filter(|(_, c)| "aeiouəɐɑɛɪɔʊʌ".contains(*c))
                        .collect();
                    if ('क'..='ह').contains(&last) && vowels.len() >= 2 {
                        let (k, _) = vowels[vowels.len() - 1];
                        let stressed = ipa[..k].ends_with(['ˈ', 'ˌ']);
                        if !stressed {
                            ipa.push('ˌ');
                        }
                        ipa.push('ə');
                    }
                }
                WordOut::new(&it.text, ipa, Lang::Hi, glue)
            })
            .collect()
    }
}

/// The reference's readings of the Hindi number words 0-99 (derived from
/// running espeak-ng 1.52.0 on the numbers 0-99), used in compatible mode.
const HI_NUM_REFERENCE: [&str; 100] = [
    "ʃˈuːnjə",
    "ˈeːk",
    "dˈoː",
    "tˈiːn",
    "cˈaːɾ",
    "pˈãc",
    "cʰˈʌh",
    "sˈaːt",
    "ˈaːʈʰ",
    "nˈɔː",
    "dˈʌs",
    "ɡjˈaːɾəh",
    "bˈaːɾəh",
    "tˈeːɾəh",
    "cˈɔːdəh",
    "pˈʌndɾəh",
    "sˈoːləh",
    "sˈʌtɾəh",
    "ʌʈʰˈaːɾəh",
    "ʊnnˈɪs",
    "bˈiːs",
    "ɪkːˈis",
    "bˈaːis",
    "tˈeːis",
    "cˈɔːbis",
    "pˈʌcːis",
    "cʰəbbˈis",
    "sətːˈaːis",
    "atʰːˈaːis",
    "ˈʊntis",
    "tˈiːs",
    "ˌɪkətˈis",
    "bˈatːis",
    "tˈɛ̃tis",
    "cˈɔ̃tis",
    "pˈɛ̃tis",
    "cʰˈʌtːis",
    "sˈɛ̃tis",
    "ˈʌɾtis",
    "ʊntˈaːlis",
    "cˈaːlis",
    "ɪktˈaːlis",
    "baɪˈaːlis",
    "tɛ̃tˈaːlis",
    "cɔːʋˈaːlis",
    "paɪntˈaːlis",
    "cʰɪjˈaːlis",
    "sɛ̃tˈaːlis",
    "ʌɾtˈaːlis",
    "ˈʊncaːs",
    "pəcˈaːs",
    "ɪkjˈaːwən",
    "bˈaːwən",
    "tˈɪɾeːpˌən",
    "cˈɔːwən",
    "pˈʌcpən",
    "cʰˈʌpːən",
    "sətˈaːwən",
    "aʈʰʈʰˈaːwən",
    "ˈʊnsəʈʰ",
    "sˈaːʈʰ",
    "ˈɪksəʈʰ",
    "bˈaːsəʈʰ",
    "tˈɪɾeːsˌəʈʰ",
    "cˈɔ̃səʈʰ",
    "pˈɛ̃səʈʰ",
    "cʰɪjˈaːsəʈʰ",
    "sˈʌr.səʈʰ",
    "ˈʌr.səʈʰ",
    "ˈʊnəhtːˌəɾ",
    "sˈʌtːəɾ",
    "ˈɪkhətːˌəɾ",
    "bˈʌhətːˌəɾ",
    "tˈɪhətːˌəɾ",
    "cɔːhˈʌtːəɾ",
    "pəchˈʌtːəɾ",
    "cʰɪhˈʌtːəɾ",
    "səthˈʌtːəɾ",
    "ʌthˈʌtːəɾ",
    "ʊnˈaːsi",
    "ˈʌssi",
    "ɪkjˈaːsi",
    "bəjˈaːsi",
    "tɪɾˈaːsi",
    "cɔːɾˈaːsi",
    "pəcˈaːsi",
    "cʰɪjˈaːsi",
    "sətːˈaːsi",
    "atʰːˈaːsi",
    "nəwˈaːsi",
    "nˈʌbːeː",
    "ɪkːjˈaːnəbˌeː",
    "bˈaːnəbˌeː",
    "tɪɾˈaːnəbˌeː",
    "cɔːɾˈaːməbˌeː",
    "pəncˈaːnəbˌeː",
    "cʰɪjˈaːnəbˌeː",
    "sətːˈaːnəbˌeː",
    "ʌtʰːˈaːnəbˌeː",
    "nɪɲjˈaːnəbˌeː",
];

/// Index in `ph` where a known suffix starts, if the word ends with one and
/// the stem is long enough to be a word.
fn suffix_boundary(word: &str, ph: &[Ph]) -> Option<usize> {
    for suf in SUFFIXES {
        if let Some(stem) = word.strip_suffix(suf) {
            if stem.chars().count() >= 3 && !stem.ends_with('\u{094d}') {
                let stem_len = parse(stem, true).len();
                if stem_len < ph.len() {
                    return Some(stem_len);
                }
            }
        }
    }
    None
}

/// Make embedded English words fit a Hindi voice.
///
/// Compatible mode keeps the British phonemes the reference emits and only
/// writes the TRAP vowel as `a`. Improved mode maps to an Indian-English
/// realisation inside the Hindi inventory.
pub fn adapt_english(words: &mut [WordOut], compat: bool) {
    for w in words {
        if compat {
            w.ipa = w.ipa.replace('æ', "a");
        } else {
            let mut s = w.ipa.clone();
            for (a, b) in [
                ("tʃ", "c"),
                ("dʒ", "ɟ"),
                ("eɪ", "eː"),
                ("əʊ", "oː"),
                ("oʊ", "oː"),
                ("iə", "ɪjə"),
                ("eə", "ɛː"),
                ("æ", "ɛ"),
                ("ɒ", "ɔ"),
                ("ɜː", "əɾ"),
                ("θ", "tʰ"),
                ("ð", "d"),
                ("t", "ʈ"),
                ("d", "ɖ"),
                ("w", "ʋ"),
                ("v", "ʋ"),
                ("ɹ", "ɾ"),
                ("ɐ", "ə"),
                ("ʒ", "ʃ"),
                ("ɑː", "aː"),
                ("ɑ", "aː"),
                ("aɪ", "aːɪ"),
                ("aʊ", "aːʊ"),
                ("ɔɪ", "ɔːɪ"),
                ("ʌ", "ə"),
                ("ɚ", "əɾ"),
                ("ᵻ", "ɪ"),
                ("ŋɡ", "ŋɡ"),
            ] {
                s = s.replace(a, b);
            }
            // The dental stops written for θ ð must stay dental.
            w.ipa = s.replace("ʈʰ", "tʰ");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hi(w: &str) -> String {
        Hindi::new(None).word(w, true)
    }

    macro_rules! same {
        ($a:expr, $b:expr) => {
            assert_eq!(crate::profile::nfd(&$a), crate::profile::nfd($b))
        };
    }

    #[test]
    fn function_words_sorted() {
        assert!(FUNCTION_WORDS.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn schwa_deletion() {
        same!(hi("कमल"), "kˈʌməl");
        same!(hi("कमला"), "kˈʌmlˌaː");
        same!(hi("समझ"), "sˈʌməɟʰ");
        same!(hi("समझना"), "sˈʌməɟʰnaː");
        same!(hi("करना"), "kˈʌɾnˌaː");
        same!(hi("कहना"), "kˈʌhənˌaː");
        same!(hi("मतलब"), "mˈʌtəlˌəb");
        same!(hi("घर"), "ɡʰˈʌɾ");
    }

    #[test]
    fn stress_and_length() {
        same!(hi("किताब"), "kɪtˈaːb");
        same!(hi("पानी"), "pˈaːni");
        same!(hi("नमकीन"), "nˌəmkˈiːn");
        same!(hi("दुकानदार"), "dˌʊkaːndˈaːɾ");
        same!(hi("आदमी"), "ˈaːdmˌi");
        same!(hi("नमस्ते"), "nəmˈʌsteː");
    }

    #[test]
    fn nasals_and_conjuncts() {
        same!(hi("हिंदी"), "hˈĩdi");
        same!(hi("हिन्दी"), "hˈɪndi");
        same!(hi("पसंद"), "pəsˈʌnd");
        same!(hi("नहीं"), "nˈʌhĩ");
        same!(hi("में"), "mˈẽː");
        same!(hi("क्या"), "kːjˈaː");
        same!(hi("सत्य"), "sˈʌtːjə");
        same!(hi("ज्ञान"), "ɡjˈaːn");
        same!(hi("मित्र"), "mˈɪtɾə");
        same!(hi("शब्द"), "ʃˈʌbd");
        same!(hi("कर्म"), "kˈʌrm");
    }

    #[test]
    fn quirks_and_improved() {
        same!(hi("लड़का"), "lˈʌr.kˌaː");
        assert_eq!(Hindi::new(None).word("लड़का", false), "lˈʌɽkˌaː");
        same!(hi("समय"), "sˈʌmˌɛj");
        same!(hi("ज़रूरी"), "zəɾˈuːɾi");
        assert_eq!(Hindi::new(None).word("लड़कपन", false), "lˈʌɽəkpən");
    }
}
