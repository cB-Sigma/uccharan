//! Kokoro English: a front end that targets misaki, the G2P Kokoro's English
//! voices were trained on.
//!
//! This module is a port to Rust of the English pipeline of misaki 0.9.4
//! (`misaki/en.py`, Copyright hexgrad, Apache-2.0; see `NOTICE`): its
//! dictionary lookup (gold, then silver), stress handling, suffix rules,
//! number reading, sub-word splitting and clause-context rules. Two parts are
//! replaced:
//!
//! * spaCy's part-of-speech tagger, by a small rule-based tagger
//!   ([`tag_tokens`]) that only has to decide what misaki's dictionary and
//!   special cases look at;
//! * misaki's espeak-ng fallback for words outside its dictionaries, by
//!   uccharan's own English rules mapped into misaki's phone set.
//!
//! misaki 0.9.4's dictionaries (Apache-2.0; upstream does not document their
//! original sources, see `DATA_LICENSES.md`) are compiled in with the
//! `embed-kokoro` feature; [`MisakiLexicon::from_json`] reads another copy.

// The port keeps misaki's ordered condition chains (one branch per case, even
// where two give the same result) so that it can be compared with the original line by line.
#![allow(clippy::if_same_then_else, clippy::needless_range_loop)]

use crate::data::Data;
use std::collections::HashMap;
use std::fmt;

/// One dictionary value: a pronunciation, or pronunciations by tag
/// (`DEFAULT`, `NOUN`, `VERB`, `ADJ`, `VBD`, …; `None` values allowed).
#[derive(Clone, Debug)]
pub enum Entry {
    /// A single pronunciation.
    One(Box<str>),
    /// Pronunciations by part-of-speech tag.
    Tagged(Vec<(Box<str>, Option<Box<str>>)>),
}

/// misaki's gold and silver dictionaries for one accent.
#[derive(Clone)]
pub struct MisakiLexicon {
    british: bool,
    gold: Store,
    silver: Store,
}

/// Where a dictionary lives: parsed from JSON, or in a compiled `.ucd` image
/// (keys prefixed `<` for gold and `>` for silver; tagged values written as
/// `TAG=pron|TAG=pron`, an empty pronunciation standing for `null`).
#[derive(Clone)]
enum Store {
    Map(HashMap<Box<str>, Entry>),
    Image(Data, char),
}

impl Store {
    fn get(&self, word: &str) -> Option<Entry> {
        match self {
            Store::Map(m) => m.get(word).cloned(),
            Store::Image(d, prefix) => {
                let probe = |w: &str| -> Option<Entry> {
                    let mut key = String::with_capacity(w.len() + 1);
                    key.push(*prefix);
                    key.push_str(w);
                    let mut out = String::new();
                    d.lookup(&key, &mut out).then(|| decode_entry(&out))
                };
                if let Some(e) = probe(word) {
                    return Some(e);
                }
                // The image stores the dictionaries as published; misaki's
                // `grow_dictionary` variants are derived here instead.
                if word.chars().count() < 2 {
                    return None;
                }
                let lower = word.to_lowercase();
                if word == lower {
                    let cap = capitalize(word);
                    (cap != word).then(|| probe(&cap)).flatten()
                } else if word == capitalize(&lower) {
                    probe(&lower)
                } else {
                    None
                }
            }
        }
    }

    fn contains_key(&self, word: &str) -> bool {
        match self {
            Store::Map(m) => m.contains_key(word),
            Store::Image(..) => self.get(word).is_some(),
        }
    }

    fn len(&self) -> usize {
        match self {
            Store::Map(m) => m.len(),
            Store::Image(d, prefix) => d
                .meta(if *prefix == '<' { "gold" } else { "silver" })
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
        }
    }
}

/// Entries of a compiled image (`(key, encoded value)`) and the gold and
/// silver entry counts after capitalisation variants are added.
pub type ImageEntries = (Vec<(String, String)>, usize, usize);

/// Encode an entry for a `.ucd` image (see [`Store`]).
pub fn encode_entry(e: &Entry) -> String {
    match e {
        Entry::One(p) => p.to_string(),
        Entry::Tagged(v) => v
            .iter()
            .map(|(t, p)| format!("{t}={}", p.as_deref().unwrap_or("")))
            .collect::<Vec<_>>()
            .join("|"),
    }
}

fn decode_entry(s: &str) -> Entry {
    if !s.contains('=') {
        return Entry::One(s.into());
    }
    Entry::Tagged(
        s.split('|')
            .map(|part| {
                let (t, p) = part.split_once('=').unwrap_or((part, ""));
                (t.into(), (!p.is_empty()).then(|| p.into()))
            })
            .collect(),
    )
}

impl fmt::Debug for MisakiLexicon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MisakiLexicon {{ british: {}, gold: {}, silver: {} }}",
            self.british,
            self.gold.len(),
            self.silver.len()
        )
    }
}

// ---------------------------------------------------------------------------
// A minimal JSON reader for the dictionary files: an object whose values are
// strings or objects of strings / null.

struct Json<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> Json<'a> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }
    fn eat(&mut self, c: u8) -> Result<(), String> {
        self.ws();
        if self.s.get(self.i) == Some(&c) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("expected '{}' at byte {}", c as char, self.i))
        }
    }
    fn peek(&mut self) -> Option<u8> {
        self.ws();
        self.s.get(self.i).copied()
    }
    fn string(&mut self) -> Result<String, String> {
        self.eat(b'"')?;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let c = *self.s.get(self.i).ok_or("unterminated string")?;
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let e = *self.s.get(self.i).ok_or("bad escape")?;
                    self.i += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b't' => out.push(b'\t'),
                        b'r' => out.push(b'\r'),
                        b'b' | b'f' => {}
                        b'u' => {
                            let hex = std::str::from_utf8(
                                self.s.get(self.i..self.i + 4).ok_or("bad \\u")?,
                            )
                            .map_err(|_| "bad \\u")?;
                            let mut cp = u32::from_str_radix(hex, 16).map_err(|_| "bad \\u")?;
                            self.i += 4;
                            if (0xd800..0xdc00).contains(&cp)
                                && self.s.get(self.i..self.i + 2) == Some(b"\\u")
                            {
                                let hex2 = std::str::from_utf8(&self.s[self.i + 2..self.i + 6])
                                    .map_err(|_| "bad \\u")?;
                                let lo = u32::from_str_radix(hex2, 16).map_err(|_| "bad \\u")?;
                                cp = 0x10000 + ((cp - 0xd800) << 10) + (lo - 0xdc00);
                                self.i += 6;
                            }
                            let ch = char::from_u32(cp).unwrap_or('\u{fffd}');
                            let mut buf = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        other => out.push(other),
                    }
                }
                c => out.push(c),
            }
        }
        String::from_utf8(out).map_err(|_| "invalid UTF-8".to_string())
    }
    fn null(&mut self) -> Result<(), String> {
        self.ws();
        if self.s.get(self.i..self.i + 4) == Some(b"null") {
            self.i += 4;
            Ok(())
        } else {
            Err(format!("unexpected value at byte {}", self.i))
        }
    }
}

fn parse_dictionary(text: &str) -> Result<HashMap<Box<str>, Entry>, String> {
    let mut p = Json {
        s: text.as_bytes(),
        i: 0,
    };
    let mut map = HashMap::new();
    p.eat(b'{')?;
    if p.peek() == Some(b'}') {
        return Ok(map);
    }
    loop {
        let key = p.string()?;
        p.eat(b':')?;
        let value = match p.peek() {
            Some(b'"') => Entry::One(p.string()?.into()),
            Some(b'{') => {
                p.eat(b'{')?;
                let mut v = Vec::new();
                if p.peek() != Some(b'}') {
                    loop {
                        let tag = p.string()?;
                        p.eat(b':')?;
                        let pron = if p.peek() == Some(b'"') {
                            Some(p.string()?.into_boxed_str())
                        } else {
                            p.null()?;
                            None
                        };
                        v.push((tag.into_boxed_str(), pron));
                        if p.peek() == Some(b',') {
                            p.eat(b',')?;
                        } else {
                            break;
                        }
                    }
                }
                p.eat(b'}')?;
                Entry::Tagged(v)
            }
            _ => return Err(format!("unexpected value at byte {}", p.i)),
        };
        map.insert(key.into_boxed_str(), value);
        if p.peek() == Some(b',') {
            p.eat(b',')?;
        } else {
            break;
        }
    }
    p.eat(b'}')?;
    Ok(map)
}

fn capitalize(s: &str) -> String {
    let mut cs = s.chars();
    match cs.next() {
        Some(f) => f
            .to_uppercase()
            .chain(cs.flat_map(|c| c.to_lowercase()))
            .collect(),
        None => String::new(),
    }
}

/// misaki's `grow_dictionary`: add capitalised forms of lower-case keys and
/// lower-case forms of capitalised keys, never overriding a real key.
fn grow(d: HashMap<Box<str>, Entry>) -> HashMap<Box<str>, Entry> {
    let mut extra: Vec<(Box<str>, Entry)> = Vec::new();
    for (k, v) in &d {
        if k.chars().count() < 2 {
            continue;
        }
        let lower = k.to_lowercase();
        if **k == *lower {
            let c = capitalize(k);
            if c != **k {
                extra.push((c.into_boxed_str(), v.clone()));
            }
        } else if **k == capitalize(&lower) {
            extra.push((lower.into_boxed_str(), v.clone()));
        }
    }
    let mut d = d;
    for (k, v) in extra {
        d.entry(k).or_insert(v);
    }
    d
}

