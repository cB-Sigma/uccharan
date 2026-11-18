//! Manipuri (Meitei) in its two scripts, and romanised input.
//!
//! Meitei is written in Meetei Mayek, its own script and the official one,
//! and in the Bengali script, long the usual one. `lang/mni.lang` reads
//! Meetei Mayek; text in Bengali script is first written in Meetei Mayek by
//! [`bengali_to_meetei`], letter for letter, with the habits of Bengali-script
//! Meitei spelling turned into the explicit Meetei Mayek ones:
//!
//! - a consonant letter without a vowel sign at the end of a word closes the
//!   syllable (মতম mɐtɐm, থবক tʰɐbɐk): the lonsum letter (ꯃꯇꯝ, ꯊꯕꯛ), or the
//!   letter with apun iyek ꯭ when it has no lonsum form (শহর ꯁꯍꯔ꯭);
//! - a consonant with hasanta before another one is a lonsum letter (মহাক্না
//!   ꯃꯍꯥꯛꯅꯥ), but stays in a cluster written with apun iyek before র য ৱ, ব
//!   after a stop (w: খ্বাই ꯈ꯭ꯋꯥꯏ), and ল at the start of a word (ক্লাস);
//! - শ ষ স are s, ছ is tʃ, the retroflex letters are the dental ones (Meitei
//!   has neither), ড় ঢ় are ɾ, য য় are j, ৱ is w.
//!
//! Both scripts leave two things to the reader, and [`prepare`] makes them
//! explicit in the Meetei Mayek text before the language file reads it:
//!
//! - a stem that ends in a consonant is often written without a lonsum letter
//!   or hasanta before a suffix (মতমদা ꯃꯇꯃꯗꯥ mɐtɐmdɐ "at the time", মোমবি
//!   mombi): a k t p m n ŋ l letter without a vowel sign, after a vowel and
//!   before a consonant that has one, closes the syllable (read right to left,
//!   as the Hindi-type deletion of inherent vowels); not after ꯑ (ꯑꯃꯁꯨꯡ
//!   ɐmɐsuŋ, ꯑꯃꯗꯥ ɐmɐdɐ);
//! - the vowel of the suffixes -bɐ -pɐ -dɐ -tɐ -nɐ -ɡɐ -kɐ is written া or ꯥ
//!   at the end of a word by many writers (লৈবা, ꯂꯥꯎꯕꯥ; Wikipedia's Meetei
//!   Mayek writes ꯂꯩꯕ): at the end of a word of more than one syllable, after
//!   ꯅ, after ꯕ ꯗ ꯒ that follow a vowel or a voiced consonant, and after ꯄ ꯇ
//!   ꯀ that follow ꯛ ꯠ ꯞ (the suffixes have p t k after these and b d ɡ
//!   elsewhere), it is ɐ, as the Wiktionary entries of these words have it
//!   (অচৌবা ɐtʃɐwbɐ, অতোপ্পা ɐtoppɐ), except in a few words with a final a
//!   (ꯀꯅꯥ "who", ꯃꯄꯥ "father"). Written number words keep their letters
//!   (ꯇꯔꯥꯃꯔꯤ tɐɾamɐɾi "fourteen").
//!
//! [`meetei_to_bengali`] writes Meetei Mayek in Bengali script the other way
//! round; uccharan uses it for the number and date words it writes into
//! Bengali-script text (normalise-only mode).
//!
//! Romanised Meitei (the casual Latin spelling of chat and signs: a for ɐ, aa
//! for a, ei, ou, kh th ph, ng, final k t p m n ng l) is written in Meetei
//! Mayek by [`transliterate`].

use crate::norm::Item;
use crate::text::{script_of, Script};

/// The Meetei Mayek letter of a Bengali consonant (nukta forms included).
fn consonant(c: char) -> Option<char> {
    Some(match c {
        'ক' => 'ꯀ',
        'খ' => 'ꯈ',
        'গ' => 'ꯒ',
        'ঘ' => 'ꯘ',
        'ঙ' => 'ꯉ',
        'চ' | 'ছ' => 'ꯆ',
        'জ' => 'ꯖ',
        'ঝ' => 'ꯓ',
        'ঞ' | 'ণ' | 'ন' => 'ꯅ',
        'ট' | 'ত' => 'ꯇ',
        'ঠ' | 'থ' => 'ꯊ',
        'ড' | 'দ' => 'ꯗ',
        'ঢ' | 'ধ' => 'ꯙ',
        'প' => 'ꯄ',
        'ফ' => 'ꯐ',
        'ব' => 'ꯕ',
        'ভ' => 'ꯚ',
        'ম' => 'ꯃ',
        'য' | '\u{09df}' => 'ꯌ',
        'র' | 'ৰ' | '\u{09dc}' | '\u{09dd}' => 'ꯔ',
        'ল' => 'ꯂ',
        'ৱ' => 'ꯋ',
        'শ' | 'ষ' | 'স' => 'ꯁ',
        'হ' => 'ꯍ',
        _ => return None,
    })
}

