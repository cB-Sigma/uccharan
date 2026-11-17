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

/// How the digits after a decimal sign are read (see `Numbers::fraction`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Fraction {
    /// Digit by digit.
    #[default]
    Digits,
    /// Leading zeros as digits, then one number of up to this many digits.
    Number(usize),
    /// One number of up to four digits not starting with a zero.
    Romanian,
    /// One number with a suffix word; `true`: the suffix only after zeros.
    Suffix(bool),
}

/// Ordinal number words (`ordinal-…` lines of `[numbers]`), from the
/// language's ordinal rule file (`lang/numbers/ord-<tag>.txt`).
#[derive(Clone, Debug)]
pub struct Ordinals {
    rules: std::sync::Arc<crate::rbnf::Rbnf>,
    /// `ordinal-dot SET [compat]`: a number with a full stop after it is an
    /// ordinal (German "3."), read with this rule set; `compat`: the
    /// reference reads it so too.
    dot: Option<(String, bool)>,
    /// `ordinal-dot-after SET w…`: the rule set after these words ("am").
    dot_after: Vec<(String, Vec<String>)>,
    /// `ordinal-suffix SUF SET [compat]`: a number with this ending is an
    /// ordinal ("1er", "2ª").
    suffixes: Vec<(String, String, bool)>,
    /// Suffixes marked `compat-said`: the reference reads the ordinal and
    /// then the ending as a word of its own ("1e" eerste e).
    said: Vec<String>,
}

impl Ordinals {
    /// The ordinal words of `n` by the rule set `set`.
    pub fn words(&self, set: &str, n: u64) -> Option<String> {
        self.rules.spell_in(&format!("%{set}"), n)
    }

    /// The rule set for a number with a full stop after the word `prev`.
    fn dot_set(&self, prev: Option<&str>) -> Option<&str> {
        let (set, _) = self.dot.as_ref()?;
        let prev = prev.map(str::to_lowercase);
        for (s, after) in &self.dot_after {
            if prev.as_ref().is_some_and(|p| after.contains(p)) {
                return Some(s);
            }
        }
        Some(set)
    }

    /// The ordinal words of `n` in the form `ordinal-dot-after` gives the
    /// word `prev`, if it is one of them.
    pub fn after_words(&self, prev: Option<&str>, n: u64) -> Option<String> {
        let prev = prev?.to_lowercase();
        let (set, _) = self.dot_after.iter().find(|(_, w)| w.contains(&prev))?;
        self.words(set, n)
    }

    /// Words that make a number with a full stop an ordinal when they come
    /// before it.
    pub fn context_words(&self) -> impl Iterator<Item = &String> {
        self.dot_after.iter().flat_map(|(_, w)| w.iter())
    }
}

/// How clock times are read (improved mode and normalise-only mode):
/// `time` lines of `[numbers]`.
#[derive(Clone, Debug, Default)]
pub struct Times {
    /// `time h colon dot dot-context`: the written forms read as times:
    /// "10h30" / "10 h 30" / "10h", "10:30", "10.30", and "10.30" only after
    /// a word of `time-context` or before a word of `time-after`.
    pub forms: Vec<String>,
    /// `time-hour heure heures`: the word for "hour" (one, more).
    pub hour: Option<(String, String)>,
    /// `time-hour-always yes`: the hour word is said with every form, not
    /// only with "h" and with `time-after`.
    pub hour_always: bool,
    /// `time-and e`: the word between the hour word and the minutes.
    pub and: Vec<String>,
    /// `time-hours1 une`: the words for an hour count (feminine forms).
    pub hours: HashMap<u64, String>,
    /// `time-context um gegen`: words before a time (lower case).
    pub context: Vec<String>,
    /// `time-after Uhr`: words written after a time, said as its hour word.
    pub after: Vec<String>,
    /// `time-hour-minutes बजियै`: the hour word said when minutes follow
    /// (Dogri दस बजियै त्रीह् मिंट), in place of the `time-hour` word.
    pub hour_minutes: Vec<String>,
    /// `time-minute मिंट`: the word said after the minutes.
    pub minute: Vec<String>,
}

