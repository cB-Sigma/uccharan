//! Normalise-only mode: the spoken form of a text, for voices that read
//! text rather than phonemes, with typed spans aligned to the input.
//!
//! ```
//! use uccharan::normalize_api::{NormOptions, SpanKind};
//! use uccharan::{Lang, Phonemizer};
//!
//! let p = Phonemizer::new();
//! let n = uccharan::normalize_api::normalize(&p, "Pay ₹500 by 10:30.", Lang::EnUs, &NormOptions::default());
//! assert_eq!(n.text, "Pay five hundred rupees by ten thirty.");
//! let money = n.spans.iter().find(|s| s.kind == SpanKind::Money).unwrap();
//! assert_eq!(&"Pay ₹500 by 10:30."[money.src.clone()], "₹500");
//! ```
//!
//! The spans tile the input: every input byte belongs to exactly one span,
//! and the outputs of the spans, in order, make up the output text.
//! Whitespace and punctuation between words are copied unchanged.

use crate::norm::Item;
use crate::respell::{self, Style};
use crate::text::{self, script_of, Chunk, Script};
use crate::wordlist::{self, WordList};
use crate::{Lang, Mode, Normalize, Options, Phonemizer};
use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

/// Which language reads digits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DigitLang {
    /// The request language, except that numbers next to Latin-script
    /// words inside an Indic-language text are read in English.
    #[default]
    Context,
    /// Always the request language.
    Request,
    /// Always this language.
    Fixed(Lang),
}

/// How English reads large numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum EnScale {
    /// Lakh and crore for numbers written with Indian grouping
    /// (`5,00,000`) and inside Indic-language texts; else international.
    #[default]
    Auto,
    /// Always lakh and crore ("five lakh").
    Indian,
    /// Always thousand, million, billion.
    International,
}

/// What to do with Latin-script words in a text of another script.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LatinPolicy {
    /// Leave them in Latin script (the voice reads English).
    #[default]
    Keep,
    /// Respell them in the request language's script.
    Transliterate,
    /// Spell them letter by letter, letter names in the request script.
    Spell,
}

/// What to do with URLs and email addresses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum UrlPolicy {
    /// Read them out ("example dot com").
    #[default]
    Read,
    /// Leave them unchanged.
    Keep,
    /// Leave them out.
    Drop,
}

/// Options of [`normalize`].
#[derive(Clone, Debug, Default)]
pub struct NormOptions {
    /// `Improved` keeps known brand names whole ("iPhone").
    pub mode: Mode,
    /// Which language reads digits.
    pub digits: DigitLang,
    /// Lakh/crore or international grouping in English.
    pub en_scale: EnScale,
    /// Latin-script words inside other scripts.
    pub latin: LatinPolicy,
    /// URLs and email addresses.
    pub urls: UrlPolicy,
    /// Respelling style for word-list entries that have phonemes but no
    /// spoken form.
    pub respell: Style,
    /// Word lists, highest priority first.
    pub wordlists: Vec<Arc<WordList>>,
}

/// The class of a span.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpanKind {
    /// A word that needed no change.
    Text,
    /// Whitespace and punctuation between words (copied).
    Punct,
    /// A cardinal number.
    Number,
    /// A number with a fractional part.
    Decimal,
    /// An ordinal ("21st", "5वां").
    Ordinal,
    /// A year read as a year ("1984").
    Year,
    /// A currency amount.
    Money,
    /// A percentage.
    Percent,
    /// A clock time.
    Time,
    /// A date.
    Date,
    /// A telephone number, read digit by digit.
    Phone,
    /// Digits read one by one (leading zeros, long strings).
    Digits,
    /// A range ("10-20").
    Range,
    /// A fraction ("3/4").
    Fraction,
    /// A number with a unit ("5km").
    Measure,
    /// An abbreviation ("Dr.", "e.g.").
    Abbreviation,
    /// Letters read one by one ("FBI").
    Acronym,
    /// A symbol read as a word ("&").
    Symbol,
    /// A URL.
    Url,
    /// An email address.
    Email,
    /// A Latin-script word inside a text of another script, kept as is.
    Latin,
    /// A word respelled in another script.
    Transliterated,
    /// A word-list entry.
    WordList,
    /// Text left out.
    Dropped,
}

impl SpanKind {
    /// Lower-case name used in JSON (`latin` is "latin-in-indic" text).
    pub fn name(self) -> &'static str {
        match self {
            SpanKind::Text => "text",
            SpanKind::Punct => "punct",
            SpanKind::Number => "number",
            SpanKind::Decimal => "decimal",
            SpanKind::Ordinal => "ordinal",
            SpanKind::Year => "year",
            SpanKind::Money => "money",
            SpanKind::Percent => "percent",
            SpanKind::Time => "time",
            SpanKind::Date => "date",
            SpanKind::Phone => "phone",
            SpanKind::Digits => "digits",
            SpanKind::Range => "range",
            SpanKind::Fraction => "fraction",
            SpanKind::Measure => "measure",
            SpanKind::Abbreviation => "abbreviation",
            SpanKind::Acronym => "acronym",
            SpanKind::Symbol => "symbol",
            SpanKind::Url => "url",
            SpanKind::Email => "email",
            SpanKind::Latin => "latin",
            SpanKind::Transliterated => "transliterated",
            SpanKind::WordList => "wordlist",
            SpanKind::Dropped => "dropped",
        }
    }
}

/// One span of the input and its spoken form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    /// Byte range in the input.
    pub src: Range<usize>,
    /// Byte range in the output text.
    pub out: Range<usize>,
    /// What the span is.
    pub kind: SpanKind,
    /// The language whose rules produced the output.
    pub lang: Lang,
    /// For [`SpanKind::WordList`]: (list index, entry index).
    pub entry: Option<(usize, usize)>,
    /// For numbers: the value read (ASCII digits).
    pub value: Option<String>,
}

/// The result of [`normalize`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Normalized {
    /// The spoken form.
    pub text: String,
    /// Sentences as byte ranges of `text`.
    pub sentences: Vec<Range<usize>>,
    /// Spans covering the input in order.
    pub spans: Vec<Span>,
}

struct Piece {
    src: Range<usize>,
    out: String,
    kind: SpanKind,
    lang: Lang,
    entry: Option<(usize, usize)>,
    value: Option<String>,
}

struct Cx<'a> {
    ph: &'a Phonemizer,
    lang: Lang,
    opt: &'a NormOptions,
    cache: HashMap<String, Option<String>>,
    /// The chunk before the one being read (it selects some ordinal forms).
    prev: String,
}

