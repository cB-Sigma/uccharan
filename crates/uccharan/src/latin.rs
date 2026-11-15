//! Data-driven alphabetic languages in Latin script (`lang/es.lang`, ...).
//!
//! A word is read in four steps, each described by the language file:
//!
//! 1. letter rules (`[pre]`, see [`crate::rules`]) turn the lower-cased
//!    spelling into phonemes; a rule may write `ˈ` right before a vowel it
//!    knows to be stressed (a written accent);
//! 2. syllables are the vowels listed in `vowels =` of `[meta]` (glides and
//!    the second half of a diphthong are not listed, so they do not count);
//! 3. the stress module places primary stress (`stress =`) when no rule
//!    did, then secondary stress on alternate syllables before it;
//! 4. phoneme rules (`[post]`).
//!
//! Inside a clause the words are then joined with `‿` and the `[clause]`
//! rules run over the whole string, for changes across word boundaries
//! (Spanish spirantisation and nasal assimilation), then the mode's own
//! phoneme rules (`[post.compat]`, `[post.improved]`) on each word.
//!
//! The stress rules:
//!
//! * `spanish`: the last syllable when the word ends in a consonant other
//!   than n or s, else the one before it;
//! * `penult`, `initial`, `final`: by position.
//!
//! `secondary = alternate` puts secondary stress on the first syllable and
//! every second one after it, up to two syllables before the primary;
//! `secondary = auto` on every free syllable between two unstressed ones.
//! Vowels listed in `unstressable =` never take stress.

use std::collections::HashMap;

use crate::langdef::{self, LangFile, Numbers};
use crate::rules::{Classes, RuleSet};

/// Where primary stress goes when no letter rule marked it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stress {
    /// Last syllable after a consonant other than n or s, else the penult.
    Spanish,
    /// The second syllable from the end.
    Penult,
    /// The first syllable.
    Initial,
    /// The last syllable.
    Final,
    /// The third syllable from the end.
    Antepenult,
}

/// A Latin-script language.
#[derive(Debug)]
pub struct Latin {
    /// Language tag.
    pub tag: String,
    classes: Classes,
    /// Letter passes (`[pre]`, `[pre.2]`, ...), applied in order.
    pre: Vec<RuleSet>,
    /// Phoneme passes after stress (`[post]`, `[post.2]`, ...).
    post: Vec<RuleSet>,
    post_compat: RuleSet,
    post_improved: RuleSet,
    clause: RuleSet,
    vowels: Vec<char>,
    /// Nuclei that take primary stress only when the word has no other
    /// (`unstressable =`): reduced vowels and the placeholders that letter
    /// rules write for unstressed prefixes.
    unstressable: Vec<char>,
    /// Letters that count as vowels when deciding whether a word can be
    /// read or must be spelled.
    spell_vowels: String,
    /// A letter name's open e is close when the name loses its stress
    /// (`spell-close-e = yes`).
    spell_close_e: bool,
    /// The main stress of a spelled abbreviation is on its first letter
    /// (`spell-stress = first`), not its last.
    spell_first: bool,
    /// A single consonant letter is read by its name even when the word
    /// table has an entry for it (`letter-names = yes`).
    letter_names: bool,
    /// The table's entries for two- and three-letter capitals are used
    /// (`caps-entries = yes`); otherwise such a token is spelled.
    caps_entries: bool,
    /// Capital `I` is the dotless `ı` (`dotless-i = yes`, Turkish).
    dotless_i: bool,
    /// Small words that are read as words when written in capitals
    /// (`caps-words = de het`; `*` stands for the `[unstressed]` list).
    /// Other capitals of two or three letters are spelled.
    caps_words: Vec<String>,
    stress: Stress,
    secondary: bool,
    /// `elision = yes`: `l'`, `qu'` and the like are read with the next word.
    elision: bool,
    /// `sandhi-block = h`: words that begin with one of these letters, and
    /// numbers written in digits, take no part in the `[clause]` rules
    /// with the word before them.
    sandhi_block: Option<String>,
    /// `[aspirated]`: words (or, with a final `*`, beginnings of words)
    /// that no clause rule reaches into.
    aspirated: Vec<(String, bool)>,
    /// `[h-muet.compat]`: in compatible mode every word written with h
    /// first is treated as in `[aspirated]` unless it is in this list
    /// (words, or beginnings of words with a final `*`), as the French
    /// reference voice does. Empty: off.
    h_muet: Vec<(String, bool)>,
    /// `fuse = na do ...`: these words make one word with the word after
    /// them, stressed again as a whole (Czech prepositions).
    fuse: Vec<String>,
    /// `promote-before-final = a az`: these words take primary stress
    /// before a last word of a clause that has only secondary stress there
    /// (`word p s` in `[unstressed]`).
    promote: Vec<String>,
    /// `clitic-verbs = ont sont` and `clitic-subjects = ils elles ...`:
    /// inside a clause, these verbs have no stress after one of these
    /// pronouns (French "ils ont", not "les enfants ont").
    clitic_verbs: Vec<String>,
    /// `[context.compat]` lines `WORD PREV|PREV... READING`: in compatible
    /// mode the word right after one of these words (`*a`: a word ending
    /// in a) is read so (French "de son", "le son": the noun, stressed and
    /// with no liaison; Catalan "i" after a vowel letter).
    context: Vec<(String, Vec<String>, String)>,
    clitic_subjects: Vec<String>,
    /// `vowel-mark = yes`: inside a clause, a word written with a vowel
    /// letter first has `⁑` before its phonemes for the `[clause]` rules.
    vowel_mark: bool,
    /// `vowel-mark = h`: also words written with a silent h and a vowel.
    vowel_mark_h: bool,
    /// `vowel-mark-except = u`: a word written with one of these letters
    /// first gets `⁂` before its `⁑` (Catalan: no glide before "un").
    vowel_mark_except: String,
    /// `secondary = auto`: trochaic secondary stresses.
    auto_secondary: bool,
    /// `stress-options =` words (see [`Latin::from_text`]).
    no_auto_secondary: bool,
    final_no_secondary: bool,
    initial_secondary: bool,
    two_syllable_secondary: bool,
    final_vowel_unstressed: bool,
    final_long: bool,
    words: HashMap<String, (String, String)>,
    /// `[words.improved]`: readings used in improved mode only, before the
    /// word table (where the reference voice's reading is a known error).
    words_improved: HashMap<String, String>,
    /// `[spelling.improved]` lines `LETTERS FROM TO`: in improved mode a
    /// word written with LETTERS has FROM in its reading replaced by TO, at
    /// most once for each time it has the letters (French gn: nj -> ɲ).
    spelling_improved: Vec<(String, String, String)>,
    unstressed: HashMap<String, (u8, u8)>,
    /// A suffix read with its own stress after the stem's (`-mente`).
    suffix: Option<(String, String)>,
    /// Letter names for spelled words (`[spell]`).
    spell: HashMap<char, String>,
    /// Letters beyond ASCII that the language file reads (`[spell]` and
    /// the letter rules of `[pre]`): see [`Latin::fold_foreign`].
    alphabet: String,
    /// Number words.
    pub numbers: Numbers,
}

