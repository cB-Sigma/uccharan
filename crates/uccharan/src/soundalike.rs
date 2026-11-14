//! Sound-alike matching: find the places where a transcript says something
//! that sounds like a term of a vocabulary (names, products, jargon) and
//! offer the term's spelling.
//!
//! ```
//! use uccharan::soundalike::{ApplyOptions, IndexOptions, SoundIndex};
//! use uccharan::{Lang, Phonemizer};
//!
//! let p = Phonemizer::new();
//! let index = SoundIndex::from_terms(&p, &["Kothari", "Paytm"], &IndexOptions::default());
//! let fixed = index.apply(&p, "send it to Coat Harry", Lang::EnUs, &ApplyOptions::default());
//! assert_eq!(fixed.text, "send it to Kothari");
//! assert_eq!(fixed.changes[0].from, "Coat Harry");
//! ```
//!
//! Everything is compared as phonemes. A term is stored as one or more
//! phoneme sequences (its IPA when the word list gives one, the reading of
//! its spoken form, or uccharan's own readings of its spelling in several
//! languages and modes); a span of one to three transcript words is read
//! the same way and compared with a weighted edit distance in which close
//! sounds are cheap to exchange. Because both sides are phonemes, a term in
//! Latin script matches a transcript in Devanagari and the other way round.
//!
//! The matcher never changes text on its own: [`SoundIndex::find`] returns
//! candidates, and [`SoundIndex::apply`] returns the new text together with
//! every change it made.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::OnceLock;

use crate::text::{script_of, Script};
use crate::wordlist::{MatchRule, WordList};
use crate::{Lang, Mode, Options, Phonemizer};

pub mod score;

// ---------------------------------------------------------------------------
// Phones

/// A phone: the index of a base sound, with one flag (aspiration for a
/// consonant, length for a vowel) in the top bit.
type Ph = u8;
const FLAG: u8 = 0x80;

/// Manner classes of consonants.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Manner {
    Stop,
    Affricate,
    Fricative,
    Nasal,
    Lateral,
    Rhotic,
    Glide,
}

/// Coarse classes for the pre-filter (a consonant skeleton).
mod class {
    pub const NONE: u8 = 0;
    pub const P: u8 = 1;
    pub const W: u8 = 2;
    pub const T: u8 = 3;
    pub const K: u8 = 4;
    pub const S: u8 = 5;
    pub const M: u8 = 6;
    pub const N: u8 = 7;
    pub const L: u8 = 8;
    pub const R: u8 = 9;
    /// A vowel or glide at the start of the sequence.
    pub const V: u8 = 10;
    pub const COUNT: usize = 11;
}

use Manner::*;

/// Consonants: symbol, place (tenths along the vocal tract), manner,
/// voiced, pre-filter class.
const CONSONANTS: &[(&str, u8, Manner, bool, u8)] = &[
    ("p", 0, Stop, false, class::P),
    ("b", 0, Stop, true, class::P),
    ("t", 22, Stop, false, class::T),
    ("d", 22, Stop, true, class::T),
    ("ʈ", 24, Stop, false, class::T),
    ("ɖ", 24, Stop, true, class::T),
    ("c", 34, Stop, false, class::S),
    ("ɟ", 34, Stop, true, class::S),
    ("k", 50, Stop, false, class::K),
    ("g", 50, Stop, true, class::K),
    ("q", 54, Stop, false, class::K),
    ("tʃ", 30, Affricate, false, class::S),
    ("dʒ", 30, Affricate, true, class::S),
    ("f", 6, Fricative, false, class::P),
    ("v", 6, Fricative, true, class::W),
    ("θ", 20, Fricative, false, class::T),
    ("ð", 20, Fricative, true, class::T),
    ("s", 22, Fricative, false, class::S),
    ("z", 22, Fricative, true, class::S),
    ("ʃ", 30, Fricative, false, class::S),
    ("ʒ", 30, Fricative, true, class::S),
    ("ʂ", 31, Fricative, false, class::S),
    ("x", 50, Fricative, false, class::K),
    ("ɣ", 50, Fricative, true, class::K),
    ("h", 70, Fricative, false, class::NONE),
    ("m", 0, Nasal, true, class::M),
    ("n", 22, Nasal, true, class::N),
    ("ɳ", 24, Nasal, true, class::N),
    ("ɲ", 34, Nasal, true, class::N),
    ("ŋ", 50, Nasal, true, class::N),
    ("l", 22, Lateral, true, class::L),
    ("ɭ", 24, Lateral, true, class::L),
    ("r", 22, Rhotic, true, class::R),
    ("ɽ", 24, Rhotic, true, class::R),
    ("j", 34, Glide, true, class::NONE),
    ("w", 3, Glide, true, class::W),
    // The English flap (the "t" of "butter" in en-US output).
    ("ɾt", 22, Stop, true, class::T),
];

/// Vowels: symbol, height (0 close … 30 open), backness (0 front … 20
/// back), rounded.
const VOWELS: &[(&str, u8, u8, bool)] = &[
    ("i", 0, 0, false),
    ("ɪ", 5, 3, false),
    ("e", 10, 0, false),
    ("ɛ", 20, 0, false),
    ("æ", 26, 2, false),
    ("a", 30, 10, false),
    ("ɑ", 30, 18, false),
    ("ə", 15, 10, false),
    ("ɔ", 20, 20, true),
    ("ɒ", 28, 20, true),
    ("o", 10, 20, true),
    ("ʊ", 5, 17, true),
    ("u", 0, 20, true),
    ("y", 0, 2, true),
    ("ø", 12, 2, true),
];

const N_CONS: usize = CONSONANTS.len();
const N_BASE: usize = N_CONS + VOWELS.len();

/// Symbols that are written differently but compared as one of the bases.
const ALIASES: &[(char, &str)] = &[
    ('ɡ', "g"),
    ('ɫ', "l"),
    ('ʎ', "l"),
    ('ɹ', "r"),
    ('ɻ', "r"),
    ('ʀ', "r"),
    ('ʁ', "r"),
    ('ɦ', "h"),
    ('ħ', "h"),
    ('χ', "x"),
    ('ç', "ʃ"),
    ('ɕ', "ʃ"),
    ('ʑ', "ʒ"),
    ('ʐ', "ʒ"),
    ('ʋ', "v"),
    ('β', "v"),
    ('ɸ', "f"),
    ('ɱ', "m"),
    ('ɥ', "j"),
    ('ɐ', "ə"),
    ('ʌ', "ə"),
    ('ɜ', "ə"),
    ('ɘ', "ə"),
    ('ɤ', "ə"),
    ('ᵻ', "ɪ"),
    ('ɨ', "ɪ"),
    ('ɯ', "u"),
    ('ɵ', "o"),
    ('œ', "ø"),
];

struct Tables {
    /// Base index by symbol.
    by_symbol: HashMap<&'static str, u8>,
    /// Substitution cost between bases, in hundredths.
    sub: Vec<u8>,
    schwa: u8,
    h: u8,
    n: u8,
    r: u8,
    flap: u8,
    j: u8,
    w: u8,
}

fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(|| {
        let mut by_symbol = HashMap::new();
        for (i, c) in CONSONANTS.iter().enumerate() {
            by_symbol.insert(c.0, i as u8);
        }
        for (i, v) in VOWELS.iter().enumerate() {
            by_symbol.insert(v.0, (N_CONS + i) as u8);
        }
        let id = |s: &str| by_symbol[s] as usize;
        let mut sub = vec![100u8; N_BASE * N_BASE];
        let manner_cost = |a: Manner, b: Manner| -> u32 {
            if a == b {
                return 0;
            }
            let pair = |x: Manner, y: Manner| (a == x && b == y) || (a == y && b == x);
            if pair(Stop, Affricate) {
                35
            } else if pair(Stop, Fricative) {
                40
            } else if pair(Affricate, Fricative) || pair(Glide, Fricative) {
                30
            } else if pair(Stop, Nasal) {
                60
            } else if pair(Lateral, Rhotic) {
                50
            } else if pair(Nasal, Lateral) || pair(Glide, Lateral) || pair(Glide, Rhotic) {
                70
            } else {
                90
            }
        };
        for (i, a) in CONSONANTS.iter().enumerate() {
            for (j, b) in CONSONANTS.iter().enumerate() {
                let place = (a.1 as i32 - b.1 as i32).unsigned_abs() * 3;
                let voice = if a.3 != b.3 { 40 } else { 0 };
                let c = place.min(100) + manner_cost(a.2, b.2) + voice;
                sub[i * N_BASE + j] = c.min(100) as u8;
            }
        }
        for (i, a) in VOWELS.iter().enumerate() {
            for (j, b) in VOWELS.iter().enumerate() {
                let dh = a.1 as f32 - b.1 as f32;
                let db = a.2 as f32 - b.2 as f32;
                let round = if a.3 != b.3 { 5.0 } else { 0.0 };
                let c = (2.2 * (dh * dh + db * db).sqrt() + round).min(70.0);
                sub[(N_CONS + i) * N_BASE + N_CONS + j] = c.round() as u8;
            }
        }
        // Pairs that are closer than their features say.
        let close: &[(&str, &str, u8)] = &[
            ("θ", "t", 20),
            ("θ", "ʈ", 22),
            ("ð", "d", 20),
            ("ð", "ɖ", 22),
            ("v", "w", 10),
            ("b", "w", 55),
            ("c", "tʃ", 5),
            ("ɟ", "dʒ", 5),
            ("z", "dʒ", 35),
            ("z", "ɟ", 35),
            ("ʒ", "dʒ", 25),
            ("ʒ", "ɟ", 25),
            ("x", "k", 30),
            ("ɣ", "g", 30),
            ("q", "k", 10),
            ("x", "h", 25),
            ("n", "ŋ", 35),
            ("m", "n", 50),
            ("ɽ", "ɖ", 20),
            ("ɽ", "r", 5),
            ("s", "ʂ", 24),
            ("ʃ", "ʂ", 5),
            ("j", "dʒ", 45),
            ("j", "ɟ", 45),
            ("ɾt", "t", 10),
            ("ɾt", "d", 10),
            ("ɾt", "ʈ", 15),
            ("ɾt", "ɖ", 15),
            ("ɾt", "r", 30),
            ("i", "j", 40),
            ("ɪ", "j", 40),
            ("u", "w", 45),
            ("ʊ", "w", 45),
            ("u", "v", 55),
            ("ʊ", "v", 55),
        ];
        for &(a, b, c) in close {
            sub[id(a) * N_BASE + id(b)] = c;
            sub[id(b) * N_BASE + id(a)] = c;
        }
        for i in 0..N_BASE {
            sub[i * N_BASE + i] = 0;
        }
        Tables {
            schwa: by_symbol["ə"],
            h: by_symbol["h"],
            n: by_symbol["n"],
            r: by_symbol["r"],
            flap: by_symbol["ɾt"],
            j: by_symbol["j"],
            w: by_symbol["w"],
            by_symbol,
            sub,
        }
    })
}

#[inline]
fn is_vowel(p: Ph) -> bool {
    (p & !FLAG) as usize >= N_CONS
}

/// Cost of exchanging two phones, in hundredths.
#[inline]
fn sub_cost(t: &Tables, a: Ph, b: Ph) -> u32 {
    if a == b {
        return 0;
    }
    let (x, y) = ((a & !FLAG) as usize, (b & !FLAG) as usize);
    if x == y {
        // Aspiration only, or vowel length only.
        return if x < N_CONS { 15 } else { 10 };
    }
    let c = t.sub[x * N_BASE + y] as u32;
    if x < N_CONS && y < N_CONS && (a ^ b) & FLAG != 0 {
        // An aspirated stop against a fricative of a near place (pʰ / f,
        // kʰ / x, t̪ʰ / θ) is close; otherwise aspiration adds a little.
        let (asp, other) = if a & FLAG != 0 { (x, y) } else { (y, x) };
        if CONSONANTS[asp].2 == Stop && CONSONANTS[other].2 == Fricative {
            return c.saturating_sub(20).max(5);
        }
        return (c + 5).min(100);
    }
    c
}

