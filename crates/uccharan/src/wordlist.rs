//! Pronunciation word lists: a TSV format (primary), W3C PLS 1.0 import and
//! export, and matching against text.
//!
//! Columns are separated by tabs (shown as `|` here):
//!
//! ```text
//! # uccharan wordlist 1
//! # lang: hi
//! word | lang | ipa | say | match | note
//! Kokoro | en | kəkˈɔːɹoʊ |  |  | product name
//! SBI | hi |  | एस बी आई | case
//! 2G | en |  | two G | exact
//! ```
//!
//! Columns: `word` (a word or phrase; the pattern for `regex` rows), `lang`
//! (`*` for any language, empty for the file default), `ipa` (espeak-style
//! IPA, used by phoneme voices), `say` (the spoken form, used by text voices),
//! `match` (`word`: whole words, any case, the default; `case`: whole words,
//! same case; `exact`: any substring, same case; `regex`) and `note`.
//!
//! Several lists are applied in order: an earlier list wins over a later
//! one for any overlap, and within one list the longest match wins (see
//! [`select`]).

use crate::text::{script_of, Script};
use crate::Lang;
use std::collections::HashMap;

/// How an entry's `word` is matched against text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MatchRule {
    /// Whole words, case-insensitive (the default).
    #[default]
    Word,
    /// Whole words, case-sensitive.
    Case,
    /// Any substring, case-sensitive (for tokens such as `2G`).
    Exact,
    /// `word` is a regular expression (see [`Regex`] for the syntax).
    Regex,
}

impl MatchRule {
    /// The name used in the `match` column.
    pub fn name(self) -> &'static str {
        match self {
            MatchRule::Word => "word",
            MatchRule::Case => "case",
            MatchRule::Exact => "exact",
            MatchRule::Regex => "regex",
        }
    }

    /// Parse a `match` column value.
    pub fn parse(s: &str) -> Option<MatchRule> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "" | "word" | "whole-word" | "nocase" => MatchRule::Word,
            "case" | "word-case" => MatchRule::Case,
            "exact" | "substring" => MatchRule::Exact,
            "regex" | "re" => MatchRule::Regex,
            _ => return None,
        })
    }
}

/// One word-list entry.
#[derive(Clone, Debug)]
pub struct Entry {
    /// The written form (or regular expression).
    pub word: String,
    /// The language the entry applies in (`None`: any language).
    pub lang: Option<Lang>,
    /// Phonemes, espeak-style IPA.
    pub ipa: Option<String>,
    /// Spoken form for text voices (PLS `alias`).
    pub say: Option<String>,
    /// How `word` is matched.
    pub rule: MatchRule,
    /// Free text.
    pub note: String,
    /// Line of the source file (1-based; 0 when built in code).
    pub line: usize,
    regex: Option<Regex>,
}

impl Entry {
    /// A new entry; fails for an invalid regular expression or an entry with
    /// an empty word.
    pub fn new(
        word: &str,
        lang: Option<Lang>,
        ipa: Option<&str>,
        say: Option<&str>,
        rule: MatchRule,
    ) -> Result<Entry, String> {
        let word = word.trim();
        if word.is_empty() {
            return Err("empty word".into());
        }
        let regex = if rule == MatchRule::Regex {
            Some(Regex::new(word)?)
        } else {
            None
        };
        let opt = |s: Option<&str>| s.map(str::trim).filter(|s| !s.is_empty()).map(String::from);
        Ok(Entry {
            word: word.to_string(),
            lang,
            ipa: opt(ipa),
            say: opt(say),
            rule,
            note: String::new(),
            line: 0,
            regex,
        })
    }

    /// True when the entry applies to text of language `lang` (an English
    /// entry also applies to Latin-script words inside other languages; a
    /// Hindi one to Bhojpuri and Maithili).
    pub fn applies(&self, lang: Lang) -> bool {
        match self.lang {
            None => true,
            Some(l) => {
                l == lang
                    || (l.is_english() && lang.is_english())
                    || (l == Lang::Hi && lang.hindi_like())
                    || l.is_english()
            }
        }
    }

    fn rank(&self, lang: Lang) -> u8 {
        match self.lang {
            Some(l) if l == lang => 0,
            Some(l) if l.is_english() && lang.is_english() => 0,
            None => 1,
            _ => 2,
        }
    }
}

/// A parse error with its line number.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordListError {
    /// 1-based line (0 when unknown).
    pub line: usize,
    /// What is wrong.
    pub message: String,
}

impl std::fmt::Display for WordListError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line > 0 {
            write!(f, "line {}: {}", self.line, self.message)
        } else {
            f.write_str(&self.message)
        }
    }
}

impl std::error::Error for WordListError {}

/// A list of entries with an index for matching.
#[derive(Clone, Debug, Default)]
pub struct WordList {
    /// Entries in file order.
    pub entries: Vec<Entry>,
    /// The `# lang:` header (default language of rows without one).
    pub default_lang: Option<Lang>,
    /// Non-fatal problems found while parsing (line, message).
    pub warnings: Vec<(usize, String)>,
    /// Entry indexes by the folded first character of the word; longest first.
    index: HashMap<char, Vec<usize>>,
    regexes: Vec<usize>,
}