const JOIN: char = '‿';
/// Stands before a word that is cut off from the word before it.
const BLOCK: char = '⁞';
/// Stands before a word written with a vowel letter first.
const VOWEL: char = '⁑';
/// Before [`VOWEL`] for a word written with a `vowel-mark-except` letter first.
const EXCEPT: char = '⁂';

impl Latin {
    /// Parse a language file.
    pub fn from_text(text: &str) -> Result<Latin, String> {
        let f = LangFile::parse(text);
        let meta = f.meta();
        let mut classes = f.classes();
        let mut passes = |name: &str| -> Result<Vec<RuleSet>, String> {
            let mut v = vec![f.rules(name, &mut classes)?];
            for k in 2..=6 {
                let r = f.rules(&format!("{name}.{k}"), &mut classes)?;
                if !r.is_empty() {
                    v.push(r);
                }
            }
            v.retain(|r| !r.is_empty());
            Ok(v)
        };
        let pre = passes("pre")?;
        let post = passes("post")?;
        let post_compat = f.rules("post.compat", &mut classes)?;
        let post_improved = f.rules("post.improved", &mut classes)?;
        let clause = f.rules("clause", &mut classes)?;
        let stress = match meta.get("stress").map(String::as_str) {
            Some("spanish") => Stress::Spanish,
            Some("initial") => Stress::Initial,
            Some("final") => Stress::Final,
            Some("antepenult") => Stress::Antepenult,
            _ => Stress::Penult,
        };
        // `stress-options =` final-no-secondary (no secondary stress on the
        // last syllable), initial-secondary (a lone syllable before the
        // primary gets one), two-syllable-secondary, final-vowel-unstressed
        // (a word-final vowel gives its stress to the syllable before),
        // final-long (penult rule: a long final vowel after a short one).
        let options = meta.get("stress-options").cloned().unwrap_or_default();
        let opt = |name: &str| options.split_whitespace().any(|o| o == name);
        Ok(Latin {
            tag: meta.get("tag").cloned().unwrap_or_default(),
            pre,
            post,
            post_compat,
            post_improved,
            clause,
            vowels: meta
                .get("vowels")
                .map(|v| {
                    v.split_whitespace()
                        .filter_map(|x| x.chars().next())
                        .collect()
                })
                .unwrap_or_default(),
            unstressable: meta
                .get("unstressable")
                .map(|v| {
                    v.split_whitespace()
                        .filter_map(|x| x.chars().next())
                        .collect()
                })
                .unwrap_or_default(),
            spell_vowels: meta
                .get("spell-vowels")
                .cloned()
                .unwrap_or_else(|| "aeiouáéíóúüy".to_string()),
            spell_close_e: meta.get("spell-close-e").is_some_and(|v| v == "yes"),
            spell_first: meta.get("spell-stress").is_some_and(|v| v == "first"),
            letter_names: meta.get("letter-names").is_some_and(|v| v == "yes"),
            caps_entries: meta.get("caps-entries").is_some_and(|v| v == "yes"),
            dotless_i: meta.get("dotless-i").is_some_and(|v| v == "yes"),
            caps_words: meta
                .get("caps-words")
                .map(|v| v.split_whitespace().map(str::to_string).collect())
                .unwrap_or_default(),
            stress,
            secondary: meta.get("secondary").is_some_and(|v| v == "alternate"),
            elision: meta.get("elision").is_some_and(|v| v == "yes"),
            sandhi_block: meta.get("sandhi-block").cloned(),
            aspirated: word_patterns(&f.lines("aspirated")),
            h_muet: word_patterns(&f.lines("h-muet.compat")),
            fuse: meta
                .get("fuse")
                .map(|v| v.split_whitespace().map(str::to_string).collect())
                .unwrap_or_default(),
            promote: meta
                .get("promote-before-final")
                .map(|v| v.split_whitespace().map(str::to_string).collect())
                .unwrap_or_default(),
            context: f
                .lines("context.compat")
                .iter()
                .filter_map(|l| {
                    let mut p = l.split_whitespace();
                    let w = p.next()?.to_string();
                    let prev = p.next()?.split('|').map(str::to_string).collect();
                    Some((w, prev, p.next()?.to_string()))
                })
                .collect(),
            clitic_verbs: meta
                .get("clitic-verbs")
                .map(|v| v.split_whitespace().map(str::to_string).collect())
                .unwrap_or_default(),
            clitic_subjects: meta
                .get("clitic-subjects")
                .map(|v| v.split_whitespace().map(str::to_string).collect())
                .unwrap_or_default(),
            vowel_mark: meta
                .get("vowel-mark")
                .is_some_and(|v| v == "yes" || v == "h"),
            vowel_mark_h: meta.get("vowel-mark").is_some_and(|v| v == "h"),
            vowel_mark_except: meta.get("vowel-mark-except").cloned().unwrap_or_default(),
            auto_secondary: meta
                .get("secondary")
                .is_some_and(|v| v == "auto" || v == "none"),
            no_auto_secondary: meta.get("secondary").is_some_and(|v| v == "none"),
            final_no_secondary: opt("final-no-secondary"),
            initial_secondary: opt("initial-secondary"),
            two_syllable_secondary: opt("two-syllable-secondary"),
            final_vowel_unstressed: opt("final-vowel-unstressed"),
            final_long: opt("final-long"),
            words: langdef::words(&f),
            spelling_improved: f
                .lines("spelling.improved")
                .iter()
                .filter_map(|l| {
                    let mut p = l.split_whitespace();
                    Some((
                        p.next()?.to_string(),
                        p.next()?.to_string(),
                        p.next()?.to_string(),
                    ))
                })
                .collect(),
            words_improved: f
                .lines("words.improved")
                .iter()
                .filter_map(|l| {
                    let (w, r) = l.split_once(char::is_whitespace)?;
                    Some((w.to_string(), r.trim().to_string()))
                })
                .collect(),
            unstressed: langdef::unstressed(&f),
            suffix: meta.get("suffix").and_then(|v| {
                let (a, b) = v.split_once(char::is_whitespace)?;
                Some((a.to_string(), b.trim().to_string()))
            }),
            numbers: Numbers::parse(&f),
            spell: f
                .lines("spell")
                .iter()
                .filter_map(|l| {
                    let (k, v) = l.split_once(char::is_whitespace)?;
                    Some((k.chars().next()?, v.trim().to_string()))
                })
                .collect(),
            alphabet: {
                // The letters of `[spell]` and of the left side of the
                // first letter rules (later passes read phonemes).
                let mut a: Vec<char> = f
                    .lines("spell")
                    .iter()
                    .filter_map(|l| l.chars().next())
                    .chain(f.lines("pre").iter().flat_map(|l| {
                        l.split_once("->")
                            .map_or("", |(lhs, _)| lhs)
                            .chars()
                            .collect::<Vec<_>>()
                    }))
                    .filter(|c| c.is_alphabetic() && !c.is_ascii())
                    .flat_map(char::to_lowercase)
                    .collect();
                a.sort_unstable();
                a.dedup();
                a.into_iter().collect()
            },
            classes,
        })
    }