/// Append the phones of an IPA string. `english` selects the reading of
/// `ɾ`: the flapped t of en-US output, or an r sound elsewhere.
fn parse_ipa(ipa: &str, english: bool, out: &mut Vec<Ph>) {
    let t = tables();
    let cs: Vec<char> = crate::profile::nfd(ipa).chars().collect();
    let mut buf = [0u8; 4];
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        let next = cs.get(i + 1).copied();
        i += 1;
        match c {
            'ʰ' | 'ʱ' => {
                if let Some(last) = out.last_mut() {
                    let b = (*last & !FLAG) as usize;
                    if b < N_CONS && matches!(CONSONANTS[b].2, Stop | Affricate) {
                        *last |= FLAG;
                    }
                }
            }
            'ː' | 'ˑ' => {
                if let Some(last) = out.last_mut() {
                    if is_vowel(*last) {
                        *last |= FLAG;
                    }
                }
            }
            '\u{0303}' => {
                // A nasalised vowel is compared as vowel + n.
                if out.last().is_some_and(|p| is_vowel(*p)) {
                    // Length marks follow the tilde in NFD order.
                    let mut long = false;
                    if cs.get(i) == Some(&'ː') {
                        long = true;
                        i += 1;
                    }
                    if long {
                        if let Some(last) = out.last_mut() {
                            *last |= FLAG;
                        }
                    }
                    out.push(t.n);
                }
            }
            'ɚ' => {
                out.push(t.schwa);
                out.push(t.r);
            }
            'ɝ' => {
                out.push(t.schwa | FLAG);
                out.push(t.r);
            }
            'ɾ' => out.push(if english { t.flap } else { t.r }),
            't' if next == Some('ʃ') || next == Some('ɕ') => {
                out.push(t.by_symbol["tʃ"]);
                i += 1;
            }
            'd' if next == Some('ʒ') || next == Some('ʑ') => {
                out.push(t.by_symbol["dʒ"]);
                i += 1;
            }
            // Closing diphthongs that other languages write as long vowels.
            'e' if next == Some('ɪ') => {
                out.push(t.by_symbol["e"] | FLAG);
                i += 1;
            }
            'o' | 'ə' if next == Some('ʊ') => {
                out.push(t.by_symbol["o"] | FLAG);
                i += 1;
            }
            _ => {
                let alias = ALIASES.iter().find(|a| a.0 == c).map(|a| a.1);
                let sym: &str = alias.unwrap_or_else(|| c.encode_utf8(&mut buf));
                if let Some(&b) = t.by_symbol.get(sym) {
                    out.push(b);
                }
            }
        }
    }
}

/// A phone sequence in the matcher's broad notation.
fn show(seq: &[Ph]) -> String {
    let mut s = String::new();
    for &p in seq {
        let b = (p & !FLAG) as usize;
        if b < N_CONS {
            s.push_str(if CONSONANTS[b].0 == "ɾt" {
                "ɾ"
            } else {
                CONSONANTS[b].0
            });
            if p & FLAG != 0 {
                s.push('ʰ');
            }
        } else {
            s.push_str(VOWELS[b - N_CONS].0);
            if p & FLAG != 0 {
                s.push('ː');
            }
        }
    }
    s
}

/// The phonemes of `text` as the matcher hears them when it reads a
/// transcript in `lang` (its first reading), in the matcher's notation.
pub fn heard(ph: &Phonemizer, text: &str, lang: Lang) -> String {
    let opt = if lang.is_english() {
        options(lang, Mode::Improved, false)
    } else {
        options(lang, Mode::Improved, lang.hindi_like())
    };
    let mut v = Vec::new();
    read(ph, text, &opt, &mut v);
    tidy(&mut v);
    show(&v)
}

/// Tidy a phone sequence in place: a stop followed by h becomes an
/// aspirated stop, doubled sounds become one.
fn tidy(seq: &mut Vec<Ph>) {
    let t = tables();
    let mut out: Vec<Ph> = Vec::with_capacity(seq.len());
    for &p in seq.iter() {
        if let Some(last) = out.last_mut() {
            let lb = (*last & !FLAG) as usize;
            if p == t.h && lb < N_CONS && matches!(CONSONANTS[lb].2, Stop | Affricate) {
                *last |= FLAG;
                continue;
            }
            if (*last & !FLAG) == (p & !FLAG) {
                if is_vowel(p) || p & FLAG != 0 {
                    *last |= FLAG;
                }
                continue;
            }
        }
        out.push(p);
    }
    *seq = out;
}

/// Cost of dropping (or inserting) each phone of a sequence, in hundredths.
fn del_costs(seq: &[Ph], out: &mut Vec<u8>) {
    let t = tables();
    out.clear();
    for (i, &p) in seq.iter().enumerate() {
        let b = p & !FLAG;
        let long = p & FLAG != 0;
        let c = if is_vowel(p) {
            if b == t.schwa {
                if long {
                    50
                } else {
                    25
                }
            } else if long {
                85
            } else if matches!(VOWELS[b as usize - N_CONS].0, "ɪ" | "ʊ") {
                50
            } else {
                70
            }
        } else if b == t.h {
            35
        } else if b == t.j || b == t.w {
            50
        } else if b == t.r {
            // An r after a vowel and not before one: silent in some accents.
            let after_vowel = i > 0 && is_vowel(seq[i - 1]);
            let before_vowel = seq.get(i + 1).is_some_and(|n| is_vowel(*n));
            if after_vowel && !before_vowel {
                40
            } else {
                80
            }
        } else if CONSONANTS[b as usize].2 == Nasal
            && seq.get(i + 1).is_some_and(|n| {
                !is_vowel(*n) && matches!(CONSONANTS[(*n & !FLAG) as usize].2, Stop | Affricate)
            })
        {
            60
        } else {
            100
        };
        out.push(c);
    }
}

/// Weighted edit distance between two phone sequences (hundredths), or
/// `None` when it exceeds `limit`.
fn distance(
    a: &[Ph],
    da: &[u8],
    b: &[Ph],
    db: &[u8],
    limit: u32,
    row: &mut Vec<u32>,
) -> Option<u32> {
    let t = tables();
    row.clear();
    row.push(0);
    let mut acc = 0u32;
    for &d in db {
        acc += d as u32;
        row.push(acc);
    }
    for (i, &x) in a.iter().enumerate() {
        let dx = da[i] as u32;
        let mut diag = row[0];
        row[0] += dx;
        let mut best = row[0];
        for (j, &y) in b.iter().enumerate() {
            let up = row[j + 1];
            let v = (diag + sub_cost(t, x, y))
                .min(up + dx)
                .min(row[j] + db[j] as u32);
            diag = up;
            row[j + 1] = v;
            best = best.min(v);
        }
        if best > limit {
            return None;
        }
    }
    let d = row[b.len()];
    (d <= limit).then_some(d)
}

/// The consonant skeleton of a sequence, for the pre-filter.
fn skeleton(seq: &[Ph], out: &mut Vec<u8>) {
    out.clear();
    for (i, &p) in seq.iter().enumerate() {
        let b = (p & !FLAG) as usize;
        let c = if b < N_CONS {
            CONSONANTS[b].4
        } else {
            class::NONE
        };
        if c != class::NONE {
            out.push(c);
        } else if i == 0 && tables().h as usize != b {
            out.push(class::V);
        }
    }
}

/// The distinct class bigrams of a skeleton (with boundaries), as a bit set.
fn bigrams(skel: &[u8]) -> [u64; 2] {
    let mut set = [0u64; 2];
    let mut prev = 0u8;
    for &c in skel.iter().chain(std::iter::once(&0u8)) {
        let k = prev as usize * class::COUNT + c as usize;
        set[k / 64] |= 1 << (k % 64);
        prev = c;
    }
    set
}

// ---------------------------------------------------------------------------
// Reading text as phones

/// Phones of `text` read with `opt`, appended to `out`.
fn read(ph: &Phonemizer, text: &str, opt: &Options, out: &mut Vec<Ph>) {
    // English words inside another language's text are given that
    // language's r, not the en-US flap.
    let flap = opt.lang.is_english();
    for s in ph.analyze(text, opt) {
        for c in s.clauses {
            for w in c.words {
                parse_ipa(&w.ipa, flap && w.lang.is_english(), out);
            }
        }
    }
}

fn options(lang: Lang, mode: Mode, hinglish: bool) -> Options {
    let mut o = Options::new(lang);
    o.mode = mode;
    o.hinglish = hinglish;
    o
}

fn first_script(s: &str) -> Script {
    s.chars()
        .map(script_of)
        .find(|s| !matches!(s, Script::Digit | Script::Other))
        .unwrap_or(Script::Latin)
}

/// "PhonePe" → "Phone Pe", "MongoDB" → "Mongo DB".
fn split_camel(word: &str) -> Option<String> {
    let cs: Vec<char> = word.chars().collect();
    let mut out = String::with_capacity(word.len() + 2);
    let mut changed = false;
    for (i, &c) in cs.iter().enumerate() {
        if i > 0 && c.is_uppercase() && cs[i - 1].is_lowercase() {
            out.push(' ');
            changed = true;
        }
        out.push(c);
    }
    changed.then_some(out)
}

/// The romanised-Indic reading of `text`: its Latin-script words are
/// rewritten in Devanagari by the romanisation rules ("Kothari" → "कोथरी")
/// and read as names, with every written vowel kept. Other words are read
/// as Hindi text. Returns false when there is no Latin-script word.
fn read_romanised(ph: &Phonemizer, text: &str, out: &mut Vec<Ph>) -> bool {
    let hindi = crate::hi::Hindi::new(ph.data(Lang::Hi));
    let plain = options(Lang::Hi, Mode::Improved, false);
    let mut any = false;
    for w in text.split_whitespace() {
        if w.len() > 1 && w.chars().all(|c| c.is_ascii_alphabetic()) {
            let deva = crate::hi::roman::transliterate(&w.to_ascii_lowercase());
            parse_ipa(&hindi.name(&deva), false, out);
            any = true;
        } else {
            read(ph, w, &plain, out);
        }
    }
    any
}