impl Times {
    /// True when the written form `f` is read as a time.
    pub fn has(&self, f: &str) -> bool {
        self.forms.iter().any(|x| x == f)
    }

    /// The `time-after` word `w` (any case), as the file writes it.
    pub fn after_word(&self, w: &str) -> Option<&str> {
        let w = w.to_lowercase();
        self.after
            .iter()
            .find(|a| a.to_lowercase() == w)
            .map(String::as_str)
    }
}

/// Plural rules for currency words (`plural …`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Plural {
    /// One: 1; plural: the rest.
    #[default]
    One,
    /// One: 0 and 1 (French).
    OneZero,
    /// One: 1; plural: ending in 2-4 but not 12-14; many: the rest (Polish).
    Slavic,
    /// One: 1; plural: 2-4; many: the rest (Czech).
    Czech,
}

/// Currency words that agree with the amount (improved mode and
/// normalise-only mode).
#[derive(Clone, Debug, Default)]
pub struct MoneyWords {
    /// `currency-plural € euros`: the word(s) after an amount that is not one.
    pub plural: Vec<(char, Vec<String>)>,
    /// `currency-singular $ dollaro`: the word(s) after the amount one,
    /// where they are not the `currency` word.
    pub singular: Vec<(char, Vec<String>)>,
    /// `currency-many $ dolarów`: the word(s) after a "many" amount
    /// (`plural slavic` / `plural czech`).
    pub many: Vec<(char, Vec<String>)>,
    /// `currency-count € 1 un`: the words for the amount N before this
    /// currency's word (`*` for every currency).
    pub count: Vec<(char, u64, Vec<String>)>,
    /// `currency-of de`: a word between the amount and the currency word
    /// after amounts ending in 00 or 20-99 (Romanian "douăzeci de euro").
    pub of: Vec<String>,
    /// `plural …`: which amounts take the plural.
    pub rule: Plural,
}

impl MoneyWords {
    /// The words of `sign` after the whole amount `n`: the singular, the
    /// plural or the many form, or `None` for the `currency` word.
    pub fn word(&self, sign: char, n: u64) -> Option<&Vec<String>> {
        fn find(v: &[(char, Vec<String>)], sign: char) -> Option<&Vec<String>> {
            v.iter().find(|x| x.0 == sign).map(|x| &x.1)
        }
        let plural = find(&self.plural, sign);
        let one = find(&self.singular, sign);
        match self.rule {
            Plural::One if n == 1 => one,
            Plural::OneZero if n <= 1 => one,
            Plural::Slavic | Plural::Czech if n == 1 => one,
            Plural::Slavic if (2..=4).contains(&(n % 10)) && !(12..=14).contains(&(n % 100)) => {
                plural
            }
            Plural::Czech if (2..=4).contains(&n) => plural,
            Plural::Slavic | Plural::Czech => find(&self.many, sign).or(plural),
            _ => plural,
        }
    }

    /// The words for the amount `n` before the word of `sign`, where the
    /// file gives them (`currency-count`).
    pub fn count_words(&self, sign: char, n: u64) -> Option<&Vec<String>> {
        self.count
            .iter()
            .find(|x| x.1 == n && x.0 == sign)
            .or_else(|| self.count.iter().find(|x| x.1 == n && x.0 == '*'))
            .map(|x| &x.2)
    }

    /// True when `currency-of` goes between the amount `n` and its word.
    pub fn of_after(&self, n: u64) -> bool {
        !self.of.is_empty() && n > 0 && (n % 100 == 0 || n % 100 >= 20)
    }
}

