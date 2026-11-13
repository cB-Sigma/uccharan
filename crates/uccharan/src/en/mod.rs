//! English (General American and Received Pronunciation).
//!
//! Word pronunciations come from the compiled lexicon (CMUdict converted by
//! `tools/lexgen`), then from morphology (suffix and compound analysis over
//! the same lexicon), then from the letter-to-sound model. A clause-level
//! pass applies function-word weak forms, fused word pairs, linking r and the
//! other connected-speech effects that espeak-style voices were trained on.

pub mod fw;
pub mod improve;

use crate::data::Data;
use crate::ipa::{self, is_vowel, nuclei, strip_stress, with_stress, PRIMARY, SECONDARY};
use crate::norm::Item;
use crate::{Lang, WordOut};

/// What the previous word leads us to expect (for homographs).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Hint {
    /// No expectation.
    #[default]
    None,
    /// A verb is likely (after "to", modals, subject pronouns).
    Verb,
    /// A noun or adjective is likely (after articles, possessives, prepositions).
    Noun,
    /// A past participle is likely (after "have", "was", …).
    Past,
}

/// Homographs: `(word, default, verb, noun, past)` as 1-based variant numbers
/// in the lexicon entry; 0 means "same as default".
static HOMOGRAPHS: &[(&str, u8, u8, u8, u8)] = &[
    ("abuse", 1, 2, 1, 0),
    ("addict", 2, 1, 2, 0),
    ("address", 2, 2, 2, 0),
    ("advocate", 1, 2, 1, 0),
    ("alternate", 1, 2, 1, 0),
    ("associate", 1, 2, 1, 0),
    ("bass", 2, 0, 0, 0),
    ("close", 2, 2, 1, 0),
    ("combine", 2, 2, 1, 0),
    ("compress", 2, 2, 1, 0),
    ("conduct", 1, 2, 1, 0),
    ("conflict", 1, 2, 1, 0),
    ("content", 1, 0, 1, 0),
    ("contents", 1, 0, 1, 0),
    ("contest", 1, 2, 1, 0),
    ("contract", 1, 2, 1, 0),
    ("contracts", 1, 2, 1, 0),
    ("contrast", 1, 2, 1, 0),
    ("convert", 2, 2, 1, 0),
    ("convict", 1, 2, 1, 0),
    ("decrease", 1, 1, 2, 0),
    ("deliberate", 3, 2, 3, 0),
    ("desert", 1, 2, 1, 0),
    ("digest", 1, 1, 2, 0),
    ("discount", 2, 0, 0, 0),
    ("duplicate", 1, 2, 1, 0),
    ("escort", 2, 1, 2, 0),
    ("estimate", 1, 2, 1, 0),
    ("excuse", 2, 2, 1, 0),
    ("exploit", 2, 2, 1, 0),
    ("extract", 1, 2, 1, 0),
    ("graduate", 3, 4, 3, 0),
    ("impact", 2, 0, 0, 0),
    ("import", 1, 1, 2, 0),
    ("increase", 1, 1, 2, 0),
    ("insult", 1, 1, 2, 0),
    ("invalid", 3, 0, 0, 0),
    ("lead", 2, 2, 2, 0),
    ("live", 2, 2, 1, 0),
    ("lives", 1, 1, 2, 0),
    ("moderate", 1, 2, 1, 0),
    ("object", 1, 2, 1, 0),
    ("objects", 1, 2, 1, 0),
    ("perfect", 2, 1, 2, 0),
    ("permit", 1, 1, 2, 0),
    ("permits", 1, 1, 2, 0),
    ("present", 1, 2, 1, 0),
    ("presents", 1, 2, 1, 0),
    ("produce", 1, 1, 2, 0),
    ("progress", 1, 2, 1, 0),
    ("project", 1, 2, 1, 0),
    ("projects", 1, 2, 1, 0),
    ("protest", 1, 2, 1, 0),
    ("read", 2, 2, 2, 1),
    ("rebel", 1, 2, 1, 0),
    ("record", 2, 3, 2, 0),
    ("records", 2, 3, 2, 0),
    ("refuse", 3, 3, 2, 0),
    ("separate", 3, 1, 3, 0),
    ("sow", 2, 0, 0, 0),
    ("subject", 2, 1, 2, 0),
    ("survey", 2, 1, 2, 0),
    ("suspect", 1, 1, 2, 0),
    ("tear", 2, 1, 2, 0),
    ("tears", 2, 1, 2, 0),
    ("transfer", 2, 1, 2, 0),
    ("transport", 2, 1, 2, 0),
    ("use", 2, 2, 1, 0),
    ("uses", 3, 3, 1, 0),
    ("wind", 2, 1, 2, 0),
    ("winds", 1, 2, 1, 0),
    ("wound", 2, 2, 2, 1),
];