/// Normalise `text` read in language `lang`.
pub fn normalize(ph: &Phonemizer, text: &str, lang: Lang, opt: &NormOptions) -> Normalized {
    let mut cx = Cx {
        ph,
        lang,
        opt,
        cache: HashMap::new(),
        prev: String::new(),
    };
    let lists: Vec<&WordList> = opt.wordlists.iter().map(|l| l.as_ref()).collect();
    let hits = wordlist::select(&lists, text, lang);
    let mut pieces: Vec<Piece> = Vec::new();
    let mut breaks: Vec<usize> = Vec::new();
    let mut pos = 0;
    for h in &hits {
        if h.start > pos {
            cx.segment(text, pos..h.start, &mut pieces, &mut breaks);
        }
        let e = &lists[h.list].entries[h.entry];
        let out = cx.entry_text(e, &text[h.start..h.end]);
        pieces.push(Piece {
            src: h.start..h.end,
            out,
            kind: SpanKind::WordList,
            lang: e.lang.unwrap_or(lang),
            entry: Some((h.list, h.entry)),
            value: None,
        });
        pos = h.end;
    }
    if pos < text.len() || text.is_empty() {
        cx.segment(text, pos..text.len(), &mut pieces, &mut breaks);
    }
    // Manipuri text in Bengali script gets its number and date words in
    // Bengali script too (the language file writes them in Meetei Mayek).
    if lang == Lang::Mni && crate::mni::writes_bengali(text) {
        for pc in &mut pieces {
            if crate::mni::has_meetei(&pc.out) {
                pc.out = crate::mni::meetei_to_bengali(&pc.out);
            }
        }
    }
    // Kashmiri text in Devanagari gets its number and date words in
    // Devanagari too (the language file writes them in Perso-Arabic).
    if lang == Lang::Ks && crate::ks::writes_devanagari(text) {
        for pc in &mut pieces {
            if pc.kind != SpanKind::Text && pc.kind != SpanKind::Punct {
                pc.out = crate::ks::arabic_to_devanagari(&pc.out);
            }
        }
    }
    assemble(text, pieces, &breaks)
}

fn assemble(text: &str, pieces: Vec<Piece>, breaks: &[usize]) -> Normalized {
    let mut n = Normalized::default();
    let mut out_breaks: Vec<usize> = Vec::new();
    for p in pieces {
        let mut o = p.out;
        if p.kind == SpanKind::Punct {
            // Text left out leaves two spaces: keep one.
            if n.text.ends_with(char::is_whitespace) && o.starts_with(char::is_whitespace) {
                let skip = o.len() - o.trim_start().len();
                for &b in breaks {
                    if p.src.contains(&b) {
                        let k = (b - p.src.start).saturating_sub(skip);
                        out_breaks.push(n.text.len() + k);
                    }
                }
                o = o.trim_start().to_string();
            } else {
                for &b in breaks {
                    if p.src.contains(&b) || b == p.src.end {
                        out_breaks.push(n.text.len() + (b - p.src.start).min(o.len()));
                    }
                }
            }
        } else {
            for &b in breaks {
                if b == p.src.end {
                    out_breaks.push(n.text.len() + o.len());
                }
            }
        }
        let start = n.text.len();
        n.text.push_str(&o);
        n.spans.push(Span {
            src: p.src,
            out: start..n.text.len(),
            kind: p.kind,
            lang: p.lang,
            entry: p.entry,
            value: p.value,
        });
    }
    let _ = text;
    // Sentences: the output split at the breaks, trimmed.
    out_breaks.sort_unstable();
    out_breaks.dedup();
    let mut start = 0;
    for b in out_breaks.into_iter().chain(std::iter::once(n.text.len())) {
        if b < start {
            continue;
        }
        let seg = &n.text[start..b];
        let lead = seg.len() - seg.trim_start().len();
        let s = start + lead;
        let e = start + seg.trim_end().len();
        if e > s {
            n.sentences.push(s..e);
        }
        start = b;
    }
    n
}

const SCALE_WORDS: &[&str] = &[
    "लाख",
    "करोड़",
    "करोड़",
    "हज़ार",
    "हजार",
    "अरब",
    "lakh",
    "lakhs",
    "crore",
    "crores",
    "cr",
    "thousand",
    "million",
    "billion",
    "करोड",
];

const CURRENCY_WORDS: &[&str] = &["rs", "inr", "रु", "रू", "रुपये"];

fn is_scale_word(s: &str) -> bool {
    SCALE_WORDS.contains(&s.to_lowercase().trim_end_matches('.'))
}

fn ascii_digits(s: &str) -> String {
    s.chars()
        .map(|c| match text::indic_digit(c) {
            Some(d) => char::from(b'0' + d),
            None => c,
        })
        .collect()
}

fn has_digit(s: &str) -> bool {
    s.chars()
        .any(|c| c.is_ascii_digit() || text::indic_digit(c).is_some())
}

fn is_number(s: &str) -> bool {
    let a = ascii_digits(s);
    !a.is_empty()
        && a.chars().next().is_some_and(|c| c.is_ascii_digit())
        && a.chars()
            .all(|c| c.is_ascii_digit() || c == ',' || c == '.')
}

/// Script class of a chunk: its first letter's script.
fn chunk_script(s: &str) -> Option<Script> {
    s.chars()
        .map(script_of)
        .find(|sc| !matches!(sc, Script::Digit | Script::Other))
}