    fn is_vowel(&self, c: char) -> bool {
        self.vowels.contains(&c)
    }

    /// Place stress marks on a phoneme string from the letter rules.
    ///
    /// Letter rules may write `ˈ`, `ˌ` or `°` (never stressed) right before
    /// a vowel. Without a written primary stress the position rule picks
    /// one; `secondary = auto` then gives secondary stress to every
    /// syllable that is free and stands between two syllables without
    /// stress, scanning from the left (a trochaic pattern).
    fn stress(&self, ph: &str, spelling: &str) -> String {
        // Short words (nearly all) are worked on without the heap.
        const SHORT: usize = 64;
        let (mut cs_short, mut nuclei_short, mut level_short) =
            (['\0'; SHORT], [0usize; SHORT], [0i8; SHORT]);
        let (mut cs_long, mut nuclei_long, mut level_long) = (Vec::new(), Vec::new(), Vec::new());
        let short = ph.len() <= SHORT;
        let cs: &mut [char] = if short {
            let mut k = 0;
            for c in ph.chars() {
                cs_short[k] = c;
                k += 1;
            }
            &mut cs_short[..k]
        } else {
            cs_long.extend(ph.chars());
            &mut cs_long
        };
        let n = cs.iter().filter(|&&c| self.is_vowel(c)).count();
        let (nuclei, level): (&mut [usize], &mut [i8]) = if short {
            (&mut nuclei_short[..n], &mut level_short[..n])
        } else {
            nuclei_long.resize(n, 0);
            level_long.resize(n, 0);
            (&mut nuclei_long, &mut level_long)
        };
        let mut k = 0;
        for (i, &c) in cs.iter().enumerate() {
            if self.is_vowel(c) {
                nuclei[k] = i;
                k += 1;
            }
        }
        let is_mark = |c: char| matches!(c, 'ˈ' | 'ˌ' | '°');
        if n == 0 {
            return ph.chars().filter(|&c| !is_mark(c)).collect();
        }
        // Levels: -1 free, 0 never stressed, 2 secondary, 4 primary.
        // `‖` and `¦` are morpheme boundaries whose next free syllable
        // takes primary or secondary stress.
        let mut pending: i8 = -1;
        let mut v = 0;
        for (i, &c) in cs.iter().enumerate() {
            match c {
                '‖' => pending = 4,
                '¦' => pending = 2,
                _ => {}
            }
            if v < n && nuclei[v] == i {
                let l = match i.checked_sub(1).map(|j| cs[j]) {
                    Some('ˈ') => 4,
                    Some('ˌ') => 2,
                    Some('°') => 0,
                    _ if self.unstressable.contains(&c) => 0,
                    _ => std::mem::replace(&mut pending, -1),
                };
                level[v] = l;
                v += 1;
            }
        }
        for c in cs.iter_mut() {
            if matches!(*c, '‖' | '¦') {
                *c = '‧';
            }
        }
        let cs: &[char] = cs;
        let long = |k: usize| cs.get(nuclei[k] + 1) == Some(&'ː');
        let marked = level.iter().position(|&l| l == 4);
        let free = |k: usize| level[k] != 0;
        let primary = marked.or_else(|| match self.stress {
            // The first free syllable, found by the scan below.
            Stress::Initial if self.auto_secondary => None,
            Stress::Initial => Some((0..n).find(|&k| free(k)).unwrap_or(0)),
            Stress::Final => Some((0..n).rev().find(|&k| free(k)).unwrap_or(n - 1)),
            Stress::Penult | Stress::Antepenult => {
                let back = if self.stress == Stress::Penult { 2 } else { 3 };
                let mut k = n.saturating_sub(back);
                if self.final_long && n >= 2 && long(n - 1) && !long(n - 2) {
                    k = n - 1;
                }
                if level[k] == 0 && n > 1 {
                    k = if k > 0 { k - 1 } else { k + 1 };
                }
                Some(k)
            }
            Stress::Spanish => {
                let last = spelling.chars().last().unwrap_or(' ');
                Some(if matches!(last, 'a' | 'e' | 'i' | 'o' | 'u' | 'n' | 's') {
                    n.saturating_sub(2)
                } else {
                    n - 1
                })
            }
        });
        if let Some(p) = primary {
            level[p] = 4;
        }
        if self.final_vowel_unstressed
            && marked.is_none()
            && n >= 2
            && level[n - 1] == 4
            && cs.last().is_some_and(|&c| self.is_vowel(c) || c == 'ː')
        {
            level[n - 1] = 0;
            level[n - 2] = 4;
        }
        if self.secondary && primary.is_some_and(|p| p >= 2) {
            let p = primary.unwrap_or(0);
            let mut k = 0;
            while k + 1 < p {
                level[k] = 2;
                k += 2;
            }
        }
        if self.auto_secondary {
            let mut next = if level.contains(&4) { 2 } else { 4 };
            if self.two_syllable_secondary && n == 2 {
                if level[0] == 4 && level[1] < 0 {
                    level[1] = 2;
                } else if level[1] == 4 && level[0] < 0 {
                    level[0] = 2;
                }
            }
            if self.initial_secondary && n >= 3 && level[0] < 0 && level[1] >= 4 {
                level[0] = 2;
            }
            for k in 0..n {
                if level[k] >= 0 {
                    continue;
                }
                if next == 2 && (self.no_auto_secondary || (self.final_no_secondary && k + 1 == n))
                {
                    continue;
                }
                let before = if k == 0 { 0 } else { level[k - 1] };
                let after = if k + 1 == n { 0 } else { level[k + 1] };
                // A following rule-written secondary stress blocks a
                // secondary here; a primary may still stand before it.
                if before <= 0 && (after <= 0 || (next == 4 && after < 2)) {
                    level[k] = next;
                    next = 2;
                }
            }
            if !level.contains(&4) {
                // Every syllable was blocked: the first one that may carry it.
                let k = (0..n).find(|&k| level[k] != 0).unwrap_or(0);
                level[k] = 4;
            }
        }
        let mut out = String::with_capacity(ph.len() + 4);
        let mut v = 0;
        for (i, &c) in cs.iter().enumerate() {
            if is_mark(c) {
                continue;
            }
            if v < n && nuclei[v] == i {
                match level[v] {
                    4 => out.push('ˈ'),
                    2 => out.push('ˌ'),
                    _ => {}
                }
                v += 1;
            }
            out.push(c);
        }
        out
    }