fn hint_after(word: &str) -> Hint {
    match word {
        "to" | "will" | "would" | "can" | "could" | "shall" | "should" | "may" | "might"
        | "must" | "i" | "you" | "we" | "they" | "he" | "she" | "don't" | "didn't" | "doesn't"
        | "won't" | "can't" | "let's" | "cannot" | "i'll" | "you'll" | "we'll" | "they'll"
        | "he'll" | "she'll" | "not" => Hint::Verb,
        "a" | "an" | "the" | "my" | "your" | "his" | "her" | "its" | "our" | "their" | "this"
        | "that" | "these" | "those" | "of" | "in" | "on" | "at" | "for" | "with" | "by"
        | "from" | "no" | "some" | "any" => Hint::Noun,
        "have" | "has" | "had" | "was" | "were" | "is" | "are" | "been" | "be" | "having"
        | "i've" | "you've" | "we've" | "they've" | "he's" | "she's" | "it's" => Hint::Past,
        _ => Hint::None,
    }
}

const LETTERS_US: [&str; 26] = [
    "ˈeɪ",
    "bˈiː",
    "sˈiː",
    "dˈiː",
    "ˈiː",
    "ˈɛf",
    "dʒˈiː",
    "ˈeɪtʃ",
    "ˈaɪ",
    "dʒˈeɪ",
    "kˈeɪ",
    "ˈɛl",
    "ˈɛm",
    "ˈɛn",
    "ˈoʊ",
    "pˈiː",
    "kjˈuː",
    "ˈɑːɹ",
    "ˈɛs",
    "tˈiː",
    "jˈuː",
    "vˈiː",
    "dˈʌbəljˌuː",
    "ˈɛks",
    "wˈaɪ",
    "zˈiː",
];
const LETTERS_GB: [&str; 26] = [
    "ˈeɪ",
    "bˈiː",
    "sˈiː",
    "dˈiː",
    "ˈiː",
    "ˈɛf",
    "dʒˈiː",
    "ˈeɪtʃ",
    "ˈaɪ",
    "dʒˈeɪ",
    "kˈeɪ",
    "ˈɛl",
    "ˈɛm",
    "ˈɛn",
    "ˈəʊ",
    "pˈiː",
    "kjˈuː",
    "ˈɑː",
    "ˈɛs",
    "tˈiː",
    "jˈuː",
    "vˈiː",
    "dˈʌbəljˌuː",
    "ˈɛks",
    "wˈaɪ",
    "zˈɛd",
];

/// The English front end for one accent.
pub struct English<'a> {
    data: &'a Data,
    lang: Lang,
}

/// Run the letter-to-sound model on a word and repair its stress pattern.
pub fn letter_to_sound(data: &Data, word: &str, lang: Lang) -> String {
    let letters: Vec<u8> = word
        .bytes()
        .filter(|b| b.is_ascii_alphabetic())
        .map(|b| b.to_ascii_lowercase())
        .collect();
    let mut raw = String::new();
    if letters.is_empty() || !data.lts(&letters, &mut raw) {
        return String::new();
    }
    polish_lts(&raw, lang)
}

