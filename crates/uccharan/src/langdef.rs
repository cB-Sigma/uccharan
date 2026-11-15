//! The text format of a data-driven language (`crates/uccharan/lang/*.lang`)
//! and the parts every such language shares: metadata, an exceptions
//! lexicon, unstressed (function) words, and number words.
//!
//! A file is a list of sections:
//!
//! ```text
//! [meta]
//! tag = mr
//! stress = weight
//!
//! [classes]
//! V = a e i o u
//!
//! [pre]            ; rewrite rules on letters, before reading (see `rules`)
//! [post]           ; rewrite rules on phonemes, after reading
//! [post.compat]    ; only in compatible mode
//! [post.improved]  ; only in improved mode
//!
//! [words]          ; exceptions: word, then phonemes (or "compat | improved")
//! [unstressed]     ; function words: word, level inside a clause, level at its end (0, s, p)
//! [numbers]        ; number words: value, then the word(s)
//! ```
//!
//! Lines starting with `#` are comments; `;` starts a comment after content.

use std::collections::HashMap;

use crate::rules::{Classes, RuleError, RuleSet};

/// A parsed language file.
#[derive(Clone, Debug, Default)]
pub struct LangFile {
    /// `(section name, [(line number, text)])` in file order.
    pub sections: Vec<(String, Vec<(usize, String)>)>,
}

impl LangFile {
    /// Split a file into sections.
    pub fn parse(text: &str) -> LangFile {
        let mut f = LangFile::default();
        let mut cur = String::new();
        let mut lines: Vec<(usize, String)> = Vec::new();
        for (n, raw) in text.lines().enumerate() {
            let t = raw.trim();
            if t.starts_with('[') && t.ends_with(']') && !t.contains(' ') {
                if !lines.is_empty() || !cur.is_empty() {
                    f.sections
                        .push((std::mem::take(&mut cur), std::mem::take(&mut lines)));
                }
                cur = t[1..t.len() - 1].to_string();
                continue;
            }
            lines.push((n + 1, raw.to_string()));
        }
        f.sections.push((cur, lines));
        f
    }

    /// The raw text of all sections with this name (joined; line numbers
    /// are lost, so rule errors report the line inside the section).
    pub fn text(&self, name: &str) -> String {
        let mut s = String::new();
        for (n, ls) in &self.sections {
            if n == name {
                for (_, l) in ls {
                    s.push_str(l);
                    s.push('\n');
                }
            }
        }
        s
    }

    /// Non-empty, comment-free lines of a section.
    pub fn lines(&self, name: &str) -> Vec<String> {
        let mut out = Vec::new();
        for (n, ls) in &self.sections {
            if n == name {
                for (_, l) in ls {
                    let t = strip(l);
                    if !t.is_empty() {
                        out.push(t.to_string());
                    }
                }
            }
        }
        out
    }

    /// `key = value` pairs of the `[meta]` section.
    pub fn meta(&self) -> HashMap<String, String> {
        self.lines("meta")
            .iter()
            .filter_map(|l| {
                l.split_once('=')
                    .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
            })
            .collect()
    }

    /// Classes from `[classes]`.
    pub fn classes(&self) -> Classes {
        let mut c = Classes::default();
        for l in self.lines("classes") {
            c.parse_line(&l);
        }
        c
    }

    /// A rule pass, or an empty one when the section is absent.
    pub fn rules(&self, name: &str, classes: &mut Classes) -> Result<RuleSet, String> {
        RuleSet::parse(&self.text(name), classes)
            .map_err(|RuleError { line, message }| format!("[{name}] line {line}: {message}"))
    }
}

fn strip(l: &str) -> &str {
    let t = l.trim();
    if t.starts_with('#') {
        return "";
    }
    match t.find(';') {
        Some(i) if i == 0 || t.as_bytes()[i - 1] != b'\\' => t[..i].trim(),
        _ => t,
    }
}

/// The exceptions lexicon: word -> (compatible, improved) phonemes.
pub fn words(f: &LangFile) -> HashMap<String, (String, String)> {
    let mut m = HashMap::new();
    for l in f.lines("words") {
        let mut it = l.splitn(2, char::is_whitespace);
        let (Some(w), Some(p)) = (it.next(), it.next()) else {
            continue;
        };
        let (c, i) = match p.split_once('|') {
            Some((a, b)) => (a.trim().to_string(), b.trim().to_string()),
            None => (p.trim().to_string(), p.trim().to_string()),
        };
        m.insert(w.to_string(), (c, i));
    }
    m
}