/// How dates are read (`[dates]`).
#[derive(Clone, Debug, Default)]
pub struct Dates {
    /// Month names, January first (`month1` … `month12`).
    pub months: Vec<String>,
    /// `pattern D de M de Y`: the words in order (D day, M month, Y year).
    pub pattern: Vec<String>,
    /// `day ordinal SET`, `day cardinal` or `day possessive SET`
    /// (Hungarian: the ordinal with -a/-e).
    pub day: (String, String),
    /// `day1 premier`: the word for the first day of a month.
    pub day1: Option<String>,
    /// `year hundreds` (1100-1999 counted in hundreds), `year ordinal SET`
    /// (Polish), else the cardinal.
    pub year: (String, String),
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
    /// `apostrophe keep`: an apostrophe between two letters stays in the
    /// word (Dogri writes a lost h with it: र’लाया); otherwise it is dropped.
    pub keep_apostrophe: bool,
    /// Word(s) for `%` after the number.
    pub percent: Vec<String>,
    /// `percent-before w`: word(s) for `%` said before the number instead
    /// (the reference's Sinhala voice).
    pub percent_before: Vec<String>,
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
    /// one number (zeros first as digits, up to five digits); `fraction
    /// two`: the same up to two digits; `fraction romanian`: one number of
    /// up to four digits that does not start with a zero; `fraction suffix`:
    /// one number with the word of `fraction-suffixN` (N digits) after it,
    /// glued ("öt tized"); `fraction suffix-zero`: one number, with the
    /// suffix word only after leading zeros (Italian "zero millesimi").
    pub fraction: Fraction,
    /// `fraction-suffix3 millesimi`: see `fraction`.
    pub fraction_suffix: HashMap<usize, String>,
    /// `dot-word punkt`: the word said for a full stop between digits that
    /// does not separate groups of three digits ("2.5", compatible mode).
    pub dot_word: Vec<String>,
    /// `dot-groups no`: a full stop never separates groups of three digits
    /// in compatible mode ("1.000" one, dot-word, zero zero zero).
    pub dot_no_groups: bool,
    /// `minus-compat no`: the minus sign is not read in compatible mode.
    pub minus_silent_compat: bool,
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
    /// `written-drop-s cents vingts`: written words that lose their final
    /// `s` when another number word follows that is not a scale word of
    /// `written-plural` ("deux cent mille", "deux cents millions").
    pub written_drop_s: Vec<String>,
    /// `[numbers.written]`: the written form of a number word, where it is
    /// not the word itself without its marks (normalise-only mode).
    written: HashMap<String, String>,
    /// `[numbers.written]` lines `*FROM TO`: replacements inside a word
    /// (`_` in TO is a space; `FROM$` only at the end of the word).
    written_replace: Vec<(String, String)>,
    /// Ordinal words, where the language has an ordinal rule file.
    pub ordinal_words: Option<Ordinals>,
    /// Date readings (`[dates]`).
    pub dates: Option<Dates>,
    /// Clock times (`time…` lines of `[numbers]`).
    pub times: Option<Times>,
    /// Agreement of a currency word with the amount (`currency-…` lines).
    pub money: MoneyWords,
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
                "currency-plural" | "currency-many" | "currency-singular" => {
                    if let Some((sign, rest)) = words.split_first() {
                        if let Some(c) = sign.chars().next() {
                            let v = match k {
                                "currency-plural" => &mut n.money.plural,
                                "currency-many" => &mut n.money.many,
                                _ => &mut n.money.singular,
                            };
                            v.push((c, rest.to_vec()));
                        }
                    }
                }
                "currency-count" => {
                    if let [sign, count, rest @ ..] = words.as_slice() {
                        if let (Some(c), Ok(k)) = (sign.chars().next(), count.parse::<u64>()) {
                            n.money.count.push((c, k, rest.to_vec()));
                        }
                    }
                }
                "currency-of" => n.money.of = words,
                "plural" => {
                    n.money.rule = match v.trim() {
                        "one-zero" => Plural::OneZero,
                        "slavic" => Plural::Slavic,
                        "czech" => Plural::Czech,
                        _ => Plural::One,
                    }
                }
                k if k == "time" || k.starts_with("time-") => {
                    let t = n.times.get_or_insert_with(Times::default);
                    match k {
                        "time" => t.forms = words,
                        "time-hour" => {
                            if let Some(one) = words.first() {
                                let more = words.get(1).unwrap_or(one);
                                t.hour = Some((one.clone(), more.clone()));
                            }
                        }
                        "time-hour-always" => t.hour_always = v.trim() == "yes",
                        "time-and" => t.and = words,
                        "time-context" => {
                            t.context = words.iter().map(|w| w.to_lowercase()).collect()
                        }
                        "time-after" => t.after = words,
                        "time-hour-minutes" => t.hour_minutes = words,
                        "time-minute" => t.minute = words,
                        k => {
                            if let Ok(h) = k["time-hours".len().min(k.len())..].parse::<u64>() {
                                if k.starts_with("time-hours") {
                                    t.hours.insert(h, v.trim().to_string());
                                }
                            }
                        }
                    }
                }
                "minus" => n.minus = words,
                "apostrophe" => n.keep_apostrophe = v.trim() == "keep",
                "percent" => n.percent = words,
                "percent-before" => n.percent_before = words,
                "grouping" => n.indian = v.trim() == "indian",
                "fraction" => {
                    n.fraction = match v.trim() {
                        "number" => Fraction::Number(5),
                        "two" => Fraction::Number(2),
                        "romanian" => Fraction::Romanian,
                        "suffix" => Fraction::Suffix(false),
                        "suffix-zero" => Fraction::Suffix(true),
                        _ => Fraction::Digits,
                    }
                }
                k if k.starts_with("fraction-suffix") => {
                    if let Ok(d) = k["fraction-suffix".len()..].parse::<usize>() {
                        n.fraction_suffix.insert(d, v.trim().to_string());
                    }
                }
                "dot-word" => n.dot_word = words,
                "dot-groups" => n.dot_no_groups = v.trim() == "no",
                "minus-compat" => n.minus_silent_compat = v.trim() == "no",
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
                "written-drop-s" => n.written_drop_s = words,
                "ordinal-dot" | "ordinal-dot-after" | "ordinal-suffix" => {
                    let tag = f.meta().get("tag").cloned().unwrap_or_default();
                    let Some(rules) = crate::rbnf::ordinal_rules_for(&tag) else {
                        continue;
                    };
                    let o = n.ordinal_words.get_or_insert_with(|| Ordinals {
                        rules: std::sync::Arc::new(crate::rbnf::Rbnf::parse(rules)),
                        dot: None,
                        dot_after: Vec::new(),
                        suffixes: Vec::new(),
                        said: Vec::new(),
                    });
                    let said = words.last().is_some_and(|w| w == "compat-said");
                    let compat = said || words.last().is_some_and(|w| w == "compat");
                    if said {
                        o.said.push(words[0].clone());
                    }
                    match (k, words.as_slice()) {
                        ("ordinal-dot", [set, ..]) => o.dot = Some((set.clone(), compat)),
                        ("ordinal-dot-after", [set, rest @ ..]) => o
                            .dot_after
                            .push((set.clone(), rest.iter().map(|w| w.to_lowercase()).collect())),
                        ("ordinal-suffix", [suf, set, ..]) => {
                            o.suffixes.push((suf.clone(), set.clone(), compat))
                        }
                        _ => {}
                    }
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
        let date_lines = f.lines("dates");
        if !date_lines.is_empty() {
            let mut d = Dates {
                months: vec![String::new(); 12],
                day: ("cardinal".into(), String::new()),
                year: ("cardinal".into(), String::new()),
                ..Dates::default()
            };
            for l in date_lines {
                let Some((k, v)) = l.split_once(char::is_whitespace) else {
                    continue;
                };
                let v = v.trim();
                let two = |v: &str| {
                    let (a, b) = v.split_once(' ').unwrap_or((v, ""));
                    (a.to_string(), b.trim().to_string())
                };
                match k {
                    "pattern" => d.pattern = v.split_whitespace().map(str::to_string).collect(),
                    "day" => d.day = two(v),
                    "day1" => d.day1 = Some(v.to_string()),
                    "year" => d.year = two(v),
                    k => {
                        if let Some(m) = k
                            .strip_prefix("month")
                            .and_then(|m| m.parse::<usize>().ok())
                        {
                            if (1..=12).contains(&m) {
                                d.months[m - 1] = v.replace('_', " ");
                            }
                        }
                    }
                }
            }
            if d.pattern.is_empty() {
                d.pattern = vec!["D".into(), "M".into(), "Y".into()];
            }
            if d.months.iter().all(|m| !m.is_empty()) {
                n.dates = Some(d);
            }
        }
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

    /// True when the written word `w` is a scale word of `written-plural`
    /// (singular or plural).
    pub fn is_plural_scale(&self, w: &str) -> bool {
        self.written_plural.iter().any(|(a, b)| a == w || b == w)
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
            percent_before: Vec::new(),
            point: words(&r.point, &self.point),
            minus: words(&r.minus, &self.minus),
            percent: words(&r.percent, &self.percent),
            speller: Some(std::sync::Arc::new(r)),
            ..self.clone()
        }
    }