/// Clean-up applied to a raw letter-to-sound prediction: doubled consonant
/// letters give one consonant, and the stress marks are repaired.
pub(crate) fn polish_lts(raw: &str, lang: Lang) -> String {
    // Doubled letters spell one consonant.
    let mut dedup = String::with_capacity(raw.len());
    let mut prev = '\0';
    for c in raw.chars() {
        if c == prev && !is_vowel(c) && c != 'ː' {
            continue;
        }
        dedup.push(c);
        prev = c;
    }
    repair_stress(&dedup, lang)
}

/// Exactly one primary stress; no secondary stress next to it.
fn repair_stress(raw: &str, _lang: Lang) -> String {
    let ns = nuclei(raw);
    if ns.is_empty() {
        return strip_stress(raw);
    }
    let weak = |v: &str| matches!(v, "ə" | "ɚ" | "ᵻ" | "ɐ" | "i" | "n̩");
    let mut level: Vec<u8> = ns.iter().map(|n| n.1).collect();
    let primary = match level.iter().position(|&l| l == 1) {
        Some(p) => p,
        None => level
            .iter()
            .position(|&l| l == 2)
            .or_else(|| ns.iter().position(|n| !weak(n.2)))
            .unwrap_or(0),
    };
    for (k, l) in level.iter_mut().enumerate() {
        if k == primary {
            *l = 1;
        } else if *l == 1 {
            *l = 2;
        }
        if *l == 2 && (k + 1 == primary || k == primary + 1 || weak(ns[k].2)) {
            *l = 0;
        }
    }
    let bare = strip_stress(raw);
    // Re-insert marks before each nucleus (indices shift, so rebuild from nuclei of the bare string).
    let bn = nuclei(&bare);
    let mut out = String::with_capacity(raw.len() + 4);
    let mut at = 0;
    for (k, n) in bn.iter().enumerate() {
        out.push_str(&bare[at..n.0]);
        match level.get(k) {
            Some(1) => out.push(PRIMARY),
            Some(2) => out.push(SECONDARY),
            _ => {}
        }
        at = n.0;
    }
    out.push_str(&bare[at..]);
    out
}

fn last_sound(ipa: &str) -> &str {
    let s = ipa.trim_end_matches('ː');
    match s.char_indices().next_back() {
        Some((i, _)) => {
            if s.ends_with("tʃ") || s.ends_with("dʒ") {
                &s[s.len() - "tʃ".len()..]
            } else {
                &s[i..]
            }
        }
        None => "",
    }
}

impl<'a> English<'a> {
    /// Create the front end over a loaded data image.
    pub fn new(data: &'a Data, lang: Lang) -> Self {
        English { data, lang }
    }

    fn us(&self) -> bool {
        self.lang == Lang::EnUs
    }

    /// True when the word (lower case) has a lexicon entry.
    pub fn in_lexicon(&self, key: &str) -> bool {
        self.entry(key).is_some()
    }