const COLUMNS: [&str; 6] = ["word", "lang", "ipa", "say", "match", "note"];

fn err(line: usize, message: impl Into<String>) -> WordListError {
    WordListError {
        line,
        message: message.into(),
    }
}

fn parse_lang(s: &str) -> Result<Option<Lang>, String> {
    let s = s.trim();
    if s.is_empty() || s == "*" || s == "any" {
        return Ok(None);
    }
    Lang::parse(s)
        .map(Some)
        .ok_or_else(|| format!("unknown language {s:?}"))
}

impl WordList {
    /// A list from entries.
    pub fn from_entries(entries: Vec<Entry>) -> WordList {
        let mut w = WordList {
            entries,
            ..WordList::default()
        };
        w.reindex();
        w
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when there are no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn reindex(&mut self) {
        self.index.clear();
        self.regexes.clear();
        for (i, e) in self.entries.iter().enumerate() {
            if e.rule == MatchRule::Regex {
                self.regexes.push(i);
            } else if let Some(c) = fold_chars(&e.word, e.rule == MatchRule::Word).next() {
                self.index.entry(c).or_default().push(i);
            }
        }
        for v in self.index.values_mut() {
            v.sort_by_key(|&i| std::cmp::Reverse(self.entries[i].word.len()));
        }
    }

    /// Parse the TSV format. Rows may give fewer columns than the header.
    pub fn parse_tsv(text: &str) -> Result<WordList, WordListError> {
        let mut list = WordList::default();
        let mut cols: Vec<usize> = (0..COLUMNS.len()).collect();
        for (n, raw) in text.lines().enumerate() {
            let line = n + 1;
            let l = raw.trim_end_matches('\r');
            if l.trim().is_empty() {
                continue;
            }
            if let Some(c) = l.strip_prefix('#') {
                let c = c.trim();
                if let Some(v) = c.strip_prefix("lang:") {
                    list.default_lang = parse_lang(v).map_err(|e| err(line, e))?;
                } else if let Some(v) = c.strip_prefix("alphabet:") {
                    let v = v.trim().to_ascii_lowercase();
                    if v != "ipa" {
                        return Err(err(line, format!("unsupported alphabet {v:?} (only ipa)")));
                    }
                }
                continue;
            }
            let fields: Vec<&str> = l.split('\t').collect();
            if fields[0].trim().eq_ignore_ascii_case("word") && list.entries.is_empty() {
                // Header row: columns may come in any order.
                cols = Vec::new();
                for f in &fields {
                    let name = f.trim().to_ascii_lowercase();
                    let k = COLUMNS
                        .iter()
                        .position(|c| *c == name)
                        .ok_or_else(|| err(line, format!("unknown column {name:?}")))?;
                    cols.push(k);
                }
                continue;
            }
            let mut v: [&str; 6] = [""; 6];
            for (f, &k) in fields.iter().zip(&cols) {
                v[k] = f;
            }
            let lang = if v[1].trim().is_empty() {
                list.default_lang
            } else {
                parse_lang(v[1]).map_err(|e| err(line, e))?
            };
            let rule = MatchRule::parse(v[4])
                .ok_or_else(|| err(line, format!("unknown match rule {:?}", v[4])))?;
            let mut e =
                Entry::new(v[0], lang, Some(v[2]), Some(v[3]), rule).map_err(|m| err(line, m))?;
            e.note = v[5].trim().to_string();
            e.line = line;
            if e.ipa.is_none() && e.say.is_none() {
                list.warnings
                    .push((line, format!("{:?} has neither ipa nor say", e.word)));
            }
            if let (Some(s), Some(l)) = (&e.say, e.lang) {
                if let Some(bad) = s.chars().map(script_of).find(|sc| {
                    !matches!(sc, Script::Other | Script::Digit | Script::Latin)
                        && *sc != l.script()
                }) {
                    list.warnings.push((
                        line,
                        format!("say {s:?} uses a script ({bad:?}) other than {}'s", l.tag()),
                    ));
                }
            }
            list.entries.push(e);
        }
        list.reindex();
        Ok(list)
    }

    /// Parse TSV, PLS (XML) or a JSON array of entry objects, by content.
    pub fn parse_any(text: &str) -> Result<WordList, WordListError> {
        let t = text.trim_start_matches('\u{feff}').trim_start();
        if t.starts_with('<') {
            WordList::parse_pls(t)
        } else if t.starts_with('[') || t.starts_with('{') {
            WordList::parse_json(t)
        } else {
            WordList::parse_tsv(text)
        }
    }

    /// Parse a JSON array of `{"word", "lang"?, "ipa"?, "say"?, "match"?,
    /// "note"?}` objects (or `{"entries": [...]}`).
    pub fn parse_json(text: &str) -> Result<WordList, WordListError> {
        use crate::minijson::Value;
        let v = crate::minijson::parse(text).map_err(|e| err(0, e))?;
        let arr = match &v {
            Value::Arr(a) => a,
            Value::Obj(_) => match v.get("entries") {
                Some(Value::Arr(a)) => a,
                _ => return Err(err(0, "expected an array of entries")),
            },
            _ => return Err(err(0, "expected an array of entries")),
        };
        let mut list = WordList::default();
        for (i, item) in arr.iter().enumerate() {
            let s = |k: &str| item.get(k).and_then(Value::as_str);
            let lang = parse_lang(s("lang").unwrap_or("")).map_err(|e| err(i + 1, e))?;
            let rule = MatchRule::parse(s("match").unwrap_or(""))
                .ok_or_else(|| err(i + 1, "unknown match rule"))?;
            let mut e = Entry::new(s("word").unwrap_or(""), lang, s("ipa"), s("say"), rule)
                .map_err(|m| err(i + 1, m))?;
            e.note = s("note").unwrap_or("").to_string();
            e.line = i + 1;
            list.entries.push(e);
        }
        list.reindex();
        Ok(list)
    }

    /// Write the TSV format (lossless for every field).
    pub fn to_tsv(&self) -> String {
        let mut out = String::from("# uccharan wordlist 1\n");
        if let Some(l) = self.default_lang {
            out.push_str(&format!("# lang: {}\n", l.tag()));
        }
        out.push_str("# alphabet: ipa\nword\tlang\tipa\tsay\tmatch\tnote\n");
        let clean = |s: &str| s.replace(['\t', '\n', '\r'], " ");
        for e in &self.entries {
            let rule = if e.rule == MatchRule::Word {
                ""
            } else {
                e.rule.name()
            };
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\n",
                clean(&e.word),
                e.lang.map_or("*", Lang::tag),
                clean(e.ipa.as_deref().unwrap_or("")),
                clean(e.say.as_deref().unwrap_or("")),
                rule,
                clean(&e.note)
            ));
        }
        out
    }

    /// Entries as a JSON array.
    pub fn to_json(&self) -> String {
        use crate::minijson::push_str;
        let mut out = String::from("[");
        for (i, e) in self.entries.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str("{\"word\":");
            push_str(&mut out, &e.word);
            out.push_str(",\"lang\":");
            push_str(&mut out, e.lang.map_or("*", Lang::tag));
            for (k, v) in [("ipa", &e.ipa), ("say", &e.say)] {
                out.push_str(&format!(",\"{k}\":"));
                match v {
                    Some(s) => push_str(&mut out, s),
                    None => out.push_str("null"),
                }
            }
            out.push_str(",\"match\":");
            push_str(&mut out, e.rule.name());
            out.push_str(",\"note\":");
            push_str(&mut out, &e.note);
            out.push_str(&format!(",\"line\":{}}}", e.line));
        }
        out.push(']');
        out
    }

    /// The languages that have entries (`None` stands for entries for any
    /// language). PLS holds one language per file: export each with
    /// [`WordList::to_pls`].
    pub fn languages(&self) -> Vec<Option<Lang>> {
        let mut out: Vec<Option<Lang>> = Vec::new();
        for e in &self.entries {
            if !out.contains(&e.lang) {
                out.push(e.lang);
            }
        }
        out
    }

    /// Export the entries of one language (and the entries for any language)
    /// as a PLS 1.0 document. Match rules other than whole-word and regular
    /// expressions have no PLS equivalent: regex rows are left out, and the
    /// rule is kept in a comment so that [`WordList::parse_pls`] restores it.
    pub fn to_pls(&self, lang: Option<Lang>) -> String {
        let tag = match lang {
            Some(Lang::EnUs) => "en-US".to_string(),
            Some(Lang::EnGb) => "en-GB".to_string(),
            Some(l) => l.tag().to_string(),
            None => "und".to_string(),
        };
        let mut out = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<lexicon version=\"1.0\" \
             xmlns=\"http://www.w3.org/2005/01/pronunciation-lexicon\" \
             alphabet=\"ipa\" xml:lang=\"{tag}\">\n"
        );
        for e in &self.entries {
            if e.rule == MatchRule::Regex || !(e.lang == lang || e.lang.is_none()) {
                continue;
            }
            out.push_str("  <lexeme>\n");
            if e.rule != MatchRule::Word
                || !e.note.is_empty()
                || (e.lang.is_none() && lang.is_some())
            {
                let mut c = String::new();
                if e.rule != MatchRule::Word {
                    c.push_str(&format!(" match={}", e.rule.name()));
                }
                if e.lang.is_none() && lang.is_some() {
                    c.push_str(" lang=*");
                }
                if !e.note.is_empty() {
                    c.push_str(&format!(" note={}", e.note.replace("--", "- -")));
                }
                out.push_str(&format!("    <!--{c} -->\n"));
            }
            out.push_str(&format!(
                "    <grapheme>{}</grapheme>\n",
                xml_escape(&e.word)
            ));
            if let Some(p) = &e.ipa {
                out.push_str(&format!("    <phoneme>{}</phoneme>\n", xml_escape(p)));
            }
            if let Some(s) = &e.say {
                out.push_str(&format!("    <alias>{}</alias>\n", xml_escape(s)));
            }
            out.push_str("  </lexeme>\n");
        }
        out.push_str("</lexicon>\n");
        out
    }

    /// Import a PLS 1.0 document (`alphabet="ipa"`). Every `<grapheme>` of
    /// a lexeme becomes an entry with the lexeme's first `<phoneme>` (the
    /// one marked `prefer="true"` if any) and first `<alias>`.
    pub fn parse_pls(text: &str) -> Result<WordList, WordListError> {
        let mut list = WordList::default();
        let toks = xml_tokens(text)?;
        let mut lang: Option<Lang> = None;
        let mut in_lexeme = false;
        let mut field: Option<&'static str> = None;
        let mut buf = String::new();
        let mut graphemes: Vec<String> = Vec::new();
        let mut phonemes: Vec<(String, bool)> = Vec::new();
        let mut alias: Option<String> = None;
        let mut comment = String::new();
        let mut lexeme_line = 0;
        let mut prefer = false;
        for (line, t) in toks {
            match t {
                Xml::Open(name, attrs) => match name.as_str() {
                    "lexicon" => {
                        if let Some(a) = attrs.iter().find(|a| a.0 == "alphabet") {
                            if !a.1.eq_ignore_ascii_case("ipa") {
                                return Err(err(
                                    line,
                                    format!("unsupported alphabet {:?} (only ipa)", a.1),
                                ));
                            }
                        }
                        if let Some(a) = attrs.iter().find(|a| a.0 == "xml:lang") {
                            lang = Lang::parse(&a.1)
                                .or_else(|| Lang::parse(a.1.split('-').next().unwrap_or("")));
                        }
                    }
                    "lexeme" => {
                        in_lexeme = true;
                        lexeme_line = line;
                        graphemes.clear();
                        phonemes.clear();
                        alias = None;
                        comment.clear();
                    }
                    "grapheme" | "phoneme" | "alias" if in_lexeme => {
                        field = Some(match name.as_str() {
                            "grapheme" => "grapheme",
                            "phoneme" => "phoneme",
                            _ => "alias",
                        });
                        prefer = attrs.iter().any(|a| a.0 == "prefer" && a.1 == "true");
                        if let Some(a) = attrs.iter().find(|a| a.0 == "alphabet") {
                            if !a.1.eq_ignore_ascii_case("ipa") {
                                return Err(err(
                                    line,
                                    format!("unsupported alphabet {:?} (only ipa)", a.1),
                                ));
                            }
                        }
                        buf.clear();
                    }
                    _ => {}
                },
                Xml::Text(s) => {
                    if field.is_some() {
                        buf.push_str(&s);
                    }
                }
                Xml::Comment(c) => {
                    if in_lexeme {
                        comment.push_str(&c);
                    }
                }
                Xml::Close(name) => match name.as_str() {
                    "grapheme" | "phoneme" | "alias" if field.is_some() => {
                        let v = buf.trim().to_string();
                        match field {
                            Some("grapheme") => graphemes.push(v),
                            Some("phoneme") => phonemes.push((v, prefer)),
                            _ => {
                                if alias.is_none() {
                                    alias = Some(v)
                                }
                            }
                        }
                        field = None;
                    }
                    "lexeme" => {
                        in_lexeme = false;
                        let ipa = phonemes
                            .iter()
                            .find(|p| p.1)
                            .or(phonemes.first())
                            .map(|p| p.0.clone());
                        let mut rule = MatchRule::Word;
                        let mut note = String::new();
                        let mut any_lang = false;
                        // Our own export comment: " match=case lang=* note=...".
                        let c = comment.trim();
                        let (head, n) = match c.find("note=") {
                            Some(i) => (&c[..i], c[i + 5..].trim()),
                            None => (c, ""),
                        };
                        for part in head.split_whitespace() {
                            if let Some(r) = part.strip_prefix("match=") {
                                rule = MatchRule::parse(r).unwrap_or_default();
                            } else if part == "lang=*" {
                                any_lang = true;
                            }
                        }
                        note.push_str(n);
                        for g in &graphemes {
                            let mut e = Entry::new(
                                g,
                                if any_lang { None } else { lang },
                                ipa.as_deref(),
                                alias.as_deref(),
                                rule,
                            )
                            .map_err(|m| err(lexeme_line, m))?;
                            e.note = note.clone();
                            e.line = lexeme_line;
                            list.entries.push(e);
                        }
                    }
                    _ => {}
                },
            }
        }
        list.default_lang = lang;
        list.reindex();
        Ok(list)
    }

    /// Every match of this list in `text` for language `lang`, before
    /// overlaps are resolved: `(start, end, entry)` byte ranges.
    pub fn find_all(&self, text: &str, lang: Lang) -> Vec<(usize, usize, usize)> {
        let mut out = Vec::new();
        let mut prev: Option<char> = None;
        for (i, c) in text.char_indices() {
            let word_start = !prev.is_some_and(|p| is_word_char(p) && !script_change(p, c));
            prev = Some(c);
            let key = fold_chars(&text[i..i + c.len_utf8()], true).next();
            let key_exact = fold_chars(&text[i..i + c.len_utf8()], false).next();
            let mut cands: Vec<usize> = Vec::new();
            for k in [key, key_exact].into_iter().flatten() {
                if let Some(v) = self.index.get(&k) {
                    for &ei in v {
                        if !cands.contains(&ei) {
                            cands.push(ei);
                        }
                    }
                }
            }
            for ei in cands {
                let e = &self.entries[ei];
                if !e.applies(lang) {
                    continue;
                }
                if e.lang.is_some_and(|l| l.is_english() && !lang.is_english())
                    && !e.word.chars().any(|c| script_of(c) == Script::Latin)
                {
                    continue;
                }
                let whole = e.rule != MatchRule::Exact;
                if whole && !word_start {
                    continue;
                }
                if let Some(end) = match_at(text, i, &e.word, e.rule == MatchRule::Word) {
                    let last = text[..end].chars().next_back();
                    if whole
                        && text[end..].chars().next().is_some_and(|n| {
                            is_word_char(n) && !last.is_some_and(|l| script_change(l, n))
                        })
                    {
                        continue;
                    }
                    out.push((i, end, ei));
                }
            }
        }
        for &ei in &self.regexes {
            let e = &self.entries[ei];
            if !e.applies(lang) {
                continue;
            }
            if let Some(re) = &e.regex {
                for (s, t) in re.find_iter(text) {
                    out.push((s, t, ei));
                }
            }
        }
        out
    }
}