/// Unstressed words: word -> (level inside a clause, level at its end),
/// each `0` (no stress), `s` (secondary at most) or `p` (keep).
pub fn unstressed(f: &LangFile) -> HashMap<String, (u8, u8)> {
    let mut m = HashMap::new();
    for l in f.lines("unstressed") {
        let p: Vec<&str> = l.split_whitespace().collect();
        if p.is_empty() {
            continue;
        }
        let lv = |s: Option<&&str>| s.and_then(|x| x.bytes().next()).unwrap_or(b'0');
        let mid = lv(p.get(1));
        let fin = p.get(2).map(|x| x.as_bytes()[0]).unwrap_or(mid);
        m.insert(p[0].to_string(), (mid, fin));
    }
    m
}

/// Number words of a language and how they combine.
#[derive(Clone, Debug, Default)]
pub struct Numbers {
    /// Exact values: `n` -> words. Keys may carry a `+` suffix for the form
    /// used when more follows (`100+` "one hundred and…"), an `x` suffix
    /// for the form used before a scale word (`2x`), or an `h` suffix for
    /// the form of a rest after a hundred (`20h`).
    table: HashMap<String, Vec<String>>,
    /// Scale words from the largest down: (value, word, form when more follows).
    scales: Vec<(u64, String, String)>,
    /// Indian grouping (thousand, lakh, crore) instead of thousand, million.
    pub indian: bool,
    /// Join "N hundred" into one output word (two stresses).
    pub glue_hundred: bool,
    /// Say "one" before a scale word.
    pub one_before_scale: bool,
    /// Tens and units make one word ("twenty" + "five" read as one).
    pub join_tens: bool,
    /// Words of a number after its first carry secondary stress at most.
    pub demote_tail: bool,
    /// Word(s) for the decimal point, minus and percent.
    pub point: Vec<String>,
    /// Word(s) for a leading minus sign (empty: the sign is not read).
    pub minus: Vec<String>,
    /// Word(s) for `%` after the number.
    pub percent: Vec<String>,
    /// `currency € ¤euro#`: the word(s) said for a currency sign, where the
    /// sign stands (compatible mode) or after the amount (improved mode).
    pub currency: Vec<(char, Vec<String>)>,
    /// `decimal comma`: the comma is the decimal sign and the full stop
    /// groups thousands.
    pub decimal_comma: bool,
    /// `swap-tens und`: units are said before the tens, the tens as one
    /// word with this linking word in front ("ein" "undzwanzig").
    pub swap_tens: Option<String>,
    /// `year-hundreds yes`: 1100 to 1999 are counted in hundreds
    /// ("nineteen hundred eighty-four").
    pub year_hundreds: bool,
    /// `tail-secondary no`: the words of a number after its first lose
    /// their secondary stresses.
    pub tail_plain: bool,
    /// `many1000 tysięcy`: the scale word after counts that do not end in
    /// 2, 3 or 4 (or that end in 12, 13, 14), as in the Slavic languages.
    many: HashMap<u64, String>,
    /// `tail-stress no`: the words of a number after its first have no
    /// stress.
    pub tail_unstressed: bool,
    /// `and-below-hundred e`: this word stands between a thousand or a
    /// million and a rest below one hundred ("dois mil e um").
    pub and_small: Option<String>,
    /// `group-stress 10000`: in a number of at least this value, the
    /// first word of each group of three digits keeps its stress where
    /// `tail-stress no` or the like would take it (Hungarian). 0: off.
    pub group_stress: u64,
    /// `group-stress-spaced 1000`: the same from this value for a number
    /// written with a space between its groups ("1 500", French). 0: off.
    pub group_stress_spaced: u64,
    /// `stress last`: only the last word of a number keeps its stress.
    pub stress_last: bool,
    /// `fraction number`: the digits after the decimal sign are read as
    /// one number.
    pub fraction_number: bool,
    /// `group space`: a space separates groups of three digits ("10 000").
    pub group_space: bool,
    /// `group space-improved`: the same in improved mode only (the
    /// reference voice reads the groups as numbers of their own).
    pub group_space_improved: bool,
    /// `roman off`: Roman numerals are read as ordinary words in
    /// compatible mode.
    pub roman_off: bool,
    /// `roman max N`: the largest Roman numeral read as a number in
    /// compatible mode (0: the default, 49). From 100 the letters C, D and M
    /// count as well.
    pub roman_max: u64,
    /// `roman-skip VI CD`: spellings that are words, not numerals.
    pub roman_skip: Vec<String>,
    /// `roman-before w` / `roman-after w`: word(s) said with a Roman
    /// numeral in compatible mode ("römisch vier", "quatre romain").
    pub roman_before: Vec<String>,
    /// See `roman_before`.
    pub roman_after: Vec<String>,
    /// `ordN word`: the ordinal a Roman numeral of value N is read as.
    ordinals: HashMap<u64, String>,
    /// `romanN word`: the word a Roman numeral of value N is read as in
    /// compatible mode, where it is not the cardinal.
    roman_forms: HashMap<u64, String>,
    /// Rules for the written number words of normalise-only mode; when
    /// set, they replace the table above.
    pub speller: Option<std::sync::Arc<crate::rbnf::Rbnf>>,
    /// `written-hyphen keep`: hyphens inside number words stay in their
    /// written form ("vingt-deux"); otherwise they become spaces.
    pub written_hyphen: bool,
    /// `written-glue tausend`: written number words joined to the number
    /// word before them ("zweitausend").
    pub written_glue: Vec<String>,
    /// `written-glue-after hundert`: the number word after one of these
    /// is joined to it ("dreihundertzwölf").
    pub written_glue_after: Vec<String>,
    /// `written-plural million millions`: pairs of a scale word and its
    /// plural, written after a count other than one.
    written_plural: Vec<(String, String)>,
    /// `[numbers.written]`: the written form of a number word, where it is
    /// not the word itself without its marks (normalise-only mode).
    written: HashMap<String, String>,
    /// `[numbers.written]` lines `*FROM TO`: replacements inside a word
    /// (`_` in TO is a space; `FROM$` only at the end of the word).
    written_replace: Vec<(String, String)>,
    /// Every word of the table, for [`Numbers::written`].
    tokens: std::collections::HashSet<String>,
}

