//! Kashmiri in its two scripts, and romanised input.
//!
//! Kashmiri is written mainly in the Perso-Arabic script, in the official
//! orthography of the Jammu and Kashmir Academy of Art, Culture and
//! Languages, which writes nearly every vowel: the Arabic marks for a i u
//! (َ ِ ُ), the Kashmiri ones for ə (ٔ), ɨ (ٕ) and ɨː (ٟ), the letters ا ٲ
//! for aː əː, وٗ و ۆ ۄ for uː oː o ɔ (ۄا for ɔː), یٖ ی یٚ for iː eː e (final
//! ی is iː, final ے eː, ےٚ e), ؠ for a palatalised consonant (ʲa inside a
//! word, ʲ at its end), ں and ن with ٛ for a nasal, ٛ on ر ی و for the second
//! consonant of a cluster (درٛۆگ droɡ, بیٛور bʲoːr). A word-final ہ carries
//! the vowel mark before it (سُہ su, تہٕ tɨ, یِہ ji), is a after a consonant
//! without a mark (اَداکارہ adaːkaːra) and h after a vowel letter (داہ daːh).
//!
//! [`explicit`] turns a written word into one where every vowel is a single
//! letter: each consonant is followed by its vowel sign or by a virama, and
//! a vowel without a consonant is an independent vowel letter. The language
//! file (`lang/ks.lang`) reads that form; its signs and vowels are Latin
//! letters with marks (ȧ ā ə ǝ … for the signs, Ȧ Ā Ə Ǝ … for the vowels),
//! which never occur in Kashmiri text. A word written without any vowel
//! mark (names, loanwords, casual text: کربلا, شہر) is left to the
//! language file's inherent vowel and Urdu-type deletion, with ی inside a
//! word read iː after the first syllable as in Urdu; frequent function words
//! written without marks are read as their usual spelling (`UNMARKED`).
//!
//! Kashmiri in Devanagari (the 2002 and 2009 proposals: ॅ ॉ or ऺ ऻ for ə əː,
//! ॖ ॗ for ɨ ɨː, ॆ ॊ for e o, ॏ for ɔ, ौ for ɔː, ्य for palatalisation) is
//! first written in Perso-Arabic by [`devanagari_to_arabic`], so that the same
//! word reads the same in both scripts; [`arabic_to_devanagari`] goes the
//! other way for the number and date words uccharan writes into
//! Devanagari-script text (normalise-only mode).
//!
//! Romanised Kashmiri (`koshur chhu akh zabaan`) is written in the
//! Perso-Arabic script by [`transliterate`], after a short list of common
//! words ([`rewrite_roman`]).

use crate::norm::Item;
use crate::text::{script_of, Script};

/// The vowels of Kashmiri.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum V {
    A,
    Aa,
    Schwa,
    Schwa2,
    I,
    Ii,
    Barred,
    Barred2,
    U,
    Uu,
    E,
    Ee,
    O,
    Oo,
    Open,
    Open2,
}

impl V {
    fn long(self) -> bool {
        matches!(
            self,
            V::Aa | V::Schwa2 | V::Ii | V::Barred2 | V::Uu | V::Ee | V::Oo | V::Open2
        )
    }

    /// The vowel sign of the explicit form (after a consonant).
    fn sign(self) -> char {
        match self {
            V::A => 'ȧ',
            V::Aa => 'ā',
            V::Schwa => 'ə',
            V::Schwa2 => 'ǝ',
            V::I => 'ì',
            V::Ii => 'ī',
            V::Barred => 'ɨ',
            V::Barred2 => 'ȉ',
            V::U => 'ù',
            V::Uu => 'ū',
            V::E => 'è',
            V::Ee => 'ē',
            V::O => 'ò',
            V::Oo => 'ō',
            V::Open => 'ɔ',
            V::Open2 => 'ǫ',
        }
    }

    /// The independent vowel letter of the explicit form.
    fn letter(self) -> char {
        match self {
            V::A => 'Ȧ',
            V::Aa => 'Ā',
            V::Schwa => 'Ə',
            V::Schwa2 => 'Ǝ',
            V::I => 'Ì',
            V::Ii => 'Ī',
            V::Barred => 'Ɨ',
            V::Barred2 => 'Ȉ',
            V::U => 'Ù',
            V::Uu => 'Ū',
            V::E => 'È',
            V::Ee => 'Ē',
            V::O => 'Ò',
            V::Oo => 'Ō',
            V::Open => 'Ɔ',
            V::Open2 => 'Ǫ',
        }
    }

    /// Kashmiri Devanagari (2009 proposal): vowel sign after a consonant
    /// (empty for the inherent a) and independent letter.
    fn devanagari(self) -> (&'static str, &'static str) {
        match self {
            V::A => ("", "अ"),
            V::Aa => ("ा", "आ"),
            V::Schwa => ("ऺ", "ॳ"),
            V::Schwa2 => ("ऻ", "ॴ"),
            V::I => ("ि", "इ"),
            V::Ii => ("ी", "ई"),
            V::Barred => ("ॖ", "ॶ"),
            V::Barred2 => ("ॗ", "ॷ"),
            V::U => ("ु", "उ"),
            V::Uu => ("ू", "ऊ"),
            V::E => ("ॆ", "ऎ"),
            V::Ee => ("े", "ए"),
            V::O => ("ॊ", "ऒ"),
            V::Oo => ("ो", "ओ"),
            V::Open => ("ॏ", "ॵ"),
            V::Open2 => ("ौ", "औ"),
        }
    }

    /// Perso-Arabic spelling after a consonant (`end`: at the end of the
    /// word) and at the start of a word.
    fn arabic(self, end: bool) -> (&'static str, &'static str) {
        match self {
            V::A => ("\u{64e}", "اَ"),
            V::Aa => ("ا", "آ"),
            V::Schwa => ("\u{654}", "أ"),
            V::Schwa2 => ("ٲ", "ٲ"),
            V::I => ("\u{650}", "اِ"),
            V::Ii if end => ("ی", "ای"),
            V::Ii => ("یٖ", "ایٖ"),
            V::Barred => ("\u{655}", "إ"),
            V::Barred2 => ("\u{65f}", "ٳ"),
            V::U => ("\u{64f}", "اُ"),
            V::Uu => ("وٗ", "اوٗ"),
            V::E if end => ("ےٚ", "اےٚ"),
            V::E => ("یٚ", "ایٚ"),
            V::Ee if end => ("ے", "اے"),
            V::Ee => ("ی", "ای"),
            V::O => ("ۆ", "اۆ"),
            V::Oo => ("و", "او"),
            V::Open => ("ۄ", "اۄ"),
            V::Open2 => ("ۄا", "اۄا"),
        }
    }
}

/// The vowel of an Arabic short-vowel mark (not ٖ ٗ ٚ, which belong to
/// a letter).
fn mark_vowel(c: char) -> Option<V> {
    Some(match c {
        '\u{64e}' => V::A,
        '\u{650}' => V::I,
        '\u{64f}' => V::U,
        '\u{654}' => V::Schwa,
        '\u{655}' => V::Barred,
        '\u{65f}' => V::Barred2,
        '\u{670}' => V::Aa,
        _ => return None,
    })
}