    /// Pronunciation of one word.
    pub fn word(&self, word: &str, compat: bool) -> String {
        // Running text repeats its words: remember recent answers.
        let space = (self as *const Latin as usize) ^ usize::from(compat);
        crate::data::memo(space, word, || Some(self.word_uncached(word, compat)))
            .unwrap_or_default()
    }

    fn word_uncached(&self, word: &str, compat: bool) -> String {
        let lower = word.to_lowercase();
        // Number words joined with `+` keep the stress of each part, all but
        // the last demoted to secondary.
        if lower.contains('+') {
            let parts: Vec<&str> = lower.split('+').filter(|p| !p.is_empty()).collect();
            let mut out = String::new();
            for (k, p) in parts.iter().enumerate() {
                // The "y" of a number ("treinta y uno") is unstressed.
                if *p == "ì" {
                    out.push('i');
                    continue;
                }
                let mut w = self.word(p, compat);
                if k + 1 < parts.len() {
                    // Word-level clause rules first (Spanish open e).
                    w = self.clause.apply(&w, &self.classes);
                    out.push_str(&w.replace('ˈ', "ˌ"));
                } else {
                    out.push_str(&w);
                }
            }
            return out;
        }
        if let Some((c, i)) = self.words.get(&lower) {
            return if compat { c.clone() } else { i.clone() };
        }
        if let Some((suf, ipa)) = &self.suffix {
            if let Some(stem) = lower.strip_suffix(suf.as_str()) {
                if stem.chars().count() >= 3 {
                    return format!("{}{ipa}", self.word(stem, compat));
                }
            }
        }
        let mut ph = lower.clone();
        for pass in &self.pre {
            ph = pass.apply(&ph, &self.classes);
        }
        let mut s = self.stress(&ph, &lower);
        for pass in &self.post {
            s = pass.apply(&s, &self.classes);
        }
        s
    }

    /// One word read by the rules alone, as it would stand by itself (the
    /// word table is not consulted).
    pub fn rule_word(&self, word: &str, compat: bool) -> String {
        let w = self.word(word, compat);
        let w = if self.clause.is_empty() {
            w
        } else {
            self.clause.apply(&w, &self.classes)
        };
        self.finish(&w, compat)
    }

    /// The mode's phoneme rules, after the clause rules.
    fn finish(&self, s: &str, compat: bool) -> String {
        let mode = if compat {
            &self.post_compat
        } else {
            &self.post_improved
        };
        if mode.is_empty() {
            s.to_string()
        } else {
            mode.apply(s, &self.classes)
        }
    }

    /// The value of a Roman numeral of at least two letters in its usual
    /// spelling (`XIV`, not `XIIII`), up to 3999.
    fn roman(w: &str) -> Option<u64> {
        const PARTS: [(u64, &str); 13] = [
            (1000, "M"),
            (900, "CM"),
            (500, "D"),
            (400, "CD"),
            (100, "C"),
            (90, "XC"),
            (50, "L"),
            (40, "XL"),
            (10, "X"),
            (9, "IX"),
            (5, "V"),
            (4, "IV"),
            (1, "I"),
        ];
        if w.len() < 2 || w.len() > 15 {
            return None;
        }
        let (mut rest, mut n) = (w, 0u64);
        for (v, s) in PARTS {
            // A subtractive pair or V, L, D once; I, X, C, M up to three times.
            let most = if s.len() == 2 || matches!(v, 5 | 50 | 500) {
                1
            } else {
                3
            };
            for _ in 0..most {
                match rest.strip_prefix(s) {
                    Some(r) => {
                        rest = r;
                        n += v;
                    }
                    None => break,
                }
            }
        }
        // Reading the value back gives the same letters only for a proper numeral.
        let mut back = String::new();
        let mut m = n;
        for (v, s) in PARTS {
            while m >= v {
                back.push_str(s);
                m -= v;
            }
        }
        (rest.is_empty() && back == w).then_some(n)
    }