impl Numbers {
    /// Read the `[numbers]` section.
    pub fn parse(f: &LangFile) -> Numbers {
        let mut n = Numbers {
            one_before_scale: true,
            ..Numbers::default()
        };
        for l in f.lines("numbers") {
            let mut it = l.splitn(2, char::is_whitespace);
            let (Some(k), Some(v)) = (it.next(), it.next()) else {
                continue;
            };
            let words: Vec<String> = v.split_whitespace().map(str::to_string).collect();
            match k {
                "point" => n.point = words,
                "currency" => {
                    if let Some((sign, rest)) = words.split_first() {
                        if let Some(c) = sign.chars().next() {
                            n.currency.push((c, rest.to_vec()));
                        }
                    }
                }
                "minus" => n.minus = words,
                "percent" => n.percent = words,
                "grouping" => n.indian = v.trim() == "indian",
                "fraction" => n.fraction_number = v.trim() == "number",
                "stress" => n.stress_last = v.trim() == "last",
                "year-hundreds" => n.year_hundreds = v.trim() == "yes",
                "decimal" => n.decimal_comma = v.trim() == "comma",
                "group" => match v.trim() {
                    "space" => n.group_space = true,
                    "space-improved" => n.group_space_improved = true,
                    _ => {}
                },
                "roman" => match v.trim() {
                    "off" => n.roman_off = true,
                    m => {
                        n.roman_max = m
                            .strip_prefix("max")
                            .map_or(0, |x| x.trim().parse().unwrap_or(0))
                    }
                },
                "roman-skip" => n.roman_skip = words,
                "roman-before" => n.roman_before = words,
                "roman-after" => n.roman_after = words,
                k if k.starts_with("ord") && k[3..].parse::<u64>().is_ok() => {
                    n.ordinals
                        .insert(k[3..].parse().unwrap_or(0), v.trim().to_string());
                }
                k if k.starts_with("roman") && k[5..].parse::<u64>().is_ok() => {
                    n.roman_forms
                        .insert(k[5..].parse().unwrap_or(0), v.trim().to_string());
                }
                "swap-tens" => n.swap_tens = Some(v.trim().to_string()),
                "tail-secondary" => n.tail_plain = v.trim() == "no",
                k if k.starts_with("many") && k[4..].parse::<u64>().is_ok() => {
                    n.many
                        .insert(k[4..].parse().unwrap_or(0), v.trim().to_string());
                }
                "tail-stress" => n.tail_unstressed = v.trim() == "no",
                "group-stress" => n.group_stress = v.trim().parse().unwrap_or(0),
                "group-stress-spaced" => n.group_stress_spaced = v.trim().parse().unwrap_or(0),
                "and-below-hundred" => n.and_small = Some(v.trim().to_string()),
                "glue-hundred" => n.glue_hundred = v.trim() == "yes",
                "written-hyphen" => n.written_hyphen = v.trim() == "keep",
                "written-glue" => n.written_glue = words,
                "written-glue-after" => n.written_glue_after = words,
                "written-plural" => {
                    n.written_plural = words
                        .chunks(2)
                        .filter(|p| p.len() == 2)
                        .map(|p| (p[0].clone(), p[1].clone()))
                        .collect()
                }
                "one-before-scale" => n.one_before_scale = v.trim() != "no",
                "join-tens" => n.join_tens = v.trim() == "yes",
                "demote-after-first" => n.demote_tail = v.trim() == "yes",
                k if k.starts_with("scale") => {
                    // "scale1000 हज़ार"; "scale1000+ …" is the form used when more follows.
                    let (num, more) = match k[5..].strip_suffix('+') {
                        Some(x) => (x, true),
                        None => (&k[5..], false),
                    };
                    if let Ok(val) = num.parse::<u64>() {
                        let w = v.trim().to_string();
                        match n.scales.iter_mut().find(|s| s.0 == val) {
                            Some(s) if more => s.2 = w,
                            Some(s) => s.1 = w,
                            None => n.scales.push((val, w.clone(), w)),
                        }
                    }
                }
                _ => {
                    n.table.insert(k.to_string(), words);
                }
            }
        }
        n.scales.sort_by_key(|s| std::cmp::Reverse(s.0));
        for l in f.lines("numbers.written") {
            let Some((k, v)) = l.split_once(char::is_whitespace) else {
                continue;
            };
            match k.strip_prefix('*') {
                Some(from) => n
                    .written_replace
                    .push((from.to_string(), v.trim().replace('_', " "))),
                None => {
                    n.written.insert(k.to_string(), v.trim().to_string());
                }
            }
        }
        let words: Vec<String> = n
            .table
            .values()
            .flatten()
            .chain(n.scales.iter().flat_map(|s| [&s.1, &s.2]))
            .chain(n.many.values())
            .cloned()
            .collect();
        for w in words {
            n.tokens.insert(w.trim_end_matches('#').to_string());
        }
        n
    }