impl MisakiLexicon {
    /// Build from the text of misaki's `us_gold.json` / `us_silver.json` (or
    /// the `gb_` files with `british = true`).
    pub fn from_json(gold: &str, silver: &str, british: bool) -> Result<MisakiLexicon, String> {
        Ok(MisakiLexicon {
            british,
            gold: Store::Map(grow(parse_dictionary(gold)?)),
            silver: Store::Map(grow(parse_dictionary(silver)?)),
        })
    }

    /// Use a compiled image built by `tools/lexgen` (`kokoro-en-us.ucd`,
    /// `kokoro-en-gb.ucd`).
    pub fn from_image(data: Data) -> Result<MisakiLexicon, String> {
        let british = match data.meta("lang") {
            Some("kokoro-en-gb") => true,
            Some("kokoro-en-us") => false,
            other => return Err(format!("not a Kokoro English image: {other:?}")),
        };
        Ok(MisakiLexicon {
            british,
            gold: Store::Image(data.clone(), '<'),
            silver: Store::Image(data, '>'),
        })
    }

    /// The entries of a compiled image built from the JSON files as published
    /// (before misaki's capitalisation variants are added, which happens at
    /// lookup time): `(key, encoded value)` with key prefixes `<` gold and `>`
    /// silver. Silver entries shadowed by a gold entry are left out (gold is
    /// always consulted first). Also returns the entry counts after growth.
    pub fn image_entries(gold: &str, silver: &str) -> Result<ImageEntries, String> {
        let g = parse_dictionary(gold)?;
        let sv = parse_dictionary(silver)?;
        let mut out: Vec<(String, String)> = g
            .iter()
            .map(|(k, v)| (format!("<{k}"), encode_entry(v)))
            .collect();
        out.extend(
            sv.iter()
                .filter(|(k, _)| !g.contains_key(*k))
                .map(|(k, v)| (format!(">{k}"), encode_entry(v))),
        );
        let (ng, ns) = (grow(g).len(), grow(sv).len());
        Ok((out, ng, ns))
    }

    /// True for the British dictionaries.
    pub fn british(&self) -> bool {
        self.british
    }

    /// Number of gold and silver entries (after the capitalisation variants are added).
    pub fn len(&self) -> (usize, usize) {
        (self.gold.len(), self.silver.len())
    }

    /// True when `word` is in either dictionary as written.
    pub fn contains(&self, word: &str) -> bool {
        self.gold.contains_key(word) || self.silver.contains_key(word)
    }
}

// ---------------------------------------------------------------------------
// Phone-level helpers (misaki's constants).

const PRIMARY: char = 'ˈ';
const SECONDARY: char = 'ˌ';
const VOWELS: &str = "AIOQWYaiuæɑɒɔəɛɜɪʊʌᵻ";
const CONSONANTS: &str = "bdfhjklmnpstvwzðŋɡɹɾʃʒʤʧθ";
const US_TAUS: &str = "AIOWYiuæɑəɛɪɹʊʌ";
const PUNCTS: &str = ";:,.!?—…\"“”";
const NON_QUOTE_PUNCTS: &str = ";:,.!?—…";
const DIPHTHONGS: &str = "AIOQWYʤʧ";
const SUBTOKEN_JUNKS: &str = "',-._‘’/";

fn is_vowel(c: char) -> bool {
    VOWELS.contains(c)
}

fn has_stress(s: &str) -> bool {
    s.contains(PRIMARY) || s.contains(SECONDARY)
}

/// Move each stress mark to just before the next vowel.
fn restress(ps: &str) -> String {
    let cs: Vec<char> = ps.chars().collect();
    let mut keyed: Vec<(f64, char)> = cs.iter().enumerate().map(|(i, &c)| (i as f64, c)).collect();
    for i in 0..cs.len() {
        if cs[i] == PRIMARY || cs[i] == SECONDARY {
            if let Some(j) = (i..cs.len()).find(|&j| is_vowel(cs[j])) {
                keyed[i].0 = j as f64 - 0.5;
            }
        }
    }
    keyed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    keyed.into_iter().map(|x| x.1).collect()
}

fn apply_stress(ps: &str, stress: Option<f64>) -> String {
    let Some(st) = stress else {
        return ps.to_string();
    };
    if st < -1.0 {
        return ps.replace([PRIMARY, SECONDARY], "");
    }
    if st == -1.0 || ((st == 0.0 || st == -0.5) && ps.contains(PRIMARY)) {
        return ps.replace(SECONDARY, "").replace(PRIMARY, "ˌ");
    }
    if (st == 0.0 || st == 0.5 || st == 1.0) && !has_stress(ps) {
        if !ps.chars().any(is_vowel) {
            return ps.to_string();
        }
        return restress(&format!("{SECONDARY}{ps}"));
    }
    if st >= 1.0 && !ps.contains(PRIMARY) && ps.contains(SECONDARY) {
        return ps.replace(SECONDARY, "ˈ");
    }
    if st > 1.0 && !has_stress(ps) {
        if !ps.chars().any(is_vowel) {
            return ps.to_string();
        }
        return restress(&format!("{PRIMARY}{ps}"));
    }
    ps.to_string()
}

fn stress_weight(ps: &str) -> usize {
    ps.chars()
        .map(|c| if DIPHTHONGS.contains(c) { 2 } else { 1 })
        .sum()
}

fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

fn is_alpha(s: &str) -> bool {
    !s.is_empty() && s.chars().all(char::is_alphabetic)
}

fn lexicon_chars(s: &str) -> bool {
    s.chars()
        .all(|c| c == '\'' || c == '-' || c.is_ascii_alphabetic())
}

fn parent_tag(tag: &str) -> &str {
    if tag.starts_with("VB") {
        "VERB"
    } else if tag.starts_with("NN") {
        "NOUN"
    } else if tag.starts_with("ADV") || tag.starts_with("RB") {
        "ADV"
    } else if tag.starts_with("ADJ") || tag.starts_with("JJ") {
        "ADJ"
    } else {
        tag
    }
}

// ---------------------------------------------------------------------------
// Numbers in words (the subset of num2words' English that misaki uses).

const ONES: [&str; 20] = [
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
];
const TENS: [&str; 10] = [
    "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
];

fn cardinal_words(n: u64, out: &mut Vec<String>) {
    if n < 20 {
        out.push(ONES[n as usize].into());
    } else if n < 100 {
        out.push(TENS[(n / 10) as usize].into());
        if n % 10 != 0 {
            out.push(ONES[(n % 10) as usize].into());
        }
    } else if n < 1000 {
        out.push(ONES[(n / 100) as usize].into());
        out.push("hundred".into());
        if n % 100 != 0 {
            out.push("and".into());
            cardinal_words(n % 100, out);
        }
    } else {
        let scales = [
            (1_000_000_000_000u64, "trillion"),
            (1_000_000_000, "billion"),
            (1_000_000, "million"),
            (1000, "thousand"),
        ];
        for (v, name) in scales {
            if n >= v {
                cardinal_words(n / v, out);
                out.push(name.into());
                let rest = n % v;
                if rest != 0 {
                    if rest < 100 {
                        out.push("and".into());
                    }
                    cardinal_words(rest, out);
                }
                return;
            }
        }
    }
}

fn ordinal_words(n: u64, out: &mut Vec<String>) {
    cardinal_words(n, out);
    if let Some(last) = out.last_mut() {
        let w = match last.as_str() {
            "one" => "first".to_string(),
            "two" => "second".into(),
            "three" => "third".into(),
            "five" => "fifth".into(),
            "eight" => "eighth".into(),
            "nine" => "ninth".into(),
            "twelve" => "twelfth".into(),
            w if w.ends_with('y') => format!("{}ieth", &w[..w.len() - 1]),
            w => format!("{w}th"),
        };
        *last = w;
    }
}

fn year_words(n: u64, out: &mut Vec<String>) {
    let (high, low) = (n / 100, n % 100);
    if high == 0 || (high % 10 == 0 && low < 10) || high >= 100 {
        cardinal_words(n, out);
    } else {
        cardinal_words(high, out);
        if low == 0 {
            out.push("hundred".into());
        } else if low < 10 {
            out.push("oh".into());
            cardinal_words(low, out);
        } else {
            cardinal_words(low, out);
        }
    }
}

/// num2words for a decimal string such as "3.14" (as `num2words(float)`).
fn float_words(s: &str, out: &mut Vec<String>) {
    let (int, frac) = s.split_once('.').unwrap_or((s, ""));
    let frac = frac.trim_end_matches('0');
    cardinal_words(int.parse().unwrap_or(0), out);
    if !frac.is_empty() {
        out.push("point".into());
        for d in frac.bytes() {
            out.push(ONES[(d - b'0') as usize].into());
        }
    }
}

// ---------------------------------------------------------------------------
// Tokens.

#[derive(Clone, Debug, Default)]
struct Tok {
    text: String,
    tag: &'static str,
    ws: bool,
    phonemes: Option<String>,
    is_head: bool,
    currency: Option<char>,
    prespace: bool,
    alias: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, Default)]
struct Ctx {
    future_vowel: Option<bool>,
    future_to: bool,
}

const CONTRACTIONS: [&str; 7] = ["n't", "'s", "'m", "'re", "'ve", "'ll", "'d"];

fn is_open_quote_context(prev_ws: bool, at_start: bool) -> bool {
    at_start || prev_ws
}