/// A match chosen by [`select`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hit {
    /// Byte range in the text.
    pub start: usize,
    /// End of the byte range.
    pub end: usize,
    /// Index of the list in the slice given to [`select`].
    pub list: usize,
    /// Index of the entry in that list.
    pub entry: usize,
}

/// Resolve matches of several lists: lists are applied in order, and a
/// match of a later list is dropped where it overlaps one of an earlier
/// list. Within one list, the leftmost-longest match wins; for the same
/// span, an entry for the request language beats one for any language,
/// which beats an English entry applied to a Latin-script word.
pub fn select(lists: &[&WordList], text: &str, lang: Lang) -> Vec<Hit> {
    let mut chosen: Vec<Hit> = Vec::new();
    for (li, l) in lists.iter().enumerate() {
        let mut hits = l.find_all(text, lang);
        hits.sort_by_key(|&(s, e, ei)| (s, std::cmp::Reverse(e - s), l.entries[ei].rank(lang), ei));
        for (s, e, ei) in hits {
            if e <= s {
                continue;
            }
            if chosen.iter().any(|h| s < h.end && h.start < e) {
                continue;
            }
            chosen.push(Hit {
                start: s,
                end: e,
                list: li,
                entry: ei,
            });
        }
    }
    chosen.sort_by_key(|h| h.start);
    chosen
}