    /// The value of a token that this language reads as a Roman numeral.
    /// Compatible mode follows the voice (`roman` lines of the language
    /// file); improved mode reads the numerals up to 49 in every language.
    fn roman_number(&self, w: &str, compat: bool) -> Option<u64> {
        if self.numbers.is_empty() || (compat && self.numbers.roman_off) {
            return None;
        }
        let n = Self::roman(w)?;
        let max = match self.numbers.roman_max {
            m if compat && m > 0 => m,
            _ => 49,
        };
        (n <= max && !self.numbers.roman_skip.iter().any(|s| s == w)).then_some(n)
    }

    /// The word table's reading: the word as written when it is all in
    /// capitals, else in lower case (`lower`).
    fn listed(&self, w: &str, lower: &str, table: Option<&crate::data::Data>) -> Option<String> {
        self.listed_as(w, lower, self.words.contains_key(lower), table)
    }

    /// [`Latin::listed`]; `own`: the language file has the word itself.
    fn listed_as(
        &self,
        w: &str,
        lower: &str,
        own: bool,
        table: Option<&crate::data::Data>,
    ) -> Option<String> {
        let t = table?;
        let mut found = String::new();
        let caps = w.chars().nth(1).is_some() && w.chars().all(|c| !c.is_lowercase());
        // Two or three capitals are an abbreviation, also when the letters
        // make a word of the language ("TA", "OB"), unless the language file
        // names it as a small word that stays a word.
        let short = caps && w.chars().count() <= 3 && w.chars().all(char::is_uppercase);
        // Where Roman numerals are not read as numbers, a short one is
        // spelled whatever the table says about its letters.
        let numeral = short && self.numbers.roman_off && Self::roman(w).is_some();
        if caps && !numeral && (!short || self.caps_entries) && t.lookup_uncached(w, &mut found) {
            return Some(found);
        }
        if (own && !short) || (caps && w == lower) {
            return None;
        }
        if short && !self.caps_word(lower) {
            return None;
        }
        t.lookup_uncached(lower, &mut found).then_some(found)
    }

    /// The reading of a word by the table's letter-to-sound model, with
    /// exactly one primary stress: of several the language's stress rule
    /// keeps the last (final and penult rules) or the first; without any,
    /// the first secondary stress or the first vowel takes it.
    pub fn model_word(&self, word: &str, table: &crate::data::Data) -> Option<String> {
        let lower = word.to_lowercase();
        let space = table.id() * 2 + 1;
        crate::data::memo(space, &lower, || self.model_lower(&lower, table))
    }

    /// [`Latin::model_word`] of a lower-case word, computed afresh.
    fn model_lower(&self, lower: &str, table: &crate::data::Data) -> Option<String> {
        let s = table.model_word(lower)?;
        let s = if table.stress_latent() {
            self.stress(&s, lower)
        } else {
            self.one_primary(&s)
        };
        // The stress mark stands before a glottal stop.
        Some(if s.contains('ʔ') {
            s.replace("ʔˈ", "ˈʔ").replace("ʔˌ", "ˌʔ")
        } else {
            s
        })
    }

    /// The segments of `model` with the stress marks of `rules`, when both
    /// have the same number of syllables.
    pub fn with_stress_of(&self, model: &str, rules: &str) -> Option<String> {
        let marks = |s: &str| -> Vec<char> {
            let mut v = Vec::new();
            let mut prev = ' ';
            for c in s.chars() {
                if self.is_vowel(c) {
                    v.push(if matches!(prev, 'ˈ' | 'ˌ') {
                        prev
                    } else {
                        ' '
                    });
                }
                prev = c;
            }
            v
        };
        let want = marks(rules);
        let plain = crate::ipa::strip_stress(model);
        if marks(&plain).len() != want.len() {
            return None;
        }
        let mut out = String::with_capacity(model.len() + 4);
        let mut k = 0;
        for c in plain.chars() {
            if self.is_vowel(c) {
                if want[k] != ' ' {
                    out.push(want[k]);
                }
                k += 1;
            }
            out.push(c);
        }
        Some(out)
    }

    /// The fewest stress marks (`ˈ`, `ˌ`, and `°` for "never stressed")
    /// from which the stress rule of the language gives back the stress of
    /// `reading`. Word-table models learn these marks, not the full pattern.
    pub fn latent_marks(&self, reading: &str, spelling: &str) -> String {
        let plain: Vec<char> = reading
            .chars()
            .filter(|c| !matches!(c, 'ˈ' | 'ˌ'))
            .collect();
        let nuclei: Vec<usize> = (0..plain.len())
            .filter(|&i| self.is_vowel(plain[i]))
            .collect();
        let build = |marks: &[char]| -> String {
            let mut out = String::new();
            let mut k = 0;
            for (i, &c) in plain.iter().enumerate() {
                if k < nuclei.len() && nuclei[k] == i {
                    if marks[k] != ' ' {
                        out.push(marks[k]);
                    }
                    k += 1;
                }
                out.push(c);
            }
            out
        };
        let levels = |s: &str| -> Vec<char> {
            let mut v = Vec::new();
            let mut prev = ' ';
            for c in s.chars() {
                if self.is_vowel(c) {
                    v.push(match prev {
                        'ˈ' => 'ˈ',
                        'ˌ' => 'ˌ',
                        _ => '°',
                    });
                }
                prev = c;
            }
            v
        };
        let want = levels(reading);
        let mut marks = vec![' '; nuclei.len()];
        if want.len() != nuclei.len() {
            return reading.to_string();
        }
        for _ in 0..=nuclei.len() {
            let got = levels(&self.stress(&build(&marks), spelling));
            match (0..want.len()).find(|&k| got.get(k) != Some(&want[k])) {
                None => return build(&marks),
                Some(k) if marks[k] == ' ' => marks[k] = want[k],
                Some(_) => break,
            }
        }
        build(&want)
    }