/// Approximation of spaCy's English tokenizer: white space, then leading
/// and trailing punctuation, then clitics (n't, 's, 'm, 're, 've, 'll, 'd).
fn tokenize(text: &str) -> Vec<Tok> {
    let mut out: Vec<Tok> = Vec::new();
    let text = text.replace(['‘', '’'], "'");
    let chunks: Vec<&str> = text.split_whitespace().collect();
    for chunk in chunks {
        let start = out.len();
        let cs: Vec<char> = chunk.chars().collect();
        let (mut a, mut b) = (0usize, cs.len());
        // leading punctuation, one character per token
        while a < b
            && "\"“(['[{$£€¿¡#".contains(cs[a])
            && !(cs[a] == '\'' && b - a > 1 && cs[a + 1].is_alphanumeric() && false)
        {
            out.push(Tok {
                text: cs[a].to_string(),
                ..Default::default()
            });
            a += 1;
        }
        // trailing punctuation
        let mut tail: Vec<String> = Vec::new();
        loop {
            if b <= a {
                break;
            }
            let c = cs[b - 1];
            if c == '.' && b - a >= 3 && cs[b - 2] == '.' && cs[b - 3] == '.' {
                tail.push("...".into());
                b -= 3;
                continue;
            }
            if c == '.' {
                // abbreviations keep their dot: "U.S.", "Mr.", "e.g."
                let core: String = cs[a..b - 1].iter().collect();
                let dotted = core.contains('.')
                    && core
                        .split('.')
                        .all(|p| p.chars().count() <= 2 && p.chars().all(char::is_alphabetic));
                let title = matches!(
                    core.as_str(),
                    "Mr" | "Mrs"
                        | "Ms"
                        | "Dr"
                        | "St"
                        | "Jr"
                        | "Sr"
                        | "Prof"
                        | "vs"
                        | "etc"
                        | "Mt"
                        | "Inc"
                        | "Ltd"
                        | "Co"
                );
                if (dotted || title) && b - a > 1 {
                    break;
                }
            }
            if "\"”')]}.,;:!?…".contains(c)
                || (c == '-' && b - a > 1 && !cs[b - 2].is_alphanumeric())
            {
                tail.push(c.to_string());
                b -= 1;
            } else {
                break;
            }
        }
        if a < b {
            let core: String = cs[a..b].iter().collect();
            // em dash inside a chunk: word—word
            let pieces: Vec<&str> = if core.contains('—') && core != "—" {
                core.split_inclusive('—').collect()
            } else {
                vec![core.as_str()]
            };
            for piece in pieces {
                let (word, dash) = match piece.strip_suffix('—') {
                    Some(w) if !w.is_empty() => (w, true),
                    _ => (piece, false),
                };
                let lower = word.to_lowercase();
                let mut split_at = None;
                if word.chars().any(char::is_alphabetic) {
                    for c in CONTRACTIONS {
                        if lower.ends_with(c) && lower.len() > c.len() {
                            split_at = Some(word.len() - c.len());
                            break;
                        }
                    }
                }
                match split_at {
                    Some(k) => {
                        out.push(Tok {
                            text: word[..k].to_string(),
                            ..Default::default()
                        });
                        out.push(Tok {
                            text: word[k..].to_string(),
                            ..Default::default()
                        });
                    }
                    None => out.push(Tok {
                        text: word.to_string(),
                        ..Default::default()
                    }),
                }
                if dash {
                    out.push(Tok {
                        text: "—".into(),
                        ..Default::default()
                    });
                }
            }
        }
        for t in tail.into_iter().rev() {
            out.push(Tok {
                text: t,
                ..Default::default()
            });
        }
        if out.len() > start {
            if let Some(last) = out.last_mut() {
                last.ws = true;
            }
        }
    }
    if let Some(last) = out.last_mut() {
        last.ws = false;
    }
    for t in &mut out {
        t.is_head = true;
    }
    out
}

// ---------------------------------------------------------------------------
// A small rule-based part-of-speech tagger (Penn tags), standing in for spaCy.

const DETERMINERS: &[&str] = &[
    "a", "an", "the", "this", "that", "these", "those", "my", "your", "his", "her", "its", "our",
    "their", "some", "any", "no", "every", "each", "another", "either", "neither", "all", "both",
    "several", "many", "much", "few", "more", "most", "what", "which", "whose",
];
const PREPOSITIONS: &[&str] = &[
    "of",
    "in",
    "on",
    "at",
    "by",
    "for",
    "with",
    "from",
    "into",
    "onto",
    "upon",
    "about",
    "over",
    "under",
    "after",
    "before",
    "between",
    "through",
    "during",
    "without",
    "within",
    "against",
    "among",
    "around",
    "behind",
    "below",
    "beneath",
    "beside",
    "besides",
    "beyond",
    "near",
    "since",
    "toward",
    "towards",
    "until",
    "across",
    "along",
    "despite",
    "except",
    "like",
    "than",
    "throughout",
    "via",
    "per",
    "unlike",
    "if",
    "because",
    "although",
    "though",
    "while",
    "whether",
    "as",
    "so",
];
const PRONOUNS: &[&str] = &[
    "i",
    "you",
    "he",
    "she",
    "it",
    "we",
    "they",
    "me",
    "him",
    "us",
    "them",
    "myself",
    "yourself",
    "himself",
    "herself",
    "itself",
    "ourselves",
    "themselves",
];
const MODALS: &[&str] = &[
    "will", "would", "can", "could", "shall", "should", "may", "might", "must", "ca", "wo", "'ll",
    "'d",
];
const BE: &[&str] = &[
    "is", "are", "was", "were", "be", "been", "being", "am", "'s", "'re", "'m",
];
const HAVE: &[&str] = &["have", "has", "had", "having", "'ve"];
const DO: &[&str] = &["do", "does", "did"];
const CONJ: &[&str] = &["and", "or", "but", "nor", "yet"];
const ADVERBS: &[&str] = &[
    "not",
    "n't",
    "never",
    "always",
    "often",
    "also",
    "just",
    "already",
    "still",
    "even",
    "only",
    "very",
    "too",
    "really",
    "then",
    "now",
    "here",
    "there",
    "soon",
    "again",
    "usually",
    "sometimes",
    "ever",
    "quickly",
    "slowly",
];

fn is_word(t: &str) -> bool {
    t.chars().any(char::is_alphanumeric)
}