    /// True when groups of three digits after a space continue a number
    /// ("1 500") in this mode.
    pub fn joins_groups(&self, compat: bool) -> bool {
        self.group_space || (!compat && self.group_space_improved)
    }

    /// True when `word` is the hundred of `glue-hundred` (it joins the
    /// word before it in writing too).
    pub fn glued_hundred(&self, word: &str) -> bool {
        self.glue_hundred
            && self
                .scales
                .iter()
                .any(|s| s.0 == 100 && (s.1 == word || s.2 == word))
    }

    /// True for a word of the number table (not the words for the decimal
    /// sign, minus, percent or a currency).
    pub fn is_number_word(&self, word: &str) -> bool {
        self.tokens.contains(word.trim_end_matches('#'))
    }

    /// The plural of a written scale word after the written count `prev`
    /// (`written-plural`), when the count is not one.
    pub fn written_plural(&self, word: &str, prev: &str) -> Option<&str> {
        let (_, plural) = self.written_plural.iter().find(|(w, _)| w == word)?;
        let one = self
            .table
            .get("1")
            .and_then(|w| w.first())
            .map(|w| self.written(w));
        (one.as_deref() != Some(prev)).then_some(plural.as_str())
    }

    /// The `*FROM TO` replacements of `[numbers.written]` applied to `w`.
    pub fn replace_written(&self, w: &str) -> String {
        let mut out = w.to_string();
        for (from, to) in &self.written_replace {
            match from.strip_suffix('$') {
                // `*FROM$ TO`: only at the end of the word.
                Some(end) => {
                    if let Some(stem) = out.strip_suffix(end) {
                        out = format!("{stem}{to}");
                    }
                }
                None => out = out.replace(from.as_str(), to),
            }
        }
        out
    }