impl Cx<'_> {
    /// Normalise the text in `range` (no word-list matches inside).
    fn segment(
        &mut self,
        full: &str,
        range: Range<usize>,
        pieces: &mut Vec<Piece>,
        breaks: &mut Vec<usize>,
    ) {
        let base = range.start;
        let sub = &full[range.clone()];
        let context = self
            .lang
            .latin()
            .map(|l| l.numbers.ordinal_context())
            .unwrap_or_default();
        let clauses = text::clauses_with(sub, &context);
        // Chunks with their source ranges, by clause.
        let mut cursor = 0usize;
        let mut cls: Vec<Vec<(Range<usize>, Chunk)>> = Vec::new();
        for c in &clauses {
            let mut v = Vec::new();
            for (k, ch) in c.chunks.iter().enumerate() {
                let Some(off) = sub[cursor..].find(ch.text.as_str()) else {
                    continue;
                };
                let s = cursor + off;
                let mut e = s + ch.text.len();
                // An ordinal at the end of a sentence keeps its full stop
                // as the sentence's ("am 15. Danach").
                let ends = k + 1 == c.chunks.len() && c.terminator == text::Terminator::Period;
                if ch.dot && !ends && sub[e..].starts_with('.') {
                    e += 1;
                }
                cursor = e;
                v.push((s..e, ch.clone()));
            }
            if c.terminator.ends_sentence() {
                // The break goes after the terminator and any closers.
                let rest = &sub[cursor..];
                let mut b = None;
                for (i, ch) in rest.char_indices() {
                    if matches!(ch, '.' | '?' | '!' | '।' | '॥' | '…' | '。' | '？' | '！')
                    {
                        b = Some(i + ch.len_utf8());
                    } else if ch.is_whitespace()
                        || (b.is_none() && !matches!(ch, '"' | '\'' | ')' | ']' | '”' | '’' | '»'))
                        || (b.is_some()
                            && !matches!(
                                ch,
                                '"' | '\'' | ')' | ']' | '”' | '’' | '»' | '.' | '?' | '!'
                            ))
                    {
                        break;
                    } else if b.is_some() {
                        b = Some(i + ch.len_utf8());
                    }
                }
                breaks.push(base + cursor + b.unwrap_or(0));
            }
            cls.push(self.join_groups(v));
        }
        let mut pos = 0usize;
        for v in &cls {
            let mut i = 0;
            while i < v.len() {
                let (r, _) = &v[i];
                if r.start > pos {
                    pieces.push(self.punct(full, base + pos..base + r.start));
                }
                let (consumed, piece) = self.chunk_group(full, base, v, i);
                pos = v[i + consumed - 1].0.end;
                pieces.push(piece);
                i += consumed;
            }
        }
        if pos < sub.len() {
            pieces.push(self.punct(full, base + pos..range.end));
        }
    }

    fn punct(&self, full: &str, r: Range<usize>) -> Piece {
        Piece {
            out: full[r.clone()].to_string(),
            src: r,
            kind: SpanKind::Punct,
            lang: self.lang,
            entry: None,
            value: None,
        }
    }

    /// Numbers written in groups of three digits ("1 500") as one chunk,
    /// where the language that reads the digits writes them so, as the
    /// phonemiser joins them (compatible mode only across a plain space).
    fn join_groups(&self, v: Vec<(Range<usize>, Chunk)>) -> Vec<(Range<usize>, Chunk)> {
        let l = match self.opt.digits {
            DigitLang::Fixed(l) => l,
            _ => self.lang,
        };
        let compat = self.opt.mode != Mode::Improved;
        if !l.latin().is_some_and(|x| x.numbers.joins_groups(compat)) {
            return v;
        }
        let chunks: Vec<Chunk> = v.iter().map(|x| x.1.clone()).collect();
        let joined = crate::gnorm::join_groups(&chunks, if compat { 6 } else { 3 }, compat);
        if joined.len() == v.len() {
            return v;
        }
        let mut out = Vec::with_capacity(joined.len());
        let mut k = 0;
        for c in joined {
            let n = c.text.matches(crate::gnorm::GROUPED).count();
            out.push((v[k].0.start..v[k + n].0.end, c));
            k += n + 1;
        }
        out
    }

    /// The language that reads the digits of chunk `i` of a clause.
    fn digit_lang(&self, v: &[(Range<usize>, Chunk)], i: usize) -> Lang {
        let req = self.lang;
        match self.opt.digits {
            DigitLang::Fixed(l) => l,
            DigitLang::Request => req,
            DigitLang::Context => {
                if req.script() == Script::Latin || self.opt.latin != LatinPolicy::Keep {
                    return req;
                }
                let sc =
                    |k: Option<&(Range<usize>, Chunk)>| k.and_then(|x| chunk_script(&x.1.text));
                let next = sc(v.get(i + 1));
                let prev = if i > 0 { sc(v.get(i - 1)) } else { None };
                let latin_next = next == Some(Script::Latin);
                let latin_prev = prev == Some(Script::Latin)
                    && !matches!(next, Some(Script::Devanagari | Script::Brahmic(_)));
                if latin_next || latin_prev {
                    Lang::EnGb
                } else {
                    req
                }
            }
        }
    }

    /// Normalise chunk `i` (possibly with following chunks it binds to).
    /// Returns how many chunks were used.
    fn chunk_group(
        &mut self,
        full: &str,
        base: usize,
        v: &[(Range<usize>, Chunk)],
        i: usize,
    ) -> (usize, Piece) {
        let (r, ch) = &v[i];
        let src = base + r.start..base + r.end;
        let t = ch.text.as_str();
        let next = v.get(i + 1).map(|x| x.1.text.as_str());
        let next2 = v.get(i + 2).map(|x| x.1.text.as_str());
        let dl = self.digit_lang(v, i);
        self.prev = i
            .checked_sub(1)
            .map_or(String::new(), |k| v[k].1.text.clone());
        let end_of = |k: usize| base + v[k].0.end;
        // "Rs. 250", "रु. 250", "INR 900": a currency word before a number.
        if CURRENCY_WORDS.contains(&t.to_lowercase().as_str())
            && next.is_some_and(is_number)
            && dl.latin().is_none()
        {
            let num = next.unwrap();
            let scale = next2.filter(|s| is_scale_word(s));
            let used = if scale.is_some() { 3 } else { 2 };
            let out = self.money(&format!("₹{num}"), scale, dl);
            return (
                used,
                self.piece(
                    base + r.start..end_of(i + used - 1),
                    out,
                    SpanKind::Money,
                    dl,
                    Some(num),
                ),
            );
        }
        // "₹5 करोड़", "$2 million": an amount with a scale word.
        if t.starts_with(['₹', '$', '£', '€'])
            && next.is_some_and(is_scale_word)
            && dl.latin().is_none()
        {
            let amount = &t[t.chars().next().unwrap().len_utf8()..];
            if is_number(amount) {
                let out = self.money(t, next, dl);
                return (
                    2,
                    self.piece(
                        src.start..end_of(i + 1),
                        out,
                        SpanKind::Money,
                        dl,
                        Some(amount),
                    ),
                );
            }
        }
        // "1.5 लाख": Hindi fractional idioms before a scale word.
        if dl.hindi_like() && is_number(t) && next.is_some_and(is_scale_word) {
            if let Some(w) = hindi_fraction(&ascii_digits(t)) {
                return (1, self.piece(src, w, SpanKind::Decimal, dl, Some(t)));
            }
        }
        // "Jan 5": an English month abbreviation before a day.
        if dl.is_english() && !ch.dot {
            if let Some(m) = crate::norm::month_before_day(t, next) {
                return (1, self.piece(src, m.to_string(), SpanKind::Date, dl, None));
            }
        }
        // "10:30 बजे": Hindi clock idioms; "3:20 बजे" is read without a
        // second बजे.
        if dl.hindi_like() && next == Some("बजे") {
            if let Some((w, keeps)) = hindi_clock(&ascii_digits(t)) {
                return if keeps {
                    (1, self.piece(src, w, SpanKind::Time, dl, None))
                } else {
                    (
                        2,
                        self.piece(src.start..end_of(i + 1), w, SpanKind::Time, dl, None),
                    )
                };
            }
        }
        // "10 h 30", "10.30 Uhr": a clock time written in more than one chunk.
        if let Some(lat) = dl.latin() {
            let texts: Vec<&str> = v[i..v.len().min(i + 3)]
                .iter()
                .map(|x| x.1.text.as_str())
                .collect();
            let open = v[i..].iter().take_while(|x| !x.1.dot).count() + 1;
            if let Some((k, text)) = crate::gnorm::time_join(&texts, &lat.numbers) {
                if k > 1 && k <= open {
                    let c = Chunk {
                        text,
                        dot: v[i + k - 1].1.dot,
                        tight: false,
                    };
                    let items = self.expand(&c, dl);
                    let out = self.join(&items, dl);
                    return (
                        k,
                        self.piece(src.start..end_of(i + k - 1), out, SpanKind::Time, dl, None),
                    );
                }
            }
        }
        // "25 €" in English: the amount and its sign ("twenty five euros").
        if let (true, Some(sign)) = (dl.is_english(), next) {
            if is_number(t) && crate::norm::is_en_currency_sign(sign) && !ch.dot {
                let c = Chunk {
                    text: format!("{t}{sign}"),
                    dot: false,
                    tight: false,
                };
                let items = self.expand(&c, dl);
                let out = self.join(&items, dl);
                return (
                    2,
                    self.piece(src.start..end_of(i + 1), out, SpanKind::Money, dl, Some(t)),
                );
            }
        }
        // "25,50 €": an amount and its currency sign.
        if let (Some(lat), Some(sign)) = (dl.latin(), next) {
            if is_number(t) && crate::gnorm::is_currency_sign(sign, &lat.numbers) && !ch.dot {
                let c = Chunk {
                    text: format!("{t}{sign}"),
                    dot: false,
                    tight: false,
                };
                let items = self.expand(&c, dl);
                let out = self.join(&items, dl);
                return (
                    2,
                    self.piece(src.start..end_of(i + 1), out, SpanKind::Money, dl, Some(t)),
                );
            }
        }
        // "+91 98765 43210": a country code and digit groups.
        let a = ascii_digits(t);
        if a.len() >= 2
            && a.len() <= 4
            && a.starts_with('+')
            && a[1..].bytes().all(|b| b.is_ascii_digit())
            && next.is_some_and(|n| {
                let n = ascii_digits(n);
                n.len() >= 3 && n.bytes().all(|b| b.is_ascii_digit())
            })
        {
            let mut k = i + 1;
            let mut digits = a[1..].to_string();
            while let Some((_, c)) = v.get(k) {
                let n = ascii_digits(&c.text);
                if n.len() >= 2 && n.bytes().all(|b| b.is_ascii_digit()) {
                    digits.push_str(&n);
                    k += 1;
                } else {
                    break;
                }
            }
            let mut words = vec![self.word_plus(dl)];
            for d in digits.bytes() {
                words.push(self.read_number(&char::from(d).to_string(), dl));
            }
            let out = words.join(" ");
            let p = self.piece(src.start..end_of(k - 1), out, SpanKind::Phone, dl, None);
            return (k - i, p);
        }
        (1, self.chunk(full, src, ch, dl))
    }

    fn piece(
        &self,
        src: Range<usize>,
        out: String,
        kind: SpanKind,
        lang: Lang,
        value: Option<&str>,
    ) -> Piece {
        Piece {
            src,
            out,
            kind,
            lang,
            entry: None,
            value: value.map(|v| ascii_digits(v).replace([',', crate::gnorm::GROUPED], "")),
        }
    }

    /// True when the request language is written in another script than
    /// Latin (Latin-script words in its text are foreign words).
    fn other_script(&self) -> bool {
        self.lang.script() != Script::Latin
    }

    fn word_plus(&self, l: Lang) -> String {
        if l.script() == Script::Devanagari {
            "प्लस".into()
        } else {
            "plus".into()
        }
    }

    /// Read a plain number string in language `l`.
    fn read_number(&mut self, s: &str, l: Lang) -> String {
        let c = Chunk {
            text: s.to_string(),
            dot: false,
            tight: false,
        };
        let items = self.expand(&c, l);
        self.join(&items, l)
    }

    fn money(&mut self, amount: &str, scale: Option<&str>, l: Lang) -> String {
        let Some(scale) = scale else {
            let c = Chunk {
                text: amount.to_string(),
                dot: false,
                tight: false,
            };
            let items = self.expand(&c, l);
            return self.join(&items, l);
        };
        let sym = amount.chars().next().unwrap_or('₹');
        let num = &amount[sym.len_utf8()..];
        let mut words = if l.hindi_like() {
            hindi_fraction(&ascii_digits(num)).unwrap_or_else(|| self.read_number(num, l))
        } else {
            self.read_number(num, l)
        };
        words.push(' ');
        words.push_str(scale.trim_end_matches('.'));
        let name = if l.script() == Script::Devanagari {
            match sym {
                '$' => "डॉलर",
                '£' => "पाउंड",
                '€' => "यूरो",
                _ => "रुपये",
            }
        } else if l.is_english() {
            match sym {
                '$' => "dollars",
                '£' => "pounds",
                '€' => "euros",
                _ => "rupees",
            }
        } else {
            rupee_word(l).unwrap_or("")
        };
        if !name.is_empty() {
            words.push(' ');
            words.push_str(name);
        }
        words
    }

    /// The number/symbol expansion of one chunk by language `l`.
    fn expand(&self, c: &Chunk, l: Lang) -> Vec<Item> {
        let improved = self.opt.mode == Mode::Improved;
        let prev = (!self.prev.is_empty()).then_some(self.prev.as_str());
        if l.is_english() {
            let a = ascii_digits(&c.text);
            let indian = match self.opt.en_scale {
                EnScale::Indian => true,
                EnScale::International => false,
                EnScale::Auto => !self.lang.is_english() || indian_grouping(&a),
            };
            return crate::norm::expand_opts(c, l, Normalize::Natural, improved, indian);
        }
        if let Some(lat) = l.latin() {
            let items = crate::gnorm::expand(c, &lat.numbers, false, !improved, true, prev);
            return written_latin(items, &lat.numbers);
        }
        // Sanskrit and Dogri read numbers in Hindi in compatible mode (as
        // the reference does) and in their own words in improved mode.
        if l.hindi_like() || (l.hindi_voice() && !improved) || l.indic().is_none() {
            let mut items = crate::norm::expand_with(c, Lang::Hi, Normalize::Natural, improved);
            if matches!(l, Lang::Bho | Lang::Mai) {
                crate::bho::adapt_numbers(&mut items, l);
            }
            return items;
        }
        let g = l.indic().expect("checked above");
        // Written number words where the language has them.
        let nums = g.numbers_text.as_ref().unwrap_or(&g.numbers);
        let mut items = crate::gnorm::expand(c, nums, false, false, true, prev);
        // Languages whose number table holds keys of the word table rather
        // than words (si, sd) write them by `[numbers.written]`.
        if g.numbers_text.is_none() {
            for it in items.iter_mut().filter(|i| i.generated) {
                it.text = nums.written(&it.text);
            }
        }
        // Currency signs the generic reader leaves out.
        if c.text.starts_with('₹') {
            if let Some(w) = rupee_word(l) {
                items.push(Item::word(w));
            }
        }
        items
    }

    /// Items as text: spelled items become letters separated by spaces.
    fn join(&mut self, items: &[Item], l: Lang) -> String {
        let mut words: Vec<String> = Vec::new();
        for it in items {
            if it.text.is_empty() {
                continue;
            }
            let latin = chunk_script(&it.text) == Some(Script::Latin);
            if it.spell {
                words.push(self.spell(&it.text, latin));
            } else if latin && self.other_script() && it.generated && l.is_english() {
                words.push(self.latin_word(&it.text, true));
            } else if latin && self.other_script() && !it.generated {
                words.push(self.latin_word(&it.text, false));
            } else {
                words.push(it.text.clone());
            }
        }
        words.join(" ")
    }

    /// Letters of an acronym, as Latin letters or letter names.
    fn spell(&mut self, s: &str, latin: bool) -> String {
        let letters: Vec<char> = s.chars().filter(|c| c.is_alphanumeric()).collect();
        if !latin || !self.other_script() || self.opt.latin == LatinPolicy::Keep {
            return letters
                .iter()
                .map(|c| c.to_uppercase().to_string())
                .collect::<Vec<_>>()
                .join(" ");
        }
        letters
            .iter()
            .map(|&c| self.letter_name(c))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn letter_name(&mut self, c: char) -> String {
        if self.lang.script() == Script::Devanagari {
            if let Some(n) = respell::letter_devanagari(c) {
                return n.to_string();
            }
        }
        if let Some(ipa) = respell::letter_ipa(c) {
            let key = format!("\u{1}{c}");
            if let Some(r) = self.cached(&key, ipa, Some(Lang::EnUs)) {
                return r;
            }
        }
        c.to_string()
    }

    fn cached(&mut self, key: &str, ipa: &str, source: Option<Lang>) -> Option<String> {
        if let Some(r) = self.cache.get(key) {
            return r.clone();
        }
        let r = respell::respell(self.ph, ipa, source, self.lang, Style::Native).map(|r| r.text);
        self.cache.insert(key.to_string(), r.clone());
        r
    }

    /// A Latin-script word inside a text of another script, by the policy.
    fn latin_word(&mut self, w: &str, generated: bool) -> String {
        let _ = generated;
        match self.opt.latin {
            LatinPolicy::Keep => w.to_string(),
            LatinPolicy::Spell => self.spell(w, true),
            LatinPolicy::Transliterate => {
                let caps = w.chars().filter(|c| c.is_alphabetic()).count();
                let all_caps = caps >= 2 && w.chars().all(|c| !c.is_lowercase());
                if all_caps && (caps <= 4 || !w.chars().any(|c| "AEIOUY".contains(c))) {
                    return self.spell(w, true);
                }
                let ipa = english_ipa(self.ph, w);
                self.cached(&w.to_lowercase(), &ipa, Some(Lang::EnUs))
                    .unwrap_or_else(|| w.to_string())
            }
        }
    }

    /// The spoken form of a word-list entry.
    fn entry_text(&mut self, e: &wordlist::Entry, matched: &str) -> String {
        if let Some(s) = &e.say {
            let latin = chunk_script(s) == Some(Script::Latin);
            if latin && !self.lang.is_english() && self.opt.latin == LatinPolicy::Transliterate {
                let ipa = english_ipa(self.ph, s);
                if let Some(r) = self.cached(&format!("\u{2}{s}"), &ipa, Some(Lang::EnUs)) {
                    return r;
                }
            }
            return s.clone();
        }
        if let Some(ipa) = &e.ipa {
            // An English entry inside another language's text keeps Latin
            // script unless Latin words are respelled.
            let target = if e.lang.is_some_and(|l| l.is_english())
                && !self.lang.is_english()
                && self.opt.latin == LatinPolicy::Keep
            {
                Lang::EnUs
            } else {
                self.lang
            };
            let style = if target.is_english() && self.opt.respell == Style::Native {
                Style::LatinPlain
            } else {
                self.opt.respell
            };
            if let Some(r) = respell::respell(self.ph, ipa, e.lang, target, style) {
                return r.text;
            }
        }
        matched.to_string()
    }

    /// Normalise one chunk on its own.
    fn chunk(&mut self, full: &str, src: Range<usize>, ch: &Chunk, dl: Lang) -> Piece {
        let t = ch.text.as_str();
        let raw = &full[src.clone()];
        // URLs and email addresses.
        if let Some(kind) = url_kind(t) {
            let out = match self.opt.urls {
                UrlPolicy::Keep => raw.to_string(),
                UrlPolicy::Drop => String::new(),
                UrlPolicy::Read => self.read_url(t, kind, dl),
            };
            let kind = if self.opt.urls == UrlPolicy::Drop {
                SpanKind::Dropped
            } else {
                kind
            };
            return self.piece(src, out, kind, dl, None);
        }
        let digits = has_digit(t);
        let script = chunk_script(t);
        let latin_in_other = script == Some(Script::Latin) && self.other_script();
        let l = if digits {
            dl
        } else if latin_in_other {
            Lang::EnGb
        } else {
            self.lang
        };
        let items = self.expand(ch, l);
        let out = self.join(&items, l);
        // The number as written in English (no group marks, decimal point).
        let mut tc = t.replace(crate::gnorm::GROUPED, "");
        if l.latin().is_some_and(|x| x.numbers.decimal_comma) {
            tc = tc
                .chars()
                .map(|c| match c {
                    ',' => '.',
                    '.' => ',',
                    c => c,
                })
                .collect();
        }
        // A word of a Latin-script language without digits or signs that
        // are read stays as written (hyphens, apostrophes, `&`).
        let plain_latin =
            !digits && l.latin().is_some() && !items.iter().any(|i| i.generated || i.spell);
        let same = plain_latin
            || out == t
            || (out.split_whitespace().collect::<Vec<_>>()
                == t.split_whitespace().collect::<Vec<_>>());
        let kind = if !digits && same {
            if latin_in_other {
                if self.opt.latin == LatinPolicy::Keep {
                    SpanKind::Latin
                } else {
                    SpanKind::Transliterated
                }
            } else {
                SpanKind::Text
            }
        } else if digits {
            classify(&tc, l, &items)
        } else if items.iter().any(|i| i.spell) {
            SpanKind::Acronym
        } else if latin_in_other && self.opt.latin != LatinPolicy::Keep {
            SpanKind::Transliterated
        } else if t.chars().any(|c| c.is_alphabetic()) {
            SpanKind::Abbreviation
        } else {
            SpanKind::Symbol
        };
        let mut out = if kind == SpanKind::Text || kind == SpanKind::Latin {
            // Keep the original exactly (joiners, case).
            t.to_string()
        } else {
            out
        };
        // A capitalised abbreviation stays capitalised ("Dr." -> "Doctor").
        if kind == SpanKind::Abbreviation && t.chars().next().is_some_and(char::is_uppercase) {
            let mut cs = out.chars();
            if let Some(f) = cs.next() {
                out = f.to_uppercase().chain(cs).collect();
            }
        }
        if out.is_empty() && !t.is_empty() {
            return self.piece(src, out, SpanKind::Dropped, l, None);
        }
        // A dot that belonged to the chunk was consumed: keep it when the
        // chunk was not an abbreviation and the dot ended a sentence.
        let value = matches!(kind, SpanKind::Number).then_some(tc.as_str());
        let mut p = self.piece(src, out, kind, l, value);
        if matches!(kind, SpanKind::Text | SpanKind::Latin) && raw.len() > t.len() {
            p.out = raw.to_string();
        }
        p
    }

    fn read_url(&mut self, t: &str, kind: SpanKind, l: Lang) -> String {
        let deva = self.lang.script() == Script::Devanagari;
        let word = |en: &'static str| -> &'static str {
            if !deva {
                return en;
            }
            match en {
                "dot" => "डॉट",
                "slash" => "स्लैश",
                "at" => "ऐट द रेट",
                "dash" => "डैश",
                "underscore" => "अंडरस्कोर",
                "colon" => "कोलन",
                "question mark" => "क्वेश्चन मार्क",
                "equals" => "इक्वल्स",
                "and" => "ऐंड",
                "hash" => "हैश",
                other => other,
            }
        };
        let mut s = t;
        for p in ["https://", "http://", "HTTPS://", "HTTP://"] {
            if let Some(r) = s.strip_prefix(p) {
                s = r;
            }
        }
        let s = s.trim_end_matches('/');
        let mut parts: Vec<String> = Vec::new();
        let mut cur = String::new();
        let flush = |cur: &mut String, parts: &mut Vec<String>, me: &mut Self| {
            if cur.is_empty() {
                return;
            }
            let w = std::mem::take(cur);
            if w.bytes().all(|b| b.is_ascii_digit()) {
                let en = if me.lang.is_english() { me.lang } else { l };
                for d in w.chars() {
                    parts.push(me.read_number(&d.to_string(), en));
                }
            } else if !w.chars().any(|c| "aeiouyAEIOUY".contains(c)) {
                let letters = me.spell(&w, true);
                parts.push(if letters.is_ascii() {
                    letters.to_lowercase()
                } else {
                    letters
                });
            } else if me.other_script() {
                parts.push(me.latin_word(&w, false));
            } else {
                parts.push(w);
            }
        };
        for c in s.chars() {
            let sep = match c {
                '.' => Some("dot"),
                '/' => Some("slash"),
                '@' => Some("at"),
                '-' => Some("dash"),
                '_' => Some("underscore"),
                ':' => Some("colon"),
                '?' => Some("question mark"),
                '=' => Some("equals"),
                '&' => Some("and"),
                '#' => Some("hash"),
                _ => None,
            };
            match sep {
                Some(w) => {
                    flush(&mut cur, &mut parts, self);
                    let w = word(w);
                    if deva || self.lang.is_english() {
                        parts.push(w.to_string());
                    } else {
                        parts.push(self.latin_word(w, false));
                    }
                }
                None => cur.push(c),
            }
        }
        flush(&mut cur, &mut parts, self);
        let _ = kind;
        parts.join(" ")
    }
}