/// Tag every token. The tagger decides only what the dictionary looks at:
/// noun, verb (base, past, participle, present), adjective, adverb,
/// determiner, preposition, pronoun, number, proper noun, punctuation.
fn tag_tokens(toks: &mut [Tok], tagged: impl Fn(&str) -> bool) {
    let n = toks.len();
    let lower: Vec<String> = toks.iter().map(|t| t.text.to_lowercase()).collect();
    let mut sentence_start = true;
    let mut quote_open = false;
    for i in 0..n {
        let w = lower[i].as_str();
        let text = toks[i].text.clone();
        let prev = if i > 0 { lower[i - 1].as_str() } else { "" };
        let prev2 = if i > 1 { lower[i - 2].as_str() } else { "" };
        let next = lower.get(i + 1).map_or("", |s| s.as_str());
        let tag: &'static str = if !is_word(&text) {
            match text.as_str() {
                "\"" | "“" | "”" => {
                    let open = if text == "“" {
                        true
                    } else if text == "”" {
                        false
                    } else {
                        !quote_open && is_open_quote_context(i > 0 && toks[i - 1].ws, i == 0)
                    };
                    quote_open = open;
                    if open {
                        "``"
                    } else {
                        "''"
                    }
                }
                "(" | "[" | "{" => "-LRB-",
                ")" | "]" | "}" => "-RRB-",
                "$" | "£" | "€" => "$",
                "#" => "#",
                "-" | "–" | "—" | "--" | ";" | ":" => ":",
                "," => ",",
                "." | "!" | "?" | "..." | "…" => ".",
                _ => "NFP",
            }
        } else if text.chars().next().is_some_and(|c| c.is_ascii_digit())
            || (text.starts_with('-') && text[1..].starts_with(|c: char| c.is_ascii_digit()))
        {
            "CD"
        } else if w == "n't" || w == "not" {
            "RB"
        } else if w == "'s" {
            if next.is_empty()
                || DETERMINERS.contains(&next)
                || next.ends_with("ing")
                || next.ends_with("ed")
                || ADVERBS.contains(&next)
                || next == "a"
            {
                "VBZ"
            } else {
                "POS"
            }
        } else if w == "to" {
            "TO"
        } else if w == "a" || w == "an" || w == "the" {
            "DT"
        } else if w == "i" {
            "PRP"
        } else if PRONOUNS.contains(&w) {
            "PRP"
        } else if MODALS.contains(&w) {
            "MD"
        } else if CONJ.contains(&w) {
            "CC"
        } else if w == "in"
            || w == "on"
            || w == "by"
            || w == "up"
            || w == "out"
            || w == "off"
            || w == "down"
            || w == "over"
            || w == "around"
        {
            // particle or adverb at the end of a clause ("come in", "stand by")
            if next.is_empty() || !is_word(next) {
                if w == "by" {
                    "RB"
                } else {
                    "RP"
                }
            } else {
                "IN"
            }
        } else if w == "that" {
            // complementiser or relative ("think that he", "the man that I saw"),
            // otherwise determiner / demonstrative pronoun
            let next_text = toks.get(i + 1).map_or("", |t| t.text.as_str());
            let clause_follows = PRONOUNS.contains(&next) && next != "it"
                || matches!(
                    next,
                    "the"
                        | "a"
                        | "an"
                        | "my"
                        | "your"
                        | "his"
                        | "her"
                        | "our"
                        | "their"
                        | "there"
                        | "this"
                        | "these"
                        | "those"
                        | "some"
                        | "no"
                        | "if"
                        | "when"
                        | "she"
                        | "he"
                        | "it"
                )
                || (next_text.chars().next().is_some_and(char::is_uppercase) && next != "i")
                || next.chars().next().is_some_and(|c| c.is_ascii_digit());
            let it_clause = next == "it"
                && lower.get(i + 2).is_some_and(|x| {
                    is_word(x) && !PREPOSITIONS.contains(&x.as_str()) && x != "and"
                });
            let complement_verb = matches!(
                prev,
                "think"
                    | "thinks"
                    | "thought"
                    | "know"
                    | "knew"
                    | "knows"
                    | "believe"
                    | "believed"
                    | "seems"
                    | "seem"
                    | "seemed"
                    | "found"
                    | "find"
                    | "assume"
                    | "imagine"
                    | "said"
                    | "says"
                    | "say"
                    | "surprised"
                    | "sure"
                    | "hope"
                    | "realize"
                    | "realized"
                    | "realise"
                    | "realised"
                    | "weird"
                    | "glad"
                    | "told"
                    | "means"
                    | "mean"
                    | "felt"
                    | "feel"
                    | "afraid"
                    | "true"
                    | "possible"
                    | "clear"
                    | "obvious"
                    | "noticed"
                    | "heard"
                    | "learned"
                    | "decided"
                    | "agree"
                    | "suggest"
                    | "suggested"
                    | "explained"
                    | "admit"
                    | "admitted"
                    | "claim"
                    | "claimed"
                    | "fact"
            );
            let verbish_next = BE.contains(&next)
                || HAVE.contains(&next)
                || MODALS.contains(&next)
                || DO.contains(&next)
                || next.ends_with("n't")
                || (next.ends_with("ed") && next.len() > 3);
            if matches!(
                prev,
                "do" | "did" | "does" | "doing" | "done" | "is" | "was" | "like"
            ) || next.is_empty()
                || !is_word(next)
            {
                "DT"
            } else if complement_verb {
                "IN"
            } else if i > 0
                && verbish_next
                && is_word(prev)
                && !PRONOUNS.contains(&prev)
                && !DETERMINERS.contains(&prev)
                && !PREPOSITIONS.contains(&prev)
                && guess_noun_like(prev)
            {
                "WDT"
            } else if (clause_follows || it_clause) && i > 0 {
                if prev.ends_with('s')
                    || is_word(prev)
                        && !BE.contains(&prev)
                        && !HAVE.contains(&prev)
                        && !PRONOUNS.contains(&prev)
                        && guess_noun_like(prev)
                {
                    "WDT"
                } else {
                    "IN"
                }
            } else {
                "DT"
            }
        } else if PREPOSITIONS.contains(&w) {
            "IN"
        } else if DETERMINERS.contains(&w) {
            "DT"
        } else if BE.contains(&w) || HAVE.contains(&w) || DO.contains(&w) {
            match w {
                "was" | "were" | "had" | "did" => "VBD",
                "is" | "has" | "does" | "'s" => "VBZ",
                "been" => "VBN",
                "being" | "having" => "VBG",
                _ => "VBP",
            }
        } else if ADVERBS.contains(&w) {
            "RB"
        } else if !sentence_start && text.chars().next().is_some_and(char::is_uppercase) {
            "NNP"
        } else if text.len() > 1
            && text.chars().all(|c| !c.is_lowercase())
            && text.chars().any(char::is_alphabetic)
        {
            "NNP"
        } else if tagged(w) || tagged(&text) {
            guess_open_class(w, prev, prev2, next)
        } else if w.ends_with("ly") {
            "RB"
        } else if w.ends_with("ing") {
            "VBG"
        } else if w.ends_with("ed") {
            if prev.is_empty() || BE.contains(&prev) || HAVE.contains(&prev) {
                "VBN"
            } else {
                "VBD"
            }
        } else {
            "NN"
        };
        toks[i].tag = tag;
        sentence_start = matches!(tag, "." | "``") || (sentence_start && !is_word(&text));
    }
}

/// Noun, verb or adjective for a word whose dictionary entry depends on it.
/// A rough guess that a word is a noun (for "the man that …").
fn guess_noun_like(w: &str) -> bool {
    !(ADVERBS.contains(&w)
        || PREPOSITIONS.contains(&w)
        || CONJ.contains(&w)
        || MODALS.contains(&w)
        || DO.contains(&w)
        || w.ends_with("ly")
        || w.ends_with("ed"))
        && !matches!(
            w,
            "said"
                | "say"
                | "says"
                | "think"
                | "thinks"
                | "thought"
                | "know"
                | "knew"
                | "knows"
                | "believe"
                | "hope"
                | "felt"
                | "feel"
                | "saw"
                | "see"
                | "told"
                | "tell"
                | "realized"
                | "realised"
                | "found"
                | "heard"
                | "means"
                | "mean"
                | "sure"
                | "so"
                | "such"
        )
}

fn guess_open_class(w: &str, prev: &str, prev2: &str, next: &str) -> &'static str {
    // look back over adverbs ("will never live", "we all live")
    let (prev, prev2) = if (ADVERBS.contains(&prev)
        || matches!(
            prev,
            "all"
                | "both"
                | "each"
                | "really"
                | "actually"
                | "probably"
                | "certainly"
                | "simply"
                | "only"
        ))
        && !prev2.is_empty()
    {
        (prev2, "")
    } else {
        (prev, prev2)
    };
    if w.ends_with("ed") {
        if HAVE.contains(&prev) || BE.contains(&prev) {
            return "VBN";
        }
        if DETERMINERS.contains(&prev) {
            return "JJ";
        }
        return "VBD";
    }
    if w == "close"
        && (next == "to"
            || matches!(
                prev,
                "very" | "so" | "too" | "quite" | "pretty" | "how" | "as"
            ))
    {
        return "JJ";
    }
    let next_is_word = is_word(next) && !next.is_empty();
    let next_starts_object = DETERMINERS.contains(&next)
        || PRONOUNS.contains(&next)
        || next.chars().next().is_some_and(|c| c.is_ascii_digit());
    if prev == "to"
        || MODALS.contains(&prev)
        || DO.contains(&prev)
        || prev == "n't"
        || prev == "please"
        || prev == "let's"
        || (prev == "not" && (MODALS.contains(&prev2) || DO.contains(&prev2)))
    {
        return "VB";
    }
    if HAVE.contains(&prev) || (HAVE.contains(&prev2) && ADVERBS.contains(&prev)) {
        return "VBN";
    }
    if BE.contains(&prev) {
        return if next_starts_object || next == "by" {
            "VBN"
        } else {
            "JJ"
        };
    }
    if DETERMINERS.contains(&prev)
        || prev.ends_with("'s")
        || prev == "'s"
        || PREPOSITIONS.contains(&prev)
    {
        // determiner + word + noun-like word: adjective; otherwise noun
        let adj_next = next_is_word
            && !PREPOSITIONS.contains(&next)
            && !CONJ.contains(&next)
            && !BE.contains(&next)
            && !MODALS.contains(&next)
            && !HAVE.contains(&next)
            && !DO.contains(&next)
            && next != "to"
            && !next.ends_with("ed")
            && !next.ends_with("ly");
        return if adj_next && w == "live" { "JJ" } else { "NN" };
    }
    if PRONOUNS.contains(&prev) && !matches!(prev, "me" | "him" | "us" | "them") {
        if matches!(prev, "he" | "she" | "it") {
            return if w.ends_with('s') { "VBZ" } else { "VBD" };
        }
        return "VBP";
    }
    if next_starts_object {
        return "VB";
    }
    "NN"
}

// ---------------------------------------------------------------------------
// The lexicon logic (misaki's `Lexicon`).

fn s_suffix(stem: &str, british: bool) -> Option<String> {
    let last = stem.chars().last()?;
    Some(if "ptkfθ".contains(last) {
        format!("{stem}s")
    } else if "szʃʒʧʤ".contains(last) {
        format!("{stem}{}z", if british { 'ɪ' } else { 'ᵻ' })
    } else {
        format!("{stem}z")
    })
}

fn ed_suffix(stem: &str, british: bool) -> Option<String> {
    let cs: Vec<char> = stem.chars().collect();
    let last = *cs.last()?;
    Some(if "pkfθʃsʧ".contains(last) {
        format!("{stem}t")
    } else if last == 'd' {
        format!("{stem}{}d", if british { 'ɪ' } else { 'ᵻ' })
    } else if last != 't' {
        format!("{stem}d")
    } else if british || cs.len() < 2 {
        format!("{stem}ɪd")
    } else if US_TAUS.contains(cs[cs.len() - 2]) {
        format!("{}ɾᵻd", cs[..cs.len() - 1].iter().collect::<String>())
    } else {
        format!("{stem}ᵻd")
    })
}

fn ing_suffix(stem: &str, british: bool) -> Option<String> {
    let cs: Vec<char> = stem.chars().collect();
    let last = *cs.last()?;
    if british {
        if "əː".contains(last) {
            return None;
        }
    } else if cs.len() > 1 && last == 't' && US_TAUS.contains(cs[cs.len() - 2]) {
        return Some(format!(
            "{}ɾɪŋ",
            cs[..cs.len() - 1].iter().collect::<String>()
        ));
    }
    Some(format!("{stem}ɪŋ"))
}

type Lookup = (Option<String>, u8);

impl MisakiLexicon {
    fn get(&self, store: &Store, word: &str) -> Option<Entry> {
        store.get(word)
    }

    fn get_nnp(&self, word: &str) -> Lookup {
        let mut ps = String::new();
        for c in word.chars().filter(|c| c.is_alphabetic()) {
            let key = c.to_uppercase().to_string();
            match self.gold.get(key.as_str()) {
                Some(Entry::One(p)) => ps.push_str(&p),
                _ => return (None, 0),
            }
        }
        let ps = apply_stress(&ps, Some(0.0));
        match ps.rfind(SECONDARY) {
            Some(i) => (
                Some(format!(
                    "{}{}{}",
                    &ps[..i],
                    PRIMARY,
                    &ps[i + SECONDARY.len_utf8()..]
                )),
                3,
            ),
            None => (Some(ps), 3),
        }
    }