const ZABAR: char = '\u{64e}';
const PESH: char = '\u{64f}';
const HAMZA_ABOVE: char = '\u{654}';
const HAMZA_BELOW: char = '\u{655}';
const WAVY_HAMZA: char = '\u{65f}';
const SUBSCRIPT_ALEF: char = '\u{656}';
const INVERTED_DAMMA: char = '\u{657}';
const SMALL_V: char = '\u{65a}';
const INVERTED_V: char = '\u{65b}';
const SUKUN: char = '\u{652}';
const SHADDA: char = '\u{651}';
const MADDA: char = '\u{653}';

/// A combining mark of the Arabic block.
fn is_mark(c: char) -> bool {
    matches!(c, '\u{610}'..='\u{61a}' | '\u{64b}'..='\u{65f}' | '\u{670}' | '\u{6d6}'..='\u{6ed}')
}

/// A consonant letter (و ی ہ are handled apart).
fn is_consonant(c: char) -> bool {
    matches!(
        c,
        'ب' | 'پ'
            | 'ت'
            | 'ٹ'
            | 'ث'
            | 'ج'
            | 'چ'
            | 'ح'
            | 'خ'
            | 'د'
            | 'ڈ'
            | 'ذ'
            | 'ر'
            | 'ڑ'
            | 'ز'
            | 'ژ'
            | 'س'
            | 'ش'
            | 'ص'
            | 'ض'
            | 'ط'
            | 'ظ'
            | 'غ'
            | 'ف'
            | 'ق'
            | 'ک'
            | 'گ'
            | 'ل'
            | 'م'
            | 'ن'
            | 'ہ'
    )
}

/// One letter of a word with its marks (ھ makes it aspirated).
#[derive(Clone, Debug, Default)]
struct Unit {
    base: char,
    marks: Vec<char>,
    aspirated: bool,
}

impl Unit {
    fn has(&self, m: char) -> bool {
        self.marks.contains(&m)
    }
    fn vowel_mark(&self) -> Option<V> {
        self.marks.iter().find_map(|&m| mark_vowel(m))
    }
}

/// The letters of a word after spelling variants are made one (Arabic kaf
/// and ye, ۅ for ۄ, ۍ for ؠ, precomposed and decomposed hamza and madda
/// forms), each with its marks.
fn units(word: &str) -> Vec<Unit> {
    let mut cs: Vec<char> = Vec::with_capacity(word.len());
    for c in word.chars() {
        match c {
            'ي' | 'ى' | 'ې' | 'ٮ' | 'ئ' => cs.push('ی'),
            'ك' => cs.push('ک'),
            'ه' => cs.push('ہ'),
            'ة' | 'ۃ' => cs.push('ت'),
            'ۅ' => cs.push('ۄ'),
            'ۍ' => cs.push('ؠ'),
            'ۯ' => cs.push('ر'),
            'ٱ' => cs.push('ا'),
            'أ' => cs.extend(['ا', HAMZA_ABOVE]),
            'إ' => cs.extend(['ا', HAMZA_BELOW]),
            'آ' => cs.extend(['ا', MADDA]),
            'ٳ' => cs.extend(['ا', WAVY_HAMZA]),
            'ؤ' => cs.extend(['و', HAMZA_ABOVE]),
            'ۂ' => cs.extend(['ہ', HAMZA_ABOVE]),
            'ۀ' => cs.push('ہ'),
            'ێ' => cs.extend(['ی', SMALL_V]),
            'ۓ' => cs.push('ے'),
            '\u{65d}' => cs.push(INVERTED_DAMMA),
            '\u{619}' => cs.push(PESH),
            'ـ' | '\u{200c}' | '\u{200d}' | '\u{200e}' | '\u{200f}' | 'ء' => {}
            _ => cs.push(c),
        }
    }
    let mut out: Vec<Unit> = Vec::with_capacity(cs.len());
    for c in cs {
        if is_mark(c) {
            if let Some(u) = out.last_mut() {
                u.marks.push(c);
            }
        } else if c == 'ھ' {
            match out.last_mut() {
                Some(u) if is_consonant(u.base) && !u.aspirated => u.aspirated = true,
                _ => out.push(Unit {
                    base: 'ہ',
                    ..Unit::default()
                }),
            }
        } else {
            out.push(Unit {
                base: c,
                ..Unit::default()
            });
        }
    }
    out
}

/// True when the word writes its vowels (a vowel mark or a letter of the
/// Kashmiri alphabet that only a marked spelling uses).
fn marked(us: &[Unit]) -> bool {
    us.iter().any(|u| {
        matches!(u.base, 'ٲ' | 'ۄ' | 'ۆ' | 'ؠ')
            || u.marks.iter().any(|&m| {
                mark_vowel(m).is_some()
                    || matches!(
                        m,
                        SUBSCRIPT_ALEF | INVERTED_DAMMA | SMALL_V | INVERTED_V | SUKUN
                    )
            })
    })
}

/// Frequent words written without vowel marks, and their usual spelling
/// (sorted): function words of the IndicCorp v2 development sentences that
/// the same text also writes with their marks (چھ as چھِ, its commoner
/// marked spelling; تہ as تہٕ).
const UNMARKED: &[(&str, &str)] = &[
    ("امہ", "اَمہِ"),
    ("اکھ", "اَکھ"),
    ("تم", "تِم"),
    ("تہ", "تہٕ"),
    ("منز", "مَنٛز"),
    ("ونان", "وَنان"),
    ("پتہ", "پَتہٕ"),
    ("چھ", "چھِ"),
    ("چھہ", "چھِ"),
    ("چھے", "چھےٚ"),
    ("کرنہ", "کَرنہٕ"),
    ("کشیر", "کٔشیٖر"),
    ("کور", "کۆر"),
    ("ہنز", "ہٕنٛز"),
    ("یتھ", "یَتھ"),
    ("یس", "یُس"),
    ("یم", "یِم"),
    ("یمن", "یِمَن"),
    ("یوان", "یِوان"),
    ("یہ", "یہِ"),
];

/// Final ہ is h in these words although a vowel mark comes before it (the
/// numbers: ten, twenty-five; twenty and thirty also as the end of a
/// compound, see [`final_h`]).
const FINAL_H: &[&str] = &["دٔہ", "دٕہ", "پٕنٛژٕہ"];