/// A change between Latin and another script inside a token ("मैंSBI") is
/// a word boundary.
fn script_change(a: char, b: char) -> bool {
    let la = script_of(a) == Script::Latin;
    let lb = script_of(b) == Script::Latin;
    la != lb && (a as u32 >= 0x0900 || b as u32 >= 0x0900)
}

/// Letters, digits and combining signs of Indic scripts are word characters.
pub(crate) fn is_word_char(c: char) -> bool {
    (c.is_alphanumeric()
        || ('\u{0900}'..='\u{0dff}').contains(&c)
        || matches!(c, '\u{200c}' | '\u{200d}' | '\u{0300}'..='\u{036f}'))
        && !matches!(c, '\u{0964}' | '\u{0965}')
}

/// Precomposed Devanagari nukta letters and their decompositions.
fn decompose_nukta(c: char) -> Option<[char; 2]> {
    let base = match c {
        '\u{0929}' => '\u{0928}',
        '\u{0931}' => '\u{0930}',
        '\u{0934}' => '\u{0933}',
        '\u{0958}' => '\u{0915}',
        '\u{0959}' => '\u{0916}',
        '\u{095a}' => '\u{0917}',
        '\u{095b}' => '\u{091c}',
        '\u{095c}' => '\u{0921}',
        '\u{095d}' => '\u{0922}',
        '\u{095e}' => '\u{092b}',
        '\u{095f}' => '\u{092f}',
        _ => return None,
    };
    Some([base, '\u{093c}'])
}