    /// The written form of a number word of this table, for normalise-only
    /// mode: the word without the marks that tell its reading apart (`#`,
    /// `num`, and the suffixes of variant forms `v2`, `2`, `x`, `-h` when
    /// the word without them is a number word too), as `[numbers.written]`
    /// says. A word that is not a number word comes back unchanged.
    pub fn written(&self, word: &str) -> String {
        if let Some(w) = self.written.get(word) {
            return w.clone();
        }
        let marked = word.contains(['#', '¤']) || word.ends_with("num");
        // `¤` marks the word of a currency sign.
        let mut base = word
            .split('#')
            .next()
            .unwrap_or(word)
            .trim_start_matches('¤');
        while marked || self.tokens.contains(base) {
            if let Some(b) = base.strip_suffix("num").filter(|b| !b.is_empty()) {
                base = b;
                continue;
            }
            let next = ["v2", "-h", "2", "x"].iter().find_map(|suf| {
                base.strip_suffix(suf)
                    .filter(|b| !b.is_empty() && self.tokens.contains(*b))
            });
            match next {
                Some(b) => base = b,
                None => break,
            }
        }
        if let Some(w) = self.written.get(base) {
            return w.clone();
        }
        let mut out = self.replace_written(base);
        if !self.written_hyphen {
            out = out.replace('-', " ");
        }
        out.trim().to_string()
    }

    /// The same numbers with the written number words of `rules`
    /// (normalise-only mode).
    pub fn with_speller(&self, rules: &str) -> Numbers {
        let r = crate::rbnf::Rbnf::parse(rules);
        let words = |s: &str, old: &Vec<String>| {
            if s.is_empty() {
                old.clone()
            } else {
                s.split_whitespace().map(str::to_string).collect()
            }
        };
        Numbers {
            point: words(&r.point, &self.point),
            minus: words(&r.minus, &self.minus),
            percent: words(&r.percent, &self.percent),
            speller: Some(std::sync::Arc::new(r)),
            ..self.clone()
        }
    }

    /// True when the language has number words.
    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    fn get(&self, n: u64, more: bool) -> Option<&Vec<String>> {
        if more {
            if let Some(w) = self.table.get(&format!("{n}+")) {
                return Some(w);
            }
        }
        self.table.get(&n.to_string())
    }

    /// The ordinal word a Roman numeral of this value is read as, if the
    /// language reads them so.
    pub fn ordinal(&self, n: u64, compat: bool) -> Option<&str> {
        self.roman_forms
            .get(&n)
            .filter(|_| compat)
            .or_else(|| self.ordinals.get(&n))
            .map(String::as_str)
    }