/// The written form of the number words of a Latin-script language
/// ([`crate::langdef::Numbers::written`]), with the words that the language
/// writes as one joined: the hundred of `glue-hundred` (not where hyphens
/// are kept), the tens of `swap-tens` ("dreiundzwanzig") and the words of
/// `written-glue` and `written-glue-after`; scale words take the plural of
/// `written-plural`.
fn written_latin(items: Vec<Item>, nums: &crate::langdef::Numbers) -> Vec<Item> {
    let mut out: Vec<Item> = Vec::with_capacity(items.len());
    let mut prev_number = false;
    for mut it in items {
        if !it.generated {
            prev_number = false;
            out.push(it);
            continue;
        }
        let mut w = nums.written(&it.text);
        if prev_number {
            if let Some(pl) = out.last().and_then(|p| nums.written_plural(&w, &p.text)) {
                w = pl.to_string();
            }
        }
        let swapped = nums
            .swap_tens
            .as_ref()
            .is_some_and(|link| it.text.starts_with(link.as_str()) && it.text != *link);
        let hundred = it.glue && nums.glued_hundred(&it.text);
        let after = out.last().is_some_and(|p| {
            nums.written_glue_after
                .iter()
                .any(|g| p.text.ends_with(g.as_str()))
        });
        let number = swapped || nums.is_number_word(&it.text);
        let joins = hundred || swapped || after || nums.written_glue.contains(&w);
        if prev_number && number && joins {
            if let Some(p) = out.last_mut() {
                if !(hundred && nums.written_hyphen) {
                    p.text.push_str(&w);
                    continue;
                }
            }
        }
        // "deux cents" + "mille": "deux cent mille".
        if prev_number && number && !nums.is_plural_scale(&w) {
            if let Some(p) = out.last_mut() {
                for end in &nums.written_drop_s {
                    if p.text.ends_with(end.as_str()) && end.ends_with('s') {
                        p.text.pop();
                        break;
                    }
                }
            }
        }
        it.text = w;
        it.glue = false;
        prev_number = number;
        out.push(it);
    }
    // Replacements that need the joined word ("einstausend").
    for it in &mut out {
        if it.generated {
            it.text = nums.replace_written(&it.text);
        }
    }
    out
}