/// Characters of `s` with nukta letters decomposed and, with `nocase`,
/// lower-cased (one char per char for the common scripts).
fn fold_chars(s: &str, nocase: bool) -> impl Iterator<Item = char> + '_ {
    s.chars().flat_map(move |c| {
        let mut v: Vec<char> = Vec::with_capacity(2);
        if let Some(d) = decompose_nukta(c) {
            v.extend(d);
        } else if nocase {
            v.extend(c.to_lowercase());
        } else {
            v.push(c);
        }
        v.into_iter()
    })
}

/// Match `pat` at byte `start` of `text`; whitespace in the pattern matches
/// any run of whitespace. Returns the end byte.
fn match_at(text: &str, start: usize, pat: &str, nocase: bool) -> Option<usize> {
    let mut p = fold_chars(pat, nocase).peekable();
    let mut it = text[start..].char_indices().peekable();
    let mut pending: Vec<char> = Vec::new();
    let mut end = start;
    while let Some(pc) = p.next() {
        if pc.is_whitespace() {
            while p.peek().is_some_and(|c| c.is_whitespace()) {
                p.next();
            }
            if !pending.is_empty() {
                return None;
            }
            let mut n = 0;
            while let Some(&(i, c)) = it.peek() {
                if !c.is_whitespace() {
                    break;
                }
                it.next();
                end = start + i + c.len_utf8();
                n += 1;
            }
            if n == 0 {
                return None;
            }
            continue;
        }
        if pending.is_empty() {
            let (i, c) = it.next()?;
            end = start + i + c.len_utf8();
            pending = fold_chars(&text[start + i..end], nocase).collect();
            pending.reverse();
        }
        if pending.pop()? != pc {
            return None;
        }
    }
    pending.is_empty().then_some(end)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Replace XML entity references.
pub(crate) fn xml_unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        let Some(j) = tail.find(';') else {
            out.push_str(tail);
            return out;
        };
        let name = &tail[1..j];
        let c = match name {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            n if n.starts_with("#x") || n.starts_with("#X") => u32::from_str_radix(&n[2..], 16)
                .ok()
                .and_then(char::from_u32),
            n if n.starts_with('#') => n[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        match c {
            Some(c) => out.push(c),
            None => out.push_str(&tail[..=j]),
        }
        rest = &tail[j + 1..];
    }
    out.push_str(rest);
    out
}

/// A token of the small XML reader shared by PLS import and SSML.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Xml {
    /// Start tag (also emitted for a self-closing tag, followed by a Close).
    Open(String, Vec<(String, String)>),
    /// End tag.
    Close(String),
    /// Text with entities replaced.
    Text(String),
    /// A comment's content.
    Comment(String),
}