/// The lonsum (syllable-closing) form of a Meetei Mayek consonant letter.
fn lonsum(c: char) -> Option<char> {
    Some(match c {
        'ꯀ' => 'ꯛ',
        'ꯂ' => 'ꯜ',
        'ꯃ' => 'ꯝ',
        'ꯄ' => 'ꯞ',
        'ꯅ' => 'ꯟ',
        'ꯇ' => 'ꯠ',
        'ꯉ' => 'ꯡ',
        _ => return None,
    })
}

fn vowel_sign(c: char) -> Option<&'static str> {
    Some(match c {
        'া' => "ꯥ",
        'ি' | 'ী' => "ꯤ",
        'ু' | 'ূ' => "ꯨ",
        'ে' => "ꯦ",
        'ৈ' => "ꯩ",
        'ো' => "ꯣ",
        'ৌ' => "ꯧ",
        'ৃ' | 'ৄ' => "꯭ꯔꯤ",
        _ => return None,
    })
}

fn independent_vowel(c: char) -> Option<&'static str> {
    Some(match c {
        'অ' => "ꯑ",
        'আ' => "ꯑꯥ",
        'ই' | 'ঈ' => "ꯏ",
        'উ' | 'ঊ' => "ꯎ",
        'ঋ' | 'ৠ' => "ꯔꯤ",
        'এ' => "ꯑꯦ",
        'ঐ' => "ꯑꯩ",
        'ও' => "ꯑꯣ",
        'ঔ' => "ꯑꯧ",
        _ => return None,
    })
}

/// Words of more than one syllable whose final ꯥ is a (not the ɐ of a
/// suffix).
const FINAL_A: &[&str] = &["ꯀꯅꯥ", "ꯃꯄꯥ", "ꯃꯅꯥ", "ꯄꯅꯥ", "ꯄꯦꯅꯥ", "ꯏꯄꯥ"];

/// Number words that follow a ten in one word (ꯇꯔꯥꯃꯔꯤ 14): their first
/// letter keeps its vowel.
const NUMBER_TAILS: &[&str] = &["ꯃꯊꯣꯏ", "ꯃꯔꯤ", "ꯃꯉꯥ", "ꯇꯔꯨꯛ", "ꯇꯔꯦꯠ", "ꯇꯔꯥ"];

fn bengali_letter(c: char) -> bool {
    ('\u{0980}'..='\u{09ff}').contains(&c) && !('\u{09e6}'..='\u{09ef}').contains(&c)
}