    /// The ordinal words of a chunk written as a number with an ordinal
    /// ending ("1er", "2ª", "3:e"); `compat`: only those the reference reads
    /// as ordinals.
    pub fn suffix_ordinal(&self, text: &str, compat: bool) -> Option<String> {
        let o = self.ordinal_words.as_ref()?;
        let digits = text.len() - text.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        if digits == 0 || digits > 4 {
            return None;
        }
        let n: u64 = text[..digits].parse().ok()?;
        let rest = &text[digits..];
        let (_, set, _) = o
            .suffixes
            .iter()
            .find(|(suf, _, c)| suf.as_str() == rest && (*c || !compat))?;
        let w = o.words(set, n)?;
        if compat && o.said.iter().any(|x| x == rest) {
            return Some(format!("{w} {rest}"));
        }
        Some(w)
    }

    /// The ordinal words of the number `n` written with a full stop after
    /// it, after the word `prev`.
    pub fn dot_ordinal(&self, n: u64, prev: Option<&str>, compat: bool) -> Option<String> {
        let o = self.ordinal_words.as_ref()?;
        if compat && !o.dot.as_ref().is_some_and(|d| d.1) {
            return None;
        }
        // The reference's own ordinal words, where the file lists them.
        if let Some(w) = self.ordinals.get(&n).filter(|_| compat) {
            return Some(w.clone());
        }
        o.words(o.dot_set(prev)?, n)
    }