    fn gold_str(&self, w: &str) -> String {
        match self.gold.get(w) {
            Some(Entry::One(p)) => p.to_string(),
            Some(Entry::Tagged(v)) => v
                .iter()
                .find(|t| &*t.0 == "DEFAULT")
                .and_then(|t| t.1.as_deref())
                .unwrap_or("")
                .to_string(),
            None => String::new(),
        }
    }

    fn gold_tag(&self, w: &str, tag: &str) -> String {
        match self.gold.get(w) {
            Some(Entry::Tagged(v)) => v
                .iter()
                .find(|t| &*t.0 == tag)
                .and_then(|t| t.1.as_deref())
                .unwrap_or("")
                .to_string(),
            _ => self.gold_str(w),
        }
    }

    fn special_case(&self, word: &str, tag: &str, stress: Option<f64>, ctx: Ctx) -> Lookup {
        match word {
            _ if tag == "ADD" && (word == "." || word == "/") => {
                return self.lookup(
                    if word == "." { "dot" } else { "slash" },
                    None,
                    Some(-0.5),
                    Some(ctx),
                );
            }
            "%" => return self.lookup("percent", None, None, Some(ctx)),
            "&" => return self.lookup("and", None, None, Some(ctx)),
            "+" => return self.lookup("plus", None, None, Some(ctx)),
            "@" => return self.lookup("at", None, None, Some(ctx)),
            _ => {}
        }
        let stripped = word.trim_matches('.');
        if stripped.contains('.')
            && is_alpha(&word.replace('.', ""))
            && word
                .split('.')
                .map(|p| p.chars().count())
                .max()
                .unwrap_or(0)
                < 3
        {
            return self.get_nnp(word);
        }
        match word {
            "a" | "A" => (
                Some(if tag == "DT" {
                    "ɐ".into()
                } else {
                    "ˈA".into()
                }),
                4,
            ),
            "am" | "Am" | "AM" => {
                if tag.starts_with("NN") {
                    self.get_nnp(word)
                } else if ctx.future_vowel.is_none()
                    || word != "am"
                    || stress.is_some_and(|s| s > 0.0)
                {
                    (Some(self.gold_str("am")), 4)
                } else {
                    (Some("ɐm".into()), 4)
                }
            }
            "an" | "An" | "AN" => {
                if word == "AN" && tag.starts_with("NN") {
                    self.get_nnp(word)
                } else {
                    (Some("ɐn".into()), 4)
                }
            }
            "I" if tag == "PRP" => (Some("ˌI".into()), 4),
            "by" | "By" | "BY" if parent_tag(tag) == "ADV" => (Some("bˈI".into()), 4),
            "to" | "To" => (Some(self.to_form(ctx)), 4),
            "TO" if tag == "TO" || tag == "IN" => (Some(self.to_form(ctx)), 4),
            "in" | "In" => (
                Some(format!(
                    "{}ɪn",
                    if ctx.future_vowel.is_none() || tag != "IN" {
                        "ˈ"
                    } else {
                        ""
                    }
                )),
                4,
            ),
            "IN" if tag != "NNP" => (
                Some(format!(
                    "{}ɪn",
                    if ctx.future_vowel.is_none() || tag != "IN" {
                        "ˈ"
                    } else {
                        ""
                    }
                )),
                4,
            ),
            "the" | "The" => (
                Some(if ctx.future_vowel == Some(true) {
                    "ði".into()
                } else {
                    "ðə".into()
                }),
                4,
            ),
            "THE" if tag == "DT" => (
                Some(if ctx.future_vowel == Some(true) {
                    "ði".into()
                } else {
                    "ðə".into()
                }),
                4,
            ),
            "used" | "Used" | "USED" => {
                if (tag == "VBD" || tag == "JJ") && ctx.future_to {
                    (Some(self.gold_tag("used", "VBD")), 4)
                } else {
                    (Some(self.gold_tag("used", "DEFAULT")), 4)
                }
            }
            _ if tag == "IN"
                && (word.eq_ignore_ascii_case("vs") || word.eq_ignore_ascii_case("vs.")) =>
            {
                self.lookup("versus", None, None, Some(ctx))
            }
            _ => (None, 0),
        }
    }

    fn to_form(&self, ctx: Ctx) -> String {
        match ctx.future_vowel {
            None => self.gold_str("to"),
            Some(false) => "tə".into(),
            Some(true) => "tʊ".into(),
        }
    }

    fn is_known(&self, word: &str) -> bool {
        if self.gold.contains_key(word)
            || self.silver.contains_key(word)
            || matches!(word, "%" | "&" | "+" | "@")
        {
            return true;
        }
        if !is_alpha(word) || !lexicon_chars(word) {
            return false;
        }
        if word.chars().count() == 1 {
            return true;
        }
        if word == word.to_uppercase() && self.gold.contains_key(word.to_lowercase().as_str()) {
            return true;
        }
        let rest: String = word.chars().skip(1).collect();
        rest == rest.to_uppercase()
    }

    fn lookup(
        &self,
        word: &str,
        tag: Option<&str>,
        stress: Option<f64>,
        ctx: Option<Ctx>,
    ) -> Lookup {
        let mut word = word.to_string();
        let mut is_nnp = false;
        if word == word.to_uppercase() && !self.gold.contains_key(word.as_str()) {
            word = word.to_lowercase();
            is_nnp = tag == Some("NNP");
        }
        let mut entry = self.get(&self.gold, &word);
        let mut rating = 4;
        if entry.is_none() && !is_nnp {
            entry = self.get(&self.silver, &word);
            rating = 3;
        }
        let ps: Option<String> = match entry {
            Some(Entry::One(p)) => Some(p.to_string()),
            Some(Entry::Tagged(v)) => {
                let has = |t: &str| v.iter().any(|e| &*e.0 == t);
                let mut t: String = tag.unwrap_or("None").to_string();
                if ctx.is_some_and(|c| c.future_vowel.is_none()) && has("None") {
                    t = "None".into();
                } else if !has(&t) {
                    t = parent_tag(&t).to_string();
                }
                match v.iter().find(|e| *e.0 == *t) {
                    Some(e) => e.1.as_deref().map(str::to_string),
                    None => v
                        .iter()
                        .find(|e| &*e.0 == "DEFAULT")
                        .and_then(|e| e.1.as_deref())
                        .map(str::to_string),
                }
            }
            None => None,
        };
        if ps.is_none() || (is_nnp && !ps.as_deref().unwrap_or("").contains(PRIMARY)) {
            let (p, r) = self.get_nnp(&word);
            if p.is_some() {
                return (p, r);
            }
        }
        (ps.map(|p| apply_stress(&p, stress)), rating)
    }

    fn stem_s(
        &self,
        word: &str,
        tag: Option<&str>,
        stress: Option<f64>,
        ctx: Option<Ctx>,
    ) -> Lookup {
        let n = word.chars().count();
        if n < 3 || !word.ends_with('s') {
            return (None, 0);
        }
        let stem = if !word.ends_with("ss") && self.is_known(&word[..word.len() - 1]) {
            word[..word.len() - 1].to_string()
        } else if (word.ends_with("'s")
            || (n > 4 && word.ends_with("es") && !word.ends_with("ies")))
            && self.is_known(&word[..word.len() - 2])
        {
            word[..word.len() - 2].to_string()
        } else if n > 4
            && word.ends_with("ies")
            && self.is_known(&format!("{}y", &word[..word.len() - 3]))
        {
            format!("{}y", &word[..word.len() - 3])
        } else {
            return (None, 0);
        };
        let (p, r) = self.lookup(&stem, tag, stress, ctx);
        (p.and_then(|p| s_suffix(&p, self.british)), r)
    }

    fn stem_ed(
        &self,
        word: &str,
        tag: Option<&str>,
        stress: Option<f64>,
        ctx: Option<Ctx>,
    ) -> Lookup {
        let n = word.chars().count();
        if n < 4 || !word.ends_with('d') {
            return (None, 0);
        }
        let stem = if !word.ends_with("dd") && self.is_known(&word[..word.len() - 1]) {
            &word[..word.len() - 1]
        } else if n > 4
            && word.ends_with("ed")
            && !word.ends_with("eed")
            && self.is_known(&word[..word.len() - 2])
        {
            &word[..word.len() - 2]
        } else {
            return (None, 0);
        };
        let (p, r) = self.lookup(stem, tag, stress, ctx);
        (p.and_then(|p| ed_suffix(&p, self.british)), r)
    }

    fn stem_ing(
        &self,
        word: &str,
        tag: Option<&str>,
        stress: Option<f64>,
        ctx: Option<Ctx>,
    ) -> Lookup {
        let n = word.chars().count();
        if n < 5 || !word.ends_with("ing") {
            return (None, 0);
        }
        let base = &word[..word.len() - 3];
        let doubled = {
            let b = base.as_bytes();
            n > 5
                && b.len() >= 2
                && ((b[b.len() - 1] == b[b.len() - 2]
                    && b"bcdgklmnprstvxz".contains(&b[b.len() - 1]))
                    || base.ends_with("ck"))
        };
        let stem = if n > 5 && self.is_known(base) {
            base.to_string()
        } else if self.is_known(&format!("{base}e")) {
            format!("{base}e")
        } else if doubled && self.is_known(&base[..base.len() - 1]) {
            base[..base.len() - 1].to_string()
        } else {
            return (None, 0);
        };
        let (p, r) = self.lookup(&stem, tag, stress, ctx);
        (p.and_then(|p| ing_suffix(&p, self.british)), r)
    }