/// One Bengali-script word (Bengali letters only) in Meetei Mayek.
fn word_to_meetei(word: &str) -> String {
    // Nukta forms as single letters.
    let mut cs: Vec<char> = Vec::with_capacity(word.len());
    for c in word.chars() {
        match (cs.last().copied(), c) {
            (Some('য'), '\u{09bc}') => *cs.last_mut().unwrap() = '\u{09df}',
            (Some('ড'), '\u{09bc}') => *cs.last_mut().unwrap() = '\u{09dc}',
            (Some('ঢ'), '\u{09bc}') => *cs.last_mut().unwrap() = '\u{09dd}',
            (_, '\u{09bc}' | '\u{200c}' | '\u{200d}' | 'ঁ' | 'ঃ') => {}
            _ => cs.push(c),
        }
    }
    let n = cs.len();
    // Vowel nuclei before position i (inherent vowels included).
    let nuclei_before = |i: usize| {
        let mut k = 0;
        for j in 0..i {
            let c = cs[j];
            if independent_vowel(c).is_some() || vowel_sign(c).is_some() {
                k += 1;
            } else if consonant(c).is_some() {
                let next = cs.get(j + 1).copied();
                let bare = next.is_none_or(|x| vowel_sign(x).is_none() && x != '্');
                if bare && j + 1 < n {
                    k += 1;
                }
            }
        }
        k
    };
    let mut out = String::with_capacity(word.len() * 2);
    let mut i = 0;
    while i < n {
        let c = cs[i];
        if let Some(v) = independent_vowel(c) {
            out.push_str(v);
            i += 1;
            continue;
        }
        if c == 'ং' {
            out.push('ꯡ');
            i += 1;
            continue;
        }
        if c == 'ৎ' {
            out.push('ꯠ');
            i += 1;
            continue;
        }
        let Some(mut m) = consonant(c) else {
            if let Some(v) = vowel_sign(c) {
                // A sign without a consonant: write it on ꯑ.
                out.push('ꯑ');
                out.push_str(v);
            }
            i += 1;
            continue;
        };
        // ব after hasanta following a stop is w (খ্ব).
        if c == 'ব' && i >= 2 && cs[i - 1] == '্' && !matches!(cs[i - 2], 'ম' | 'ন' | 'ল' | 'র')
        {
            m = 'ꯋ';
        }
        let next = cs.get(i + 1).copied();
        match next {
            Some('্') => {
                let c2 = cs.get(i + 2).copied().filter(|x| consonant(*x).is_some());
                match c2 {
                    None => {
                        // Hasanta at the end: a closed syllable.
                        match lonsum(m) {
                            Some(l) => out.push(l),
                            None => {
                                out.push(m);
                                out.push('꯭');
                            }
                        }
                        i += 2;
                    }
                    Some(c2) => {
                        let glide = matches!(c2, 'র' | 'ৰ' | 'য' | '\u{09df}' | 'ৱ')
                            || (c2 == 'ব' && !matches!(c, 'ম' | 'ন' | 'ল' | 'র'));
                        let onset_l = c2 == 'ল' && nuclei_before(i) == 0;
                        match lonsum(m).filter(|_| !glide && !onset_l) {
                            Some(l) => out.push(l),
                            None => {
                                out.push(m);
                                out.push('꯭');
                            }
                        }
                        i += 2;
                    }
                }
            }
            Some(s) if vowel_sign(s).is_some() => {
                out.push(m);
                out.push_str(vowel_sign(s).unwrap());
                i += 2;
            }
            None => {
                // A bare consonant at the end closes the syllable; ꯌ after a
                // vowel is read as a glide by the rules.
                match lonsum(m) {
                    Some(l) => out.push(l),
                    None if m == 'ꯌ' => out.push(m),
                    None => {
                        out.push(m);
                        out.push('꯭');
                    }
                }
                i += 1;
            }
            _ => {
                out.push(m);
                i += 1;
            }
        }
    }
    out
}

fn mm_consonant(c: char) -> bool {
    ('\u{abc0}'..='\u{abcd}').contains(&c)
        || ('\u{abd0}'..='\u{abda}').contains(&c) && c != 'ꯑ'
        || ('\u{aae2}'..='\u{aaea}').contains(&c)
}

fn mm_sign(c: char) -> bool {
    ('\u{abe3}'..='\u{abe9}').contains(&c) || ('\u{aaeb}'..='\u{aaef}').contains(&c)
}

/// Consonants of the suffixes whose vowel is ɐ.
fn suffix_consonant(c: char) -> bool {
    matches!(c, 'ꯕ' | 'ꯄ' | 'ꯗ' | 'ꯇ' | 'ꯅ' | 'ꯒ' | 'ꯀ')
}

/// Is `c` (a suffix consonant) the one the suffix has after `prev`? p t k
/// after ꯛ ꯠ ꯞ, b d ɡ after anything else, n anywhere.
fn suffix_after(prev: char, c: char) -> bool {
    let voiceless = matches!(prev, 'ꯛ' | 'ꯠ' | 'ꯞ');
    match c {
        'ꯅ' => true,
        'ꯄ' | 'ꯇ' | 'ꯀ' => voiceless,
        _ => !voiceless,
    }
}