    fn one_primary(&self, s: &str) -> String {
        let n = s.matches('ˈ').count();
        if n == 1 {
            return s.to_string();
        }
        if n == 0 {
            if s.contains('ˌ') {
                return s.replacen('ˌ', "ˈ", 1);
            }
            return match s.char_indices().find(|&(_, c)| self.is_vowel(c)) {
                Some((i, _)) => format!("{}ˈ{}", &s[..i], &s[i..]),
                None => s.to_string(),
            };
        }
        let last = !matches!(self.stress, Stress::Initial);
        let keep = if last { n - 1 } else { 0 };
        let mut k = 0;
        s.chars()
            .map(|c| {
                if c == 'ˈ' {
                    k += 1;
                    if k - 1 != keep {
                        return 'ˌ';
                    }
                }
                c
            })
            .collect()
    }

    /// True for a small word that is read as a word when in capitals.
    fn caps_word(&self, lower: &str) -> bool {
        self.caps_words
            .iter()
            .any(|c| c == lower || (c == "*" && self.unstressed.contains_key(lower)))
            || self
                .numbers
                .roman_skip
                .iter()
                .any(|r| r.eq_ignore_ascii_case(lower))
    }

    /// A word read letter by letter: an acronym in capitals, or a word
    /// without vowels. `lower` is `w` in lower case.
    fn spelled(&self, w: &str, lower: &str, own: bool) -> Option<String> {
        if self.spell.is_empty() {
            return None;
        }
        let n = w.chars().count();
        // Capitals of two or three letters are an abbreviation; longer ones
        // are read as a word when they have a vowel (`NATO`, `COVID`).
        let caps = (2..=3).contains(&n) && w.chars().all(|c| c.is_uppercase());
        let no_vowel = n >= 2
            && w.chars().all(char::is_alphabetic)
            && !lower.chars().any(|c| self.spell_vowels.contains(c));
        // A word of the language file is spelled only as a short abbreviation.
        let abbreviation = caps && !self.caps_word(lower);
        if !(abbreviation || (no_vowel && !own)) {
            return None;
        }
        // Turkish capital I is the dotless letter.
        let names: Vec<&String> = if self.dotless_i && caps {
            w.chars()
                .map(|c| match c {
                    'I' => 'ı',
                    'İ' => 'i',
                    c => c.to_lowercase().next().unwrap_or(c),
                })
                .map(|c| self.spell.get(&c))
                .collect::<Option<Vec<_>>>()?
        } else {
            lower
                .chars()
                .map(|c| self.spell.get(&c))
                .collect::<Option<Vec<_>>>()?
        };
        // The main stress is on the last letter (or the first, by the
        // language file); the others have secondary stress.
        let main = if self.spell_first { 0 } else { names.len() - 1 };
        let mut out = String::new();
        for (k, name) in names.iter().enumerate() {
            if k == main {
                out.push_str(name);
            } else if self.spell_close_e {
                out.push_str(&name.replace("ˈɛ", "ˌe").replace('ˈ', "ˌ"));
            } else {
                out.push_str(&name.replace('ˈ', "ˌ"));
            }
        }
        Some(out)
    }

    /// The reading of one word (`lower`: the same in lower case): from the
    /// word table, as a spelled word, from the table's model or from the
    /// letter rules.
    fn read(
        &self,
        w: &str,
        lower: &str,
        compat: bool,
        table: Option<&crate::data::Data>,
    ) -> String {
        let r = self.read_plain(w, lower, compat, table);
        if compat || self.spelling_improved.is_empty() {
            return r;
        }
        let mut r = r;
        for (letters, from, to) in &self.spelling_improved {
            let n = lower.matches(letters.as_str()).count();
            if n > 0 {
                r = r.replacen(from.as_str(), to, n);
            }
        }
        r
    }

    fn read_plain(
        &self,
        w: &str,
        lower: &str,
        compat: bool,
        table: Option<&crate::data::Data>,
    ) -> String {
        if !compat {
            if let Some(r) = self.words_improved.get(lower) {
                return r.clone();
            }
        }
        let own = self.words.contains_key(lower);
        // With `letter-names = yes` a consonant letter by itself is said by
        // its name (elsewhere it may be a word: Polish `w`, Czech `k`).
        let mut one = lower.chars();
        let letter = match (one.next(), one.next()) {
            (Some(c), None)
                if !own
                    && c.is_alphabetic()
                    && !self.spell_vowels.contains(c)
                    && !self.unstressed.contains_key(lower)
                    && !self.fuse.iter().any(|f| f == lower) =>
            {
                self.spell.get(&c)
            }
            _ => None,
        };
        if let Some(name) = letter.filter(|_| self.letter_names) {
            return name.clone();
        }
        if let Some(s) = self.listed_as(w, lower, own, table) {
            return s;
        }
        if let Some(s) = self.spelled(w, lower, own) {
            return s;
        }
        if own {
            return self.word(w, compat);
        }
        if let Some(r) = table.and_then(|t| self.model_lower(lower, t)) {
            return r;
        }
        // Letters of other alphabets (names: Łódź, Erdoğan) are read as
        // their base letters, as the reference voices read them.
        if let Some(folded) = self.fold_foreign(lower, table) {
            if let Some(r) = table.and_then(|t| self.model_lower(&folded, t)) {
                return r;
            }
            return self.word(&folded, compat);
        }
        self.word(w, compat)
    }

    /// `lower` with each letter that neither the language file nor the
    /// table's letter-to-sound model knows replaced by its base letter
    /// (`ł` l, `ř` r, `ø` o); `None` when there is no such letter.
    fn fold_foreign(&self, lower: &str, table: Option<&crate::data::Data>) -> Option<String> {
        let model = table.map_or(&[][..], crate::data::Data::model_letters);
        let known = |c: char| c.is_ascii() || self.alphabet.contains(c) || model.contains(&c);
        if lower.chars().all(known) {
            return None;
        }
        let mut out = String::with_capacity(lower.len());
        let mut changed = false;
        for c in lower.chars() {
            match crate::text::base_letter(c).filter(|_| !known(c)) {
                Some(b) => {
                    out.push_str(b);
                    changed = true;
                }
                None => out.push(c),
            }
        }
        changed.then_some(out)
    }