    fn get_word(&self, word: &str, tag: &str, stress: Option<f64>, ctx: Ctx) -> Lookup {
        let sc = self.special_case(word, tag, stress, ctx);
        if sc.0.is_some() {
            return sc;
        }
        let wl = word.to_lowercase();
        let mut word = word.to_string();
        let plain = word.replace('\'', "");
        let rest: String = word.chars().skip(1).collect();
        if word.chars().count() > 1
            && is_alpha(&plain)
            && word != wl
            && (tag != "NNP" || word.chars().count() > 7)
            && !self.contains(&word)
            && (word == word.to_uppercase() || rest == rest.to_lowercase())
            && (self.contains(&wl)
                || self.stem_s(&wl, Some(tag), stress, Some(ctx)).0.is_some()
                || self.stem_ed(&wl, Some(tag), stress, Some(ctx)).0.is_some()
                || self.stem_ing(&wl, Some(tag), stress, Some(ctx)).0.is_some())
        {
            word = wl;
        }
        if self.is_known(&word) {
            return self.lookup(&word, Some(tag), stress, Some(ctx));
        }
        if let Some(stem) = word.strip_suffix("s'") {
            let w2 = format!("{stem}'s");
            if self.is_known(&w2) {
                return self.lookup(&w2, Some(tag), stress, Some(ctx));
            }
        }
        if let Some(stem) = word.strip_suffix('\'') {
            if self.is_known(stem) {
                return self.lookup(stem, Some(tag), stress, Some(ctx));
            }
        }
        let r = self.stem_s(&word, Some(tag), stress, Some(ctx));
        if r.0.is_some() {
            return r;
        }
        let r = self.stem_ed(&word, Some(tag), stress, Some(ctx));
        if r.0.is_some() {
            return r;
        }
        let r = self.stem_ing(&word, Some(tag), Some(stress.unwrap_or(0.5)), Some(ctx));
        if r.0.is_some() {
            return r;
        }
        (None, 0)
    }

    fn currency_names(c: char) -> Option<(&'static str, &'static str)> {
        match c {
            '$' => Some(("dollar", "cent")),
            '£' => Some(("pound", "pence")),
            '€' => Some(("euro", "cent")),
            _ => None,
        }
    }

    fn is_number(word: &str, is_head: bool) -> bool {
        if !word.bytes().any(|b| b.is_ascii_digit()) {
            return false;
        }
        let mut w = word;
        for s in ["ing", "'d", "ed", "'s", "st", "nd", "rd", "th", "s"] {
            if let Some(x) = w.strip_suffix(s) {
                w = x;
                break;
            }
        }
        w.chars().enumerate().all(|(i, c)| {
            c.is_ascii_digit() || c == ',' || c == '.' || (is_head && i == 0 && c == '-')
        })
    }

    fn number(&self, word: &str, currency: Option<char>, is_head: bool) -> Lookup {
        let suffix_len = word
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_lowercase() || *c == '\'')
            .count();
        let (mut num, suffix) = word.split_at(word.len() - suffix_len);
        let suffix = if suffix.is_empty() {
            None
        } else {
            Some(suffix)
        };
        let mut result: Vec<Lookup> = Vec::new();
        if let Some(rest) = num.strip_prefix('-') {
            result.push(self.lookup("minus", None, None, None));
            num = rest;
        }
        let ordinal = matches!(suffix, Some("st" | "nd" | "rd" | "th"));
        let extend = |words: Vec<String>, result: &mut Vec<Lookup>| {
            for w in words {
                if w == "and" {
                    continue;
                }
                result.push(self.lookup(
                    &w,
                    None,
                    if w == "point" { Some(-2.0) } else { None },
                    None,
                ));
            }
        };
        let card = |n: u64| {
            let mut v = Vec::new();
            cardinal_words(n, &mut v);
            v
        };
        let digits = |s: &str, result: &mut Vec<Lookup>| {
            for d in s.bytes().filter(u8::is_ascii_digit) {
                result.push(self.lookup(ONES[(d - b'0') as usize], None, None, None));
            }
        };
        if is_digits(num) && ordinal {
            let mut v = Vec::new();
            ordinal_words(num.parse().unwrap_or(0), &mut v);
            extend(v, &mut result);
        } else if result.is_empty()
            && num.len() == 4
            && currency.and_then(Self::currency_names).is_none()
            && is_digits(num)
        {
            let mut v = Vec::new();
            year_words(num.parse().unwrap_or(0), &mut v);
            extend(v, &mut result);
        } else if !is_head && !num.contains('.') {
            let n = num.replace(',', "");
            if n.starts_with('0') || n.len() > 3 {
                digits(&n, &mut result);
            } else if n.len() == 3 && !n.ends_with("00") {
                extend(card(n[..1].parse().unwrap_or(0)), &mut result);
                if &n[1..2] == "0" {
                    result.push(self.lookup("O", None, Some(-2.0), None));
                    extend(card(n[2..].parse().unwrap_or(0)), &mut result);
                } else {
                    extend(card(n[1..].parse().unwrap_or(0)), &mut result);
                }
            } else {
                extend(card(n.parse().unwrap_or(0)), &mut result);
            }
        } else if num.matches('.').count() > 1 || !is_head {
            for part in num.replace(',', "").split('.') {
                if part.is_empty() {
                    continue;
                }
                if part.starts_with('0')
                    || (part.len() != 2 && part[1..].bytes().any(|b| b != b'0'))
                {
                    digits(part, &mut result);
                } else {
                    extend(card(part.parse().unwrap_or(0)), &mut result);
                }
            }
        } else if let Some((unit, sub)) = currency.and_then(Self::currency_names).filter(|_| {
            !num.contains('.')
                || (num.matches('.').count() == 1
                    && num.split('.').nth(1).is_some_and(|c| c.len() < 3))
        }) {
            let clean = num.replace(',', "");
            let mut parts: Vec<(u64, &str)> = clean
                .split('.')
                .zip([unit, sub])
                .map(|(n, u)| (n.parse().unwrap_or(0), u))
                .collect();
            if parts.len() > 1 {
                if parts[1].0 == 0 {
                    parts.truncate(1);
                } else if parts[0].0 == 0 {
                    parts.remove(0);
                }
            }
            for (k, (n, u)) in parts.iter().enumerate() {
                if k > 0 {
                    result.push(self.lookup("and", None, None, None));
                }
                extend(card(*n), &mut result);
                if *n != 1 && *u != "pence" {
                    result.push(self.stem_s(&format!("{u}s"), None, None, None));
                } else {
                    result.push(self.lookup(u, None, None, None));
                }
            }
        } else if is_digits(num) {
            extend(card(num.parse().unwrap_or(0)), &mut result);
        } else if !num.contains('.') {
            let n: u64 = num.replace(',', "").parse().unwrap_or(0);
            let mut v = Vec::new();
            if ordinal {
                ordinal_words(n, &mut v);
            } else {
                cardinal_words(n, &mut v);
            }
            extend(v, &mut result);
        } else {
            let clean = num.replace(',', "");
            let mut v = Vec::new();
            if let Some(f) = clean.strip_prefix('.') {
                v.push("point".to_string());
                for d in f.bytes() {
                    v.push(ONES[(d - b'0') as usize].into());
                }
            } else {
                float_words(&clean, &mut v);
            }
            extend(v, &mut result);
        }
        if result.is_empty() || result.iter().any(|r| r.0.is_none()) {
            return (None, 0);
        }
        let rating = result.iter().map(|r| r.1).min().unwrap_or(0);
        let joined = result
            .into_iter()
            .filter_map(|r| r.0)
            .collect::<Vec<_>>()
            .join(" ");
        let out = match suffix {
            Some("s" | "'s") => s_suffix(&joined, self.british),
            Some("ed" | "'d") => ed_suffix(&joined, self.british),
            Some("ing") => ing_suffix(&joined, self.british),
            _ => Some(joined),
        };
        (out, rating)
    }

    fn append_currency(&self, ps: String, currency: Option<char>) -> String {
        match currency.and_then(Self::currency_names) {
            Some((unit, _)) => match self.stem_s(&format!("{unit}s"), None, None, None).0 {
                Some(c) => format!("{ps} {c}"),
                None => ps,
            },
            None => ps,
        }
    }

    /// misaki's `Lexicon.__call__` for one (possibly merged) token.
    fn token(
        &self,
        text: &str,
        tag: &str,
        currency: Option<char>,
        is_head: bool,
        ctx: Ctx,
    ) -> Lookup {
        let word: String = text.replace(['‘', '’'], "'");
        let stress = if word == word.to_lowercase() {
            None
        } else if word == word.to_uppercase() {
            Some(2.0)
        } else {
            Some(0.5)
        };
        let (ps, rating) = self.get_word(&word, tag, stress, ctx);
        if let Some(ps) = ps {
            return (Some(self.append_currency(ps, currency)), rating);
        }
        if Self::is_number(&word, is_head) {
            return self.number(&word, currency, is_head);
        }
        (None, 0)
    }
}

// ---------------------------------------------------------------------------
// Sub-word splitting (misaki's `subtokenize` regular expression, hand-written).

fn is_q(c: char) -> bool {
    matches!(c, '\'' | '‘' | '’')
}