/// One Meetei Mayek word with its closed syllables and suffix vowels
/// written out (see the module documentation).
fn prepare_word(word: &str) -> Option<String> {
    let mut cs: Vec<char> = word.chars().collect();
    let n = cs.len();
    let mut hit = false;
    // Does the consonant at `j` carry a vowel (a sign, its inherent vowel,
    // or the vowel of the cluster it starts)?
    let has_vowel = |cs: &[char], j: usize| -> bool {
        let mut j = j;
        loop {
            if j >= cs.len() || !mm_consonant(cs[j]) {
                return false;
            }
            match cs.get(j + 1) {
                Some(&c) if mm_sign(c) || c == 'ꯪ' => return true,
                Some('꯭') => j += 2,
                Some(_) => return true,
                // A final consonant keeps its vowel after the suffix
                // consonants (lang/mni.lang, final-keep-after).
                None => return suffix_consonant(cs[j]) || cs[j] == 'ꯃ',
            }
        }
    };
    let mut i = n;
    while i > 1 {
        i -= 1;
        let c = cs[i];
        let Some(l) = lonsum(c) else { continue };
        let next = cs.get(i + 1).copied();
        let bare = next.is_some_and(mm_consonant);
        let prev = cs[i - 1];
        let after_vowel = mm_sign(prev) || (mm_consonant(prev) && !matches!(prev, 'ꯌ' | 'ꯋ'));
        let number = || {
            let rest: String = cs[i..].iter().collect();
            NUMBER_TAILS.iter().any(|t| rest.starts_with(t))
        };
        if bare && after_vowel && has_vowel(&cs, i + 1) && !number() {
            cs[i] = l;
            hit = true;
        }
    }
    let n = cs.len();
    // A suffix vowel written ꯥ at the end of a longer word.
    if n >= 3
        && cs[n - 1] == 'ꯥ'
        && suffix_consonant(cs[n - 2])
        && suffix_after(cs[n - 3], cs[n - 2])
        && cs[..n - 2]
            .iter()
            .any(|&c| mm_consonant(c) || mm_sign(c) || matches!(c, 'ꯑ' | 'ꯏ' | 'ꯎ'))
        && !FINAL_A.contains(&word)
    {
        cs.pop();
        hit = true;
    }
    hit.then(|| cs.into_iter().collect())
}

/// Meetei Mayek text with the closed syllables and suffix vowels that the
/// spelling leaves to the reader written out (see the module
/// documentation); `None` when nothing changes.
pub fn prepare(s: &str) -> Option<String> {
    if !has_meetei(s) {
        return None;
    }
    let mut out = String::with_capacity(s.len());
    let mut word = String::new();
    let mut hit = false;
    let flush = |word: &mut String, out: &mut String, hit: &mut bool| {
        if let Some(w) = prepare_word(word) {
            out.push_str(&w);
            *hit = true;
        } else {
            out.push_str(word);
        }
        word.clear();
    };
    for c in s.chars() {
        if script_of(c) == Script::Brahmic(0xabc0) {
            word.push(c);
        } else {
            flush(&mut word, &mut out, &mut hit);
            out.push(c);
        }
    }
    flush(&mut word, &mut out, &mut hit);
    hit.then_some(out)
}

/// True when `s` has a letter of the Bengali script.
pub fn has_bengali(s: &str) -> bool {
    s.chars().any(bengali_letter)
}

/// True when `s` is written in Bengali script (a Bengali letter or digit,
/// no Meetei Mayek letter): its number words are written in Bengali script.
pub fn writes_bengali(s: &str) -> bool {
    s.chars().any(|c| ('\u{0980}'..='\u{09ff}').contains(&c)) && !has_meetei(s)
}

/// True when `s` has a letter of Meetei Mayek.
pub fn has_meetei(s: &str) -> bool {
    s.chars().any(|c| script_of(c) == Script::Brahmic(0xabc0))
}

/// Bengali-script Meitei written in Meetei Mayek (Bengali digits become
/// Meetei Mayek digits); `None` when `s` has no Bengali letter.
pub fn bengali_to_meetei(s: &str) -> Option<String> {
    if !has_bengali(s) {
        return None;
    }
    let mut out = String::with_capacity(s.len() * 2);
    let mut word = String::new();
    for c in s.chars() {
        if bengali_letter(c) || c == '\u{200c}' || c == '\u{200d}' {
            word.push(c);
            continue;
        }
        if !word.is_empty() {
            out.push_str(&word_to_meetei(&word));
            word.clear();
        }
        match c {
            '০'..='৯' => out.push(char::from_u32(c as u32 - 0x09e6 + 0xabf0).unwrap()),
            _ => out.push(c),
        }
    }
    if !word.is_empty() {
        out.push_str(&word_to_meetei(&word));
    }
    Some(out)
}