/// English IPA of a word for respelling in an Indian script: British
/// vowels with the r of the American reading (Indian English is rhotic).
fn english_ipa(ph: &Phonemizer, w: &str) -> String {
    let read = |l: Lang| {
        let mut o = Options::new(l);
        o.mode = Mode::Improved;
        let mut parts: Vec<String> = Vec::new();
        for s in ph.analyze(w, &o) {
            for c in s.clauses {
                for wd in c.words {
                    parts.push(wd.ipa);
                }
            }
        }
        parts.join(" ")
    };
    respell::restore_r(&read(Lang::EnGb), &read(Lang::EnUs))
}

fn indian_grouping(a: &str) -> bool {
    let int = a.split('.').next().unwrap_or("");
    let digits: String = int
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit() || *c == ',')
        .collect();
    let groups: Vec<&str> = digits.split(',').collect();
    groups.len() >= 3
        && groups.last().is_some_and(|g| g.len() == 3)
        && groups[1..groups.len() - 1].iter().all(|g| g.len() == 2)
}

pub(crate) fn rupee_word(l: Lang) -> Option<&'static str> {
    Some(match l {
        Lang::Mr | Lang::Kok => "रुपये",
        Lang::Ne => "रुपैयाँ",
        Lang::Bn => "টাকা",
        Lang::As => "টকা",
        Lang::Gu => "રૂપિયા",
        Lang::Pa => "ਰੁਪਏ",
        Lang::Or => "ଟଙ୍କା",
        Lang::Te => "రూపాయలు",
        Lang::Kn => "ರೂಪಾಯಿ",
        Lang::Ta => "ரூபாய்",
        Lang::Ml => "രൂപ",
        Lang::Si => "රුපියල්",
        Lang::Ur => "روپے",
        Lang::Sd => "رپيا",
        _ => return None,
    })
}