/// True when the final ہ of the word is h after a vowel mark: listed
/// words, and words ending in وُہ (twenty: وُہ wuh, اَکہٕ وُہ) or in
/// ر with ٛ and a vowel mark before ہ (thirty: ترٕٛہ trɨh; درِٛہ drih).
fn final_h(word: &str, us: &[Unit]) -> bool {
    let n = us.len();
    FINAL_H.contains(&word)
        || n >= 2
            && us[n - 1].base == 'ہ'
            && us[n - 1].marks.is_empty()
            && ((us[n - 2].base == 'و' && us[n - 2].has(PESH))
                || (us[n - 2].base == 'ر'
                    && us[n - 2].has(INVERTED_V)
                    && us[n - 2].vowel_mark().is_some()))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum T {
    /// A consonant letter (Arabic, with ھ when aspirated).
    C(char, bool),
    Sign(V),
    Vowel(V),
    Virama,
    /// Palatalisation (ؠ, یٛ after a consonant).
    Pal,
    Nasal,
}

/// A vowel-like letter that can follow a consonant as its vowel.
fn vowel_letter(u: &Unit) -> bool {
    match u.base {
        'ا' | 'ٲ' | 'ۆ' | 'ۄ' | 'ے' => true,
        'و' => !u.marks.iter().any(|&m| mark_vowel(m).is_some()) && !u.has(INVERTED_V),
        'ی' => !u.marks.iter().any(|&m| mark_vowel(m).is_some()) && !u.has(INVERTED_V),
        _ => false,
    }
}

/// A unit that begins with a vowel (after which و and ی are consonants).
fn starts_vowel(u: &Unit) -> bool {
    match u.base {
        'ا' | 'ٲ' | 'ۆ' | 'ۄ' => true,
        'و' => u.has(INVERTED_DAMMA) || u.has(SMALL_V),
        'ی' => u.has(SUBSCRIPT_ALEF) || u.has(SMALL_V),
        _ => false,
    }
}

/// The word as consonants and vowels.
fn tokens(word: &str) -> Vec<T> {
    let mut us = units(word);
    if let Ok(i) = UNMARKED.binary_search_by(|p| p.0.cmp(word)) {
        us = units(UNMARKED[i].1);
    }
    let vowelled = marked(&us);
    let final_h = final_h(word, &us);
    let n = us.len();
    let mut out: Vec<T> = Vec::with_capacity(n + 4);
    // A consonant written last that has no vowel yet.
    let mut pending = false;
    // An alif at the start that carries the next vowel letter.
    let mut carrier = false;
    // Close a consonant that has no vowel: a virama in a marked word (a
    // where the word leaves out its first vowel), nothing in an unmarked one.
    let close = |out: &mut Vec<T>, pending: &mut bool, before: Option<&Unit>| {
        if !*pending {
            return;
        }
        *pending = false;
        if !vowelled {
            return;
        }
        // A word cannot begin with two consonants unless the second is r,
        // y or w (or marked with ٛ): a marked word that leaves out the
        // first vowel (ہنزِ) is read with a.
        let first = out.len() == 1;
        let cluster_ok = before.is_none_or(|u| {
            matches!(u.base, 'ر' | 'و' | 'ی' | 'ؠ' | 'ے') || u.has(INVERTED_V) || u.base == 'ں'
        });
        if first && !cluster_ok {
            out.push(T::Sign(V::A));
        } else {
            out.push(T::Virama);
        }
    };
    let vowel = |out: &mut Vec<T>, pending: &mut bool, carrier: &mut bool, v: V| {
        if *pending {
            out.push(T::Sign(v));
        } else {
            out.push(T::Vowel(v));
        }
        *pending = false;
        *carrier = false;
    };
    let mut i = 0;
    while i < n {
        let u = &us[i];
        let next = us.get(i + 1);
        let last = i + 1 == n;
        match u.base {
            'ا' | 'ع' => {
                let v = if u.has(MADDA) {
                    Some(V::Aa)
                } else {
                    u.vowel_mark()
                };
                if let Some(v) = v {
                    if pending && v == V::Aa {
                        vowel(&mut out, &mut pending, &mut carrier, V::Aa);
                    } else if pending && v == V::Schwa && u.base == 'ا' {
                        // أ after a consonant is typed for ٲ (آبأدِی).
                        vowel(&mut out, &mut pending, &mut carrier, V::Schwa2);
                    } else {
                        close(&mut out, &mut pending, Some(u));
                        vowel(&mut out, &mut pending, &mut carrier, v);
                    }
                } else if pending || (carrier && u.base == 'ا') {
                    // After a consonant, or after ع at the start (عام aːm).
                    vowel(&mut out, &mut pending, &mut carrier, V::Aa);
                } else if out.last() == Some(&T::Sign(V::A)) && u.base == 'ا' {
                    // Zabar and alif (آسَان): aː.
                    out.pop();
                    out.push(T::Sign(V::Aa));
                } else if next.is_some_and(|x| vowel_letter(x) || x.base == 'ؠ') {
                    carrier = true;
                } else if out.is_empty() || u.base == 'ا' {
                    vowel(&mut out, &mut pending, &mut carrier, V::A);
                }
            }
            'ٲ' => {
                vowel(&mut out, &mut pending, &mut carrier, V::Schwa2);
            }
            'ۆ' => vowel(&mut out, &mut pending, &mut carrier, V::O),
            'ۄ' => {
                if next.is_some_and(|x| x.base == 'ا' && x.marks.is_empty()) {
                    vowel(&mut out, &mut pending, &mut carrier, V::Open2);
                    i += 1;
                } else {
                    vowel(&mut out, &mut pending, &mut carrier, V::Open);
                }
            }
            'و' => {
                if u.has(INVERTED_DAMMA) {
                    vowel(&mut out, &mut pending, &mut carrier, V::Uu);
                } else if u.has(SMALL_V) {
                    // وٚ: o (روٚزَن, اوٚش).
                    vowel(&mut out, &mut pending, &mut carrier, V::O);
                } else if let Some(v) = u.vowel_mark() {
                    close(&mut out, &mut pending, Some(u));
                    out.push(T::C('و', false));
                    vowel(&mut out, &mut true, &mut carrier, v);
                } else if u.has(INVERTED_V) || u.has(SUKUN) {
                    close(&mut out, &mut pending, Some(u));
                    out.push(T::C('و', false));
                    pending = true;
                    if u.has(SUKUN) {
                        close(&mut out, &mut pending, None);
                    }
                } else if (pending || carrier) && !next.is_some_and(starts_vowel) {
                    vowel(&mut out, &mut pending, &mut carrier, V::Oo);
                } else {
                    close(&mut out, &mut pending, Some(u));
                    carrier = false;
                    out.push(T::C('و', false));
                    pending = true;
                }
            }
            'ی' => {
                if u.has(INVERTED_V) && pending {
                    close(&mut out, &mut pending, Some(u));
                    out.push(T::Pal);
                    if let Some(v) = u.vowel_mark() {
                        out.push(T::Sign(v));
                    } else {
                        pending = true;
                    }
                } else if u.has(SUBSCRIPT_ALEF) {
                    vowel(&mut out, &mut pending, &mut carrier, V::Ii);
                } else if u.has(SMALL_V) {
                    vowel(&mut out, &mut pending, &mut carrier, V::E);
                } else if let Some(v) = u.vowel_mark() {
                    close(&mut out, &mut pending, Some(u));
                    out.push(T::C('ی', false));
                    vowel(&mut out, &mut true, &mut carrier, v);
                } else if out.last() == Some(&T::Sign(V::I))
                    && !pending
                    && !next.is_some_and(|x| starts_vowel(x) || x.base == 'ا')
                {
                    // Zer and ye (آبأدِی): iː.
                    out.pop();
                    out.push(T::Sign(V::Ii));
                } else if !vowelled
                    && pending
                    && i + 2 == n
                    && us[n - 1].base == 'ہ'
                    && us[n - 1].marks.is_empty()
                {
                    // -یہ at the end of an unmarked word: ja (آچاریہ).
                    out.push(T::C('ی', false));
                    pending = true;
                } else if (pending || carrier)
                    && !next.is_some_and(|x| starts_vowel(x) || x.base == 'ا')
                {
                    let v = if last {
                        V::Ii
                    } else if vowelled || carrier {
                        V::Ee
                    } else if out.len() == 1 {
                        // Unmarked words (Urdu habit): eː in the first
                        // syllable, iː after it.
                        V::Ee
                    } else {
                        V::Ii
                    };
                    vowel(&mut out, &mut pending, &mut carrier, v);
                } else {
                    close(&mut out, &mut pending, Some(u));
                    carrier = false;
                    out.push(T::C('ی', false));
                    pending = true;
                    if u.has(SUKUN) {
                        close(&mut out, &mut pending, None);
                    }
                }
            }
            'ے' => {
                let v = if u.has(SMALL_V) { V::E } else { V::Ee };
                if pending || carrier || out.is_empty() {
                    vowel(&mut out, &mut pending, &mut carrier, v);
                } else {
                    // After a vowel: the glide y (جاے d͡ʒaːj).
                    out.push(T::C('ی', false));
                    pending = true;
                    close(&mut out, &mut pending, None);
                    if !vowelled {
                        out.push(T::Virama);
                    }
                }
            }
            'ؠ' => {
                if pending || carrier {
                    // Palatalised consonant: ʲ at the end, ʲa before a
                    // consonant, ʲ + the vowel of a following alif; اؠ at
                    // the start of a word is ʲa.
                    if pending {
                        out.push(T::Virama);
                    }
                    out.push(T::Pal);
                    pending = false;
                    carrier = false;
                    if let Some(v) = u.vowel_mark() {
                        out.push(T::Sign(v));
                    } else if next.is_some_and(|x| x.base == 'ا' && x.marks.is_empty()) {
                        pending = true;
                    } else if !last {
                        out.push(T::Sign(V::A));
                    }
                } else {
                    out.push(T::C('ی', false));
                    pending = true;
                }
            }
            'ں' => {
                close(&mut out, &mut pending, Some(u));
                out.push(T::Nasal);
            }
            'ہ' if last && !u.aspirated => {
                // Final he: a carrier of the vowel mark before it or on it,
                // h after a vowel letter.
                let mark = u.vowel_mark();
                let after_mark = matches!(out.last(), Some(T::Sign(_)))
                    && i > 0
                    && us[i - 1].vowel_mark().is_some();
                if u.has(SUKUN) {
                    // ہْ: h (Devanagari text written in Perso-Arabic).
                    close(&mut out, &mut pending, Some(u));
                    out.push(T::C('ہ', false));
                    out.push(T::Virama);
                } else if pending && us[i - 1].base == 'ہ' && mark.is_none() {
                    // ہہ: the first is h, the second only closes the word.
                    close(&mut out, &mut pending, None);
                } else if pending {
                    let v = mark.unwrap_or(V::A);
                    if matches!(v, V::A | V::I | V::Barred | V::Barred2 | V::U | V::Schwa) {
                        vowel(&mut out, &mut pending, &mut carrier, v);
                    } else {
                        close(&mut out, &mut pending, Some(u));
                        out.push(T::C('ہ', false));
                        vowel(&mut out, &mut true, &mut carrier, v);
                    }
                } else if after_mark && !final_h && mark.is_none() {
                    // سُہ su, تٕہ tɨ: silent.
                } else {
                    out.push(T::C('ہ', false));
                    match mark {
                        Some(v) => out.push(T::Sign(v)),
                        None if vowelled => out.push(T::Virama),
                        None => {}
                    }
                }
            }
            'ن' if u.has(INVERTED_V) && u.vowel_mark().is_none() && !last => {
                // ن with ٛ: the nasal of the vowel before it (see
                // `explicit`); after a consonant without a vowel mark (منٛز)
                // that vowel is a.
                if pending {
                    out.push(T::Sign(V::A));
                    pending = false;
                }
                out.push(T::Nasal);
            }
            c if is_consonant(c) => {
                close(&mut out, &mut pending, Some(u));
                carrier = false;
                if u.has(SHADDA) {
                    out.push(T::C(c, false));
                    out.push(T::Virama);
                }
                out.push(T::C(c, u.aspirated));
                if let Some(v) = u.vowel_mark() {
                    out.push(T::Sign(v));
                } else if u.has(SUKUN) {
                    out.push(T::Virama);
                } else {
                    pending = true;
                }
            }
            _ => {
                // Letters of other Arabic-script languages: kept as they are.
                close(&mut out, &mut pending, Some(u));
                out.push(T::C(u.base, false));
                pending = true;
            }
        }
        i += 1;
    }
    close(&mut out, &mut pending, None);
    if vowelled {
        fill_vowels(&mut out);
    }
    out
}

/// A marked word that leaves out a vowel the reader supplies: with no vowel
/// at all (پرٛتھ prtʰ), a after its last consonant but one; with three
/// consonants at its end (ارتٲجہن), a before the last.
fn fill_vowels(out: &mut [T]) {
    let consonants: Vec<usize> = (0..out.len())
        .filter(|&k| matches!(out[k], T::C(..)))
        .collect();
    let has_vowel = out.iter().any(|t| matches!(t, T::Sign(_) | T::Vowel(_)));
    let c = consonants.len();
    let at = if !has_vowel && c >= 1 {
        Some(consonants[c.saturating_sub(2)])
    } else if c >= 3 {
        let t = &consonants[c - 3..];
        let bare = |k: usize| out.get(k + 1) == Some(&T::Virama);
        (t.iter().all(|&k| bare(k))
            && t[1] == t[0] + 2
            && t[2] == t[1] + 2
            && t[2] + 2 == out.len())
        .then_some(t[1])
    } else {
        None
    };
    if let Some(k) = at {
        if out.get(k + 1) == Some(&T::Virama) {
            out[k + 1] = T::Sign(V::A);
        }
    }
}

/// The written word with every vowel spelled out (see the module
/// documentation); `lang/ks.lang` reads this form.
pub fn explicit(word: &str) -> String {
    let mut s = String::with_capacity(word.len() * 2);
    let mut long = false;
    let ts = tokens(word);
    for (k, &t) in ts.iter().enumerate() {
        let was_long = long;
        long = matches!(t, T::Sign(v) | T::Vowel(v) if v.long());
        // A nasal after a short vowel is n before t d ts tʃ dʒ s z ʃ.
        let dental = matches!(
            ts.get(k + 1),
            Some(T::C(
                'ت' | 'د'
                    | 'ج'
                    | 'چ'
                    | 'ژ'
                    | 'ز'
                    | 'س'
                    | 'ش'
                    | 'ط'
                    | 'ث'
                    | 'ص'
                    | 'ذ'
                    | 'ض'
                    | 'ظ',
                _
            ))
        );
        match t {
            T::C(c, asp) => {
                s.push(c);
                if asp {
                    s.push('ھ');
                }
            }
            T::Sign(v) => s.push(v.sign()),
            T::Vowel(v) => s.push(v.letter()),
            T::Virama => s.push(SUKUN),
            T::Pal => s.push('ʲ'),
            // A nasal vowel (ñ) after a long vowel, and after a short one
            // except before a dental or alveolar consonant, where it is the
            // anusvara (n). After an unwritten vowel (a word without marks)
            // the anusvara gives a nasal consonant of the place of the next
            // one.
            T::Nasal => s.push(
                if was_long || (k > 0 && matches!(ts[k - 1], T::Sign(_) | T::Vowel(_)) && !dental) {
                    'ñ'
                } else {
                    'ں'
                },
            ),
        }
    }
    s
}

/// The Devanagari letter of a Perso-Arabic consonant.
fn deva_consonant(c: char, asp: bool) -> &'static str {
    match (c, asp) {
        ('ب', false) => "ब",
        ('ب', true) => "भ",
        ('پ', false) => "प",
        ('پ', true) => "फ",
        ('ت' | 'ط', false) => "त",
        ('ت' | 'ط', true) => "थ",
        ('ٹ', false) => "ट",
        ('ٹ', true) => "ठ",
        ('ث' | 'س' | 'ص', _) => "स",
        ('ج', false) => "ज",
        ('ج', true) => "झ",
        ('چ', false) => "च",
        ('چ', true) => "छ",
        ('ح' | 'ہ', _) => "ह",
        ('خ', _) => "ख़",
        ('د', false) => "द",
        ('د', true) => "ध",
        ('ڈ', false) => "ड",
        ('ڈ', true) => "ढ",
        ('ذ' | 'ز' | 'ض' | 'ظ', _) => "ज़",
        ('ر', _) => "र",
        ('ڑ', _) => "ड़",
        ('ژ', false) => "च़",
        ('ژ', true) => "छ़",
        ('ش', _) => "श",
        ('غ', _) => "ग़",
        ('ف', _) => "फ़",
        ('ق', _) => "क़",
        ('ک', false) => "क",
        ('ک', true) => "ख",
        ('گ', false) => "ग",
        ('گ', true) => "घ",
        ('ل', _) => "ल",
        ('م', _) => "म",
        ('ن', _) => "न",
        ('و', _) => "व",
        ('ی', _) => "य",
        _ => "",
    }
}