fn meetei_consonant(c: char) -> Option<char> {
    Some(match c {
        'ꯀ' | 'ꯛ' => 'ক',
        'ꯈ' => 'খ',
        'ꯒ' => 'গ',
        'ꯘ' => 'ঘ',
        'ꯉ' | 'ꯡ' => 'ঙ',
        'ꯆ' => 'চ',
        'ꯖ' => 'জ',
        'ꯓ' => 'ঝ',
        'ꯇ' | 'ꯠ' => 'ত',
        'ꯊ' => 'থ',
        'ꯗ' => 'দ',
        'ꯙ' => 'ধ',
        'ꯅ' | 'ꯟ' => 'ন',
        'ꯄ' | 'ꯞ' => 'প',
        'ꯐ' => 'ফ',
        'ꯕ' => 'ব',
        'ꯚ' => 'ভ',
        'ꯃ' | 'ꯝ' => 'ম',
        'ꯌ' => '\u{09df}',
        'ꯔ' => 'র',
        'ꯂ' | 'ꯜ' => 'ল',
        'ꯋ' => 'ৱ',
        'ꯁ' => 'শ',
        'ꯍ' => 'হ',
        _ => return None,
    })
}

fn meetei_sign(c: char) -> Option<char> {
    Some(match c {
        'ꯥ' => 'া',
        'ꯤ' => 'ি',
        'ꯨ' => 'ু',
        'ꯦ' => 'ে',
        'ꯩ' => 'ৈ',
        'ꯣ' => 'ো',
        'ꯧ' => 'ৌ',
        _ => return None,
    })
}

fn is_lonsum(c: char) -> bool {
    ('\u{abdb}'..='\u{abe2}').contains(&c)
}

/// Meetei Mayek written in Bengali script, as Bengali-script Meitei spells:
/// a lonsum letter is the bare consonant at the end of a word (ꯠ ৎ, ꯡ ং) and
/// takes hasanta before a consonant; a final consonant letter without a vowel
/// sign (the vowel ɐ) takes া; ꯁ is শ, ꯌ য়, ꯋ ৱ. Characters of other scripts
/// stay.
pub fn meetei_to_bengali(s: &str) -> String {
    let cs: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let is_mm = |c: Option<&char>| c.is_some_and(|c| script_of(*c) == Script::Brahmic(0xabc0));
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        let next = cs.get(i + 1);
        let word_start = i == 0 || !is_mm(cs.get(i - 1));
        match c {
            'ꯑ' => match next.copied().and_then(|n| {
                Some(match n {
                    'ꯥ' => "আ",
                    'ꯤ' => "ই",
                    'ꯨ' => "উ",
                    'ꯦ' => "এ",
                    'ꯩ' => "ঐ",
                    'ꯣ' => "ও",
                    'ꯧ' => "ঔ",
                    _ => return None,
                })
            }) {
                Some(v) => {
                    out.push_str(v);
                    i += 2;
                    continue;
                }
                None => out.push('অ'),
            },
            'ꯏ' | 'ꯢ' => out.push('ই'),
            'ꯎ' => out.push('উ'),
            'ꯪ' | 'ꯡ' => out.push('ং'),
            'ꯠ' if !is_mm(next) || next == Some(&'꯫') => out.push('ৎ'),
            '꯭' => out.push('্'),
            '꯬' => {}
            '\u{abf0}'..='\u{abf9}' => {
                out.push(char::from_u32(c as u32 - 0xabf0 + 0x09e6).unwrap());
            }
            '꯫' => out.push('।'),
            _ => {
                if let Some(b) = meetei_consonant(c) {
                    out.push(b);
                    if is_lonsum(c) {
                        if next.is_some_and(|n| meetei_consonant(*n).is_some()) {
                            out.push('্');
                        }
                    } else if !next.is_some_and(|n| meetei_sign(*n).is_some() || *n == '꯭')
                        && !is_mm(next)
                        && !word_start
                    {
                        // The final vowel ɐ.
                        out.push('া');
                    }
                } else if let Some(v) = meetei_sign(c) {
                    out.push(v);
                } else {
                    out.push(c);
                }
            }
        }
        i += 1;
    }
    out
}