    /// True when the language reads a number with a full stop after it as
    /// an ordinal.
    pub fn has_dot_ordinals(&self) -> bool {
        self.ordinal_words.as_ref().is_some_and(|o| o.dot.is_some())
    }

    /// Lower-case words that make a number with a full stop after it an
    /// ordinal when they stand next to it: `ordinal-dot-after` words before
    /// it, month names after it (and before it where dates put the month
    /// first, Hungarian "május 15."); empty when the language has no such
    /// ordinals.
    pub fn ordinal_context(&self) -> crate::text::OrdinalContext {
        let mut c = crate::text::OrdinalContext::default();
        let Some(o) = self.ordinal_words.as_ref().filter(|o| o.dot.is_some()) else {
            return c;
        };
        c.before = o.context_words().cloned().collect();
        if let Some(d) = &self.dates {
            let months = d.months.iter().map(|m| m.to_lowercase());
            let pos = |t: &str| d.pattern.iter().position(|x| x == t);
            if pos("M") < pos("D") {
                c.month_before.extend(months.clone());
            }
            c.after.extend(months);
        }
        c
    }

    /// The year `n` read as a number with 1100-1999 counted in hundreds.
    pub fn cardinal_year(&self, n: u64, out: &mut Vec<(String, bool)>) {
        if self.year_hundreds || !(1100..2000).contains(&n) || n % 1000 < 100 {
            return self.cardinal(n, out);
        }
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
        } else {
            self.cardinal(n, out);
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