/// The readings of a written form: phone sequences for the index.
fn readings(ph: &Phonemizer, word: &str, lang: Option<Lang>, langs: &[Lang]) -> Vec<Vec<Ph>> {
    let mut out: Vec<Vec<Ph>> = Vec::new();
    let script = first_script(word);
    if script == Script::Latin && langs.iter().chain(lang.iter()).any(|l| !l.is_english()) {
        let mut v = Vec::new();
        if read_romanised(ph, word, &mut v) {
            tidy(&mut v);
            if !v.is_empty() {
                out.push(v);
            }
        }
    }
    let mut add = |text: &str, opt: &Options| {
        let mut v = Vec::new();
        read(ph, text, opt, &mut v);
        tidy(&mut v);
        if !v.is_empty() && !out.contains(&v) {
            out.push(v);
        }
    };
    if script == Script::Latin {
        let english: Vec<Lang> = langs
            .iter()
            .copied()
            .chain(lang)
            .filter(|l| l.is_english())
            .collect();
        let english = if english.is_empty() {
            vec![Lang::EnUs]
        } else {
            english
        };
        for &l in &english {
            add(word, &options(l, Mode::Compatible, false));
            add(word, &options(l, Mode::Improved, false));
        }
        if let Some(split) = split_camel(word) {
            add(&split, &options(english[0], Mode::Compatible, false));
        }
        let letters = word.chars().filter(|c| c.is_alphabetic()).count();
        if (2..=6).contains(&letters)
            && word
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        {
            // An initialism may be said letter by letter.
            let spelled: String = word.chars().flat_map(|c| [c, ' ']).collect();
            add(
                spelled.trim(),
                &options(english[0], Mode::Compatible, false),
            );
        }
        // Romanised Indic words: the Hindi reading of the Latin spelling.
        if langs.iter().chain(lang.iter()).any(|l| !l.is_english()) {
            add(word, &options(Lang::Hi, Mode::Improved, true));
        }
    } else {
        let mut ls: Vec<Lang> = Vec::new();
        if let Some(l) = lang.filter(|l| l.script() == script) {
            ls.push(l);
        }
        ls.extend(langs.iter().copied().filter(|l| l.script() == script));
        if let Some(l) = Lang::for_script(script) {
            ls.push(l);
        }
        ls.dedup();
        for l in ls.into_iter().take(2) {
            add(word, &options(l, Mode::Compatible, false));
            add(word, &options(l, Mode::Improved, false));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Tokens

/// A word of the transcript.
#[derive(Clone, Debug)]
struct Token {
    /// Byte range in the transcript.
    range: Range<usize>,
    /// Index of the whitespace-separated word that contains it.
    word: usize,
}

fn word_char(c: char) -> bool {
    c.is_alphanumeric()
        || matches!(c, '\u{0300}'..='\u{036f}' | '\u{200c}' | '\u{200d}')
        || matches!(script_of(c), Script::Devanagari | Script::Brahmic(_))
            && !matches!(c, '।' | '॥' | '،' | '؛' | '؟' | '۔')
}

fn tokenize(text: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut word = 0usize;
    let mut in_space = true;
    let mut start: Option<(usize, usize)> = None;
    let mut possessive = usize::MAX;
    let cs: Vec<(usize, char)> = text.char_indices().collect();
    for (k, &(i, c)) in cs.iter().enumerate() {
        if c.is_whitespace() {
            if !in_space {
                word += 1;
            }
            in_space = true;
        } else {
            in_space = false;
        }
        if k == possessive {
            continue;
        }
        // An apostrophe between letters stays in the word ("don't"),
        // except for a final 's, which belongs to no word.
        let apostrophe = matches!(c, '\'' | '’')
            && start.is_some()
            && cs.get(k + 1).is_some_and(|n| n.1.is_alphabetic());
        let final_s = apostrophe
            && cs.get(k + 1).is_some_and(|n| matches!(n.1, 's' | 'S'))
            && cs.get(k + 2).is_none_or(|n| !word_char(n.1));
        if final_s {
            possessive = k + 1;
        }
        if word_char(c) || (apostrophe && !final_s) {
            if start.is_none() {
                start = Some((i, word));
            }
        } else if let Some((s, word)) = start.take() {
            out.push(Token { range: s..i, word });
        }
    }
    if let Some((s, word)) = start {
        out.push(Token {
            range: s..text.len(),
            word,
        });
    }
    out
}

/// True when two neighbouring tokens may belong to one span: only spaces
/// and light punctuation between them.
fn joinable(gap: &str) -> bool {
    gap.chars().count() <= 3
        && gap
            .chars()
            .all(|c| c.is_whitespace() || matches!(c, '-' | ',' | '.' | '\'' | '’' | '"'))
}

/// The key used to find a term written out in the transcript: its words
/// in lower case, joined by single spaces.
fn exact_key(text: &str) -> String {
    let mut key = String::new();
    for t in tokenize(text) {
        if !key.is_empty() {
            key.push(' ');
        }
        for c in text[t.range].chars() {
            key.extend(c.to_lowercase());
        }
    }
    key
}

// ---------------------------------------------------------------------------
// Ordinary words

/// Frequent Hindi words (hand-written): a span made only of such words is
/// ordinary speech far more often than a misheard term.
const HINDI_COMMON: &str = "\
का की के को से में पर ने और या तथा एवं कि जो तो भी ही न ना नहीं मत हाँ हां जी \
है हैं हूँ हूं हो था थी थे थीं होगा होगी होंगे होता होती होते हुआ हुई हुए होना होने \
मैं मैंने मुझे मुझको मुझसे मेरा मेरी मेरे हम हमें हमने हमारा हमारी हमारे तू तुम तुम्हें तुमने तुम्हारा तुम्हारी तुम्हारे \
आप आपको आपने आपका आपकी आपके आपसे वह वो उसे उसको उसने उसका उसकी उसके उससे वे उन्हें उनको उन्होंने उनका उनकी उनके उनसे \
यह ये इसे इसको इसने इसका इसकी इसके इससे इन्हें इनको इन्होंने इनका इनकी इनके इस उस इन उन जिस जिन किस किन \
कौन क्या क्यों कब कहाँ कहां कैसे कैसा कैसी कितना कितनी कितने किसी किसने किसको कुछ कोई सब सभी हर कई \
एक दो तीन चार पाँच पांच छह सात आठ नौ दस बीस सौ हज़ार हजार लाख पहला पहली पहले दूसरा दूसरी दूसरे \
अब तब जब कभी अभी यहाँ यहां वहाँ वहां जहाँ जहां इधर उधर आज कल परसों सुबह शाम रात दिन दोपहर \
बहुत ज़्यादा ज्यादा कम थोड़ा थोड़ी थोड़े सारा सारी सारे पूरा पूरी पूरे अधिक काफ़ी काफी बिल्कुल बिलकुल सिर्फ़ सिर्फ केवल \
अच्छा अच्छी अच्छे बुरा बुरी बुरे बड़ा बड़ी बड़े छोटा छोटी छोटे नया नई नए पुराना पुरानी पुराने लंबा लंबी सही ग़लत गलत \
कर करना करने करता करती करते किया की किए कीजिए करो करें करेगा करेगी करेंगे करूँगा करूंगा रहा रही रहे रहना रहता रहती रहते \
जा जाना जाने जाता जाती जाते गया गई गए जाओ जाएँ जाएं जाएगा जाएगी जाएंगे आ आना आने आता आती आते आया आई आए आओ आएँ आएं आएगा आएगी आएंगे \
दे देना देने देता देती देते दिया दी दिए दो दीजिए दें ले लेना लेने लेता लेती लेते लिया ली लिए लो लीजिए लें \
कह कहना कहने कहता कहती कहते कहा कही बोल बोलना बोला बोली बोले बता बताना बताया बताइए बताओ सुन सुनना सुना सुनी सुनो \
देख देखना देखा देखी देखो देखिए मिल मिलना मिला मिली मिले चल चलना चला चली चले चलो चलिए बैठ बैठना बैठा बैठिए उठ उठना \
सक सकता सकती सकते सका सकी सके चाह चाहता चाहती चाहते चाहिए पड़ा पड़ी पड़े पड़ता पड़ेगा लग लगा लगी लगे लगता लगती \
पा पाना पाया पाई पाए पाएंगे रख रखना रखा रखी रखो रखिए खा खाना खाया खाई पी पीना पिया सो सोना सोया \
लिख लिखना लिखा लिखी पढ़ पढ़ना पढ़ा पढ़ी समझ समझना समझा जान जानता जानती जानते जानना माना मानना सोच सोचना सोचा \
भेज भेजना भेजा भेजी भेजो भेजिए ख़रीद खरीद खरीदा बेच बना बनाना बनाया बनाई बन बनना खोल खोलो बंद \
बात बातें काम समय वक़्त वक्त दिन साल महीना महीने हफ़्ता हफ्ता घंटा घंटे मिनट लोग लोगों आदमी औरत बच्चा बच्चे बच्चों लड़का लड़की \
घर दफ़्तर दफ्तर स्कूल शहर गाँव गांव देश दुनिया जगह रास्ता पानी खाना पैसा पैसे रुपये रुपए किताब नाम पता नंबर फ़ोन फोन \
साथ बाद पहले लिए बारे तरफ़ तरफ ओर पास दूर ऊपर नीचे अंदर बाहर सामने पीछे बीच बिना तक द्वारा कारण वजह तरह प्रकार \
लेकिन मगर परंतु क्योंकि इसलिए अगर यदि तो फिर ताकि जैसे वैसे ऐसे ऐसा ऐसी जैसा जैसी शायद ज़रूर जरूर हमेशा अक्सर \
ठीक साहब साहिब श्री श्रीमती डॉक्टर भाई बहन माँ मां पिता बेटा बेटी दोस्त मित्र सर मैडम \
बैठक जानकारी इस्तेमाल उपयोग ख़बर खबर अख़बार अखबार लेख रिपोर्ट फ़ाइल फाइल परिवार सवाल जवाब मदद ज़रूरत जरूरत कृपया धन्यवाद \
बातों कामों दिनों सालों महीनों लोगो घरों शहरों गाँवों गांवों देशों जगहों रास्ते रास्तों पैसों रुपयों किताबें किताबों नामों \
आदमियों औरतों लड़के लड़कों लड़कियाँ लड़कियां लड़कियों दोस्तों भाइयों बहनों बेटे बेटों बेटियों माता पति पत्नी चाचा मामा दादा दादी नाना नानी \
गाड़ी गाड़ियाँ गाड़ियां गाड़ियों बस ट्रेन रेल सड़क दुकान दुकानें बाज़ार बाजार कमरा कमरे दरवाज़ा दरवाजा खिड़की मेज़ मेज कुर्सी \
हाथ पैर सिर आँख आंख आँखें आंखें कान मुँह मुंह दिल मन शरीर चेहरा आवाज़ आवाज \
चाय दूध रोटी चावल दाल सब्ज़ी सब्जी फल मिठाई नमक चीनी भूख प्यास नींद \
सूरज चाँद चांद तारा तारे आसमान ज़मीन जमीन पेड़ फूल नदी पहाड़ समुद्र हवा बारिश धूप गर्मी सर्दी मौसम \
कुत्ता बिल्ली गाय घोड़ा पक्षी चिड़िया मछली \
राजा रानी सरकार पुलिस अदालत कानून क़ानून नेता मंत्री चुनाव जनता समाज धर्म भगवान ईश्वर मंदिर \
शिक्षा पढ़ाई कक्षा छात्र शिक्षक अध्यापक परीक्षा विद्यालय भाषा शब्द वाक्य अर्थ कहानी कविता गीत संगीत फ़िल्म फिल्म खेल \
सुंदर प्यारा प्यारी ख़ुश खुश दुखी उदास नाराज़ नाराज गरीब ग़रीब अमीर तेज़ तेज धीरे जल्दी देर आसान मुश्किल कठिन ज़रूरी जरूरी ख़ास खास आम \
साफ़ साफ गंदा गर्म ठंडा मीठा खट्टा भारी हल्का ऊँचा ऊंचा गहरा सफ़ेद सफेद काला लाल पीला नीला हरा रंग \
सच झूठ प्यार प्रेम डर ग़ुस्सा गुस्सा ख़ुशी खुशी दुख चिंता आशा उम्मीद विश्वास भरोसा याद इंतज़ार इंतजार कोशिश मेहनत \
शुरू ख़त्म खत्म तैयार मौजूद ज़िंदा जिंदा मालूम पसंद सहमत ज़िंदगी जिंदगी जीवन मौत उम्र हाल हालत \
मार मारा मारना मरा मरी मरे मरना गिर गिरा गिरना रुक रुका रुकना रुको भाग भागा भागना दौड़ खेला खेलना हँस हंस हँसना रो रोना रोया \
पूछ पूछा पूछना माँग मांग माँगा मांगा ढूँढ ढूंढ ढूँढा खोज खोया खोना बदल बदला बदलना छोड़ छोड़ा छोड़ना पकड़ पकड़ा \
पहुँच पहुंच पहुँचा पहुंचा पहुँचे पहुंचे लौट लौटा निकल निकला निकली निकले घूम घूमना उड़ उड़ा तैर चढ़ उतर \
सीख सीखा सीखना सिखाया जीत जीता हार हारा बचा बचाया डाल डाला डालना उठा उठाया उठाना लाना लाया लाई लाए लाओ \
मुझसे तुमसे हमसे तुझे तेरा तेरी तेरे अपना अपनी अपने ख़ुद खुद स्वयं आपस दोनों तीनों चारों \
छठा सातवाँ आठवाँ ग्यारह बारह तेरह चौदह पंद्रह सोलह सत्रह अठारह उन्नीस तीस चालीस पचास साठ सत्तर अस्सी नब्बे करोड़ आधा डेढ़ ढाई पौने सवा साढ़े \
सोमवार मंगलवार बुधवार गुरुवार शुक्रवार शनिवार रविवार जनवरी फ़रवरी फरवरी मार्च अप्रैल मई जून जुलाई अगस्त सितंबर अक्टूबर नवंबर दिसंबर \
भारत हिंदी अंग्रेज़ी अंग्रेजी दिल्ली मुंबई";

/// The same in Arabic script (hand-written): some recognisers write Hindi
/// speech this way.
const URDU_COMMON: &str = "\
کا کی کے کو سے میں پر نے اور یا کہ جو تو بھی ہی نہ نہیں مت ہاں جی \
ہے ہیں ہوں ہو تھا تھی تھے تھیں ہوگا ہوگی ہونگے ہوتا ہوتی ہوتے ہوا ہوئی ہوئے ہونا ہونے گا گی گے \
مجھے مجھ میرا میری میرے ہم ہمیں ہمارا ہماری ہمارے تم تمہیں تمہارا تمہاری تمہارے آپ آپکو آپکا آپکی آپکے \
وہ اسے اس ان انہیں انہوں یہ ہر جس جن کس کن کیا کیوں کب کہاں کیسے کیسا کون کوئی کچھ سب سبھی کئی \
ایک دو تین چار پانچ چھ سات آٹھ نو دس بیس سو ہزار لاکھ پہلا پہلی پہلے دوسرا دوسری دوسرے \
اب تب جب کبھی ابھی یہاں وہاں جہاں آج کل صبح شام رات دن دوپہر \
بہت زیادہ کم تھوڑا تھوڑی سارا ساری سارے پورا پوری پورے کافی بالکل صرف \
اچھا اچھی اچھے برا بڑا بڑی بڑے چھوٹا چھوٹی چھوٹے نیا نئی نئے پرانا لمبا صحیح غلط \
کر کرنا کرنے کرتا کرتی کرتے کیے کیجیے کرو کریں کرے رہا رہی رہے رہنا رہتا رہتی رہتے \
جا جانا جانے جاتا جاتی جاتے گیا گئی گئے جاؤ جائیں آ آنا آنے آتا آتی آتے آیا آئی آئے آؤ آئیں \
دے دینا دینے دیتا دیتی دیتے دیا دی دیے دیں دیجیے لے لینا لینے لیتا لیتی لیتے لیا لی لیے لو لیں \
کہ کہنا کہا کہی بول بولنا بولا بتا بتانا بتایا سن سننا سنا سنو دیکھ دیکھنا دیکھا دیکھو مل ملنا ملا ملی ملے \
چل چلنا چلا چلو بیٹھ اٹھ سک سکتا سکتی سکتے سکا چاہ چاہتا چاہتی چاہتے چاہیے چاہئے پڑا پڑی لگ لگا لگی لگتا \
پا پانا پایا رکھ رکھنا رکھا کھا کھانا کھایا پی پینا لکھ لکھنا لکھا پڑھ پڑھنا سمجھ جان جانتا جانتے سوچ \
بھیج بھیجنا بھیجا بھیجو خرید بیچ بنا بنانا بنایا بن کھول بند \
بات باتیں کام وقت سال مہینہ ہفتہ گھنٹہ منٹ لوگ لوگوں آدمی عورت بچہ بچے بچوں لڑکا لڑکی \
گھر دفتر اسکول شہر گاؤں دیش ملک دنیا جگہ راستہ پانی پیسہ پیسے روپے کتاب نام پتا پتہ نمبر فون \
ساتھ بعد لئے بارے طرف پاس دور اوپر نیچے اندر باہر سامنے پیچھے بیچ بنا تک وجہ طرح \
لیکن مگر کیونکہ اگر پھر تاکہ جیسے ویسے ایسے ایسا ایسی شاید ضرور ہمیشہ اکثر \
ٹھیک صاحب ڈاکٹر بھائی بہن ماں بیٹا بیٹی دوست سر \
بیٹھک جانکاری استعمال خبر اخبار لیکھ رپورٹ فائل پریوار سوال جواب مدد ضرورت شکریہ";

fn hindi_common() -> &'static std::collections::HashSet<&'static str> {
    static SET: OnceLock<std::collections::HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| {
        HINDI_COMMON
            .split_whitespace()
            .chain(URDU_COMMON.split_whitespace())
            .collect()
    })
}

/// True for a transcript word that is ordinary vocabulary: a word of the
/// English lexicon (personal names apart), a listed frequent Hindi word
/// (in Devanagari, Arabic script or romanised), or a number.
fn ordinary(ph: &Phonemizer, word: &str) -> bool {
    if word.chars().any(|c| c.is_numeric()) {
        return true;
    }
    let mut s = String::new();
    match first_script(word) {
        Script::Latin => {
            let lower = word.to_lowercase();
            let english = ph
                .data(Lang::EnUs)
                .or(ph.data(Lang::EnGb))
                .is_some_and(|d| d.lookup(&lower, &mut s) || d.known(&lower));
            english
                || ph
                    .data(Lang::Hi)
                    .is_some_and(|d| d.lookup(&format!("~{lower}"), &mut s))
        }
        Script::Devanagari | Script::Brahmic(0x0600) => hindi_common().contains(word),
        _ => false,
    }
}

/// Capitalised words that are not names of people or products.
const CALENDAR: &[&str] = &[
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
    "i",
    "i'm",
    "i'll",
    "i've",
    "i'd",
    "god",
    "english",
    "hindi",
    "indian",
    "india",
    "mr",
    "mrs",
    "ms",
    "miss",
    "dr",
    "sir",
    "madam",
    "mister",
    "doctor",
    "professor",
];
/// Titles: a capital after one of these (and its full stop) is a name.
const TITLES: &[&str] = &["mr", "mrs", "ms", "dr", "prof", "shri", "smt", "st"];

/// True when `word` is written with a capital that the sentence does not
/// call for: `before` is the text in front of it, `prev` the word before.
fn proper_noun(word: &str, before: &str, prev: Option<&str>) -> bool {
    let mut cs = word.chars();
    if !cs.next().is_some_and(char::is_uppercase) || !cs.any(char::is_lowercase) {
        return false;
    }
    if CALENDAR.contains(&word.to_lowercase().as_str()) {
        return false;
    }
    let gap = before
        .trim_end_matches(|c: char| c.is_whitespace() || matches!(c, '"' | '“' | '‘' | '\'' | '('));
    match gap.chars().next_back() {
        None => false,
        Some('.') => prev.is_some_and(|p| TITLES.contains(&p.to_lowercase().as_str())),
        Some('?' | '!' | ':' | ';' | '।' | '…' | '\n') => false,
        Some(_) => true,
    }
}

// ---------------------------------------------------------------------------
// The index

/// One vocabulary term.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Term {
    /// The spelling that replaces a matched span.
    pub word: String,
    /// Language of the entry, when the vocabulary gave one.
    pub lang: Option<Lang>,
    /// Phonemes given by the vocabulary (espeak-style IPA).
    pub ipa: Option<String>,
    /// Spoken form given by the vocabulary.
    pub say: Option<String>,
}

impl Term {
    /// A term with nothing but its spelling.
    pub fn new(word: &str) -> Term {
        Term {
            word: word.trim().to_string(),
            lang: None,
            ipa: None,
            say: None,
        }
    }
}

/// How an index is built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexOptions {
    /// Languages a term without phonemes is read in. A Latin-script term is
    /// read as English when the list has an English accent and with the
    /// romanised-Hindi reading when it has any other language; a term in
    /// another script is read in the languages of that script.
    pub langs: Vec<Lang>,
    /// More ordinary words (see [`SpanKind::Ordinary`]), in any script and
    /// compared without case: the frequent words of a language the built-in
    /// lists do not cover, or of the domain of the transcripts.
    pub ordinary_words: Vec<String>,
}