/// Common romanised Meitei words and their Meetei Mayek spelling (sorted):
/// the words that signal a Meitei clause and words whose spelling the rules
/// would not find. Spellings as in IndicCorp v2 Manipuri text.
const ROMAN_MNI: &[(&str, &str)] = &[
    ("adu", "ꯑꯗꯨ"),
    ("amadi", "ꯑꯃꯗꯤ"),
    ("amasung", "ꯑꯃꯁꯨꯡ"),
    ("asi", "ꯑꯁꯤ"),
    ("ei", "ꯑꯩ"),
    ("eikhoi", "ꯑꯩꯈꯣꯏ"),
    ("eina", "ꯑꯩꯅꯥ"),
    ("haibadi", "ꯍꯥꯏꯕꯗꯤ"),
    ("houjik", "ꯍꯧꯖꯤꯛ"),
    ("karamna", "ꯀꯔꯝꯅꯥ"),
    ("kari", "ꯀꯔꯤ"),
    ("khara", "ꯈꯔ"),
    ("leibak", "ꯂꯩꯕꯥꯛ"),
    ("mahak", "ꯃꯍꯥꯛ"),
    ("makhoi", "ꯃꯈꯣꯌ"),
    ("manipur", "ꯃꯅꯤꯄꯨꯔ"),
    ("matam", "ꯃꯇꯝ"),
    ("meetei", "ꯃꯤꯇꯩ"),
    ("meeteilon", "ꯃꯤꯇꯩꯂꯣꯟ"),
    ("meitei", "ꯃꯩꯇꯩ"),
    ("meiteilon", "ꯃꯩꯇꯩꯂꯣꯟ"),
    ("nang", "ꯅꯪ"),
    ("nangbu", "ꯅꯪꯕꯨ"),
    ("natte", "ꯅꯠꯇꯦ"),
    ("ngasi", "ꯉꯁꯤ"),
    ("nungsi", "ꯅꯨꯡꯁꯤ"),
    ("thagatchari", "ꯊꯥꯒꯠꯆꯔꯤ"),
    ("yamna", "ꯌꯥꯝꯅꯥ"),
];

/// Listed words that mark a clause as Meitei on their own.
const SIGNAL: &[&str] = &[
    "adu",
    "amadi",
    "amasung",
    "asi",
    "eikhoi",
    "eina",
    "haibadi",
    "houjik",
    "karamna",
    "khara",
    "leibak",
    "mahak",
    "makhoi",
    "meeteilon",
    "meiteilon",
    "nangbu",
    "natte",
    "ngasi",
    "nungsi",
    "thagatchari",
    "yamna",
];

/// The Meetei Mayek letter of a romanised consonant and the number of Latin
/// letters it takes.
fn roman_consonant(cs: &[char]) -> Option<(char, usize)> {
    let two: String = cs.iter().take(2).collect();
    let two_letter = match two.as_str() {
        "kh" => Some('ꯈ'),
        "gh" => Some('ꯘ'),
        "ng" => Some('ꯉ'),
        "ch" => Some('ꯆ'),
        "jh" => Some('ꯓ'),
        "th" => Some('ꯊ'),
        "dh" => Some('ꯙ'),
        "ph" => Some('ꯐ'),
        "bh" => Some('ꯚ'),
        "sh" => Some('ꯁ'),
        _ => None,
    };
    if let Some(l) = two_letter {
        return Some((l, 2));
    }
    Some((
        match cs[0] {
            'k' | 'q' | 'c' => 'ꯀ',
            'g' => 'ꯒ',
            'j' | 'z' => 'ꯖ',
            't' => 'ꯇ',
            'd' => 'ꯗ',
            'n' => 'ꯅ',
            'p' => 'ꯄ',
            'f' => 'ꯐ',
            'b' => 'ꯕ',
            'm' => 'ꯃ',
            'y' => 'ꯌ',
            'r' => 'ꯔ',
            'l' => 'ꯂ',
            'w' | 'v' => 'ꯋ',
            's' => 'ꯁ',
            'h' => 'ꯍ',
            _ => return None,
        },
        1,
    ))
}

/// A romanised vowel at the start of `cs`: (sign after a consonant, letters
/// at the start of a word, Latin letters taken). The sign is empty for the
/// inherent vowel ɐ (a).
fn roman_vowel(cs: &[char]) -> Option<(&'static str, &'static str, usize)> {
    let two: String = cs.iter().take(2).collect();
    Some(match two.as_str() {
        "aa" => ("ꯥ", "ꯑꯥ", 2),
        "ei" => ("ꯩ", "ꯑꯩ", 2),
        "ou" => ("ꯧ", "ꯑꯧ", 2),
        "ee" => ("ꯤ", "ꯏ", 2),
        "oo" => ("ꯨ", "ꯎ", 2),
        _ => match cs.first()? {
            'a' => ("", "ꯑ", 1),
            'i' => ("ꯤ", "ꯏ", 1),
            'u' => ("ꯨ", "ꯎ", 1),
            'e' => ("ꯦ", "ꯑꯦ", 1),
            'o' => ("ꯣ", "ꯑꯣ", 1),
            _ => return None,
        },
    })
}