/// "1.5" -> डेढ़, "2.5" -> ढाई, "3.5" -> साढ़े तीन, "4.25" -> सवा चार,
/// "4.75" -> पौने पाँच (before a scale word or for a clock time).
fn hindi_fraction(a: &str) -> Option<String> {
    let (i, f) = a.split_once('.')?;
    let n: u64 = i.parse().ok()?;
    if n >= 100 {
        return None;
    }
    let num = |k: u64| crate::norm::HI_NUM[k as usize].to_string();
    Some(match (n, f) {
        (1, "5") => "डेढ़".into(),
        (2, "5") => "ढाई".into(),
        (n, "5") if n >= 3 => format!("साढ़े {}", num(n)),
        (n, "25") if n >= 1 => format!("सवा {}", num(n)),
        (n, "75") if n < 99 => format!("पौने {}", num(n + 1)),
        _ => return None,
    })
}

/// A clock time written before बजे: "10:30" -> साढ़े दस, "4:15" -> सवा चार,
/// "4:45" -> पौने पाँच, "9:00" -> नौ, each followed by the बजे of the text
/// (`true`); other minutes, "3:20" -> तीन बजकर बीस मिनट, which takes the
/// place of that बजे (`false`).
pub(crate) fn hindi_clock(a: &str) -> Option<(String, bool)> {
    let (h, m) = a.split_once(':')?;
    let h: u64 = h.parse().ok()?;
    let mm: u64 = m.parse().ok()?;
    if h > 24 || m.len() != 2 || !m.bytes().all(|b| b.is_ascii_digit()) || mm >= 60 {
        return None;
    }
    let num = |k: u64| crate::norm::HI_NUM[k as usize].to_string();
    Some(match m {
        "00" => (num(h), true),
        "30" if h == 1 => ("डेढ़".into(), true),
        "30" if h == 2 => ("ढाई".into(), true),
        "30" => (format!("साढ़े {}", num(h)), true),
        "15" => (format!("सवा {}", num(h)), true),
        "45" => (format!("पौने {}", num(h % 12 + 1)), true),
        _ => (format!("{} बजकर {} मिनट", num(h), num(mm)), false),
    })
}

/// URL or email address.
fn url_kind(t: &str) -> Option<SpanKind> {
    let lower = t.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("www.") {
        return Some(SpanKind::Url);
    }
    if !t.is_ascii() {
        return None;
    }
    if let Some((local, domain)) = t.split_once('@') {
        if !local.is_empty()
            && domain.contains('.')
            && !domain.starts_with('.')
            && !domain.ends_with('.')
            && local
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "._-+".contains(c))
            && domain
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || ".-".contains(c))
        {
            return Some(SpanKind::Email);
        }
        return None;
    }
    // A bare domain with a common top-level domain: example.com/path.
    let host = lower.split('/').next().unwrap_or("");
    let tld = host.rsplit('.').next().unwrap_or("");
    if host.contains('.')
        && host.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ".-".contains(c))
        && matches!(
            tld,
            "com"
                | "org"
                | "net"
                | "in"
                | "io"
                | "gov"
                | "edu"
                | "co"
                | "uk"
                | "ai"
                | "dev"
                | "app"
        )
    {
        return Some(SpanKind::Url);
    }
    None
}