    /// Words for `n`; each entry is `(word, glued to the previous one)`.
    pub fn cardinal(&self, n: u64, out: &mut Vec<(String, bool)>) {
        if let Some(r) = &self.speller {
            let words = r.spell(n);
            out.extend(words.split(' ').map(|w| (w.to_string(), false)));
            return;
        }
        if let Some(w) = self.get(n, false) {
            out.extend(w.iter().map(|x| (x.clone(), false)));
            return;
        }
        if self.year_hundreds && (1100..2000).contains(&n) && n % 1000 >= 100 {
            if let Some((_, word, word_more)) = self.scales.iter().find(|s| s.0 == 100) {
                let r = n % 100;
                self.cardinal(n / 100, out);
                out.push((
                    if r > 0 { word_more } else { word }.clone(),
                    self.glue_hundred,
                ));
                if r > 0 {
                    self.cardinal(r, out);
                }
                return;
            }
        }
        for (val, word, word_more) in &self.scales {
            if n >= *val {
                let (q, r) = (n / val, n % val);
                // The form before a rest, else the listed multiple itself.
                let listed = if r > 0 && q < 1000 {
                    self.get(q * val, true)
                } else {
                    None
                };
                if let Some(w) = listed {
                    // A listed multiple (e.g. "two hundred" as one word).
                    out.extend(w.iter().map(|x| (x.clone(), false)));
                } else {
                    if q > 1 || self.one_before_scale {
                        // A combining form before a scale word ("2x" in the file).
                        match self.table.get(&format!("{q}x")) {
                            Some(w) => out.extend(w.iter().map(|x| (x.clone(), false))),
                            None => self.cardinal(q, out),
                        }
                    }
                    let glue = *val == 100 && self.glue_hundred && (q > 1 || self.one_before_scale);
                    let few = (2..=4).contains(&(q % 10)) && !(12..=14).contains(&(q % 100));
                    match self.many.get(val).filter(|_| !few && q > 1) {
                        Some(m) => out.push((m.clone(), glue)),
                        None => out.push((if r > 0 { word_more } else { word }.clone(), glue)),
                    }
                }
                if r > 0 {
                    if let (Some(and), true) = (&self.and_small, *val >= 1000 && r < 100) {
                        out.push((and.clone(), false));
                    }
                    // A rest's own form after a hundred ("20h" in the file).
                    match self.table.get(&format!("{r}h")).filter(|_| *val == 100) {
                        Some(w) => out.extend(w.iter().map(|x| (x.clone(), false))),
                        None => self.cardinal(r, out),
                    }
                }
                return;
            }
        }
        // Below the smallest scale and not listed: tens + units.
        if n < 100 {
            let (t, u) = (n / 10 * 10, n % 10);
            if let (Some(link), true, Some(tens)) = (&self.swap_tens, u > 0, self.get(t, false)) {
                // The unit's own form before the tens ("1u" in the file).
                match self.table.get(&format!("{u}u")) {
                    Some(w) => out.extend(w.iter().map(|x| (x.clone(), false))),
                    None => self.cardinal(u, out),
                }
                out.push((format!("{link}{}", tens.concat()), false));
                return;
            }
            if let Some(w) = self.get(t, u > 0) {
                let start = out.len();
                out.extend(w.iter().map(|x| (x.clone(), false)));
                if u > 0 {
                    self.cardinal(u, out);
                }
                if self.join_tens && out.len() > start + 1 {
                    let joined: String = out.drain(start..).map(|x| x.0).collect();
                    out.push((joined, false));
                }
                return;
            }
        }
        // Fall back to digits.
        for d in n.to_string().bytes() {
            if let Some(w) = self.get(u64::from(d - b'0'), false) {
                out.extend(w.iter().map(|x| (x.clone(), false)));
            }
        }
    }

    /// The places in the words of [`Numbers::cardinal`] for `n` where a
    /// later group of three digits starts, for languages with
    /// `group-stress` and numbers it applies to; `spaced`: the number was
    /// written with spaces between its groups.
    pub fn group_starts(&self, n: u64, spaced: bool) -> Vec<usize> {
        let mut starts = Vec::new();
        let applies = |from: u64| from > 0 && n >= from;
        let on = applies(self.group_stress) || (spaced && applies(self.group_stress_spaced));
        if !on || self.speller.is_some() {
            return starts;
        }
        let count = |v: u64| {
            let mut w = Vec::new();
            self.cardinal(v, &mut w);
            w.len()
        };
        let total = count(n);
        for unit in [1_000_000_000u64, 1_000_000, 1_000] {
            let rest = n % unit;
            if rest > 0 && rest < n {
                let at = total.saturating_sub(count(rest));
                if at > 0 && !starts.contains(&at) {
                    starts.push(at);
                }
            }
        }
        starts
    }

    /// Words for a digit string, one per digit.
    pub fn digits(&self, s: &str, out: &mut Vec<(String, bool)>) {
        for d in s.bytes().filter(u8::is_ascii_digit) {
            if let Some(r) = &self.speller {
                out.push((r.spell(u64::from(d - b'0')), false));
                continue;
            }
            if let Some(w) = self.get(u64::from(d - b'0'), false) {
                out.extend(w.iter().map(|x| (x.clone(), false)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_and_numbers() {
        let f = LangFile::parse(
            "[meta]\ntag = xx ; comment\n[numbers]\n1 one\n2 two\n20 twenty\n\
             scale100 hundred\nscale1000 thousand\n",
        );
        assert_eq!(f.meta()["tag"], "xx");
        let n = Numbers::parse(&f);
        let mut out = Vec::new();
        n.cardinal(2122, &mut out);
        let w: Vec<&str> = out.iter().map(|x| x.0.as_str()).collect();
        assert_eq!(w, ["two", "thousand", "one", "hundred", "twenty", "two"]);
    }
}