/// Romanised Meitei in Meetei Mayek by rule: a is the inherent vowel ɐ (no
/// sign), aa ꯥ, i ee ꯤ, u oo ꯨ, e ꯦ, o ꯣ, ei ꯩ, ou ꯧ; i and u (o after a)
/// after a vowel close a diphthong, written ꯏ ꯎ, and the a before them is
/// ꯥ (lai ꯂꯥꯏ, chaoba ꯆꯥꯎꯕ: romanised Meitei writes a for both vowels
/// there); kh gh ng ch jh th dh ph bh sh with their letters; a consonant not
/// followed by a vowel is a lonsum letter (k l m p n t ng: ꯛ ꯜ ꯝ ꯞ ꯟ ꯠ ꯡ), or
/// takes apun iyek ꯭ before r w y and when it has no lonsum form (thr ꯊ꯭ꯔ).
/// An apostrophe or a hyphen separates syllables (laak'i ꯂꯥꯛꯏ).
pub fn transliterate(word: &str) -> String {
    let cs: Vec<char> = word.to_lowercase().chars().collect();
    let is_vowel_at = |k: usize| roman_vowel(&cs[k.min(cs.len())..]).is_some();
    // i u (o after a) right after a vowel and not before one: a glide.
    let glide_at = |k: usize, prev: char| {
        (matches!(cs[k], 'i' | 'u') || (cs[k] == 'o' && prev == 'a')) && !is_vowel_at(k + 1)
    };
    let mut out = String::new();
    let mut i = 0;
    let mut start = true;
    let mut prev_vowel: Option<char> = None;
    while i < cs.len() {
        let c = cs[i];
        if matches!(c, '\'' | '’' | 'ʼ' | '-') {
            i += 1;
            start = true;
            prev_vowel = None;
            continue;
        }
        if let Some(p) = prev_vowel {
            if glide_at(i, p) {
                out.push(if c == 'i' { 'ꯏ' } else { 'ꯎ' });
                i += 1;
                prev_vowel = None;
                continue;
            }
        }
        if let Some((sign, initial, len)) = roman_vowel(&cs[i..]) {
            let diphthong = len == 1 && c == 'a' && i + 1 < cs.len() && glide_at(i + 1, 'a');
            if start || prev_vowel.is_some() {
                // At the start, and the second of two vowels (ꯃꯤꯑꯣꯏ): on ꯑ.
                out.push_str(if diphthong { "ꯑꯥ" } else { initial });
            } else {
                out.push_str(if diphthong { "ꯥ" } else { sign });
            }
            i += len;
            start = false;
            prev_vowel = Some(cs[i - 1]);
            continue;
        }
        let Some((letter, len)) = roman_consonant(&cs[i..]) else {
            i += 1;
            continue;
        };
        let j = i + len;
        if j < cs.len() && is_vowel_at(j) {
            out.push(letter);
        } else {
            match lonsum(letter).filter(|_| !matches!(cs.get(j), Some('r' | 'w' | 'y'))) {
                Some(l) => out.push(l),
                None => {
                    out.push(letter);
                    out.push('꯭');
                }
            }
        }
        i = j;
        start = false;
        prev_vowel = None;
    }
    out
}