/// Perso-Arabic Kashmiri words in Devanagari (2009 proposal), for the
/// number and date words written into Devanagari-script text. Other
/// characters are kept.
pub fn arabic_to_devanagari(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        if !word.is_empty() {
            out.push_str(&word_to_devanagari(word));
            word.clear();
        }
    };
    for c in s.chars() {
        if script_of(c) == Script::Brahmic(0x0600) {
            word.push(c);
        } else {
            flush(&mut word, &mut out);
            out.push(match c {
                '۔' => '।',
                '،' => ',',
                '؟' => '?',
                _ => c,
            });
        }
    }
    flush(&mut word, &mut out);
    out
}

fn word_to_devanagari(word: &str) -> String {
    let ts = tokens(word);
    let mut out = String::new();
    for (k, t) in ts.iter().enumerate() {
        match *t {
            T::C(c, asp) => {
                out.push_str(deva_consonant(c, asp));
                // An unmarked consonant before another one, or at the end:
                // the Devanagari reader drops its vowel the same way.
                if let Some(T::Pal) = ts.get(k + 1) {
                    out.push('्');
                }
            }
            T::Sign(v) => out.push_str(v.devanagari().0),
            T::Vowel(v) => out.push_str(v.devanagari().1),
            T::Virama => {
                // Only between consonants; a final one is not written.
                if matches!(ts.get(k + 1), Some(T::C(..))) {
                    out.push('्');
                }
            }
            T::Pal => out.push('य'),
            T::Nasal => out.push('ं'),
        }
    }
    out
}