/// Tokenise an XML document (no DTD support). Each token carries its line.
pub(crate) fn xml_tokens(s: &str) -> Result<Vec<(usize, Xml)>, WordListError> {
    let mut out = Vec::new();
    let mut i = 0;
    let line_at = |i: usize| s[..i].matches('\n').count() + 1;
    while i < s.len() {
        let rest = &s[i..];
        if let Some(r) = rest.strip_prefix("<!--") {
            let j = r
                .find("-->")
                .ok_or_else(|| err(line_at(i), "unterminated comment"))?;
            out.push((line_at(i), Xml::Comment(r[..j].to_string())));
            i += 4 + j + 3;
        } else if rest.starts_with("<?") || rest.starts_with("<!") {
            let j = rest
                .find('>')
                .ok_or_else(|| err(line_at(i), "unterminated declaration"))?;
            i += j + 1;
        } else if let Some(r) = rest.strip_prefix("</") {
            let j = r
                .find('>')
                .ok_or_else(|| err(line_at(i), "unterminated end tag"))?;
            out.push((line_at(i), Xml::Close(local(r[..j].trim()))));
            i += 2 + j + 1;
        } else if rest.starts_with('<') {
            // Find the end of the tag, skipping quoted attribute values.
            let b = rest.as_bytes();
            let mut j = 1;
            let mut q: Option<u8> = None;
            while j < b.len() {
                match (q, b[j]) {
                    (None, b'"' | b'\'') => q = Some(b[j]),
                    (Some(x), c) if c == x => q = None,
                    (None, b'>') => break,
                    _ => {}
                }
                j += 1;
            }
            if j >= b.len() {
                return Err(err(line_at(i), "unterminated tag"));
            }
            let inner = &rest[1..j];
            let self_closing = inner.ends_with('/');
            let inner = inner.trim_end_matches('/');
            let name_end = inner
                .find(|c: char| c.is_whitespace())
                .unwrap_or(inner.len());
            let name = local(&inner[..name_end]);
            if name.is_empty() {
                return Err(err(line_at(i), "empty tag name"));
            }
            let attrs = parse_attrs(&inner[name_end..]).map_err(|m| err(line_at(i), m))?;
            out.push((line_at(i), Xml::Open(name.clone(), attrs)));
            if self_closing {
                out.push((line_at(i), Xml::Close(name)));
            }
            i += j + 1;
        } else {
            let j = rest.find('<').unwrap_or(rest.len());
            out.push((line_at(i), Xml::Text(xml_unescape(&rest[..j]))));
            i += j;
        }
    }
    Ok(out)
}

/// Tag name without a namespace prefix (`xml:lang` attributes keep theirs).
fn local(name: &str) -> String {
    name.rsplit(':').next().unwrap_or(name).to_string()
}