/// The class of a chunk that contains digits.
fn classify(t: &str, l: Lang, items: &[Item]) -> SpanKind {
    // Dates and ordinals of the languages with `[dates]` and ordinal
    // rules: a month name among the words; only written words (no number
    // words) from a number with an ordinal ending.
    let dates = l
        .latin()
        .map(|x| &x.numbers)
        .or_else(|| l.indic().map(|x| &x.numbers))
        .and_then(|n| n.dates.as_ref());
    if let Some(d) = dates {
        let month = |w: &str| d.months.iter().any(|m| m.split(' ').any(|p| p == w));
        if t.matches(['/', '.', '-', ',']).count() == 2 && items.iter().any(|i| month(&i.text)) {
            return SpanKind::Date;
        }
    }
    if l.latin()
        .is_some_and(|x| crate::gnorm::is_time(t, &x.numbers, None))
    {
        return SpanKind::Time;
    }
    if l.latin().is_some()
        && !items.is_empty()
        && items.iter().all(|i| !i.generated)
        && t.starts_with(|c: char| c.is_ascii_digit())
    {
        return SpanKind::Ordinal;
    }
    let a = ascii_digits(t);
    let lower = a.to_lowercase();
    let first = a.chars().next().unwrap_or(' ');
    if matches!(first, '$' | '£' | '€' | '₹') {
        return SpanKind::Money;
    }
    if lower.ends_with('%') {
        return SpanKind::Percent;
    }
    let b = a.as_bytes();
    if a.contains(':') && b.first().is_some_and(u8::is_ascii_digit) {
        return SpanKind::Time;
    }
    if a.matches('/').count() == 2
        || a.matches('.').count() == 2
        || (a.matches('-').count() == 2 && a.len() == 10)
    {
        // A date has a month name; a phone number is read digit by digit.
        const EN: [&str; 12] = [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ];
        const HI: [&str; 12] = [
            "जनवरी",
            "फ़रवरी",
            "मार्च",
            "अप्रैल",
            "मई",
            "जून",
            "जुलाई",
            "अगस्त",
            "सितंबर",
            "अक्टूबर",
            "नवंबर",
            "दिसंबर",
        ];
        if items
            .iter()
            .any(|i| EN.contains(&i.text.as_str()) || HI.contains(&i.text.as_str()))
        {
            return SpanKind::Date;
        }
    }
    if a.matches('/').count() == 1 && items.len() <= 3 {
        return SpanKind::Fraction;
    }
    let digits_only = |s: &str| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit());
    if a.contains('-') && a.trim_start_matches(['-', '+']).contains('-') {
        let groups: Vec<&str> = a.trim_start_matches('+').split('-').collect();
        if groups.iter().all(|g| digits_only(g)) {
            let total: usize = groups.iter().map(|g| g.len()).sum();
            return if groups.len() >= 3 || total >= 7 {
                SpanKind::Phone
            } else {
                SpanKind::Range
            };
        }
    }
    if a.starts_with('+') && a.len() > 10 {
        return SpanKind::Phone;
    }
    let body = a.trim_start_matches(['-', '+']);
    let split = body
        .find(|c: char| !(c.is_ascii_digit() || c == ',' || c == '.'))
        .unwrap_or(body.len());
    let (num, tail) = body.split_at(split);
    let tail_l = tail.to_lowercase();
    if !tail.is_empty() {
        if matches!(
            tail_l.as_str(),
            "st" | "nd" | "rd" | "th" | "वां" | "वाँ" | "वीं" | "वें"
        ) {
            return SpanKind::Ordinal;
        }
        if matches!(tail_l.as_str(), "s" | "'s") {
            return SpanKind::Year;
        }
        return SpanKind::Measure;
    }
    if num.len() >= 10 && digits_only(num) && l.hindi_like() {
        return SpanKind::Phone;
    }
    if num.len() > 1 && num.starts_with('0') && !num.contains('.') {
        return SpanKind::Digits;
    }
    if num.contains('.') {
        return SpanKind::Decimal;
    }
    if l.is_english()
        && num.len() == 4
        && digits_only(num)
        && (1100..2100).contains(&num.parse::<u32>().unwrap_or(0))
        && !a.starts_with('-')
    {
        return SpanKind::Year;
    }
    if items.len() > 1 && digits_only(num) && num.len() >= 10 {
        return SpanKind::Phone;
    }
    SpanKind::Number
}

// ---------------------------------------------------------------------------
// Inverse reading, for tests: spoken number words back to a value.

/// Read English or Hindi number words back to a value (cardinals only):
/// the check that a spoken form says the same number as the digits.
pub fn spoken_number(words: &str, lang: Lang) -> Option<u64> {
    let ws: Vec<String> = words
        .split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| {
                !c.is_alphanumeric() && !('\u{0900}'..='\u{097f}').contains(&c)
            })
            .to_lowercase()
        })
        .filter(|w| !w.is_empty() && w != "and")
        .collect();
    if ws.is_empty() {
        return None;
    }
    let mut total: u64 = 0;
    let mut cur: u64 = 0;
    let mut last_scale: u64 = 0;
    let mut any = false;
    for w in &ws {
        let w = w.replace('\u{093c}', "").replace('ँ', "ं");
        let small = if lang.is_english() {
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
                "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty",
                "ninety",
            ];
            ONES.iter()
                .position(|x| *x == w)
                .map(|v| v as u64)
                .or_else(|| {
                    TENS.iter()
                        .position(|x| !x.is_empty() && *x == w)
                        .map(|v| v as u64 * 10)
                })
        } else {
            crate::norm::HI_NUM
                .iter()
                .position(|x| x.replace('\u{093c}', "").replace('ँ', "ं") == w)
                .map(|v| v as u64)
        };
        if let Some(v) = small {
            cur += v;
            any = true;
            continue;
        }
        let scale: u64 = match w.as_str() {
            "hundred" | "सौ" => {
                cur = cur.max(1) * 100;
                any = true;
                continue;
            }
            "thousand" | "हजार" => 1000,
            "lakh" | "लाख" => 100_000,
            "million" => 1_000_000,
            "crore" | "करोड" => 10_000_000,
            "billion" | "अरब" => 1_000_000_000,
            "खरब" => 100_000_000_000,
            "trillion" => 1_000_000_000_000,
            _ => return None,
        };
        if last_scale != 0 && last_scale < scale {
            // "ten thousand crore": the larger scale multiplies all before it.
            total = (total + cur.max(if total == 0 { 1 } else { 0 })) * scale;
        } else {
            total += cur.max(1) * scale;
        }
        last_scale = scale;
        cur = 0;
        any = true;
    }
    any.then_some(total + cur)
}

// ---------------------------------------------------------------------------
// Options and results as JSON (for the C, Python and WebAssembly bindings).

/// Parse options from a JSON object: `{"mode": "improved", "digits":
/// "context"|"request"|<lang>, "en_scale": "auto"|"indian"|"international",
/// "latin": "keep"|"transliterate"|"spell", "urls": "read"|"keep"|"drop",
/// "respell": "native"|"latin-hyphen"|"latin-plain", "wordlists": [<TSV,
/// PLS or JSON text>, ...]}`. Every member is optional.
pub fn options_from_json(json: &str) -> Result<NormOptions, String> {
    let mut o = NormOptions::default();
    if json.trim().is_empty() {
        return Ok(o);
    }
    let v = crate::minijson::parse(json)?;
    let s = |k: &str| v.get(k).and_then(|x| x.as_str());
    if let Some(m) = s("mode") {
        o.mode = match m {
            "improved" => Mode::Improved,
            "compatible" | "compat" => Mode::Compatible,
            other => return Err(format!("unknown mode {other:?}")),
        };
    }
    if let Some(b) = v.get("improved").and_then(|x| x.as_bool()) {
        o.mode = if b { Mode::Improved } else { Mode::Compatible };
    }
    if let Some(d) = s("digits") {
        o.digits = match d {
            "context" | "" => DigitLang::Context,
            "request" => DigitLang::Request,
            other => DigitLang::Fixed(
                Lang::parse(other).ok_or_else(|| format!("unknown digits language {other:?}"))?,
            ),
        };
    }
    if let Some(e) = s("en_scale") {
        o.en_scale = match e {
            "auto" | "" => EnScale::Auto,
            "indian" | "lakh" => EnScale::Indian,
            "international" | "western" => EnScale::International,
            other => return Err(format!("unknown en_scale {other:?}")),
        };
    }
    if let Some(l) = s("latin") {
        o.latin = match l {
            "keep" | "" => LatinPolicy::Keep,
            "transliterate" => LatinPolicy::Transliterate,
            "spell" => LatinPolicy::Spell,
            other => return Err(format!("unknown latin policy {other:?}")),
        };
    }
    if let Some(u) = s("urls") {
        o.urls = match u {
            "read" | "" => UrlPolicy::Read,
            "keep" => UrlPolicy::Keep,
            "drop" => UrlPolicy::Drop,
            other => return Err(format!("unknown urls policy {other:?}")),
        };
    }
    if let Some(r) = s("respell") {
        o.respell = Style::parse(r).ok_or_else(|| format!("unknown respell style {r:?}"))?;
    }
    if let Some(crate::minijson::Value::Arr(lists)) = v.get("wordlists") {
        for (i, l) in lists.iter().enumerate() {
            let t = l.as_str().ok_or("wordlists must be strings")?;
            let wl = WordList::parse_any(t).map_err(|e| format!("wordlists[{i}]: {e}"))?;
            o.wordlists.push(Arc::new(wl));
        }
    }
    Ok(o)
}

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