/// True when `s` has a Devanagari letter.
pub fn has_devanagari(s: &str) -> bool {
    s.chars().any(|c| {
        ('\u{0904}'..='\u{0939}').contains(&c)
            || ('\u{0958}'..='\u{0961}').contains(&c)
            || ('\u{0972}'..='\u{097f}').contains(&c)
    })
}

/// True when `s` is written in Devanagari (a Devanagari letter, no
/// Perso-Arabic one): its number words are written in Devanagari too.
pub fn writes_devanagari(s: &str) -> bool {
    (has_devanagari(s) || s.chars().any(|c| ('\u{0966}'..='\u{096f}').contains(&c)))
        && !s.chars().any(|c| {
            ('\u{0620}'..='\u{064a}').contains(&c) || ('\u{0671}'..='\u{06d3}').contains(&c)
        })
}

/// A Devanagari consonant (nukta forms included): the Perso-Arabic letter
/// and whether it is aspirated.
fn dv_consonant(c: char, nukta: bool) -> Option<(char, bool)> {
    Some(match (c, nukta) {
        ('क', false) => ('ک', false),
        ('क', true) | ('\u{0958}', _) => ('ق', false),
        ('ख', false) => ('ک', true),
        ('ख', true) | ('\u{0959}', _) => ('خ', false),
        ('ग', false) => ('گ', false),
        ('ग', true) | ('\u{095a}', _) => ('غ', false),
        ('घ', _) => ('گ', true),
        ('ङ' | 'ञ' | 'ण' | 'न' | 'ऩ', _) => ('ن', false),
        ('च', false) => ('چ', false),
        ('च', true) => ('ژ', false),
        ('छ', false) => ('چ', true),
        ('छ', true) => ('ژ', true),
        ('ज', false) => ('ج', false),
        ('ज', true) | ('\u{095b}', _) => ('ز', false),
        ('झ', _) => ('ج', true),
        ('ट', _) => ('ٹ', false),
        ('ठ', _) => ('ٹ', true),
        ('ड', false) => ('ڈ', false),
        ('ड', true) | ('\u{095c}', _) => ('ڑ', false),
        ('ढ', false) => ('ڈ', true),
        ('ढ', true) | ('\u{095d}', _) => ('ڑ', true),
        ('त', _) => ('ت', false),
        ('थ', _) => ('ت', true),
        ('द', _) => ('د', false),
        ('ध', _) => ('د', true),
        ('प', _) => ('پ', false),
        ('फ', false) => ('پ', true),
        ('फ', true) | ('\u{095e}', _) => ('ف', false),
        ('ब', _) => ('ب', false),
        ('भ', _) => ('ب', true),
        ('म', _) => ('م', false),
        ('य' | '\u{095f}', _) => ('ی', false),
        ('र' | 'ऱ', _) => ('ر', false),
        ('ल' | 'ळ' | 'ऴ', _) => ('ل', false),
        ('व', _) => ('و', false),
        ('श' | 'ष', _) => ('ش', false),
        ('स', _) => ('س', false),
        ('ह', _) => ('ہ', false),
        _ => return None,
    })
}