impl Default for IndexOptions {
    fn default() -> IndexOptions {
        IndexOptions {
            langs: vec![Lang::EnUs, Lang::Hi],
            ordinary_words: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
struct Variant {
    /// Offset of the phones in the arena.
    off: u32,
    len: u16,
    term: u32,
    /// Sum of the deletion costs: the length used to normalise.
    weight: u32,
}

/// The stored sequences that share one skeleton.
#[derive(Clone, Debug)]
struct Group {
    /// The skeleton's bigrams.
    set: [u64; 2],
    /// Number of distinct bigrams.
    bigrams: u8,
    /// Range of variants.
    first: u32,
    end: u32,
}

/// What the matcher knows about a transcript word: its readings and
/// whether it is an ordinary word.
#[derive(Debug)]
struct WordInfo {
    readings: [Vec<Ph>; READINGS],
    ordinary: bool,
}

/// Transcript words seen before, by language tag. Transcripts repeat
/// their words, and reading a word costs more than comparing it.
#[derive(Debug, Default)]
struct WordCache(
    std::sync::RwLock<HashMap<&'static str, HashMap<String, std::sync::Arc<WordInfo>>>>,
);

/// Words remembered per language before the cache starts again.
const CACHE_WORDS: usize = 50_000;

impl Clone for WordCache {
    fn clone(&self) -> WordCache {
        WordCache::default()
    }
}

impl WordCache {
    fn get(&self, lang: Lang, word: &str) -> Option<std::sync::Arc<WordInfo>> {
        let map = self.0.read().ok()?;
        map.get(lang.tag())?.get(word).cloned()
    }

    fn put(&self, lang: Lang, word: &str, info: std::sync::Arc<WordInfo>) {
        if let Ok(mut map) = self.0.write() {
            let words = map.entry(lang.tag()).or_default();
            if words.len() >= CACHE_WORDS {
                words.clear();
            }
            words.insert(word.to_string(), info);
        }
    }
}

/// A vocabulary prepared for matching. Build it once and share it: matching
/// takes `&self`.
#[derive(Clone, Debug, Default)]
pub struct SoundIndex {
    terms: Vec<Term>,
    phones: Vec<Ph>,
    dels: Vec<u8>,
    /// Variants, ordered by skeleton.
    variants: Vec<Variant>,
    /// The distinct skeletons, ordered by length.
    groups: Vec<Group>,
    /// First group of each skeleton length.
    by_len: Vec<u32>,
    /// Group ids by skeleton bigram.
    postings: Vec<Vec<u32>>,
    /// Terms by their lower-cased words.
    exact: HashMap<String, u32>,
    /// Largest number of words in a term.
    max_term_words: usize,
    /// Largest number of words in a spoken form ("my S Q L").
    max_say_words: usize,
    /// Terms that could not be read (index, reason).
    skipped: Vec<(usize, String)>,
    /// Ordinary words given with the index, in lower case.
    ordinary: std::collections::HashSet<String>,
    cache: WordCache,
}

impl SoundIndex {
    /// An index over plain terms.
    pub fn from_terms<S: AsRef<str>>(
        ph: &Phonemizer,
        terms: &[S],
        opt: &IndexOptions,
    ) -> SoundIndex {
        let terms: Vec<Term> = terms.iter().map(|t| Term::new(t.as_ref())).collect();
        SoundIndex::build(ph, terms, opt)
    }

    /// An index over the entries of word lists (regular-expression entries
    /// are left out). An entry's `ipa` and `say` columns give its sound;
    /// entries without either are read from their spelling.
    pub fn from_wordlists(ph: &Phonemizer, lists: &[&WordList], opt: &IndexOptions) -> SoundIndex {
        let mut terms = Vec::new();
        for l in lists {
            for e in &l.entries {
                if e.rule == MatchRule::Regex {
                    continue;
                }
                terms.push(Term {
                    word: e.word.clone(),
                    lang: e.lang,
                    ipa: e.ipa.clone(),
                    say: e.say.clone(),
                });
            }
        }
        SoundIndex::build(ph, terms, opt)
    }

    /// An index over terms.
    pub fn build(ph: &Phonemizer, terms: Vec<Term>, opt: &IndexOptions) -> SoundIndex {
        let mut ix = SoundIndex {
            postings: vec![Vec::new(); class::COUNT * class::COUNT],
            ordinary: opt
                .ordinary_words
                .iter()
                .map(|w| w.trim().to_lowercase())
                .filter(|w| !w.is_empty())
                .collect(),
            ..SoundIndex::default()
        };
        // (skeleton, phones, term)
        let mut pending: Vec<(Vec<u8>, Vec<Ph>, u32)> = Vec::new();
        let mut skel = Vec::new();
        for (ti, t) in terms.iter().enumerate() {
            let mut seqs: Vec<Vec<Ph>> = Vec::new();
            if let Some(ipa) = &t.ipa {
                let mut v = Vec::new();
                parse_ipa(ipa, t.lang.is_some_and(Lang::is_english), &mut v);
                tidy(&mut v);
                if !v.is_empty() {
                    seqs.push(v);
                }
            }
            if let Some(say) = &t.say {
                ix.max_say_words = ix.max_say_words.max(tokenize(say).len());
                for v in readings(ph, say, t.lang, &opt.langs) {
                    if !seqs.contains(&v) {
                        seqs.push(v);
                    }
                }
            }
            if seqs.is_empty() {
                seqs = readings(ph, &t.word, t.lang, &opt.langs);
            }
            seqs.retain(|v| v.len() >= 2 && v.len() < u16::MAX as usize);
            seqs.truncate(8);
            if seqs.is_empty() {
                ix.skipped.push((ti, "no phonemes".into()));
            }
            for v in seqs {
                skeleton(&v, &mut skel);
                pending.push((skel.clone(), v, ti as u32));
            }
            let key = exact_key(&t.word);
            if !key.is_empty() {
                ix.max_term_words = ix.max_term_words.max(key.split(' ').count());
                ix.exact.entry(key).or_insert(ti as u32);
            }
        }
        // Sequences with the same skeleton form one group of the pre-filter.
        pending.sort_by(|a, b| (a.0.len(), &a.0, a.2).cmp(&(b.0.len(), &b.0, b.2)));
        let mut dels = Vec::new();
        let mut last: Option<Vec<u8>> = None;
        for (skel, v, term) in pending {
            let id = ix.variants.len() as u32;
            if last.as_ref() != Some(&skel) {
                let group = ix.groups.len() as u32;
                while ix.by_len.len() <= skel.len() {
                    ix.by_len.push(group);
                }
                let set = bigrams(&skel);
                for k in 0..class::COUNT * class::COUNT {
                    if set[k / 64] >> (k % 64) & 1 == 1 {
                        ix.postings[k].push(group);
                    }
                }
                ix.groups.push(Group {
                    set,
                    bigrams: (set[0].count_ones() + set[1].count_ones()) as u8,
                    first: id,
                    end: id,
                });
                last = Some(skel);
            }
            if let Some(g) = ix.groups.last_mut() {
                g.end = id + 1;
            }
            del_costs(&v, &mut dels);
            ix.variants.push(Variant {
                off: ix.phones.len() as u32,
                len: v.len() as u16,
                term,
                weight: dels.iter().map(|d| *d as u32).sum(),
            });
            ix.phones.extend_from_slice(&v);
            ix.dels.extend_from_slice(&dels);
        }
        ix.by_len.push(ix.groups.len() as u32);
        ix.terms = terms;
        ix
    }

    /// The terms, in the order they were given.
    pub fn terms(&self) -> &[Term] {
        &self.terms
    }

    /// Number of terms.
    pub fn len(&self) -> usize {
        self.terms.len()
    }

    /// True when the index has no terms.
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// Number of stored phoneme sequences (a term has one or more).
    pub fn variants(&self) -> usize {
        self.variants.len()
    }

    /// Terms that were left out of the sound index (index, reason); they are
    /// still found when the transcript spells them out.
    pub fn skipped(&self) -> &[(usize, String)] {
        &self.skipped
    }

    /// The phoneme sequences stored for a term, in the matcher's own
    /// broad notation (to see how a term is heard).
    pub fn sounds(&self, term: usize) -> Vec<String> {
        self.variants
            .iter()
            .enumerate()
            .filter(|(_, v)| v.term as usize == term)
            .map(|(id, _)| show(self.variant(id).0))
            .collect()
    }

    fn variant(&self, id: usize) -> (&[Ph], &[u8]) {
        let v = &self.variants[id];
        let r = v.off as usize..v.off as usize + v.len as usize;
        (&self.phones[r.clone()], &self.dels[r])
    }
}

// ---------------------------------------------------------------------------
// Matching

/// The default score a correction needs (see `docs/soundalike.md`).
pub const DEFAULT_THRESHOLD: f32 = 0.80;
/// The default score a correction of a [`SpanKind::Uncertain`] span needs.
pub const DEFAULT_UNCERTAIN_THRESHOLD: f32 = 0.90;
/// The default score a correction of ordinary words needs.
pub const DEFAULT_ORDINARY_THRESHOLD: f32 = 0.92;

/// How candidates are searched.
#[derive(Clone, Debug, PartialEq)]
pub struct MatchOptions {
    /// Longest span, in words (1 to 8). Spans grow to the longest spoken
    /// form of the vocabulary (up to 6 words) when that is longer.
    pub max_words: usize,
    /// Lowest score that is reported.
    pub min_score: f32,
    /// Report every span and term above `min_score` instead of the best
    /// non-overlapping ones.
    pub all: bool,
    /// Use the pre-filter (off: compare every span with every term).
    pub prefilter: bool,
    /// Also read Latin-script transcript words as romanised Hindi
    /// ("Kotari" as "कोटारी").
    pub romanised: bool,
}

impl Default for MatchOptions {
    fn default() -> MatchOptions {
        MatchOptions {
            max_words: 3,
            min_score: 0.6,
            all: false,
            prefilter: true,
            romanised: true,
        }
    }
}

/// A place in the transcript that sounds like a term.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    /// Byte range of the span in the transcript.
    pub src: Range<usize>,
    /// Index of the term in [`SoundIndex::terms`].
    pub term: usize,
    /// Similarity, 0 to 1 (1: the same phonemes, five or more of them;
    /// shorter sequences score a little lower).
    pub score: f32,
    /// The transcript already has the term here (compared without case).
    pub exact: bool,
    /// Whitespace-separated words of the transcript the span touches.
    pub words: Range<usize>,
    /// How likely the span is to be ordinary speech rather than a misheard
    /// term, judged from its words alone.
    pub kind: SpanKind,
}

/// What the words of a span say about it, apart from their sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpanKind {
    /// An English transcript has a word here that is not in the English
    /// lexicon ("Calpana", "Niggum"): the recogniser wrote down a sound.
    Unknown,
    /// Nothing can be said: known words with a capital inside a sentence
    /// ("Coat Harry"), or words of a language without a full lexicon here
    /// that are not on the list of frequent words.
    Uncertain,
    /// Every word is an ordinary word ("at the", "send"): such a span is
    /// usually just what was said.
    Ordinary,
}

impl SpanKind {
    /// The name used in JSON results.
    pub fn name(self) -> &'static str {
        match self {
            SpanKind::Unknown => "unknown",
            SpanKind::Uncertain => "uncertain",
            SpanKind::Ordinary => "ordinary",
        }
    }
}

