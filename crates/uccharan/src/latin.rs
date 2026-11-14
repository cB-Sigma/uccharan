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
    stress: Stress,
    secondary: bool,
    /// `elision = yes`: `l'`, `qu'` and the like are read with the next word.
    elision: bool,
    /// `sandhi-block = h`: words that begin with one of these letters, and
    /// numbers written in digits, take no part in the `[clause]` rules
    /// with the word before them.
    sandhi_block: Option<String>,
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
    unstressed: HashMap<String, (u8, u8)>,
    /// A suffix read with its own stress after the stem's (`-mente`).
    suffix: Option<(String, String)>,
    /// Letter names for spelled words (`[spell]`).
    spell: HashMap<char, String>,
    /// Number words.
    pub numbers: Numbers,
}

const JOIN: char = '‿';
/// Stands before a word that is cut off from the word before it.
const BLOCK: char = '⁞';

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
            stress,
            secondary: meta.get("secondary").is_some_and(|v| v == "alternate"),
            elision: meta.get("elision").is_some_and(|v| v == "yes"),
            sandhi_block: meta.get("sandhi-block").cloned(),
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
        let cs: Vec<char> = ph.chars().collect();
        let nuclei: Vec<usize> = (0..cs.len()).filter(|&i| self.is_vowel(cs[i])).collect();
        let n = nuclei.len();
        let is_mark = |c: char| matches!(c, 'ˈ' | 'ˌ' | '°');
        if n == 0 {
            return ph.chars().filter(|&c| !is_mark(c)).collect();
        }
        // Levels: -1 free, 0 never stressed, 2 secondary, 4 primary.
        // `‖` and `¦` are morpheme boundaries whose next free syllable
        // takes primary or secondary stress.
        let mut level: Vec<i8> = Vec::with_capacity(n);
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
                level.push(l);
                v += 1;
            }
        }
        let cs: Vec<char> = cs
            .into_iter()
            .map(|c| if matches!(c, '‖' | '¦') { '‧' } else { c })
            .collect();
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

    /// The value of a Roman numeral of at least two letters (I V X L).
    fn roman(w: &str) -> Option<u64> {
        if w.chars().count() < 2 || !w.chars().all(|c| matches!(c, 'I' | 'V' | 'X' | 'L')) {
            return None;
        }
        let val = |c| match c {
            'I' => 1,
            'V' => 5,
            'X' => 10,
            _ => 50,
        };
        let cs: Vec<u64> = w.chars().map(val).collect();
        let mut n = 0u64;
        for (i, &v) in cs.iter().enumerate() {
            if cs.get(i + 1).is_some_and(|&nx| nx > v) {
                n = n.saturating_sub(v);
                if !matches!(v, 1 | 10) {
                    return None;
                }
            } else {
                n += v;
            }
        }
        (n > 0 && n < 90).then_some(n)
    }

    /// The word table's reading: the word as written when it is all in
    /// capitals, else in lower case.
    fn listed(&self, w: &str, table: Option<&crate::data::Data>) -> Option<String> {
        let t = table?;
        let mut found = String::new();
        let caps = w.chars().count() > 1 && w.chars().all(|c| !c.is_lowercase());
        if caps && t.lookup(w, &mut found) {
            return Some(found);
        }
        let lower = w.to_lowercase();
        if self.words.contains_key(&lower) {
            return None;
        }
        t.lookup(&lower, &mut found).then_some(found)
    }

    /// The reading of a word by the table's letter-to-sound model, with
    /// exactly one primary stress: of several the language's stress rule
    /// keeps the last (final and penult rules) or the first; without any,
    /// the first secondary stress or the first vowel takes it.
    pub fn model_word(&self, word: &str, table: &crate::data::Data) -> Option<String> {
        let lower = word.to_lowercase();
        let space = table.id() * 2 + 1;
        crate::data::memo(space, &lower, || {
            let s = table.model_word(&lower)?;
            Some(if table.meta("stress") == Some("latent") {
                self.stress(&s, &lower)
            } else {
                self.one_primary(&s)
            })
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

    /// A word read letter by letter: an acronym in capitals, or a word
    /// without vowels.
    fn spelled(&self, w: &str) -> Option<String> {
        if self.spell.is_empty() || self.words.contains_key(&w.to_lowercase()) {
            return None;
        }
        let n = w.chars().count();
        let caps = (2..=5).contains(&n) && w.chars().all(|c| c.is_uppercase());
        let no_vowel = n >= 2
            && w.chars().all(char::is_alphabetic)
            && !w
                .to_lowercase()
                .chars()
                .any(|c| self.spell_vowels.contains(c));
        if !caps && !no_vowel {
            return None;
        }
        let names: Vec<&String> = w
            .to_lowercase()
            .chars()
            .map(|c| self.spell.get(&c))
            .collect::<Option<Vec<_>>>()?;
        let mut out = String::new();
        for (k, name) in names.iter().enumerate() {
            if k + 1 < names.len() {
                if self.spell_close_e {
                    out.push_str(&name.replace("ˈɛ", "ˌe").replace('ˈ', "ˌ"));
                } else {
                    out.push_str(&name.replace('ˈ', "ˌ"));
                }
            } else {
                out.push_str(name);
            }
        }
        Some(out)
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
        let read = |w: &str| match self.listed(w, table) {
            Some(s) => s,
            None => match self.spelled(w) {
                Some(s) => s,
                None => table
                    .filter(|_| !self.words.contains_key(&w.to_lowercase()))
                    .and_then(|t| self.model_word(w, t))
                    .unwrap_or_else(|| self.word(w, compat)),
            },
        };
        let text = text.replace('’', "'");
        match text.split_once('\'') {
            Some((head, tail))
                if self.elision
                    && !tail.is_empty()
                    && self
                        .words
                        .contains_key(&format!("{}'", head.to_lowercase()))
                    && self.listed(&text, table).is_none()
                    && !self.words.contains_key(&text.to_lowercase()) =>
            {
                let h = self.word(&format!("{head}'"), compat);
                (
                    tail.to_lowercase(),
                    format!("{}{}", crate::ipa::strip_stress(&h), read(tail)),
                )
            }
            _ => (text.to_lowercase(), read(&text)),
        }
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
        let n = items.len();
        let mut out: Vec<crate::WordOut> = Vec::with_capacity(n);
        for (i, it) in items.iter().enumerate() {
            if let Some(v) = Self::roman(&it.text).filter(|_| !self.numbers.is_empty()) {
                let mut ws = Vec::new();
                self.numbers.cardinal(v, &mut ws);
                for (w, glue) in ws {
                    out.push(crate::WordOut::new(&w, self.word(&w, compat), lang, glue));
                }
                continue;
            }
            let (key, mut ipa) = self.token(&it.text, compat, table);
            // Number words before the last of a number lose their stress.
            if self.numbers.stress_last && items.get(i + 1).is_some_and(|x| x.number_tail) {
                ipa = crate::ipa::strip_stress(&ipa);
            }
            if it.number_tail && self.numbers.demote_tail {
                ipa = ipa.replace('ˈ', "ˌ");
            }
            if let Some(&(mid, fin)) = self.unstressed.get(key.as_str()) {
                let level = if i + 1 == n && clause_final { fin } else { mid };
                ipa = match level {
                    b'0' => crate::ipa::strip_stress(&ipa),
                    b's' => ipa.replace('ˈ', "ˌ"),
                    _ => ipa,
                };
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
            let shown = it.text.trim_end_matches(['#', 'x']);
            let shown = if it.text.contains('#') {
                shown
            } else {
                &it.text
            };
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
                    w.ipa = p.to_string();
                }
            }
        }
        for w in &mut out {
            if self.sandhi_block.is_some() {
                w.ipa.retain(|c| c != BLOCK);
            }
            w.ipa = self.finish(&w.ipa, compat);
        }
        out.retain(|w| !w.ipa.is_empty());
        out
    }
}