/// A Devanagari vowel sign.
fn dv_sign(c: char) -> Option<V> {
    Some(match c {
        'ा' => V::Aa,
        'ि' => V::I,
        'ी' => V::Ii,
        'ु' => V::U,
        'ू' => V::Uu,
        'े' => V::Ee,
        'ो' => V::Oo,
        'ौ' => V::Open2,
        'ॅ' | 'ऺ' | 'ै' => V::Schwa,
        'ॉ' | 'ऻ' => V::Schwa2,
        'ॆ' | 'ॕ' => V::E,
        'ॊ' => V::O,
        'ॏ' => V::Open,
        'ॖ' => V::Barred,
        'ॗ' => V::Barred2,
        'ृ' => V::I,
        _ => return None,
    })
}

/// A Devanagari independent vowel.
fn dv_vowel(c: char) -> Option<V> {
    Some(match c {
        'अ' => V::A,
        'आ' => V::Aa,
        'इ' => V::I,
        'ई' => V::Ii,
        'उ' => V::U,
        'ऊ' => V::Uu,
        'ए' => V::Ee,
        'ओ' => V::Oo,
        'औ' => V::Open2,
        'ऐ' | 'ॲ' | 'ॳ' | 'ऍ' => V::Schwa,
        'ऑ' | 'ॴ' => V::Schwa2,
        'ऎ' | 'ऄ' => V::E,
        'ऒ' => V::O,
        'ॵ' => V::Open,
        'ॶ' => V::Barred,
        'ॷ' => V::Barred2,
        'ऋ' => V::I,
        _ => return None,
    })
}

/// One sound of a Devanagari word, before it is written in Perso-Arabic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum D {
    /// Consonant, aspirated, followed by a virama (no vowel).
    C(char, bool),
    /// Palatalisation (्य).
    Pal,
    /// The vowel of the consonant before it; `true` for the inherent one.
    Sign(V, bool),
    Vowel(V),
    Nasal,
}

fn devanagari_word(word: &str) -> String {
    let cs: Vec<char> = word
        .chars()
        .filter(|c| !matches!(c, '\u{200c}' | '\u{200d}'))
        .collect();
    let mut ds: Vec<D> = Vec::with_capacity(cs.len() * 2);
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        let nukta = cs.get(i + 1) == Some(&'़');
        if let Some((a, asp)) = dv_consonant(c, nukta) {
            i += 1 + usize::from(nukta);
            // ्य after a consonant: palatalisation; ्व with the inherent
            // vowel: ɔ (श्वंग ʃɔŋɡ).
            let after_virama = matches!(ds.last(), Some(D::C(..)));
            let sign = cs.get(i).copied().and_then(dv_sign);
            if a == 'ن'
                && matches!(c, 'ण' | 'ङ' | 'ञ')
                && cs.get(i) == Some(&'्')
                && cs
                    .get(i + 1)
                    .is_some_and(|&x| dv_consonant(x, false).is_some())
            {
                // ण ङ ञ with halant before a consonant: the nasal (गॅण्डिथ).
                ds.push(D::Nasal);
                i += 1;
                continue;
            }
            if after_virama && a == 'ی' {
                ds.push(D::Pal);
            } else if after_virama
                && a == 'و'
                && sign.is_none()
                && cs.get(i) != Some(&'्')
                && cs.get(i).is_some()
            {
                ds.push(D::Sign(V::Open, false));
                continue;
            } else {
                ds.push(D::C(a, asp));
            }
            match cs.get(i) {
                Some('्') => i += 1,
                Some(&s) if dv_sign(s).is_some() => {
                    ds.push(D::Sign(dv_sign(s).unwrap(), false));
                    i += 1;
                }
                _ => ds.push(D::Sign(V::A, true)),
            }
            continue;
        }
        if let Some(v) = dv_vowel(c) {
            ds.push(D::Vowel(v));
        } else if matches!(c, 'ं' | 'ँ') {
            ds.push(D::Nasal);
        } else if let Some(v) = dv_sign(c) {
            // A sign without a consonant (typing slip): its vowel.
            ds.push(D::Vowel(v));
        }
        i += 1;
    }
    // Inherent vowels that are not said: at the end of a word of more than
    // one vowel, and in V C _ C V (Hindi-type deletion, right to left, the
    // consonant after it followed by its vowel: पदमान padmaːn, मासटर
    // maːsʈar, but नवम्बरॖ naʋambarɨ).
    let vowels = ds
        .iter()
        .filter(|d| matches!(d, D::Sign(..) | D::Vowel(_)))
        .count();
    if vowels > 1 {
        if let Some(D::Sign(_, true)) = ds.last() {
            ds.pop();
        }
    }
    let mut k = ds.len();
    while k > 0 {
        k -= 1;
        if let D::Sign(_, true) = ds[k] {
            let vc = k >= 2
                && matches!(ds[k - 1], D::C(..))
                && matches!(ds[k - 2], D::Sign(..) | D::Vowel(_) | D::Nasal);
            let mut j = k + 1;
            let cons_next = matches!(ds.get(j), Some(D::C(..)));
            j += 1;
            if matches!(ds.get(j), Some(D::Pal)) {
                j += 1;
            }
            let vowel_after = matches!(ds.get(j), Some(D::Sign(..)));
            if vc && cons_next && vowel_after {
                ds.remove(k);
            }
        }
    }
    // Write it out.
    let mut out = String::new();
    let n = ds.len();
    for k in 0..n {
        let end = k + 1 == n;
        match ds[k] {
            D::C(a, asp) => {
                out.push(a);
                if asp {
                    out.push('ھ');
                }
                // No vowel: a sukun, so that the word reads as marked.
                if !matches!(ds.get(k + 1), Some(D::Sign(..)) | Some(D::Pal)) {
                    out.push(SUKUN);
                }
            }
            D::Pal => {
                match ds.get(k + 1) {
                    Some(D::Sign(V::A, _)) | None => out.push('ؠ'),
                    Some(D::Sign(V::Aa, _)) => out.push('ؠ'),
                    _ => {
                        out.push('ی');
                        out.push(INVERTED_V);
                    }
                }
                if !matches!(ds.get(k + 1), Some(D::Sign(..))) && !end {
                    out.push(SUKUN);
                }
            }
            D::Sign(v, _) => {
                if k > 0 && ds[k - 1] == D::Pal && v == V::A {
                    // ؠ inside a word is ʲa already.
                    if end {
                        out.push(ZABAR);
                    }
                    continue;
                }
                let tail_end = end || matches!(ds.get(k + 1), Some(D::Nasal)) && k + 2 == n;
                out.push_str(v.arabic(tail_end).0);
            }
            D::Vowel(v) => {
                let tail_end = end || matches!(ds.get(k + 1), Some(D::Nasal)) && k + 2 == n;
                if k > 0 {
                    // A vowel after a vowel: on alif (or the glide letter).
                    out.push_str(v.arabic(tail_end).1);
                } else {
                    out.push_str(v.arabic(tail_end).1);
                }
            }
            D::Nasal => {
                if matches!(ds.get(k + 1), Some(D::C(..))) {
                    out.push('ن');
                    out.push(INVERTED_V);
                } else {
                    out.push('ں');
                }
            }
        }
    }
    out
}