fn parse_attrs(s: &str) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    let mut rest = s.trim();
    while !rest.is_empty() {
        let eq = rest.find('=').ok_or("attribute without a value")?;
        let name = rest[..eq].trim().to_string();
        let r = rest[eq + 1..].trim_start();
        let q = r.chars().next().ok_or("attribute without a value")?;
        if q != '"' && q != '\'' {
            return Err("unquoted attribute value".into());
        }
        let end = r[1..].find(q).ok_or("unterminated attribute value")?;
        out.push((name, xml_unescape(&r[1..1 + end])));
        rest = r[end + 2..].trim_start();
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// A small backtracking regular-expression engine for `regex` rows.

/// A regular expression: literals, `.`, classes `[a-z]` `[^…]`, escapes
/// `\d \w \s \D \W \S \b \B` and escaped punctuation, groups `( )` and
/// `(?: )`, alternation `|`, quantifiers `* + ? {m} {m,} {m,n}` (greedy,
/// `?` suffix for lazy), anchors `^ $`, and a leading `(?i)` for
/// case-insensitive matching. Matching is bounded in steps, so a
/// pathological pattern fails instead of hanging.
#[derive(Clone, Debug)]
pub struct Regex {
    root: Node,
    nocase: bool,
}

#[derive(Clone, Debug)]
enum ClassItem {
    Range(char, char),
    Digit(bool),
    Word(bool),
    Space(bool),
}

#[derive(Clone, Debug)]
enum Node {
    Char(char),
    Any,
    Class(Vec<ClassItem>, bool),
    Start,
    End,
    WordB(bool),
    Group(Vec<Vec<Node>>),
    Repeat(Box<Node>, usize, usize, bool),
}

const STEP_LIMIT: usize = 200_000;

impl Regex {
    /// Compile a pattern.
    pub fn new(pattern: &str) -> Result<Regex, String> {
        let (nocase, p) = match pattern.strip_prefix("(?i)") {
            Some(r) => (true, r),
            None => (false, pattern),
        };
        let cs: Vec<char> = p.chars().collect();
        let mut i = 0;
        let alts = parse_alts(&cs, &mut i)?;
        if i != cs.len() {
            return Err(format!("unbalanced ')' in regex {pattern:?}"));
        }
        Ok(Regex {
            root: Node::Group(alts),
            nocase,
        })
    }

    /// Non-overlapping matches as byte ranges.
    pub fn find_iter(&self, text: &str) -> Vec<(usize, usize)> {
        let cs: Vec<(usize, char)> = text.char_indices().collect();
        let byte = |k: usize| cs.get(k).map_or(text.len(), |x| x.0);
        let chars: Vec<char> = cs.iter().map(|x| x.1).collect();
        let mut out = Vec::new();
        let mut k = 0;
        while k <= chars.len() {
            let m = Matcher {
                t: &chars,
                nocase: self.nocase,
                steps: std::cell::Cell::new(0),
            };
            let mut found: Option<usize> = None;
            m.one(&self.root, k, &mut |e| {
                found = Some(e);
                true
            });
            match found {
                Some(e) if e > k => {
                    out.push((byte(k), byte(e)));
                    k = e;
                }
                _ => k += 1,
            }
        }
        out
    }

    /// True when the pattern matches somewhere in `text`.
    pub fn is_match(&self, text: &str) -> bool {
        !self.find_iter(text).is_empty()
    }
}

fn parse_alts(cs: &[char], i: &mut usize) -> Result<Vec<Vec<Node>>, String> {
    let mut alts = vec![Vec::new()];
    while *i < cs.len() {
        let c = cs[*i];
        match c {
            ')' => break,
            '|' => {
                *i += 1;
                alts.push(Vec::new());
                continue;
            }
            _ => {}
        }
        let atom = parse_atom(cs, i)?;
        let atom = parse_quant(cs, i, atom)?;
        alts.last_mut().unwrap().push(atom);
    }
    Ok(alts)
}

fn parse_quant(cs: &[char], i: &mut usize, atom: Node) -> Result<Node, String> {
    let Some(&q) = cs.get(*i) else {
        return Ok(atom);
    };
    let (min, max) = match q {
        '*' => (0, usize::MAX),
        '+' => (1, usize::MAX),
        '?' => (0, 1),
        '{' => {
            let close = cs[*i..]
                .iter()
                .position(|&c| c == '}')
                .ok_or("unterminated {")?;
            let body: String = cs[*i + 1..*i + close].iter().collect();
            let (a, b) = match body.split_once(',') {
                Some((a, "")) => (a.trim().parse().map_err(|_| "bad {m,}")?, usize::MAX),
                Some((a, b)) => (
                    a.trim().parse().map_err(|_| "bad {m,n}")?,
                    b.trim().parse().map_err(|_| "bad {m,n}")?,
                ),
                None => {
                    let n = body.trim().parse().map_err(|_| "bad {m}")?;
                    (n, n)
                }
            };
            *i += close;
            (a, b)
        }
        _ => return Ok(atom),
    };
    if matches!(atom, Node::Start | Node::End | Node::WordB(_)) {
        return Err("quantifier after an anchor".into());
    }
    *i += 1;
    let lazy = cs.get(*i) == Some(&'?');
    if lazy {
        *i += 1;
    }
    Ok(Node::Repeat(Box::new(atom), min, max, lazy))
}

fn escape_item(c: char) -> Option<ClassItem> {
    Some(match c {
        'd' => ClassItem::Digit(true),
        'D' => ClassItem::Digit(false),
        'w' => ClassItem::Word(true),
        'W' => ClassItem::Word(false),
        's' => ClassItem::Space(true),
        'S' => ClassItem::Space(false),
        _ => return None,
    })
}

fn escaped_char(c: char) -> char {
    match c {
        'n' => '\n',
        't' => '\t',
        'r' => '\r',
        other => other,
    }
}

fn parse_atom(cs: &[char], i: &mut usize) -> Result<Node, String> {
    let c = cs[*i];
    *i += 1;
    Ok(match c {
        '.' => Node::Any,
        '^' => Node::Start,
        '$' => Node::End,
        '*' | '+' | '?' | '{' => return Err(format!("nothing to repeat before {c:?}")),
        '(' => {
            if cs.get(*i) == Some(&'?') && cs.get(*i + 1) == Some(&':') {
                *i += 2;
            }
            let alts = parse_alts(cs, i)?;
            if cs.get(*i) != Some(&')') {
                return Err("unbalanced '('".into());
            }
            *i += 1;
            Node::Group(alts)
        }
        '\\' => {
            let e = *cs.get(*i).ok_or("trailing backslash")?;
            *i += 1;
            match e {
                'b' => Node::WordB(true),
                'B' => Node::WordB(false),
                _ => match escape_item(e) {
                    Some(it) => Node::Class(vec![it], false),
                    None => Node::Char(escaped_char(e)),
                },
            }
        }
        '[' => {
            let mut items = Vec::new();
            let neg = cs.get(*i) == Some(&'^');
            if neg {
                *i += 1;
            }
            let mut first = true;
            loop {
                let c = *cs.get(*i).ok_or("unterminated [")?;
                *i += 1;
                if c == ']' && !first {
                    break;
                }
                first = false;
                let lo = if c == '\\' {
                    let e = *cs.get(*i).ok_or("unterminated [")?;
                    *i += 1;
                    if let Some(it) = escape_item(e) {
                        items.push(it);
                        continue;
                    }
                    escaped_char(e)
                } else {
                    c
                };
                if cs.get(*i) == Some(&'-') && cs.get(*i + 1).is_some_and(|&n| n != ']') {
                    let mut hi = cs[*i + 1];
                    *i += 2;
                    if hi == '\\' {
                        hi = escaped_char(*cs.get(*i).ok_or("unterminated [")?);
                        *i += 1;
                    }
                    if hi < lo {
                        return Err("bad class range".into());
                    }
                    items.push(ClassItem::Range(lo, hi));
                } else {
                    items.push(ClassItem::Range(lo, lo));
                }
            }
            Node::Class(items, neg)
        }
        other => Node::Char(other),
    })
}

struct Matcher<'a> {
    t: &'a [char],
    nocase: bool,
    steps: std::cell::Cell<usize>,
}