/// Lowest set-overlap (Dice) of skeleton bigrams a term needs to be compared.
const MIN_DICE: f32 = 0.6;
/// About how many stored sequences are compared in full for one span.
const MAX_COMPARED: usize = 128;

/// Readings tried for a transcript word: the language's own, a second
/// one, and the romanised-Indic one.
const READINGS: usize = 3;

/// What a short pair of sequences loses: a few phonemes that agree are
/// weak evidence (0.06 for every phoneme short of five).
fn short_penalty(norm: u32) -> f32 {
    0.06 * 500u32.saturating_sub(norm) as f32 / 100.0
}

struct Scratch {
    seen: Vec<bool>,
    touched: Vec<u32>,
    lists: Vec<(u32, u32, u32)>,
    row: Vec<u32>,
    skel: Vec<u8>,
    dels: Vec<u8>,
    seq: Vec<Ph>,
    ranked: Vec<(u32, u32)>,
}

impl SoundIndex {
    /// Best (score, term) pairs for one phone sequence, added to `best`
    /// (term → score).
    fn lookup(&self, s: &mut Scratch, opt: &MatchOptions, best: &mut HashMap<u32, f32>) {
        let seq = std::mem::take(&mut s.seq);
        del_costs(&seq, &mut s.dels);
        let weight: u32 = s.dels.iter().map(|d| *d as u32).sum();
        // Unless every term is wanted, only a term that beats the best one
        // found for the span so far matters: the comparison stops early.
        let mut floor = if opt.all {
            opt.min_score
        } else {
            best.values().fold(opt.min_score, |a, b| a.max(*b))
        };
        let mut compare = |s: &mut Scratch, id: usize| {
            let v = &self.variants[id];
            let norm = weight.max(v.weight).max(1);
            let short = short_penalty(norm);
            let room = 1.0 - short - floor;
            if room < 0.0 {
                return;
            }
            let limit = (room * norm as f32) as u32;
            let (p, d) = self.variant(id);
            if let Some(dist) = distance(&seq, &s.dels, p, d, limit, &mut s.row) {
                let score = 1.0 - short - dist as f32 / norm as f32;
                let e = best.entry(v.term).or_insert(0.0);
                if score > *e {
                    *e = score;
                }
                if !opt.all && score > floor {
                    floor = score;
                }
            }
        };
        if !opt.prefilter {
            for id in 0..self.variants.len() {
                compare(s, id);
            }
            s.seq = seq;
            return;
        }
        skeleton(&seq, &mut s.skel);
        let set = bigrams(&s.skel);
        let nq = set[0].count_ones() + set[1].count_ones();
        // Terms whose skeleton is much shorter or longer cannot be close.
        let n = s.skel.len();
        let slack = 2 + n / 3;
        let lo = self.by_len[n.saturating_sub(slack).min(self.by_len.len() - 1)];
        let hi = self.by_len[(n + slack + 1).min(self.by_len.len() - 1)];
        // The posting lists of the span's bigrams, within those lengths.
        s.lists.clear();
        for k in 0..class::COUNT * class::COUNT {
            if set[k / 64] >> (k % 64) & 1 == 1 {
                let list = &self.postings[k];
                let from = list.partition_point(|&id| id < lo);
                let to = from + list[from..].partition_point(|&id| id < hi);
                s.lists.push((k as u32, from as u32, to as u32));
            }
        }
        // A group must share `need` bigrams with the span to pass, so one
        // that is in none of the lists left after dropping the `need - 1`
        // longest cannot pass: those lists need not be read.
        let need = (MIN_DICE * (nq + 1) as f32 / 2.0).ceil().max(1.0) as usize;
        s.lists
            .sort_unstable_by_key(|l| std::cmp::Reverse(l.2 - l.1));
        s.touched.clear();
        for &(k, from, to) in s.lists.iter().skip(need - 1) {
            for &id in &self.postings[k as usize][from as usize..to as usize] {
                let seen = &mut s.seen[id as usize];
                if !*seen {
                    *seen = true;
                    s.touched.push(id);
                }
            }
        }
        s.ranked.clear();
        for &id in &s.touched {
            s.seen[id as usize] = false;
            let g = &self.groups[id as usize];
            let shared = (set[0] & g.set[0]).count_ones() + (set[1] & g.set[1]).count_ones();
            let total = nq + g.bigrams as u32;
            if (2 * shared) as f32 >= MIN_DICE * total as f32 {
                // Rank by overlap, in thousandths.
                s.ranked.push((2000 * shared / total, id));
            }
        }
        let order = |a: &(u32, u32), b: &(u32, u32)| b.0.cmp(&a.0).then(a.1.cmp(&b.1));
        if s.ranked.len() > MAX_COMPARED {
            // Every group holds at least one sequence.
            s.ranked.select_nth_unstable_by(MAX_COMPARED, order);
            s.ranked.truncate(MAX_COMPARED);
        }
        s.ranked.sort_unstable_by(order);
        let ranked = std::mem::take(&mut s.ranked);
        let mut compared = 0;
        for &(_, id) in &ranked {
            let g = &self.groups[id as usize];
            for v in g.first..g.end {
                compare(s, v as usize);
            }
            compared += (g.end - g.first) as usize;
            if compared >= MAX_COMPARED {
                break;
            }
        }
        s.ranked = ranked;
        s.seq = seq;
    }