/// Devanagari-script Kashmiri written in the Perso-Arabic script (with its
/// vowel marks, and a sukun on consonants without a vowel), danda as the
/// Arabic full stop, Devanagari digits as ASCII digits; `None` when `s`
/// has no Devanagari letter.
pub fn devanagari_to_arabic(s: &str) -> Option<String> {
    if !has_devanagari(s) {
        return None;
    }
    let mut out = String::with_capacity(s.len());
    let mut word = String::new();
    for c in s.chars() {
        if ('\u{0900}'..='\u{0963}').contains(&c)
            || ('\u{0971}'..='\u{097f}').contains(&c)
            || matches!(c, '\u{200c}' | '\u{200d}')
        {
            word.push(c);
            continue;
        }
        if !word.is_empty() {
            out.push_str(&devanagari_word(&word));
            word.clear();
        }
        match c {
            '।' | '॥' => out.push('۔'),
            '०'..='९' => out.push(char::from(b'0' + (c as u32 - 0x0966) as u8)),
            _ => out.push(c),
        }
    }
    if !word.is_empty() {
        out.push_str(&devanagari_word(&word));
    }
    Some(out)
}

/// Common romanised Kashmiri words and their Perso-Arabic spelling
/// (sorted): the words that signal a Kashmiri clause and short words whose
/// casual spelling the rules would misread. Spellings as in Kashmiri
/// Wikipedia text (IndicCorp v2) and Wiktionary.
const ROMAN_KS: &[(&str, &str)] = &[
    ("aes", "ٲس"),
    ("akh", "اَکھ"),
    ("asi", "أسؠ"),
    ("bu", "بہٕ"),
    ("chhe", "چھےٚ"),
    ("chhi", "چھِ"),
    ("chhu", "چھُ"),
    ("chhus", "چھُس"),
    ("chi", "چھِ"),
    ("chu", "چھُ"),
    ("dah", "داہ"),
    ("kasheer", "کٔشیٖر"),
    ("kashir", "کٔشیٖر"),
    ("kath", "کَتھ"),
    ("koshur", "کٲشُر"),
    ("kus", "کُس"),
    ("kyah", "کیاہ"),
    ("manz", "مَنٛز"),
    ("me", "مےٚ"),
    ("naav", "ناو"),
    ("oos", "اوس"),
    ("os", "اوس"),
    ("su", "سُہ"),
    ("ta", "تہٕ"),
    ("te", "تہٕ"),
    ("tim", "تِم"),
    ("tse", "ژٕ"),
    ("tsi", "ژٕ"),
    ("yi", "یہِ"),
    ("yim", "یِم"),
    ("yus", "یُس"),
];

/// Listed words that mark a clause as Kashmiri on their own.
const SIGNAL: &[&str] = &[
    "aes", "asi", "chhe", "chhi", "chhu", "chhus", "chu", "kashir", "kasheer", "koshur", "kyah",
    "manz", "oos", "tse", "tsi", "yim", "yus",
];

/// The Perso-Arabic letters of a romanised consonant and the number of
/// Latin letters it takes.
fn roman_consonant(cs: &[char]) -> Option<(&'static str, usize)> {
    let s: String = cs.iter().take(3).collect();
    for (r, a) in [("chh", "چھ"), ("tsh", "ژھ")] {
        if s.starts_with(r) {
            return Some((a, 3));
        }
    }
    let two: String = cs.iter().take(2).collect();
    let a = match two.as_str() {
        "ch" => Some("چ"),
        "ts" => Some("ژ"),
        "kh" => Some("کھ"),
        "gh" => Some("غ"),
        "th" => Some("تھ"),
        "ṭh" => Some("ٹھ"),
        "ċh" => Some("ژھ"),
        "ph" => Some("پھ"),
        "sh" => Some("ش"),
        "zh" => Some("ژ"),
        "dh" => Some("دھ"),
        "bh" => Some("بھ"),
        "jh" => Some("جھ"),
        _ => None,
    };
    if let Some(a) = a {
        return Some((a, 2));
    }
    Some((
        match cs[0] {
            'k' => "ک",
            'c' => "چ",
            'g' => "گ",
            'j' => "ج",
            't' => "ت",
            'ṭ' => "ٹ",
            'd' => "د",
            'ḍ' => "ڈ",
            'ṛ' => "ڑ",
            'n' | 'ñ' => "ن",
            'p' => "پ",
            'f' => "ف",
            'b' => "ب",
            'm' => "م",
            'y' => "ی",
            'r' => "ر",
            'l' => "ل",
            'w' | 'v' => "و",
            's' => "س",
            'š' | 'ś' => "ش",
            'ċ' => "ژ",
            'z' => "ز",
            'h' => "ہ",
            'q' => "ق",
            'x' => "خ",
            _ => return None,
        },
        1,
    ))
}

/// A romanised vowel at the start of `cs`: the vowel and the number of
/// Latin letters it takes. Casual spelling: a, aa, i, ee, u, oo, e, o, ae
/// (ə); the scholarly letters ā ạ ạ̄ ī ü ǖ ū ē ë ō ó ọ ọ̄ too.
fn roman_vowel(cs: &[char]) -> Option<(V, usize)> {
    let two: String = cs.iter().take(2).collect();
    Some(match two.as_str() {
        "aa" => (V::Aa, 2),
        "ae" => (V::Schwa, 2),
        "ee" | "ii" => (V::Ii, 2),
        "oo" | "uu" => (V::Uu, 2),
        "ạ̄" => (V::Schwa2, 2),
        "ọ̄" => (V::Open2, 2),
        _ => (
            match cs.first()? {
                'a' => V::A,
                'ā' => V::Aa,
                'ạ' | 'ö' | 'ȧ' => V::Schwa,
                'ä' | 'ȫ' => V::Schwa2,
                'i' | 'í' => V::I,
                'ī' => V::Ii,
                'ü' | 'ụ' => V::Barred,
                'ǖ' => V::Barred2,
                'u' => V::U,
                'ū' => V::Uu,
                'e' | 'ë' | 'ĕ' => V::E,
                'ē' => V::Ee,
                'o' | 'ō' => V::Oo,
                'ó' => V::O,
                'ọ' | 'ŏ' => V::Open,
                _ => return None,
            },
            1,
        ),
    })
}