impl Matcher<'_> {
    fn eq(&self, a: char, b: char) -> bool {
        a == b || (self.nocase && a.to_lowercase().eq(b.to_lowercase()))
    }

    fn tick(&self) -> bool {
        let n = self.steps.get() + 1;
        self.steps.set(n);
        n <= STEP_LIMIT
    }

    fn class_has(&self, items: &[ClassItem], c: char) -> bool {
        items.iter().any(|it| match it {
            ClassItem::Range(a, b) => {
                (*a..=*b).contains(&c)
                    || (self.nocase
                        && c.to_lowercase()
                            .chain(c.to_uppercase())
                            .any(|x| (*a..=*b).contains(&x)))
            }
            ClassItem::Digit(p) => c.is_numeric() == *p,
            ClassItem::Word(p) => (is_word_char(c) || c == '_') == *p,
            ClassItem::Space(p) => c.is_whitespace() == *p,
        })
    }

    fn seq(&self, s: &[Node], pos: usize, k: &mut dyn FnMut(usize) -> bool) -> bool {
        match s.split_first() {
            None => k(pos),
            Some((Node::Repeat(n, min, max, lazy), rest)) => {
                self.rep(n, (*min, *max, *lazy), 0, pos, rest, k)
            }
            Some((first, rest)) => self.one(first, pos, &mut |p| self.seq(rest, p, k)),
        }
    }

    fn rep(
        &self,
        n: &Node,
        q: (usize, usize, bool),
        count: usize,
        pos: usize,
        rest: &[Node],
        k: &mut dyn FnMut(usize) -> bool,
    ) -> bool {
        let (min, max, lazy) = q;
        if !self.tick() {
            return false;
        }
        if lazy && count >= min && self.seq(rest, pos, k) {
            return true;
        }
        if count < max
            && self.one(n, pos, &mut |p| {
                p != pos && self.rep(n, q, count + 1, p, rest, k)
            })
        {
            return true;
        }
        !lazy && count >= min && self.seq(rest, pos, k)
    }

    fn one(&self, n: &Node, pos: usize, k: &mut dyn FnMut(usize) -> bool) -> bool {
        if !self.tick() {
            return false;
        }
        let t = self.t;
        let wb = |p: usize| {
            let a = p > 0 && is_word_char(t[p - 1]);
            let b = p < t.len() && is_word_char(t[p]);
            a != b
        };
        match n {
            Node::Char(c) => pos < t.len() && self.eq(t[pos], *c) && k(pos + 1),
            Node::Any => pos < t.len() && t[pos] != '\n' && k(pos + 1),
            Node::Class(items, neg) => {
                pos < t.len() && self.class_has(items, t[pos]) != *neg && k(pos + 1)
            }
            Node::Start => pos == 0 && k(pos),
            Node::End => pos == t.len() && k(pos),
            Node::WordB(want) => wb(pos) == *want && k(pos),
            Node::Group(alts) => alts.iter().any(|a| self.seq(a, pos, k)),
            Node::Repeat(inner, min, max, lazy) => {
                self.rep(inner, (*min, *max, *lazy), 0, pos, &[], k)
            }
        }
    }
}