impl Normalized {
    /// The result as JSON: `{"text", "sentences": [{"start", "end"}],
    /// "spans": [{"kind", "lang", "src": {...}, "out": {...}, "text",
    /// "spoken", "entry"?, "value"?}]}` where each range gives `byte`,
    /// `char` (code point) and `utf16` start and end offsets.
    pub fn to_json(&self, input: &str) -> String {
        use crate::minijson::push_str;
        let (icp, iu) = offset_tables(input);
        let (ocp, ou) = offset_tables(&self.text);
        let range = |out: &mut String, r: &Range<usize>, cp: &[usize], u: &[usize]| {
            out.push_str(&format!(
                "{{\"byte\":[{},{}],\"char\":[{},{}],\"utf16\":[{},{}]}}",
                r.start, r.end, cp[r.start], cp[r.end], u[r.start], u[r.end]
            ));
        };
        let mut o = String::from("{\"text\":");
        push_str(&mut o, &self.text);
        o.push_str(",\"sentences\":[");
        for (i, s) in self.sentences.iter().enumerate() {
            if i > 0 {
                o.push(',');
            }
            range(&mut o, s, &ocp, &ou);
        }
        o.push_str("],\"spans\":[");
        for (i, s) in self.spans.iter().enumerate() {
            if i > 0 {
                o.push(',');
            }
            o.push_str("{\"kind\":");
            push_str(&mut o, s.kind.name());
            o.push_str(",\"lang\":");
            push_str(&mut o, s.lang.tag());
            o.push_str(",\"src\":");
            range(&mut o, &s.src, &icp, &iu);
            o.push_str(",\"out\":");
            range(&mut o, &s.out, &ocp, &ou);
            o.push_str(",\"text\":");
            push_str(&mut o, &input[s.src.clone()]);
            o.push_str(",\"spoken\":");
            push_str(&mut o, &self.text[s.out.clone()]);
            if let Some((l, e)) = s.entry {
                o.push_str(&format!(",\"entry\":[{l},{e}]"));
            }
            if let Some(v) = &s.value {
                o.push_str(",\"value\":");
                push_str(&mut o, v);
            }
            o.push('}');
        }
        o.push_str("]}");
        o
    }
}

/// [`normalize`] with JSON options in and a JSON result out (see
/// [`options_from_json`] and [`Normalized::to_json`]).
pub fn normalize_json(
    ph: &Phonemizer,
    text: &str,
    lang: Lang,
    options: &str,
) -> Result<String, String> {
    let o = options_from_json(options)?;
    Ok(normalize(ph, text, lang, &o).to_json(text))
}

// ---------------------------------------------------------------------------
// Word lists in phonemisation.

const PLACEHOLDER_LETTERS: &[u8] = b"bcfhjkmpqtvwxz";

fn placeholder(k: usize) -> String {
    let mut s = String::from("qxzv");
    let mut n = k;
    loop {
        s.push(PLACEHOLDER_LETTERS[n % PLACEHOLDER_LETTERS.len()] as char);
        n /= PLACEHOLDER_LETTERS.len();
        if n == 0 {
            break;
        }
    }
    s.push_str("zq");
    s
}

/// Phonemise with word lists: matched words get the entry's IPA (rendered
/// in the profile's phone set like any other word), or the phonemes of its
/// spoken form when it has no IPA. The output has the shape of
/// [`Phonemizer::render`].
pub fn render_with_wordlists(
    ph: &Phonemizer,
    text: &str,
    opt: &Options,
    prof: crate::profile::Profile,
    punctuation: bool,
    lists: &[Arc<WordList>],
) -> String {
    let refs: Vec<&WordList> = lists.iter().map(|l| l.as_ref()).collect();
    let hits = wordlist::select(&refs, text, opt.lang);
    if hits.is_empty() {
        return ph.render(text, opt, prof, punctuation);
    }
    // Replace matches with placeholders the pipeline passes through as one
    // word, then put the entry's phonemes in their place.
    let mut s = String::new();
    let mut ipa_of: Vec<(String, String, Lang)> = Vec::new();
    let mut pos = 0;
    for h in &hits {
        s.push_str(&text[pos..h.start]);
        let e = &refs[h.list].entries[h.entry];
        match (&e.ipa, &e.say) {
            (Some(ipa), _) => {
                let p = placeholder(ipa_of.len());
                s.push_str(&p);
                ipa_of.push((p, ipa.clone(), e.lang.unwrap_or(opt.lang)));
            }
            (None, Some(say)) => s.push_str(say),
            (None, None) => s.push_str(&text[h.start..h.end]),
        }
        pos = h.end;
    }
    s.push_str(&text[pos..]);
    if prof == crate::profile::Profile::Kokoro && opt.lang.is_english() && ph.has_misaki(opt.lang) {
        // Kokoro's English front end: substitute in its token stream.
        if let Some(mut k) = ph.kokoro_english(&s, opt.lang) {
            for (p, ipa, _) in &ipa_of {
                let m = crate::kokoro::espeak_style_to_misaki(ipa, opt.lang == Lang::EnGb);
                let guess = ph.kokoro_english(p, opt.lang).unwrap_or_default();
                let guess = guess.trim();
                if !guess.is_empty() {
                    k = k.replacen(guess, &m, 1);
                }
            }
            return k;
        }
    }
    let mut sentences = ph.analyze(&s, opt);
    for sent in &mut sentences {
        for c in &mut sent.clauses {
            for w in &mut c.words {
                if let Some((_, ipa, l)) = ipa_of
                    .iter()
                    .find(|(p, _, _)| w.text.starts_with(p.as_str()))
                {
                    w.ipa = ipa.replace([' ', '-'], "");
                    w.lang = *l;
                    w.text = text_placeholder_restore(&w.text);
                }
            }
            // Placeholders split into pieces by the reader are not expected,
            // but never let one leak out.
        }
    }
    match prof {
        crate::profile::Profile::EspeakIpa => crate::profile::espeak_ipa(
            &sentences,
            punctuation,
            opt.keep_language_flags.then_some(opt.lang),
        ),
        other => crate::profile::render_text(&sentences, other, opt.lang, punctuation),
    }
}

fn text_placeholder_restore(t: &str) -> String {
    t.to_string()
}