/// Romanised Kashmiri in the Perso-Arabic script by rule: a consonant
/// takes the mark or letter of the vowel after it (a َ, aa ا, i ِ, ee یٖ,
/// u ُ, oo وٗ, e یٚ, o و, ae ٔ; final a, u, ü, ae on the carrier ہ as the
/// orthography writes them: نَہ); a vowel at the start of a word is
/// written on alif; chh tsh ch ts kh th ph sh with their letters (h after
/// a consonant is aspiration, h at the end after a vowel is ہہ: dah دَہہ);
/// an apostrophe after a consonant is ؠ (palatalised: lạṭ' لٔٹؠ); y after a
/// consonant and before a vowel is the consonant y (kyah کیَہہ); in a word
/// without a short vowel a consonant without a vowel takes a sukun (paanch
/// پانْچھْ), so that it is not read as an unmarked spelling.
/// Retroflex t d are written only in the scholarly spelling (ṭ ḍ ṛ).
pub fn transliterate(word: &str) -> String {
    let cs: Vec<char> = word.to_lowercase().chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let mut after_consonant = false;
    let mut start = true;
    // Places after a consonant that has no vowel.
    let mut bare: Vec<usize> = Vec::new();
    while i < cs.len() {
        let c = cs[i];
        if matches!(c, '\'' | '’' | 'ʼ') {
            if after_consonant {
                out.push('ؠ');
                after_consonant = false;
            }
            i += 1;
            continue;
        }
        if c == '-' {
            i += 1;
            start = true;
            after_consonant = false;
            continue;
        }
        if let Some((v, len)) = roman_vowel(&cs[i..]) {
            let end = i + len == cs.len();
            if after_consonant {
                if end && matches!(v, V::A | V::U | V::Barred | V::Schwa) {
                    // Final short vowels on the carrier he.
                    out.push_str(v.arabic(true).0);
                    out.push('ہ');
                } else {
                    out.push_str(v.arabic(end).0);
                }
            } else if start {
                out.push_str(v.arabic(end).1);
            } else {
                // A vowel after a vowel: on alif, or y between them.
                let (_, initial) = v.arabic(end);
                out.push_str(initial);
            }
            i += len;
            after_consonant = false;
            start = false;
            continue;
        }
        let Some((letters, len)) = roman_consonant(&cs[i..]) else {
            i += 1;
            continue;
        };
        if letters == "ہ" && i + len == cs.len() && !after_consonant && !start {
            // h at the end after a vowel: ہہ (dah دَہہ), as بَہہ.
            out.push_str("ہہ");
            i += len;
            continue;
        }
        if after_consonant {
            bare.push(out.len());
        }
        out.push_str(letters);
        i += len;
        after_consonant = true;
        start = false;
    }
    if after_consonant {
        bare.push(out.len());
    }
    // A word without any vowel mark (paanch پانچھ) would be read as an
    // unmarked spelling, with inherent vowels: its bare consonants take a
    // sukun.
    if !marked(&units(&out)) {
        for &at in bare.iter().rev() {
            out.insert(at, SUKUN);
        }
    }
    out
}

/// Rewrite the Latin words of a clause that are romanised Kashmiri in the
/// Perso-Arabic script: listed words always, other words unless `english`
/// knows them, and only in a clause with a listed word of `SIGNAL`.
pub fn rewrite_roman(items: &mut [Item], english: impl Fn(&str) -> bool) {
    let latin = |it: &Item| {
        !it.spell
            && !it.generated
            && it.text.chars().any(char::is_alphabetic)
            && it.text.chars().all(|c| {
                script_of(c) == Script::Latin
                    || matches!(c, '\'' | '’' | '\u{323}' | '\u{304}' | '\u{306}')
            })
    };
    let listed = |w: &str| ROMAN_KS.binary_search_by(|p| p.0.cmp(w)).ok();
    let signal = items.iter().any(|it| {
        let w = it.text.to_lowercase();
        latin(it) && SIGNAL.contains(&w.as_str())
    });
    if !signal {
        return;
    }
    for it in items.iter_mut().filter(|it| latin(it)) {
        let w = it.text.to_lowercase();
        if let Some(k) = listed(&w) {
            it.text = ROMAN_KS[k].1.to_string();
        } else if !english(&w) {
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
        assert!(UNMARKED.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(ROMAN_KS.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(SIGNAL.iter().all(|s| ROMAN_KS.iter().any(|p| p.0 == *s)));
    }

    #[test]
    fn explicit_forms() {
        for (w, e) in [
            ("کٲشُر", "کǝشùرْ"),
            ("چھِ", "چھì"),
            ("سُہ", "سù"),
            ("تہٕ", "تɨ"),
            // A word without marks keeps its bare consonants.
            ("داہ", "دāہ"),
            ("شہر", "شہر"),
            ("عام", "Āم"),
            ("منٛز", "مȧںزْ"),
            ("بیٛور", "بْʲōرْ"),
            ("لؠوان", "لْʲȧوāنْ"),
            ("پانٛژھ", "پāñژھْ"),
            ("اَنٛدَر", "Ȧںدȧرْ"),
            ("سونٛتھ", "سōñتھْ"),
        ] {
            assert_eq!(explicit(w), e, "{w}");
        }
    }

    #[test]
    fn devanagari() {
        for (d, a) in [
            ("कॉशुर", "کٲشُرْ"),
            ("मासटर", "ماسْٹَرْ"),
            ("डॆसम्बर", "ڈیٚسَمْبَرْ"),
            ("आशन्य", "آشَنؠ"),
            ("श्वंगनावुन", "شۄنٛگْناوُنْ"),
            ("गॅण्डिथ", "گٔنٛڈِتھْ"),
            ("ल्यवान", "لؠوانْ"),
            ("कटॖ सॖंज़ मादॖ।", "کَٹٕ سٕنٛزْ مادٕ۔"),
        ] {
            assert_eq!(devanagari_to_arabic(d).unwrap(), a, "{d}");
        }
        assert_eq!(devanagari_to_arabic("کٲشُر"), None);
        for (a, d) in [
            ("پٕنٛژٕہ", "पॖंच़ॖह"),
            ("رۄپَے", "रॏपय"),
            ("اَگَست", "अगस्त"),
            ("زٕ ساس ژۆوُہ", "ज़ॖ सास च़ॊवुह"),
            ("پانٛژھ ہَتھ", "पांछ़ हथ"),
        ] {
            assert_eq!(arabic_to_devanagari(a), d, "{a}");
        }
        assert!(writes_devanagari("कॉशुर 25") && writes_devanagari("२५"));
        assert!(!writes_devanagari("کٲشُر २५"));
    }

    #[test]
    fn rules() {
        for (r, a) in [
            ("koshur", "کوشُر"),
            ("jannat", "جَننَت"),
            ("lạṭ'", "لٔٹؠ"),
            ("zabaan", "زَبان"),
            ("na", "نَہ"),
            ("dah", "دَہہ"),
            ("bemaar", "بیٚمار"),
            ("paanch", "پانْچْ"),
        ] {
            assert_eq!(transliterate(r), a, "{r}");
        }
    }

    #[test]
    fn rewrites_a_clause() {
        let mut items = vec![
            Item::word("Koshur"),
            Item::word("chhu"),
            Item::word("office"),
            Item::word("zabaan"),
        ];
        for it in &mut items {
            // Words of the text, not of a number reading.
            it.generated = false;
        }
        rewrite_roman(&mut items, |w| w == "office");
        let t: Vec<&str> = items.iter().map(|i| i.text.as_str()).collect();
        assert_eq!(t, ["کٲشُر", "چھُ", "office", "زَبان"]);
        let mut english = vec![Item::word("my"), Item::word("office")];
        for it in &mut english {
            it.generated = false;
        }
        rewrite_roman(&mut english, |_| false);
        assert_eq!(english[0].text, "my");
    }
}