    /// One written word by itself: its key in the `[unstressed]` list and
    /// its phonemes from the word table, as a spelled word, from the
    /// table's model or from the letter rules. An elided word (`l'`, `qu'`)
    /// is read with the word after it.
    pub fn token(
        &self,
        text: &str,
        compat: bool,
        table: Option<&crate::data::Data>,
    ) -> (String, String) {
        let (mut ipa, at) = self.token_joined(text, compat, table);
        let key = ipa.get(at + 1..).unwrap_or("").to_string();
        ipa.truncate(at);
        (key, ipa)
    }

    /// [`Latin::token`] in one string: the phonemes, a separator at the
    /// byte offset that is returned, and the key.
    fn token_joined(
        &self,
        text: &str,
        compat: bool,
        table: Option<&crate::data::Data>,
    ) -> (String, usize) {
        // Running text repeats its words: remember recent answers.
        let space = (
            (self as *const Latin as usize) ^ usize::from(compat),
            table.map_or(0, crate::data::Data::id) * 2 + 1,
        );
        let both = crate::data::memo2(space, text, || {
            let (key, mut both) = self.token_uncached(text, compat, table);
            both.reserve(key.len() + 1);
            both.push('\u{1}');
            both.push_str(&key);
            Some(both)
        })
        .unwrap_or_default();
        let at = both.find('\u{1}').unwrap_or(both.len());
        (both, at)
    }

    fn token_uncached(
        &self,
        text: &str,
        compat: bool,
        table: Option<&crate::data::Data>,
    ) -> (String, String) {
        let owned;
        let text: &str = if text.contains('’') {
            owned = text.replace('’', "'");
            &owned
        } else {
            text
        };
        let lower = text.to_lowercase();
        match text.split_once('\'') {
            Some((head, tail))
                if self.elision
                    && !tail.is_empty()
                    && self
                        .words
                        .contains_key(&format!("{}'", head.to_lowercase()))
                    && self.listed(text, &lower, table).is_none()
                    && !self.words.contains_key(&lower) =>
            {
                let h = self.word(&format!("{head}'"), compat);
                let tail_lower = tail.to_lowercase();
                let read = self.read(tail, &tail_lower, compat, table);
                (
                    tail_lower,
                    format!("{}{}", crate::ipa::strip_stress(&h), read),
                )
            }
            _ => {
                let read = self.read(text, &lower, compat, table);
                (lower, read)
            }
        }
    }

    /// The items with each hyphenated word that the language file lists as
    /// a whole ("dix-huit", "vingt-six") made one item.
    fn join_hyphens(&self, items: &[crate::norm::Item]) -> Vec<crate::norm::Item> {
        let mut out: Vec<crate::norm::Item> = Vec::with_capacity(items.len());
        let mut i = 0;
        while i < items.len() {
            let mut end = i;
            let mut key = items[i].text.to_lowercase();
            let mut best = None;
            for (j, x) in items.iter().enumerate().skip(i + 1) {
                if !x.hyphen || x.generated {
                    break;
                }
                key.push('-');
                key.push_str(&x.text.to_lowercase());
                if self.words.contains_key(&key) {
                    best = Some(j);
                }
            }
            if let Some(j) = best {
                end = j;
            }
            let mut it = items[i].clone();
            if end > i {
                let parts: Vec<&str> = items[i..=end].iter().map(|x| x.text.as_str()).collect();
                it.text = parts.join("-");
            }
            out.push(it);
            i = end + 1;
        }
        out
    }