/// Rewrite the Latin words of a clause that are romanised Meitei in Meetei
/// Mayek: listed words always, other words unless `english` knows them, and
/// only in a clause with a listed word of [`SIGNAL`].
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
    let listed = |w: &str| ROMAN_MNI.binary_search_by(|p| p.0.cmp(w)).ok();
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
            it.text = ROMAN_MNI[k].1.to_string();
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
        assert!(ROMAN_MNI.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(SIGNAL.iter().all(|s| ROMAN_MNI.iter().any(|p| p.0 == *s)));
        assert!(FINAL_A.iter().all(|w| prepare_word(w).is_none()));
    }

    #[test]
    fn bengali_script() {
        for (b, m) in [
            ("মতম", "ꯃꯇꯝ"),
            ("থবক", "ꯊꯕꯛ"),
            ("মহাক্না", "ꯃꯍꯥꯛꯅꯥ"),
            ("লৈবা", "ꯂꯩꯕꯥ"),
            ("অসিদা", "ꯑꯁꯤꯗꯥ"),
            ("না", "ꯅꯥ"),
            ("মমা", "ꯃꯃꯥ"),
            ("খ্বাইদগী", "ꯈ꯭ꯋꯥꯏꯗꯒꯤ"),
            ("করিগুম্বা", "ꯀꯔꯤꯒꯨꯝꯕꯥ"),
            ("শহর", "ꯁꯍꯔ꯭"),
            ("মীংথোল", "ꯃꯤꯡꯊꯣꯜ"),
            ("নুমিৎ", "ꯅꯨꯃꯤꯠ"),
            ("ৱাখল", "ꯋꯥꯈꯜ"),
            ("হায়না", "ꯍꯥꯌꯅꯥ"),
            ("মখোয়", "ꯃꯈꯣꯌ"),
            ("ত্রিপ", "ꯇ꯭ꯔꯤꯞ"),
            ("কুন্থ্রা", "ꯀꯨꯟꯊ꯭ꯔꯥ"),
            ("তরা", "ꯇꯔꯥ"),
        ] {
            assert_eq!(bengali_to_meetei(b).unwrap(), m, "{b}");
        }
        assert_eq!(bengali_to_meetei("১৭শুবা।").unwrap(), "꯱꯷ꯁꯨꯕꯥ।");
        assert_eq!(bengali_to_meetei("ꯑꯁꯤ"), None);
    }

    #[test]
    fn reading_habits() {
        for (w, p) in [
            ("ꯃꯇꯃꯗꯥ", Some("ꯃꯇꯝꯗ")),
            ("ꯂꯥꯎꯕꯥ", Some("ꯂꯥꯎꯕ")),
            ("ꯑꯃꯁꯨꯡ", None),
            ("ꯑꯃꯗꯥ", Some("ꯑꯃꯗ")),
            ("ꯑꯉꯀꯄꯥ", Some("ꯑꯉꯛꯄ")),
            ("ꯍꯧꯔꯀꯈꯤ", Some("ꯍꯧꯔꯛꯈꯤ")),
            ("ꯀꯅꯥ", None),
            ("ꯅꯥ", None),
            ("ꯑꯗꯨꯃꯛ", None),
            ("ꯁꯤꯖꯤꯟꯅꯕ", None),
        ] {
            assert_eq!(prepare_word(w).as_deref(), p, "{w}");
        }
        assert_eq!(prepare("abc"), None);
    }

    #[test]
    fn to_bengali() {
        for (m, b) in [
            ("ꯑꯃꯥ", "অমা"),
            ("ꯂꯤꯁꯤꯡ", "লিশিং"),
            ("ꯇꯔꯦꯠ", "তরেৎ"),
            ("ꯅꯤꯄꯥꯟ", "নিপান"),
            ("ꯀꯨꯟꯊ꯭ꯔꯥ", "কুন্থ্রা"),
            ("ꯆꯥꯅꯤꯒ ꯁꯨꯞꯅ", "চানিগা শুপ্না"),
            ("ꯇꯔꯨꯛ", "তরুক"),
        ] {
            assert_eq!(meetei_to_bengali(m), b, "{m}");
        }
    }

    #[test]
    fn rules() {
        for (r, o) in [
            ("chaaba", "ꯆꯥꯕ"),
            ("laak'i", "ꯂꯥꯛꯏ"),
            ("leibak", "ꯂꯩꯕꯛ"),
            ("thoibi", "ꯊꯣꯏꯕꯤ"),
            ("lai", "ꯂꯥꯏ"),
            ("chaoba", "ꯆꯥꯎꯕ"),
            ("imphal", "ꯏꯝꯐꯜ"),
            ("ngaraang", "ꯉꯔꯥꯡ"),
            ("kangleipak", "ꯀꯡꯂꯩꯄꯛ"),
            ("mioi", "ꯃꯤꯑꯣꯏ"),
        ] {
            assert_eq!(transliterate(r), o, "{r}");
        }
    }

    #[test]
    fn rewrites_a_clause() {
        let mut items = vec![
            Item::word("Eina"),
            Item::word("office"),
            Item::word("chatli"),
            Item::word("ngasi"),
        ];
        for it in &mut items {
            // Words of the text, not of a number reading.
            it.generated = false;
        }
        rewrite_roman(&mut items, |w| w == "office");
        let t: Vec<&str> = items.iter().map(|i| i.text.as_str()).collect();
        assert_eq!(t, ["ꯑꯩꯅꯥ", "office", "ꯆꯠꯂꯤ", "ꯉꯁꯤ"]);
        let mut english = vec![Item::word("my"), Item::word("office")];
        for it in &mut english {
            it.generated = false;
        }
        rewrite_roman(&mut english, |_| false);
        assert_eq!(english[0].text, "my");
    }
}