    /// Candidates in `text`, a transcript in language `lang`, in order of
    /// position. Without [`MatchOptions::all`] the candidates do not
    /// overlap: where several spans or terms compete, an exact occurrence
    /// wins, then the higher score.
    pub fn find(
        &self,
        ph: &Phonemizer,
        text: &str,
        lang: Lang,
        opt: &MatchOptions,
    ) -> Vec<Candidate> {
        let tokens = tokenize(text);
        if tokens.is_empty() || self.terms.is_empty() {
            return Vec::new();
        }
        // A spoken form may have more words than the usual span.
        let max_words = opt.max_words.max(self.max_say_words.min(6)).clamp(1, 8);
        let mut found: Vec<Candidate> = Vec::new();
        let span = |i: usize, n: usize| -> Option<Range<usize>> {
            let last = i + n - 1;
            if last >= tokens.len() {
                return None;
            }
            for k in i..last {
                if !joinable(&text[tokens[k].range.end..tokens[k + 1].range.start]) {
                    return None;
                }
            }
            Some(tokens[i].range.start..tokens[last].range.end)
        };
        let words = |i: usize, n: usize| tokens[i].word..tokens[i + n - 1].word + 1;
        // Terms that are already written out.
        let mut exact_spans: Vec<(usize, usize)> = Vec::new();
        let lower: Vec<String> = tokens
            .iter()
            .map(|t| {
                text[t.range.clone()]
                    .chars()
                    .flat_map(char::to_lowercase)
                    .collect()
            })
            .collect();
        for i in 0..tokens.len() {
            for n in 1..=self.max_term_words.min(tokens.len() - i) {
                let Some(src) = span(i, n) else { break };
                let key = lower[i..i + n].join(" ");
                if let Some(&term) = self.exact.get(&key) {
                    found.push(Candidate {
                        src,
                        term: term as usize,
                        score: 1.0,
                        exact: true,
                        words: words(i, n),
                        kind: SpanKind::Uncertain,
                    });
                    exact_spans.push((i, n));
                }
            }
        }
        // The readings of every word: the language's own, and a second one
        // where another reading is plausible.
        let primary = if lang.is_english() {
            options(lang, Mode::Improved, false)
        } else {
            options(lang, Mode::Improved, lang.hindi_like())
        };
        let secondary = if lang.is_english() {
            options(lang, Mode::Compatible, false)
        } else {
            options(Lang::EnUs, Mode::Compatible, false)
        };
        let mut infos: Vec<std::sync::Arc<WordInfo>> = Vec::with_capacity(tokens.len());
        for t in &tokens {
            let w = &text[t.range.clone()];
            if let Some(info) = self.cache.get(lang, w) {
                infos.push(info);
                continue;
            }
            let mut r: [Vec<Ph>; READINGS] = Default::default();
            read(ph, w, &primary, &mut r[0]);
            if lang.is_english() || first_script(w) == Script::Latin {
                read(ph, w, &secondary, &mut r[1]);
            }
            if !read_romanised(ph, w, &mut r[2]) {
                r[2].clear();
            }
            for k in 1..READINGS {
                if r[..k].contains(&r[k]) {
                    r[k].clear();
                }
            }
            let info = std::sync::Arc::new(WordInfo {
                readings: r,
                ordinary: ordinary(ph, w)
                    || (!self.ordinary.is_empty() && self.ordinary.contains(&w.to_lowercase())),
            });
            self.cache.put(lang, w, info.clone());
            infos.push(info);
        }
        // In a transcript written in Title Case capitals say nothing.
        let capitals = tokens
            .iter()
            .skip(1)
            .filter(|t| text[t.range.clone()].starts_with(char::is_uppercase))
            .count();
        // Nor do they in other languages' transcripts, where Latin-script
        // words are capitalised without system.
        let title_case =
            !lang.is_english() || tokens.len() >= 5 && 5 * capitals > 3 * (tokens.len() - 1);
        let mut plain: Vec<bool> = Vec::with_capacity(tokens.len());
        // Words the English lexicon would have if they were words.
        let novel: Vec<bool> = tokens
            .iter()
            .zip(&infos)
            .map(|(t, info)| {
                lang.is_english()
                    && !info.ordinary
                    && first_script(&text[t.range.clone()]) == Script::Latin
            })
            .collect();
        for (k, t) in tokens.iter().enumerate() {
            let w = &text[t.range.clone()];
            // A capital inside a sentence marks what the recogniser took
            // for a name, whatever the word.
            let before = &text[..t.range.start];
            let prev = k.checked_sub(1).map(|j| &text[tokens[j].range.clone()]);
            plain.push(infos[k].ordinary && (title_case || !proper_noun(w, before, prev)));
        }
        let kind_of = |i: usize, n: usize| {
            if novel[i..i + n].iter().any(|x| *x) {
                SpanKind::Unknown
            } else if plain[i..i + n].iter().all(|x| *x) {
                SpanKind::Ordinary
            } else {
                SpanKind::Uncertain
            }
        };
        for (c, &(i, n)) in found.iter_mut().zip(&exact_spans) {
            c.kind = kind_of(i, n);
        }
        let mut s = Scratch {
            seen: vec![false; self.groups.len()],
            touched: Vec::new(),
            lists: Vec::new(),
            row: Vec::new(),
            skel: Vec::new(),
            dels: Vec::new(),
            seq: Vec::new(),
            ranked: Vec::new(),
        };
        let mut best: HashMap<u32, f32> = HashMap::new();
        for i in 0..tokens.len() {
            for n in 1..=max_words.min(tokens.len() - i) {
                let Some(src) = span(i, n) else { break };
                best.clear();
                for k in 0..READINGS {
                    if k == 2 && !opt.romanised {
                        continue;
                    }
                    let span = &infos[i..i + n];
                    if k > 0 && span.iter().all(|w| w.readings[k].is_empty()) {
                        continue;
                    }
                    s.seq.clear();
                    for w in span {
                        let r = &w.readings;
                        s.seq
                            .extend_from_slice(if r[k].is_empty() { &r[0] } else { &r[k] });
                    }
                    tidy(&mut s.seq);
                    if s.seq.len() < 2 {
                        continue;
                    }
                    self.lookup(&mut s, opt, &mut best);
                }
                if best.is_empty() {
                    continue;
                }
                let key = lower[i..i + n].join(" ");
                let mut hits: Vec<(u32, f32)> = best.iter().map(|(t, sc)| (*t, *sc)).collect();
                hits.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
                if !opt.all {
                    hits.truncate(1);
                }
                for (term, score) in hits {
                    // An exact occurrence was reported above.
                    if self.exact.get(&key) == Some(&term) {
                        continue;
                    }
                    found.push(Candidate {
                        src: src.clone(),
                        term: term as usize,
                        score,
                        exact: false,
                        words: words(i, n),
                        kind: kind_of(i, n),
                    });
                }
            }
        }
        if !opt.all {
            found.sort_by(|a, b| {
                b.exact
                    .cmp(&a.exact)
                    .then(b.score.total_cmp(&a.score))
                    .then((b.src.end - b.src.start).cmp(&(a.src.end - a.src.start)))
                    .then(a.src.start.cmp(&b.src.start))
            });
            let mut kept: Vec<Candidate> = Vec::new();
            for c in found {
                if kept
                    .iter()
                    .all(|k| c.src.end <= k.src.start || c.src.start >= k.src.end)
                {
                    kept.push(c);
                }
            }
            found = kept;
        }
        found.sort_by(|a, b| {
            a.src
                .start
                .cmp(&b.src.start)
                .then(a.src.end.cmp(&b.src.end))
                .then(b.score.total_cmp(&a.score))
        });
        found
    }
}

// ---------------------------------------------------------------------------
// Applying corrections

/// When a candidate becomes a correction.
#[derive(Clone, Debug, PartialEq)]
pub struct ApplyOptions {
    /// How candidates are searched. `min_score` is the lowest score among
    /// [`Applied::candidates`] (by default the default threshold; it is
    /// lowered to `threshold` when that is lower).
    pub search: MatchOptions,
    /// Lowest score that is applied.
    pub threshold: f32,
    /// Lowest score that is applied to a span of [`SpanKind::Uncertain`]
    /// (when higher than `threshold`).
    pub uncertain_threshold: f32,
    /// Lowest score that is applied to a span of [`SpanKind::Ordinary`]
    /// (when higher than `threshold`).
    pub ordinary_threshold: f32,
    /// The recogniser's confidence for each whitespace-separated word of
    /// the transcript, 0 to 1. With it, a span is corrected only when the
    /// mean confidence of its words is at most `max_confidence`.
    pub confidence: Option<Vec<f32>>,
    /// See `confidence`.
    pub max_confidence: f32,
    /// Also restore the term's capitals where the transcript has the term
    /// in another case ("paytm" → "Paytm").
    pub fix_case: bool,
}

impl Default for ApplyOptions {
    fn default() -> ApplyOptions {
        ApplyOptions {
            // Candidates below the threshold are of no use to `apply`,
            // and a higher floor makes the search quicker.
            search: MatchOptions {
                min_score: DEFAULT_THRESHOLD,
                ..MatchOptions::default()
            },
            threshold: DEFAULT_THRESHOLD,
            uncertain_threshold: DEFAULT_UNCERTAIN_THRESHOLD,
            ordinary_threshold: DEFAULT_ORDINARY_THRESHOLD,
            confidence: None,
            max_confidence: 0.9,
            fix_case: true,
        }
    }
}