fn subtokenize(word: &str) -> Vec<String> {
    let cs: Vec<char> = word.chars().collect();
    let n = cs.len();
    let mut out = Vec::new();
    let mut i = 0;
    let letter = |k: usize| k < n && cs[k].is_alphabetic();
    let upper = |k: usize| k < n && cs[k].is_uppercase();
    let lowerc = |k: usize| k < n && cs[k].is_lowercase();
    let digit = |k: usize| k < n && cs[k].is_ascii_digit();
    while i < n {
        // ^['‘’]+
        if i == 0 && is_q(cs[0]) {
            let mut j = 0;
            while j < n && is_q(cs[j]) {
                j += 1;
            }
            out.push(cs[..j].iter().collect());
            i = j;
            continue;
        }
        // \p{Lu}(?=\p{Lu}\p{Ll})
        if upper(i) && upper(i + 1) && lowerc(i + 2) {
            out.push(cs[i].to_string());
            i += 1;
            continue;
        }
        // (?:^-)?(?:\d?[,.]?\d)+
        {
            let sep = |k: usize| k < n && (cs[k] == ',' || cs[k] == '.');
            let start = if i == 0 && cs[0] == '-' { 1 } else { i };
            let mut j = start;
            loop {
                if digit(j) && sep(j + 1) && digit(j + 2) {
                    j += 3;
                } else if digit(j) && digit(j + 1) {
                    j += 2;
                } else if sep(j) && digit(j + 1) {
                    j += 2;
                } else if digit(j) {
                    j += 1;
                } else {
                    break;
                }
            }
            if j > start {
                out.push(cs[i..j].iter().collect());
                i = j;
                continue;
            }
        }
        // [-_]+
        if cs[i] == '-' || cs[i] == '_' {
            let mut j = i;
            while j < n && (cs[j] == '-' || cs[j] == '_') {
                j += 1;
            }
            out.push(cs[i..j].iter().collect());
            i = j;
            continue;
        }
        // ['‘’]{2,}
        if is_q(cs[i]) && i + 1 < n && is_q(cs[i + 1]) {
            let mut j = i;
            while j < n && is_q(cs[j]) {
                j += 1;
            }
            out.push(cs[i..j].iter().collect());
            i = j;
            continue;
        }
        // \p{L}*?(?:['‘’]\p{L})*?\p{Ll}(?=\p{Lu})  — a camelCase hump
        if letter(i) || is_q(cs[i]) {
            let mut k = i;
            let mut found = None;
            while k < n {
                if lowerc(k) && upper(k + 1) {
                    found = Some(k + 1);
                    break;
                }
                if letter(k) {
                    k += 1;
                } else if is_q(cs[k]) && letter(k + 1) {
                    k += 2;
                    if lowerc(k - 1) && upper(k) {
                        found = Some(k);
                        break;
                    }
                } else {
                    break;
                }
            }
            if let Some(j) = found {
                if letter(i) || (is_q(cs[i]) && letter(i + 1)) {
                    out.push(cs[i..j].iter().collect());
                    i = j;
                    continue;
                }
            }
        }
        // \p{L}+(?:['‘’]\p{L})*
        if letter(i) {
            let mut j = i;
            while letter(j) {
                j += 1;
            }
            while j + 1 < n && is_q(cs[j]) && letter(j + 1) {
                j += 2;
            }
            out.push(cs[i..j].iter().collect());
            i = j;
            continue;
        }
        // ['‘’]+$
        if is_q(cs[i]) && cs[i..].iter().all(|&c| is_q(c)) {
            out.push(cs[i..].iter().collect());
            break;
        }
        // [^-_\p{L}'‘’\d]
        if !(cs[i] == '-'
            || cs[i] == '_'
            || cs[i].is_alphabetic()
            || is_q(cs[i])
            || cs[i].is_ascii_digit())
        {
            out.push(cs[i].to_string());
        }
        i += 1;
    }
    out
}

// ---------------------------------------------------------------------------
// The pipeline.

/// Words that misaki cannot find go to this function (the role misaki gives
/// to espeak-ng). It returns misaki-style phonemes or `None`.
pub type Fallback<'a> = dyn Fn(&str) -> Option<String> + 'a;

fn token_context(ctx: Ctx, ps: Option<&str>, text: &str, tag: &str) -> Ctx {
    let mut vowel = ctx.future_vowel;
    if let Some(ps) = ps {
        for c in ps.chars() {
            if NON_QUOTE_PUNCTS.contains(c) {
                vowel = None;
                break;
            }
            if is_vowel(c) {
                vowel = Some(true);
                break;
            }
            if CONSONANTS.contains(c) {
                vowel = Some(false);
                break;
            }
        }
    }
    let future_to = text == "to" || text == "To" || (text == "TO" && (tag == "TO" || tag == "IN"));
    Ctx {
        future_vowel: vowel,
        future_to,
    }
}

fn merged_text(toks: &[Tok]) -> String {
    let mut s = String::new();
    for (k, t) in toks.iter().enumerate() {
        s.push_str(&t.text);
        if k + 1 < toks.len() && t.ws {
            s.push(' ');
        }
    }
    s
}

fn merged_tag(toks: &[Tok]) -> &'static str {
    toks.iter()
        .max_by_key(|t| {
            t.text
                .chars()
                .map(|c| {
                    if c == c.to_lowercase().next().unwrap_or(c) {
                        1
                    } else {
                        2
                    }
                })
                .sum::<usize>()
        })
        .map_or("NN", |t| t.tag)
}

enum Word {
    One(Tok),
    Group(Vec<Tok>),
}

/// Text to Kokoro tokens with misaki's conventions.
pub fn phonemize(lex: &MisakiLexicon, text: &str, fallback: &Fallback<'_>) -> String {
    let mut toks = tokenize(text);
    tag_tokens(&mut toks, |w| {
        matches!(lex.gold.get(w), Some(Entry::Tagged(_)))
    });
    // retokenize
    let mut words: Vec<Word> = Vec::new();
    let mut currency: Option<char> = None;
    let ntok = toks.len();
    for (i, token) in toks.iter().enumerate() {
        let subs = subtokenize(&token.text);
        let subs = if subs.is_empty() {
            vec![token.text.clone()]
        } else {
            subs
        };
        let mut tks: Vec<Tok> = subs
            .into_iter()
            .map(|t| Tok {
                text: t,
                tag: token.tag,
                ws: false,
                is_head: true,
                ..Default::default()
            })
            .collect();
        let m = tks.len();
        tks[m - 1].ws = token.ws;
        for j in 0..m {
            let mut tk = std::mem::take(&mut tks[j]);
            let single_punct_tag = matches!(
                tk.tag,
                "." | "," | "-LRB-" | "-RRB-" | "``" | "''" | ":" | "$" | "#" | "NFP"
            );
            if tk.tag == "$"
                && tk.text.chars().count() == 1
                && MisakiLexicon::currency_names(tk.text.chars().next().unwrap_or(' ')).is_some()
            {
                currency = tk.text.chars().next();
                tk.phonemes = Some(String::new());
            } else if tk.tag == ":" && (tk.text == "-" || tk.text == "–") {
                tk.phonemes = Some("—".into());
            } else if single_punct_tag && !tk.text.chars().all(|c| c.is_ascii_alphabetic()) {
                tk.phonemes = Some(match tk.tag {
                    "-LRB-" => "(".into(),
                    "-RRB-" => ")".into(),
                    "``" => "“".into(),
                    "''" => "”".into(),
                    _ => tk.text.chars().filter(|c| PUNCTS.contains(*c)).collect(),
                });
            } else if let Some(c) = currency {
                if tk.tag != "CD" {
                    currency = None;
                } else if j + 1 == m && (i + 1 == ntok || toks[i + 1].tag != "CD") {
                    tk.currency = Some(c);
                }
            } else if 0 < j && j + 1 < m && tk.text == "2" {
                let a = tks[j - 1].text.chars().last().unwrap_or(' ');
                let b = tks[j + 1].text.chars().next().unwrap_or(' ');
                if a.is_alphabetic() && b.is_alphabetic() {
                    tk.alias = Some("to");
                }
            }
            if tk.alias.is_some() || tk.phonemes.is_some() {
                words.push(Word::One(tk));
            } else if let Some(Word::Group(g)) = words
                .last_mut()
                .filter(|w| matches!(w, Word::Group(g) if g.last().is_some_and(|t| !t.ws)))
            {
                tk.is_head = false;
                g.push(tk);
            } else if tk.ws {
                words.push(Word::One(tk));
            } else {
                words.push(Word::Group(vec![tk]));
            }
        }
    }
    let words: Vec<Word> = words
        .into_iter()
        .map(|w| match w {
            Word::Group(mut g) if g.len() == 1 => Word::One(g.pop().unwrap_or_default()),
            w => w,
        })
        .collect();
    let mut words = words;
    // right to left
    let mut ctx = Ctx::default();
    for w in words.iter_mut().rev() {
        match w {
            Word::One(tk) => {
                if tk.phonemes.is_none() {
                    let text = tk
                        .alias
                        .map(str::to_string)
                        .unwrap_or_else(|| tk.text.clone());
                    let (ps, _) = lex.token(&text, tk.tag, tk.currency, tk.is_head, ctx);
                    tk.phonemes = ps.or_else(|| fallback(&tk.text));
                }
                ctx = token_context(ctx, tk.phonemes.as_deref(), &tk.text, tk.tag);
            }
            Word::Group(g) => {
                let (mut left, mut right) = (0usize, g.len());
                let mut should_fallback = false;
                while left < right {
                    let any_set = g[left..right]
                        .iter()
                        .any(|t| t.alias.is_some() || t.phonemes.is_some());
                    let found = if any_set {
                        (None, 0)
                    } else {
                        let text = merged_text(&g[left..right]);
                        lex.token(
                            &text,
                            merged_tag(&g[left..right]),
                            g[right - 1].currency,
                            g[left].is_head,
                            ctx,
                        )
                    };
                    if let Some(ps) = found.0 {
                        let text = merged_text(&g[left..right]);
                        let tag = merged_tag(&g[left..right]);
                        g[left].phonemes = Some(ps.clone());
                        for x in &mut g[left + 1..right] {
                            x.phonemes = Some(String::new());
                        }
                        ctx = token_context(ctx, Some(&ps), &text, tag);
                        right = left;
                        left = 0;
                    } else if left + 1 < right {
                        left += 1;
                    } else {
                        right -= 1;
                        let tk = &mut g[right];
                        if tk.phonemes.is_none() {
                            if tk.text.chars().all(|c| SUBTOKEN_JUNKS.contains(c)) {
                                tk.phonemes = Some(String::new());
                            } else {
                                should_fallback = true;
                                break;
                            }
                        }
                        left = 0;
                    }
                }
                if should_fallback {
                    let text = merged_text(g);
                    let ps = fallback(&text);
                    g[0].phonemes = ps;
                    for x in &mut g[1..] {
                        x.phonemes = Some(String::new());
                    }
                } else {
                    resolve_group(g);
                }
            }
        }
    }
    // join
    let mut out = String::new();
    for w in &words {
        match w {
            Word::One(tk) => {
                out.push_str(tk.phonemes.as_deref().unwrap_or(""));
                if tk.ws {
                    out.push(' ');
                }
            }
            Word::Group(g) => {
                let mut ps = String::new();
                for tk in g {
                    if tk.prespace
                        && !ps.is_empty()
                        && !ps.ends_with(char::is_whitespace)
                        && tk.phonemes.as_deref().is_some_and(|p| !p.is_empty())
                    {
                        ps.push(' ');
                    }
                    ps.push_str(tk.phonemes.as_deref().unwrap_or(""));
                }
                out.push_str(&ps);
                if g.last().is_some_and(|t| t.ws) {
                    out.push(' ');
                }
            }
        }
    }
    out.replace('ɾ', "T").replace('ʔ', "t")
}