    /// Lexicon entry with all variants (`a|b|c`).
    /// The name-origin model of the language data (empty when absent).
    pub fn name_model(&self) -> &'a [u8] {
        self.data.name_model()
    }

    /// True when `w` (lower case) has an entry in the lexicon or the names list.
    pub fn knows(&self, w: &str) -> bool {
        self.entry(w).is_some() || self.entry(&format!("^{w}")).is_some()
    }

    fn entry(&self, key: &str) -> Option<String> {
        // Running text repeats its words: remember recent answers.
        crate::data::memo(self.data.id() * 2 + 1, key, || self.entry_uncached(key))
    }

    fn entry_uncached(&self, key: &str) -> Option<String> {
        let mut s = String::new();
        if self.data.lookup_uncached(key, &mut s) {
            return Some(s);
        }
        // Words left out because the letter-to-sound model reproduces them
        // (the "^" names list and words with an apostrophe are never left out).
        (!key.starts_with('^') && !key.contains('\'') && self.data.known(key))
            .then(|| letter_to_sound(self.data, key, self.lang))
    }

    /// Lexicon pronunciation, choosing among variants by the hint.
    fn lexicon(&self, key: &str, hint: Hint) -> Option<String> {
        let e = self.entry(key)?;
        if !e.contains('|') {
            return Some(e);
        }
        let mut pick = 1usize;
        if let Ok(i) = HOMOGRAPHS.binary_search_by(|h| h.0.cmp(key)) {
            let h = HOMOGRAPHS[i];
            let by_hint = match hint {
                Hint::Verb => h.2,
                Hint::Noun => h.3,
                Hint::Past => h.4,
                Hint::None => 0,
            };
            pick = if by_hint != 0 { by_hint } else { h.1 } as usize;
        }
        // Variants are de-duplicated after conversion, so the index may be out of range.
        let v: Vec<&str> = e.split('|').collect();
        Some(v.get(pick - 1).or(v.first()).unwrap_or(&"").to_string())
    }

    /// Plural / possessive / third-person ending.
    fn add_s(&self, stem: &str) -> String {
        let mut out = self.unfinal(stem);
        let last = last_sound(stem);
        let ending = match last {
            "s" | "z" | "ʃ" | "ʒ" | "tʃ" | "dʒ" => {
                if self.us() {
                    "ᵻz"
                } else {
                    "ɪz"
                }
            }
            "p" | "t" | "k" | "f" | "θ" => "s",
            _ => "z",
        };
        out.push_str(ending);
        out
    }

    fn add_ed(&self, stem: &str) -> String {
        let last = last_sound(stem);
        match last {
            "t" | "d" => {
                let mut out = self.flap_final(stem);
                out.push_str(if self.us() { "ᵻd" } else { "ɪd" });
                out
            }
            "p" | "k" | "f" | "θ" | "s" | "ʃ" | "tʃ" => format!("{}t", self.unfinal(stem)),
            _ => format!("{}d", self.unfinal(stem)),
        }
    }

    /// A stem-final /t/ becomes a flap before a vowel-initial suffix (en-US).
    fn flap_final(&self, stem: &str) -> String {
        if self.us() && stem.ends_with('t') {
            let before = &stem[..stem.len() - 1];
            let prev = before.trim_end_matches('ː').chars().next_back();
            if prev.is_some_and(|c| is_vowel(c) || c == 'ɹ') {
                return format!("{before}ɾ");
            }
        }
        stem.to_string()
    }

    /// Undo word-final-only realisations before a suffix is attached
    /// (RP final `ɐ` is `ə` inside a word).
    fn unfinal(&self, stem: &str) -> String {
        if !self.us() && stem.ends_with('ɐ') {
            return format!("{}ə", &stem[..stem.len() - 'ɐ'.len_utf8()]);
        }
        stem.to_string()
    }

    /// Stem + vowel-initial suffix, with linking r and flap.
    fn add_vowel_suffix(&self, stem_text: &str, stem: &str, suffix: &str) -> String {
        let mut s = self.flap_final(&self.unfinal(stem));
        // happy + ly/er: the final vowel shortens.
        if s.ends_with('i') && !s.ends_with("ˈi") {
            s.pop();
            s.push('ɪ');
        }
        let r_spelled = stem_text.ends_with('r') || stem_text.ends_with("re");
        let ends_vowel = s
            .trim_end_matches('ː')
            .chars()
            .next_back()
            .is_some_and(is_vowel);
        if r_spelled && ends_vowel && !s.ends_with('ɹ') {
            s.push('ɹ');
        }
        // struggle + ing: the syllabic l loses its schwa.
        s.push_str(suffix);
        s
    }

    /// Try to analyse `w` as stem + suffix with the stem in the lexicon.
    fn morphology(&self, w: &str, depth: u8) -> Option<String> {
        let stem_of = |s: &str| -> Option<String> {
            if s.len() < 2 {
                return None;
            }
            self.lexicon(s, Hint::None).or_else(|| {
                if depth == 0 {
                    self.morphology(s, 1)
                } else {
                    None
                }
            })
        };
        if let Some(s) = w.strip_suffix("'s") {
            return stem_of(s).map(|p| self.add_s(&p));
        }
        if let Some(s) = w.strip_suffix("s'") {
            return stem_of(s)
                .map(|p| self.add_s(&p))
                .or_else(|| stem_of(&w[..w.len() - 1]));
        }
        if let Some(s) = w.strip_suffix('\'') {
            return stem_of(s);
        }
        let us = self.us();
        let happy = if us { "i" } else { "ɪ" };
        // -ies / -ied / -ier / -iest / -ily / -iness from a stem in -y
        for (suf, add) in [
            ("ies", "z"),
            ("ied", "d"),
            ("ier", if us { "ɚ" } else { "ɐ" }),
            ("iest", "ɪst"),
            ("ily", "l@"),
            ("iness", "nəs"),
        ] {
            if let Some(s) = w.strip_suffix(suf) {
                if let Some(p) = stem_of(&format!("{s}y")) {
                    let mut base = self.unfinal(&p);
                    if add != "z" && add != "d" && base.ends_with('i') {
                        base.pop();
                        base.push('ɪ');
                    }
                    return Some(format!("{base}{}", add.replace('@', happy)));
                }
            }
        }
        if let Some(s) = w.strip_suffix("es") {
            if s.ends_with(['s', 'x', 'z']) || s.ends_with("ch") || s.ends_with("sh") {
                if let Some(p) = stem_of(s) {
                    return Some(self.add_s(&p));
                }
            }
        }
        if let Some(s) = w.strip_suffix('s') {
            if !s.ends_with('s') {
                if let Some(p) = stem_of(s) {
                    return Some(self.add_s(&p));
                }
            }
        }
        // Stems that lost an e or doubled a consonant.
        let stems = |s: &str| -> Vec<String> {
            let mut v = vec![s.to_string(), format!("{s}e")];
            let b = s.as_bytes();
            if b.len() >= 3 && b[b.len() - 1] == b[b.len() - 2] {
                v.push(s[..s.len() - 1].to_string());
            }
            v
        };
        if let Some(s) = w.strip_suffix("ed") {
            for st in stems(s) {
                if let Some(p) = stem_of(&st) {
                    return Some(self.add_ed(&p));
                }
            }
        }
        for (suf, add) in [
            ("ing", "ɪŋ"),
            ("ings", "ɪŋz"),
            ("er", if us { "ɚ" } else { "ɐ" }),
            ("ers", if us { "ɚz" } else { "əz" }),
            ("est", "ɪst"),
            ("able", "əbəl"),
            ("ish", "ɪʃ"),
        ] {
            if let Some(s) = w.strip_suffix(suf) {
                for st in stems(s) {
                    if let Some(p) = stem_of(&st) {
                        return Some(self.add_vowel_suffix(&st, &p, add));
                    }
                }
            }
        }
        for (suf, add) in [
            ("ly", "l@"),
            ("ness", "nəs"),
            ("less", "ləs"),
            ("ment", "mənt"),
            ("ments", "mənts"),
            ("ful", "fəl"),
            ("fully", "fəl@"),
            ("lessly", "ləsl@"),
            ("like", "lˌaɪk"),
            ("ship", "ʃˌɪp"),
            ("hood", "hˌʊd"),
        ] {
            if let Some(s) = w.strip_suffix(suf) {
                if let Some(p) = stem_of(s) {
                    return Some(format!("{}{}", self.unfinal(&p), add.replace('@', happy)));
                }
            }
        }
        None
    }

    /// Prefix + word or word + word, both parts in the lexicon.
    fn compound(&self, w: &str) -> Option<String> {
        const PREFIXES: [(&str, &str, &str); 12] = [
            ("un", "ʌn", "ʌn"),
            ("non", "nˌɑːn", "nˌɒn"),
            ("re", "ɹˌiː", "ɹˌiː"),
            ("pre", "pɹˌiː", "pɹˌiː"),
            ("mis", "mɪs", "mɪs"),
            ("dis", "dɪs", "dɪs"),
            ("anti", "ˌæntɪ", "ˌæntɪ"),
            ("semi", "sˌɛmɪ", "sˌɛmɪ"),
            ("multi", "mˌʌltɪ", "mˌʌltɪ"),
            ("micro", "mˌaɪkɹoʊ", "mˌaɪkɹəʊ"),
            ("cyber", "sˌaɪbɚ", "sˌaɪbə"),
            ("co", "kˌoʊ", "kˌəʊ"),
        ];
        let rest_pron = |r: &str| {
            self.lexicon(r, Hint::None)
                .or_else(|| self.morphology(r, 0))
        };
        for (p, us, gb) in PREFIXES {
            if let Some(rest) = w.strip_prefix(p) {
                if rest.len() >= 4 {
                    if let Some(r) = rest_pron(rest) {
                        let pre = if self.us() { us } else { gb };
                        return Some(self.join_parts(pre, &r));
                    }
                }
            }
        }
        // Longest first part wins; both parts at least three letters.
        if w.len() >= 6 && w.is_ascii() {
            for cut in (3..=w.len() - 3).rev() {
                let (a, b) = w.split_at(cut);
                if let Some(pa) = self.lexicon(a, Hint::None) {
                    if let Some(pb) = rest_pron(b) {
                        return Some(self.join_parts(&pa, &pb));
                    }
                }
            }
        }
        None
    }

    /// Concatenate two parts: the second part's stress is demoted, and
    /// dropped when it would sit right next to the first part's stress.
    fn join_parts(&self, a: &str, b: &str) -> String {
        let a_has_primary = a.contains(PRIMARY);
        let a = self.unfinal(a);
        if !a_has_primary {
            // An unstressed or secondary-stressed prefix: the stem keeps its stress,
            // but two marks never sit on neighbouring syllables.
            let an = nuclei(&a);
            let bn = nuclei(b);
            let clash = an.last().is_some_and(|n| n.1 != 0) && bn.first().is_some_and(|n| n.1 != 0);
            let a2 = if clash { strip_stress(&a) } else { a };
            return format!("{a2}{b}");
        }
        let an = nuclei(&a);
        let bn = nuclei(b);
        let a_last_stressed = an.last().is_some_and(|n| n.1 == 1);
        let b_first_stressed = bn.first().is_some_and(|n| n.1 != 0);
        let b2 = if a_last_stressed && b_first_stressed {
            // Remove the first mark of b.
            let mut done = false;
            b.chars()
                .filter(|c| {
                    if !done && ipa::is_stress(*c) {
                        done = true;
                        false
                    } else {
                        true
                    }
                })
                .collect::<String>()
                .replace(PRIMARY, "ˌ")
        } else {
            b.replace(PRIMARY, "ˌ")
        };
        format!("{a}{b2}")
    }

    /// Letter names joined into one word; the last letter carries the primary stress.
    pub fn spell(&self, text: &str) -> String {
        let table = if self.us() { &LETTERS_US } else { &LETTERS_GB };
        let names: Vec<&str> = text
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .map(|c| table[(c.to_ascii_lowercase() as u8 - b'a') as usize])
            .collect();
        let n = names.len();
        let mut out = String::new();
        for (i, name) in names.iter().enumerate() {
            if i + 1 == n {
                out.push_str(name);
            } else {
                out.push_str(&name.replace(PRIMARY, "ˌ"));
            }
        }
        out
    }

    /// Citation pronunciation of one word (lower case), never empty for a
    /// word that contains a letter.
    pub fn word(&self, w: &str, hint: Hint) -> String {
        if let Some(p) = self.lexicon(w, hint) {
            return p;
        }
        let ascii: String = w.chars().map(fold_accent).collect();
        if ascii != w {
            if let Some(p) = self.lexicon(&ascii, hint) {
                return p;
            }
        }
        let w = ascii.as_str();
        if w.len() == 1 {
            return self.spell(w);
        }
        if let Some(p) = self.morphology(w, 0) {
            return p;
        }
        if let Some(p) = self.compound(w) {
            return p;
        }
        let letters: String = w.chars().filter(|c| c.is_ascii_alphabetic()).collect();
        if letters.is_empty() {
            return String::new();
        }
        // No vowel letter at all: spell it.
        if !letters.chars().any(|c| "aeiouy".contains(c)) {
            return self.spell(&letters);
        }
        let p = letter_to_sound(self.data, &letters, self.lang);
        if p.is_empty() {
            self.spell(&letters)
        } else {
            p
        }
    }

    /// Phonemes for a run of English items inside one clause.
    ///
    /// `clause_final` says whether the last item also ends the clause.
    /// `compat` enables the fused pairs and other reference-specific habits.
    pub fn span(&self, items: &[Item], clause_final: bool, compat: bool) -> Vec<WordOut> {
        let us = self.us();
        let lower: Vec<String> = items
            .iter()
            .map(|i| i.text.to_lowercase().replace('’', "'"))
            .collect();
        let mut out: Vec<WordOut> = Vec::with_capacity(items.len());
        // (index of the source item that ended the output word, r-final spelling)
        let mut src: Vec<usize> = Vec::with_capacity(items.len());
        let mut hint = Hint::None;
        let n = items.len();
        let mut i = 0;
        while i < n {
            let it = &items[i];
            let w = lower[i].as_str();
            let last = i + 1 == n && clause_final;
            if it.spell {
                out.push(WordOut::new(
                    &it.text,
                    self.spell(&it.text),
                    self.lang,
                    it.glue,
                ));
                src.push(i);
                hint = Hint::None;
                i += 1;
                continue;
            }
            if compat && i + 1 < n && !items[i + 1].spell && !it.generated {
                if let Some(p) = fw::pair(w, lower[i + 1].as_str()) {
                    let form = if us { p.2 } else { p.3 };
                    let ipa = if form.is_empty() {
                        format!(
                            "{}{}",
                            self.weak(w, false),
                            self.weak(lower[i + 1].as_str(), false)
                        )
                    } else {
                        form.to_string()
                    };
                    let text = format!("{} {}", it.text, items[i + 1].text);
                    out.push(WordOut::new(&text, ipa, self.lang, it.glue));
                    src.push(i + 1);
                    hint = hint_after(lower[i + 1].as_str());
                    i += 2;
                    continue;
                }
            }
            if !compat {
                let texts: Vec<&str> = items.iter().map(|x| x.text.as_str()).collect();
                let named = self.entry(&format!("^{w}")).or_else(|| {
                    w.strip_suffix("'s")
                        .and_then(|stem| self.entry(&format!("^{stem}")).map(|p| self.add_s(&p)))
                });
                let picked = if named.is_some() { named } else { None };
                let picked = if picked.is_some() {
                    picked
                } else {
                    match improve::pick(&lower, &texts, i, hint) {
                        Some(improve::Pick::Ipa(a, b)) => Some(if us { a } else { b }.to_string()),
                        Some(improve::Pick::Hint(h)) => Some(self.word(w, h)),
                        None => None,
                    }
                };
                if let Some(ipa) = picked {
                    out.push(WordOut::new(&it.text, ipa, self.lang, it.glue));
                    src.push(i);
                    hint = hint_after(w);
                    i += 1;
                    continue;
                }
            }
            let ipa = if let (Some(_), false) = (
                fw::word(w),
                it.generated && w != "and" && w != "a" && w != "of" && w != "the" && w != "to",
            ) {
                // A final unstressed "to" or "it" leaves the nucleus to the word before it.
                let before_final_to =
                    compat && clause_final && i + 2 == n && matches!(lower[i + 1].as_str(), "to");
                let initial_wh =
                    compat && i == 0 && n > 1 && matches!(w, "what" | "what's" | "when");
                if initial_wh {
                    with_stress(&self.weak(w, true), b's')
                } else {
                    self.weak(w, last || before_final_to)
                }
            } else {
                let capital_polish = w == "polish" && it.text.starts_with('P');
                if capital_polish {
                    self.entry("polish")
                        .and_then(|e| e.split('|').nth(1).map(str::to_string))
                        .unwrap_or_default()
                } else {
                    self.word(w, hint)
                }
            };
            // The decimal "point" of a number is unstressed in the reference.
            let ipa = if compat && it.generated && w == "point" {
                ipa.replace(['ˈ', 'ˌ'], "")
            } else {
                ipa
            };
            out.push(WordOut::new(&it.text, ipa, self.lang, it.glue));
            src.push(i);
            hint = hint_after(w);
            i += 1;
        }
        // Context effects between neighbouring words.
        let m = out.len();
        for k in 0..m {
            let next_vowel = out
                .get(k + 1)
                .is_some_and(|w| ipa::starts_with_vowel(&w.ipa));
            let is_last = k + 1 == m && clause_final;
            let text = lower[src[k]].as_str();
            let cur = &mut out[k];
            let single = !cur.text.contains(' ');
            match text {
                "the" if next_vowel && cur.ipa.ends_with("ðə") => {
                    cur.ipa.truncate(cur.ipa.len() - 'ə'.len_utf8());
                    cur.ipa.push('ɪ');
                }
                "to" if single && is_last => cur.ipa = "tuː".into(),
                "to" if single && next_vowel => cur.ipa = "tʊ".into(),
                "a" if single && is_last => cur.ipa = "ˈeɪ".into(),
                "an" if single && next_vowel => cur.ipa = "ɐn".into(),
                "it" | "at" if us && compat && single && next_vowel && cur.ipa.ends_with('t') => {
                    cur.ipa.pop();
                    cur.ipa.push('ɾ');
                }
                _ => {}
            }
            let pause_next = compat
                && src
                    .get(k + 1)
                    .is_some_and(|&j| matches!(lower[j].as_str(), "and" | "or" | "but"));
            if next_vowel && !pause_next {
                let r_spelled =
                    text.ends_with('r') || text.ends_with("re") || text.ends_with("r's");
                let ends = cur.ipa.trim_end_matches('ː').chars().next_back();
                let link = if us {
                    r_spelled && (cur.ipa.ends_with('ɚ') || cur.ipa.ends_with("ɜː"))
                } else {
                    (r_spelled && ends.is_some_and(is_vowel) && !text.ends_with("r's"))
                        || (compat && cur.ipa.ends_with('ɐ'))
                };
                if link {
                    cur.ipa.push('ɹ');
                }
            }
        }
        out
    }

    /// Weak or clause-final form of a function word.
    fn weak(&self, w: &str, clause_final: bool) -> String {
        let Some(f) = fw::word(w) else {
            return self.word(w, Hint::None);
        };
        let base = if self.us() { f.3 } else { f.4 };
        let base = if base.is_empty() {
            self.word(w, Hint::None)
        } else {
            base.to_string()
        };
        let level = if clause_final { f.2 } else { f.1 };
        with_stress(&base, level)
    }
}

fn fold_accent(c: char) -> char {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => 'a',
        'ç' => 'c',
        'è' | 'é' | 'ê' | 'ë' => 'e',
        'ì' | 'í' | 'î' | 'ï' => 'i',
        'ñ' => 'n',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' => 'o',
        'ù' | 'ú' | 'û' | 'ü' => 'u',
        'ý' | 'ÿ' => 'y',
        _ => c,
    }
}