/// One replacement made by [`SoundIndex::apply`].
#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    /// Byte range that was replaced, in the transcript.
    pub src: Range<usize>,
    /// Byte range of the replacement, in the new text.
    pub out: Range<usize>,
    /// The replaced text.
    pub from: String,
    /// The text put in its place (the term).
    pub to: String,
    /// Index of the term in [`SoundIndex::terms`].
    pub term: usize,
    /// Score of the candidate.
    pub score: f32,
    /// Only the case differed.
    pub case_only: bool,
}

/// The result of [`SoundIndex::apply`].
#[derive(Clone, Debug, PartialEq)]
pub struct Applied {
    /// The transcript with the corrections made.
    pub text: String,
    /// Every replacement, in order.
    pub changes: Vec<Change>,
    /// All candidates that were considered, applied or not.
    pub candidates: Vec<Candidate>,
}

impl SoundIndex {
    /// Correct `text`: replace every candidate that passes
    /// [`ApplyOptions`] with its term, and say what was replaced.
    pub fn apply(&self, ph: &Phonemizer, text: &str, lang: Lang, opt: &ApplyOptions) -> Applied {
        let mut search = opt.search.clone();
        search.all = false;
        search.min_score = search.min_score.min(opt.threshold);
        let candidates = self.find(ph, text, lang, &search);
        let mut out = String::with_capacity(text.len());
        let mut changes = Vec::new();
        let mut pos = 0;
        for c in &candidates {
            let from = &text[c.src.clone()];
            let to = self.terms[c.term].word.as_str();
            let need = opt.threshold.max(match c.kind {
                SpanKind::Unknown => 0.0,
                SpanKind::Uncertain => opt.uncertain_threshold,
                SpanKind::Ordinary => opt.ordinary_threshold,
            });
            if from == to || (c.exact && !opt.fix_case) || (!c.exact && c.score < need) {
                continue;
            }
            if !c.exact {
                if let Some(conf) = &opt.confidence {
                    let words = &conf[c.words.start.min(conf.len())..c.words.end.min(conf.len())];
                    if words.is_empty()
                        || words.iter().sum::<f32>() / words.len() as f32 > opt.max_confidence
                    {
                        continue;
                    }
                }
            }
            out.push_str(&text[pos..c.src.start]);
            let start = out.len();
            out.push_str(to);
            changes.push(Change {
                src: c.src.clone(),
                out: start..out.len(),
                from: from.to_string(),
                to: to.to_string(),
                term: c.term,
                score: c.score,
                case_only: c.exact,
            });
            pos = c.src.end;
        }
        out.push_str(&text[pos..]);
        Applied {
            text: out,
            changes,
            candidates,
        }
    }
}

// ---------------------------------------------------------------------------
// JSON

/// Character and UTF-16 offsets of every char boundary of `s`, by byte.
fn offset_tables(s: &str) -> (Vec<usize>, Vec<usize>) {
    let mut cp = vec![0usize; s.len() + 1];
    let mut u16s = vec![0usize; s.len() + 1];
    let (mut c, mut u) = (0, 0);
    for (i, ch) in s.char_indices() {
        cp[i] = c;
        u16s[i] = u;
        c += 1;
        u += ch.len_utf16();
    }
    cp[s.len()] = c;
    u16s[s.len()] = u;
    (cp, u16s)
}

fn push_range(out: &mut String, r: &Range<usize>, cp: &[usize], u: &[usize]) {
    out.push_str(&format!(
        "{{\"byte\":[{},{}],\"char\":[{},{}],\"utf16\":[{},{}]}}",
        r.start, r.end, cp[r.start], cp[r.end], u[r.start], u[r.end]
    ));
}

fn score_json(x: f32) -> String {
    format!("{:.4}", x.clamp(0.0, 1.0))
}

impl SoundIndex {
    /// An index from a vocabulary in text form: a word list (TSV, PLS or
    /// JSON, see [`WordList::parse_any`]), a JSON array of terms (strings,
    /// or objects with `word` and optional `lang`, `ipa`, `say`), or plain
    /// text with one term per line. `options` is a JSON object with
    /// optional `langs` and `ordinary_words` arrays (see [`IndexOptions`]).
    pub fn from_text(
        ph: &Phonemizer,
        vocabulary: &str,
        options: &str,
    ) -> Result<SoundIndex, String> {
        use crate::minijson::{parse, Value};
        let mut opt = IndexOptions::default();
        if !options.trim().is_empty() {
            let v = parse(options)?;
            if !matches!(v, Value::Obj(_)) {
                return Err("options: expected a JSON object".into());
            }
            if let Some(Value::Arr(a)) = v.get("langs") {
                opt.langs.clear();
                for l in a {
                    let s = l.as_str().ok_or("langs: expected strings")?;
                    opt.langs
                        .push(Lang::parse(s).ok_or_else(|| format!("unknown language {s:?}"))?);
                }
            }
            if let Some(Value::Arr(a)) = v.get("ordinary_words") {
                for w in a {
                    let w = w.as_str().ok_or("ordinary_words: expected strings")?;
                    opt.ordinary_words.push(w.to_string());
                }
            }
        }
        let trimmed = vocabulary.trim_start();
        if trimmed.starts_with('[') {
            let Value::Arr(items) = parse(vocabulary)? else {
                return Err("expected a JSON array of terms".into());
            };
            let mut terms = Vec::with_capacity(items.len());
            for it in &items {
                match it {
                    Value::Str(s) => terms.push(Term::new(s)),
                    Value::Obj(_) => {
                        let word = it
                            .get("word")
                            .and_then(Value::as_str)
                            .ok_or("a term object needs a \"word\"")?;
                        let text = |k: &str| {
                            it.get(k)
                                .and_then(Value::as_str)
                                .map(str::trim)
                                .filter(|s| !s.is_empty())
                                .map(String::from)
                        };
                        let lang = match text("lang") {
                            Some(l) if l != "*" => Some(
                                Lang::parse(&l).ok_or_else(|| format!("unknown language {l:?}"))?,
                            ),
                            _ => None,
                        };
                        terms.push(Term {
                            word: word.trim().to_string(),
                            lang,
                            ipa: text("ipa"),
                            say: text("say"),
                        });
                    }
                    _ => return Err("terms must be strings or objects".into()),
                }
            }
            terms.retain(|t| !t.word.is_empty());
            return Ok(SoundIndex::build(ph, terms, &opt));
        }
        let looks_like_list = trimmed.starts_with("# uccharan wordlist")
            || trimmed.starts_with('<')
            || trimmed.starts_with('{')
            || vocabulary.contains('\t');
        if looks_like_list {
            let list = WordList::parse_any(vocabulary).map_err(|e| e.to_string())?;
            return Ok(SoundIndex::from_wordlists(ph, &[&list], &opt));
        }
        let terms: Vec<Term> = vocabulary
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(Term::new)
            .collect();
        Ok(SoundIndex::build(ph, terms, &opt))
    }

    fn candidate_json(&self, o: &mut String, c: &Candidate, text: &str, cp: &[usize], u: &[usize]) {
        use crate::minijson::push_str;
        o.push_str("{\"src\":");
        push_range(o, &c.src, cp, u);
        o.push_str(",\"text\":");
        push_str(o, &text[c.src.clone()]);
        o.push_str(",\"term\":");
        push_str(o, &self.terms[c.term].word);
        o.push_str(&format!(
            ",\"index\":{},\"score\":{},\"exact\":{},\"kind\":\"{}\",\"words\":[{},{}]}}",
            c.term,
            score_json(c.score),
            c.exact,
            c.kind.name(),
            c.words.start,
            c.words.end
        ));
    }

    /// [`SoundIndex::find`] with JSON options in and JSON out:
    /// `{"candidates": [{"src": {"byte": [s, e], "char": [s, e], "utf16":
    /// [s, e]}, "text", "term", "index", "score", "exact", "kind", "words": [s,
    /// e]}]}`. Options: `max_words`, `min_score` (default 0.6), `all`,
    /// `prefilter`, `romanised`.
    pub fn find_json(
        &self,
        ph: &Phonemizer,
        text: &str,
        lang: Lang,
        options: &str,
    ) -> Result<String, String> {
        let mut opt = apply_options_from_json(options)?;
        if !options.contains("\"min_score\"") {
            opt.search.min_score = MatchOptions::default().min_score;
        }
        let (cp, u) = offset_tables(text);
        let mut o = String::from("{\"candidates\":[");
        for (i, c) in self.find(ph, text, lang, &opt.search).iter().enumerate() {
            if i > 0 {
                o.push(',');
            }
            self.candidate_json(&mut o, c, text, &cp, &u);
        }
        o.push_str("]}");
        Ok(o)
    }

    /// [`SoundIndex::apply`] with JSON options in and JSON out: `{"text",
    /// "changes": [{"src": {...}, "out": {...}, "from", "to", "index",
    /// "score", "case_only"}], "candidates": [...]}`. Options: those of
    /// [`SoundIndex::find_json`] (`min_score`: the lowest score among the
    /// candidates, by default the lower of 0.8 and `threshold`) and
    /// `threshold`, `uncertain_threshold`,
    /// `ordinary_threshold`,
    /// `confidence` (an array
    /// with one number per whitespace-separated word), `max_confidence`,
    /// `fix_case`.
    pub fn apply_json(
        &self,
        ph: &Phonemizer,
        text: &str,
        lang: Lang,
        options: &str,
    ) -> Result<String, String> {
        use crate::minijson::push_str;
        let opt = apply_options_from_json(options)?;
        if let Some(c) = &opt.confidence {
            let n = text.split_whitespace().count();
            if c.len() != n {
                return Err(format!("confidence has {} values for {n} words", c.len()));
            }
        }
        let a = self.apply(ph, text, lang, &opt);
        let (cp, u) = offset_tables(text);
        let (ocp, ou) = offset_tables(&a.text);
        let mut o = String::from("{\"text\":");
        push_str(&mut o, &a.text);
        o.push_str(",\"changes\":[");
        for (i, c) in a.changes.iter().enumerate() {
            if i > 0 {
                o.push(',');
            }
            o.push_str("{\"src\":");
            push_range(&mut o, &c.src, &cp, &u);
            o.push_str(",\"out\":");
            push_range(&mut o, &c.out, &ocp, &ou);
            o.push_str(",\"from\":");
            push_str(&mut o, &c.from);
            o.push_str(",\"to\":");
            push_str(&mut o, &c.to);
            o.push_str(&format!(
                ",\"index\":{},\"score\":{},\"case_only\":{}}}",
                c.term,
                score_json(c.score),
                c.case_only
            ));
        }
        o.push_str("],\"candidates\":[");
        for (i, c) in a.candidates.iter().enumerate() {
            if i > 0 {
                o.push(',');
            }
            self.candidate_json(&mut o, c, text, &cp, &u);
        }
        o.push_str("]}");
        Ok(o)
    }
}