fn resolve_group(toks: &mut [Tok]) {
    let text = merged_text(toks);
    let classes: std::collections::BTreeSet<u8> = text
        .chars()
        .filter(|c| !SUBTOKEN_JUNKS.contains(*c))
        .map(|c| {
            if c.is_alphabetic() {
                0
            } else if c.is_ascii_digit() {
                1
            } else {
                2
            }
        })
        .collect();
    let prespace = text.contains(' ') || text.contains('/') || classes.len() > 1;
    let n = toks.len();
    for i in 0..n {
        if toks[i].phonemes.is_none() {
            if i + 1 == n
                && NON_QUOTE_PUNCTS.contains(toks[i].text.as_str())
                && toks[i].text.chars().count() == 1
            {
                toks[i].phonemes = Some(toks[i].text.clone());
            } else if toks[i].text.chars().all(|c| SUBTOKEN_JUNKS.contains(c)) {
                toks[i].phonemes = Some(String::new());
            }
        } else if i > 0 {
            toks[i].prespace = prespace;
        }
    }
    if prespace {
        return;
    }
    let mut idx: Vec<(bool, usize, usize)> = toks
        .iter()
        .enumerate()
        .filter_map(|(i, t)| {
            t.phonemes
                .as_deref()
                .filter(|p| !p.is_empty())
                .map(|p| (p.contains(PRIMARY), stress_weight(p), i))
        })
        .collect();
    if idx.len() == 2 && toks[idx[0].2].text.chars().count() == 1 {
        let i = idx[1].2;
        let p = toks[i].phonemes.take().unwrap_or_default();
        toks[i].phonemes = Some(apply_stress(&p, Some(-0.5)));
        return;
    }
    let primaries = idx.iter().filter(|x| x.0).count();
    if idx.len() < 2 || primaries <= idx.len().div_ceil(2) {
        return;
    }
    idx.sort();
    let half = idx.len() / 2;
    for &(_, _, i) in &idx[..half] {
        let p = toks[i].phonemes.take().unwrap_or_default();
        toks[i].phonemes = Some(apply_stress(&p, Some(-0.5)));
    }
}

/// uccharan's espeak-style IPA → misaki phones (the conversion misaki
/// applies to its espeak-ng fallback, `EspeakFallback.E2M`).
pub fn espeak_style_to_misaki(ipa: &str, british: bool) -> String {
    let mut s = ipa.to_string();
    for (a, b) in [
        ("ʔn\u{329}", "tn"),
        ("aɪ", "I"),
        ("aʊ", "W"),
        ("dʒ", "ʤ"),
        ("eɪ", "A"),
        ("tʃ", "ʧ"),
        ("ɔɪ", "Y"),
        ("əl", "ᵊl"),
        ("ɚ", "əɹ"),
        ("r", "ɹ"),
        ("x", "k"),
        ("ç", "k"),
        ("ɐ", "ə"),
        ("ɬ", "l"),
        ("\u{303}", ""),
    ] {
        s = s.replace(a, b);
    }
    while let Some(i) = s.find('\u{329}') {
        let start = s[..i].char_indices().next_back().map_or(0, |x| x.0);
        let c = s[start..i].to_string();
        s.replace_range(start..i + '\u{329}'.len_utf8(), &format!("ᵊ{c}"));
    }
    if british {
        s = s
            .replace("eə", "ɛː")
            .replace("iə", "ɪə")
            .replace("əʊ", "Q")
            .replace('æ', "a")
            .replace("ɜːɹ", "ɜː");
        // misaki's British dictionaries write the final happy vowel as i
        if s.ends_with('ɪ')
            && s.chars()
                .rev()
                .nth(1)
                .is_some_and(|c| !is_vowel(c) && c != 'ˈ' && c != 'ˌ')
        {
            s.pop();
            s.push('i');
        }
    } else {
        s = s
            .replace("oʊ", "O")
            .replace("ɜːɹ", "ɜɹ")
            .replace("ɜː", "ɜɹ")
            .replace("ɪə", "iə")
            .replace('ː', "");
    }
    s.replace('o', "ɔ").replace('ɾ', "T").replace('ʔ', "t")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex() -> MisakiLexicon {
        let gold = r#"{"hello": "həlˈO", "world": "wˈɜɹld", "the": "ðə", "to": "tʊ", "read": {"DEFAULT": "ɹˈid", "VBD": "ɹˈɛd", "VBN": "ɹˈɛd", "VBP": "ɹˈɛd", "ADJ": "ɹˈɛd"},
            "book": "bˈʊk", "five": "fˈIv", "dollar": "dˈɑləɹ", "cent": "sˈɛnt", "and": "ænd", "fifty": "fˈɪfti", "I": "ˈI", "am": "ˈæm",
            "nineteen": "nˌIntˈin", "ninety": "nˈIndi", "nine": "nˈIn", "U": "jˈu", "S": "ˈɛs", "walk": "wˈɔk", "it": "ɪt", "art": "ˈɑɹt"}"#;
        MisakiLexicon::from_json(gold, "{}", false).unwrap()
    }

    #[test]
    fn json_and_growth() {
        let l = lex();
        assert!(l.contains("Hello") && l.contains("hello") && l.contains("Read"));
        assert!(
            MisakiLexicon::from_json("{\"a\": \"\\u0259\"}", "{}", false)
                .unwrap()
                .contains("a")
        );
        assert!(MisakiLexicon::from_json("{\"a\": 1}", "{}", false).is_err());
    }

    #[test]
    fn pipeline() {
        let l = lex();
        let none = |_: &str| None;
        assert_eq!(phonemize(&l, "Hello, world!", &none), "həlˈO, wˈɜɹld!");
        assert_eq!(phonemize(&l, "I read the book.", &none), "ˌI ɹˈɛd ðə bˈʊk.");
        assert_eq!(
            phonemize(&l, "It costs $5.50.", &|_| Some("kˈɔsts".into())),
            "ˌɪt kˈɔsts fˈIv dˈɑləɹz ænd fˈɪfti sˈɛnts."
        );
        assert_eq!(
            phonemize(&l, "in 1999", &|_| None),
            "ɪn nˌIntˈin nˈIndi nˈIn"
        );
        assert_eq!(phonemize(&l, "walked", &none), "wˈɔkt");
        assert_eq!(phonemize(&l, "the art", &none), "ði ˈɑɹt");
    }

    #[test]
    fn subtokens() {
        assert_eq!(subtokenize("well-known"), ["well", "-", "known"]);
        assert_eq!(subtokenize("iPhone"), ["i", "Phone"]);
        assert_eq!(subtokenize("3.14"), ["3.14"]);
        assert_eq!(subtokenize("don't"), ["don't"]);
        assert_eq!(subtokenize("ABCDef"), ["ABCDef"]);
        assert_eq!(subtokenize("they're"), ["they'r", "e"]);
        assert_eq!(subtokenize("3:30pm"), ["3", ":", "30", "pm"]);
        assert_eq!(subtokenize("555-1234"), ["555", "-", "1234"]);
    }

    #[test]
    fn numbers_in_words() {
        let mut v = Vec::new();
        cardinal_words(1234, &mut v);
        assert_eq!(v.join(" "), "one thousand two hundred and thirty four");
        v.clear();
        year_words(2005, &mut v);
        assert_eq!(v.join(" "), "two thousand and five");
        v.clear();
        year_words(1805, &mut v);
        assert_eq!(v.join(" "), "eighteen oh five");
        v.clear();
        ordinal_words(21, &mut v);
        assert_eq!(v.join(" "), "twenty first");
    }
}