    /// Phonemes for a run of words of this language inside one clause.
    pub fn span(
        &self,
        items: &[crate::norm::Item],
        clause_final: bool,
        compat: bool,
        lang: crate::Lang,
        table: Option<&crate::data::Data>,
    ) -> Vec<crate::WordOut> {
        let merged;
        let items = if items.iter().any(|x| x.hyphen) {
            merged = self.join_hyphens(items);
            &merged[..]
        } else {
            items
        };
        let n = items.len();
        let mut out: Vec<crate::WordOut> = Vec::with_capacity(n);
        let mut fuse_at: Option<usize> = None;
        for (i, it) in items.iter().enumerate() {
            if let Some(v) = self.roman_number(&it.text, compat) {
                let mut ws: Vec<(String, bool)> = Vec::new();
                let said = |list: &[String]| -> Vec<(String, bool)> {
                    list.iter()
                        .filter(|_| compat)
                        .map(|w| (w.clone(), false))
                        .collect()
                };
                ws.extend(said(&self.numbers.roman_before));
                match self.numbers.ordinal(v, compat) {
                    Some(o) => ws.push((o.to_string(), false)),
                    None => self.numbers.cardinal(v, &mut ws),
                }
                ws.extend(said(&self.numbers.roman_after));
                let last = ws.len() - 1;
                for (k, (w, glue)) in ws.into_iter().enumerate() {
                    let mut ipa = self.word(&w, compat);
                    // Only the last word keeps its stress (French).
                    if self.numbers.stress_last && k < last {
                        ipa.retain(|c| c != 'ˈ' && c != 'ˌ');
                    }
                    out.push(crate::WordOut::new(&w, ipa, lang, glue));
                }
                continue;
            }
            // What the key says is looked up before the phonemes change.
            let (mut ipa, at) = self.token_joined(&it.text, compat, table);
            let key = ipa.get(at + 1..).unwrap_or("");
            let context = if compat && i > 0 && !self.context.is_empty() {
                let prev = items[i - 1].text.to_lowercase();
                self.context
                    .iter()
                    .find(|(w, ps, _)| {
                        w == key
                            && ps.iter().any(|p| match p.strip_prefix('*') {
                                Some(end) => prev.ends_with(end),
                                None => *p == prev,
                            })
                    })
                    .map(|(_, _, r)| r.clone())
            } else {
                None
            };
            let unstressed = if context.is_some() {
                None
            } else {
                self.unstressed.get(key).copied()
            };
            let fuses = !self.fuse.is_empty() && self.fuse.iter().any(|f| f == key);
            let promoted = !self.promote.is_empty()
                && i + 2 == n
                && clause_final
                && self.promote.iter().any(|p| p == key)
                && items.get(i + 1).is_some_and(|x| {
                    self.unstressed.get(x.text.to_lowercase().as_str()) == Some(&(b'p', b's'))
                });
            let clitic = !self.clitic_verbs.is_empty()
                && i > 0
                && !(i + 1 == n && clause_final)
                && self.clitic_verbs.iter().any(|v| v == key)
                && {
                    let prev = items[i - 1].text.to_lowercase().replace('’', "'");
                    self.clitic_subjects.contains(&prev)
                };
            ipa.truncate(at);
            if let Some(r) = context {
                ipa = r;
            }
            if clitic {
                ipa = crate::ipa::strip_stress(&ipa);
            }
            // Number words before the last of a number lose their stress.
            if self.numbers.stress_last && items.get(i + 1).is_some_and(|x| x.number_tail) {
                ipa = crate::ipa::strip_stress(&ipa);
            }
            if it.number_tail && self.numbers.tail_unstressed {
                ipa = crate::ipa::strip_stress(&ipa);
            }
            if it.number_tail && self.numbers.tail_plain {
                ipa.retain(|c| c != 'ˌ');
            }
            if it.number_tail && self.numbers.demote_tail {
                ipa = ipa.replace('ˈ', "ˌ");
            }
            if promoted {
                ipa = ipa.replace('ˌ', "ˈ");
            } else if let Some((mid, fin)) = unstressed {
                let level = if i + 1 == n && clause_final { fin } else { mid };
                ipa = match level {
                    b'0' => crate::ipa::strip_stress(&ipa),
                    b's' => ipa.replace('ˈ', "ˌ"),
                    _ => ipa,
                };
            }
            if self.vowel_mark && !it.text.contains('#') {
                // A silent h before the vowel counts with `vowel-mark = h`.
                let first = it
                    .text
                    .chars()
                    .find(|c| !(self.vowel_mark_h && matches!(c, 'h' | 'H')))
                    .map(|c| c.to_lowercase().next().unwrap_or(c));
                if first.is_some_and(|c| self.spell_vowels.contains(c)) {
                    ipa.insert(0, VOWEL);
                    if first.is_some_and(|c| self.vowel_mark_except.contains(c)) {
                        ipa.insert(0, EXCEPT);
                    }
                }
            }
            // Words before which nothing links (French h aspiré, huit, onze).
            if !self.aspirated.is_empty() || (compat && !self.h_muet.is_empty()) {
                let low = it.text.to_lowercase();
                if matches_pattern(&self.aspirated, &low)
                    || (compat
                        && !self.h_muet.is_empty()
                        && low.starts_with('h')
                        && !matches_pattern(&self.h_muet, &low))
                {
                    ipa.insert(0, BLOCK);
                }
            }
            if let Some(block) = &self.sandhi_block {
                let first = it
                    .text
                    .chars()
                    .next()
                    .map(|c| c.to_lowercase().next().unwrap_or(c));
                if (it.text.contains('#') && !it.number_tail)
                    || first.is_some_and(|c| block.contains(c))
                {
                    ipa.insert(0, BLOCK);
                }
            }
            // Number words may carry a `#` that keeps them apart from the
            // same word in running text.
            let shown = it.text.trim_end_matches(['#', 'x']).trim_start_matches('¤');
            let shown = if it.text.contains('#') {
                shown
            } else {
                &it.text
            };
            if fuse_at == Some(out.len()) && !it.text.contains('#') {
                if let Some(prev) = out.last_mut() {
                    let whole = format!("{}{}", prev.ipa, ipa);
                    prev.ipa = self.stress(&crate::ipa::strip_stress(&whole), "");
                    prev.text = format!("{} {}", prev.text, shown);
                    fuse_at = None;
                    continue;
                }
            }
            fuse_at = fuses.then_some(out.len() + 1);
            out.push(crate::WordOut::new(shown, ipa, lang, it.glue));
        }
        if !self.clause.is_empty() && !out.is_empty() {
            let mut joined = String::new();
            for (k, w) in out.iter().enumerate() {
                if k > 0 {
                    joined.push(JOIN);
                }
                joined.push_str(&w.ipa);
            }
            let done = self.clause.apply(&joined, &self.classes);
            let parts: Vec<&str> = done.split(JOIN).collect();
            if parts.len() == out.len() {
                for (w, p) in out.iter_mut().zip(parts) {
                    if w.ipa != p {
                        w.ipa = p.to_string();
                    }
                }
            }
        }
        for w in &mut out {
            if self.sandhi_block.is_some()
                || self.vowel_mark
                || !self.aspirated.is_empty()
                || !self.h_muet.is_empty()
            {
                w.ipa.retain(|c| c != BLOCK && c != VOWEL && c != EXCEPT);
            }
            let mode = if compat {
                &self.post_compat
            } else {
                &self.post_improved
            };
            if !mode.is_empty() {
                w.ipa = mode.apply(&w.ipa, &self.classes);
            }
        }
        out.retain(|w| !w.ipa.is_empty());
        out
    }
}

/// Words of a list section, `(text, prefix)`: a final `*` makes the entry
/// match every word that starts with it.
fn word_patterns(lines: &[String]) -> Vec<(String, bool)> {
    lines
        .iter()
        .flat_map(|l| l.split_whitespace())
        .map(|w| match w.strip_suffix('*') {
            Some(p) => (p.to_string(), true),
            None => (w.to_string(), false),
        })
        .collect()
}

/// True when the lower-case word `low` matches an entry of `list`.
fn matches_pattern(list: &[(String, bool)], low: &str) -> bool {
    list.iter().any(|(w, prefix)| {
        if *prefix {
            low.starts_with(w.as_str())
        } else {
            low == w
        }
    })
}