/// [`ApplyOptions`] from a JSON object (empty text: the defaults). Keys:
/// `max_words`, `min_score`, `all`, `prefilter`, `romanised`, `threshold`,
/// `uncertain_threshold`, `ordinary_threshold`, `confidence`,
/// `max_confidence`, `fix_case`.
pub fn apply_options_from_json(json: &str) -> Result<ApplyOptions, String> {
    use crate::minijson::{parse, Value};
    let mut o = ApplyOptions::default();
    if json.trim().is_empty() {
        return Ok(o);
    }
    let v = parse(json)?;
    let Value::Obj(map) = &v else {
        return Err("options: expected a JSON object".into());
    };
    let num = |k: &str, v: &Value| match v {
        Value::Num(n) => Ok(*n as f32),
        _ => Err(format!("{k}: expected a number")),
    };
    let flag = |k: &str, v: &Value| {
        v.as_bool()
            .ok_or_else(|| format!("{k}: expected true or false"))
    };
    for (k, val) in map {
        match k.as_str() {
            "max_words" => o.search.max_words = num(k, val)?.max(1.0) as usize,
            "min_score" => o.search.min_score = num(k, val)?.clamp(0.0, 1.0),
            "all" => o.search.all = flag(k, val)?,
            "prefilter" => o.search.prefilter = flag(k, val)?,
            "romanised" => o.search.romanised = flag(k, val)?,
            // A threshold above 1 applies nothing.
            "threshold" => o.threshold = num(k, val)?.max(0.0),
            "uncertain_threshold" => o.uncertain_threshold = num(k, val)?.max(0.0),
            "ordinary_threshold" => o.ordinary_threshold = num(k, val)?.max(0.0),
            "max_confidence" => o.max_confidence = num(k, val)?,
            "fix_case" => o.fix_case = flag(k, val)?,
            "confidence" => match val {
                Value::Null => o.confidence = None,
                Value::Arr(a) => {
                    let mut c = Vec::with_capacity(a.len());
                    for x in a {
                        c.push(num(k, x)?);
                    }
                    o.confidence = Some(c);
                }
                _ => return Err("confidence: expected an array of numbers".into()),
            },
            other => return Err(format!("unknown option {other:?}")),
        }
    }
    Ok(o)
}

/// Similarity of two IPA strings with the matcher's measure, 0 to 1.
pub fn ipa_similarity(a: &str, b: &str) -> f32 {
    let (mut x, mut y) = (Vec::new(), Vec::new());
    parse_ipa(a, false, &mut x);
    parse_ipa(b, false, &mut y);
    tidy(&mut x);
    tidy(&mut y);
    let (mut dx, mut dy) = (Vec::new(), Vec::new());
    del_costs(&x, &mut dx);
    del_costs(&y, &mut dy);
    let norm = dx
        .iter()
        .map(|d| *d as u32)
        .sum::<u32>()
        .max(dy.iter().map(|d| *d as u32).sum())
        .max(1);
    let d = distance(&x, &dx, &y, &dy, u32::MAX, &mut Vec::new()).unwrap_or(norm);
    (1.0 - d as f32 / norm as f32).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index(terms: &[&str]) -> (Phonemizer, SoundIndex) {
        let p = Phonemizer::new();
        let ix = SoundIndex::from_terms(&p, terms, &IndexOptions::default());
        (p, ix)
    }

    #[test]
    fn close_sounds_are_cheap() {
        // Voicing, aspiration, length, retroflex / dental.
        assert!(ipa_similarity("pal", "bal") > 0.85);
        assert!(ipa_similarity("kʰaːn", "kaːn") > 0.9);
        assert!(ipa_similarity("ʈiːm", "tim") > 0.9);
        assert!(ipa_similarity("d̪aːl", "ɖaːl") > 0.9);
        // Unrelated sounds are not.
        assert!(ipa_similarity("pal", "sal") < 0.75);
        assert!(ipa_similarity("kaːm", "kaːs") < 0.75);
        // Schwa and stress are nearly free.
        assert!(ipa_similarity("kəmˈaːl", "kmaːl") > 0.9);
        assert_eq!(ipa_similarity("ˈkamal", "kamal"), 1.0);
        assert_eq!(ipa_similarity("", ""), 1.0);
    }

    #[test]
    fn mishearings() {
        let (p, ix) = index(&["Kothari", "Paytm", "Natarajan", "Zerodha", "Kalpana"]);
        let o = ApplyOptions::default();
        for (heard, want) in [
            (
                "please send it to Mr. Coat Harry by Friday",
                "please send it to Mr. Kothari by Friday",
            ),
            (
                "I paid with pay tm yesterday",
                "I paid with Paytm yesterday",
            ),
            ("ask Kalpna to send it", "ask Kalpana to send it"),
        ] {
            let a = ix.apply(&p, heard, Lang::EnUs, &o);
            assert_eq!(a.text, want, "{heard}: {:?}", a.candidates);
            assert_eq!(a.changes.len(), 1);
            let c = &a.changes[0];
            assert_eq!(&heard[c.src.clone()], c.from);
            assert_eq!(&a.text[c.out.clone()], c.to);
        }
    }

    #[test]
    fn exact_terms_and_plain_text_are_left_alone() {
        let (p, ix) = index(&["Kothari", "Paytm"]);
        let o = ApplyOptions::default();
        let a = ix.apply(&p, "Mr. Kothari paid with Paytm.", Lang::EnUs, &o);
        assert!(a.changes.is_empty());
        assert_eq!(a.candidates.len(), 2);
        assert!(a.candidates.iter().all(|c| c.exact && c.score == 1.0));
        let a = ix.apply(
            &p,
            "The weather was fine and we walked to the station.",
            Lang::EnUs,
            &o,
        );
        assert!(a.changes.is_empty(), "{:?}", a.changes);
        assert_eq!(a.text, "The weather was fine and we walked to the station.");
        // Case is restored, and reported.
        let a = ix.apply(&p, "pay with paytm", Lang::EnUs, &o);
        assert_eq!(a.text, "pay with Paytm");
        assert!(a.changes[0].case_only);
        let mut keep = o.clone();
        keep.fix_case = false;
        assert!(ix
            .apply(&p, "pay with paytm", Lang::EnUs, &keep)
            .changes
            .is_empty());
    }

    #[test]
    fn cross_script() {
        // Latin vocabulary, Devanagari transcript.
        let (p, ix) = index(&["Kothari", "Agarwal"]);
        let a = ix.apply(
            &p,
            "कोठारी जी को फ़ोन करना है।",
            Lang::Hi,
            &ApplyOptions::default(),
        );
        assert_eq!(a.text, "Kothari जी को फ़ोन करना है।", "{:?}", a.candidates);
        // Devanagari vocabulary, Latin transcript.
        let (p, ix) = index(&["कोठारी", "कुलकर्णी"]);
        let a = ix.apply(
            &p,
            "call Mr. Kulkarni now",
            Lang::EnUs,
            &ApplyOptions::default(),
        );
        assert_eq!(a.text, "call Mr. कुलकर्णी now", "{:?}", a.candidates);
    }

    #[test]
    fn words_of_a_transcript() {
        let text = "Mr. O'Brien's e-cart, कोठारी जी है। 2G";
        let words: Vec<&str> = tokenize(text)
            .iter()
            .map(|t| &text[t.range.clone()])
            .collect();
        assert_eq!(
            words,
            ["Mr", "O'Brien", "e", "cart", "कोठारी", "जी", "है", "2G"]
        );
        let index: Vec<usize> = tokenize(text).iter().map(|t| t.word).collect();
        assert_eq!(index, [0, 1, 2, 2, 3, 4, 5, 6]);
        assert_eq!(exact_key("Wow  Momo"), "wow momo");
    }

    #[test]
    fn span_kinds() {
        let p = Phonemizer::new();
        let opt = IndexOptions {
            ordinary_words: vec!["Kotari".into()],
            ..IndexOptions::default()
        };
        let plain = SoundIndex::from_terms(&p, &["Kothari"], &IndexOptions::default());
        let listed = SoundIndex::from_terms(&p, &["Kothari"], &opt);
        let kind = |ix: &SoundIndex, text: &str, lang: Lang| {
            ix.find(&p, text, lang, &MatchOptions::default())[0].kind
        };
        // Not an English word: the recogniser wrote down a sound.
        assert_eq!(
            kind(&plain, "ask kotari now", Lang::EnUs),
            SpanKind::Unknown
        );
        // English words, but with a capital inside the sentence.
        assert_eq!(
            kind(&plain, "ask Coat Harry now", Lang::EnUs),
            SpanKind::Uncertain
        );
        // English words and nothing else to go by.
        assert_eq!(
            kind(&plain, "ask coat harry now", Lang::EnUs),
            SpanKind::Ordinary
        );
        // A word given as ordinary with the index.
        assert_eq!(
            kind(&listed, "ask kotari now", Lang::EnUs),
            SpanKind::Ordinary
        );
        // Hindi words are judged by the list of frequent words only.
        assert_eq!(kind(&plain, "कोठारी जी", Lang::Hi), SpanKind::Uncertain);
    }

    #[test]
    fn confidence_gate() {
        let (p, ix) = index(&["Kothari"]);
        let text = "send it to Coat Harry";
        let mut o = ApplyOptions {
            confidence: Some(vec![0.99, 0.98, 0.99, 0.97, 0.96]),
            max_confidence: 0.6,
            ..ApplyOptions::default()
        };
        let a = ix.apply(&p, text, Lang::EnUs, &o);
        assert!(a.changes.is_empty());
        assert_eq!(a.candidates.len(), 1);
        assert_eq!(a.candidates[0].words, 3..5);
        o.confidence = Some(vec![0.99, 0.98, 0.99, 0.41, 0.52]);
        assert_eq!(
            ix.apply(&p, text, Lang::EnUs, &o).text,
            "send it to Kothari"
        );
    }

    #[test]
    fn wordlist_and_json() {
        let p = Phonemizer::new();
        let tsv = "# uccharan wordlist 1\nword\tlang\tipa\tsay\nnginx\ten\t\tengine X\nJio\ten\tdʒˈiːoʊ\t\n";
        let ix = SoundIndex::from_text(&p, tsv, "").unwrap();
        assert_eq!(ix.len(), 2);
        let j = ix
            .apply_json(
                &p,
                "restart engine x and call Geo",
                Lang::EnUs,
                r#"{"uncertain_threshold": 0.8}"#,
            )
            .unwrap();
        let v = crate::minijson::parse(&j).unwrap();
        assert_eq!(
            v.get("text").and_then(|t| t.as_str()),
            Some("restart nginx and call Jio")
        );
        let ix = SoundIndex::from_text(
            &p,
            r#"["Kothari", {"word": "Paytm", "say": "pay T M"}]"#,
            r#"{"langs": ["en"]}"#,
        )
        .unwrap();
        let j = ix
            .find_json(&p, "ñ Coat Harry", Lang::EnUs, r#"{"min_score": 0.7}"#)
            .unwrap();
        assert!(
            j.contains(r#""src":{"byte":[3,13],"char":[2,12],"utf16":[2,12]}"#),
            "{j}"
        );
        assert!(j.contains(r#""term":"Kothari""#));
        assert!(ix
            .apply_json(&p, "a b", Lang::EnUs, r#"{"confidence": [0.5]}"#)
            .is_err());
        assert!(apply_options_from_json(r#"{"treshold": 1}"#).is_err());
        let ix = SoundIndex::from_text(&p, "Kothari\n# note\nPaytm\n", "").unwrap();
        assert_eq!(ix.len(), 2);
    }

    #[test]
    fn prefilter_agrees_with_full_search() {
        let (p, ix) = index(&[
            "Kothari",
            "Natarajan",
            "Kalpana",
            "Vistara",
            "Kibana",
            "Mishra",
            "Nigam",
        ]);
        let full = MatchOptions {
            prefilter: false,
            ..MatchOptions::default()
        };
        for text in [
            "Mr. Not Regen called",
            "ask Kelp on it",
            "Dr. Niggum will see you",
            "Keep on, I came",
        ] {
            let a = ix.find(&p, text, Lang::EnUs, &MatchOptions::default());
            let b = ix.find(&p, text, Lang::EnUs, &full);
            assert_eq!(a, b, "{text}");
        }
    }
}
