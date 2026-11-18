//! Text segmentation: sentences → clauses → raw tokens, plus script detection.
//!
//! The segmenter keeps the clause terminators because Piper-style voices were
//! trained with them (`,` `.` `?` `!` `:` `;` are tokens of the model).

/// How a clause ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Terminator {
    /// No punctuation (end of input, a dash, a line break).
    #[default]
    None,
    /// `,`
    Comma,
    /// `:`
    Colon,
    /// `;`
    Semicolon,
    /// `.` (also the danda `।` and an ellipsis)
    Period,
    /// `?`
    Question,
    /// `!`
    Exclamation,
}

impl Terminator {
    /// The punctuation character, if any.
    pub fn as_char(self) -> Option<char> {
        match self {
            Terminator::None => None,
            Terminator::Comma => Some(','),
            Terminator::Colon => Some(':'),
            Terminator::Semicolon => Some(';'),
            Terminator::Period => Some('.'),
            Terminator::Question => Some('?'),
            Terminator::Exclamation => Some('!'),
        }
    }

    /// True for `.`, `?` and `!`.
    pub fn ends_sentence(self) -> bool {
        matches!(
            self,
            Terminator::Period | Terminator::Question | Terminator::Exclamation
        )
    }

    fn from_char(c: char) -> Option<Terminator> {
        Some(match c {
            ',' | '،' | '、' | '\u{aaf0}' => Terminator::Comma,
            ':' => Terminator::Colon,
            ';' | '؛' => Terminator::Semicolon,
            '.' | '।' | '॥' | '…' | '。' | '۔' | '\u{1c7e}' | '\u{1c7f}' | '\u{abeb}' => {
                Terminator::Period
            }
            '?' | '？' | '؟' | '\u{aaf1}' => Terminator::Question,
            '!' | '！' => Terminator::Exclamation,
            _ => return None,
        })
    }
}

/// Writing system of a token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Script {
    /// Basic Latin and Latin letters with diacritics.
    Latin,
    /// Devanagari (U+0900–U+097F).
    Devanagari,
    /// Another Brahmic script block (Bengali U+0980, Gurmukhi U+0A00, …),
    /// named by the first code point of its block.
    Brahmic(u32),
    /// ASCII or Indic-script digits.
    Digit,
    /// Anything else.
    Other,
}

/// Script of one character.
pub fn script_of(c: char) -> Script {
    match c {
        'a'..='z' | 'A'..='Z' | '\u{00c0}'..='\u{024f}' | '\u{1e00}'..='\u{1eff}' => Script::Latin,
        '0'..='9' => Script::Digit,
        '\u{0900}'..='\u{0dff}' if indic_digit(c).is_some() => Script::Digit,
        '\u{0900}'..='\u{097f}' | '\u{200c}' | '\u{200d}' => Script::Devanagari,
        '\u{0980}'..='\u{0dff}' => Script::Brahmic(c as u32 & !0x7f),
        // Ol Chiki (Santali): digits, the two mucaad stops, letters and marks.
        '\u{1c50}'..='\u{1c59}' => Script::Digit,
        '\u{1c7e}' | '\u{1c7f}' => Script::Other,
        '\u{1c5a}'..='\u{1c7d}' => Script::Brahmic(0x1c00),
        // Meetei Mayek (Manipuri): digits, cheikhei ꯫ and the extension
        // marks ꫰ ꫱ ꫲ ꫳ ꫴ, letters and signs of the block and its extensions.
        '\u{abf0}'..='\u{abf9}' => Script::Digit,
        '\u{abeb}' | '\u{aaf0}'..='\u{aaf4}' => Script::Other,
        '\u{abc0}'..='\u{abed}' | '\u{aae0}'..='\u{aaef}' | '\u{aaf5}' | '\u{aaf6}' => {
            Script::Brahmic(0xabc0)
        }
        // Arabic script (Urdu, Sindhi): one "block" for the whole range.
        '\u{0660}'..='\u{0669}' | '\u{06f0}'..='\u{06f9}' => Script::Digit,
        '\u{060c}' | '\u{061b}' | '\u{061f}' | '\u{06d4}' | '\u{066a}'..='\u{066d}' => {
            Script::Other
        }
        '\u{0600}'..='\u{06ff}'
        | '\u{0750}'..='\u{077f}'
        | '\u{fb50}'..='\u{fdff}'
        | '\u{fe70}'..='\u{feff}' => Script::Brahmic(0x0600),
        _ => Script::Other,
    }
}

/// Value of a digit of an Indic script (Devanagari to Sinhala blocks, Ol
/// Chiki, Meetei Mayek).
pub fn indic_digit(c: char) -> Option<u8> {
    let u = c as u32;
    if (0x1c50..=0x1c59).contains(&u) {
        return Some((u - 0x1c50) as u8);
    }
    if (0xabf0..=0xabf9).contains(&u) {
        return Some((u - 0xabf0) as u8);
    }
    // Arabic-Indic and Extended (Urdu/Persian) digits.
    if (0x0660..=0x0669).contains(&u) {
        return Some((u - 0x0660) as u8);
    }
    if (0x06f0..=0x06f9).contains(&u) {
        return Some((u - 0x06f0) as u8);
    }
    if !(0x0900..0x0e00).contains(&u) {
        return None;
    }
    let off = u & 0x7f;
    (0x66..=0x6f).contains(&off).then(|| (off - 0x66) as u8)
}

/// A whitespace-delimited chunk with its surrounding punctuation removed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chunk {
    /// The chunk text (never empty).
    pub text: String,
    /// True when a full stop directly followed the chunk and was not used as a
    /// clause terminator (an abbreviation such as `Dr.`).
    pub dot: bool,
    /// True when a no-break or thin space (U+00A0, U+2007, U+2009, U+202F)
    /// came before the chunk, not an ordinary space.
    pub tight: bool,
}

/// One clause of raw chunks.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RawClause {
    /// Chunks in order.
    pub chunks: Vec<Chunk>,
    /// How the clause ends.
    pub terminator: Terminator,
}

const TITLES: &[&str] = &[
    "mr",
    "mrs",
    "ms",
    "dr",
    "st",
    "prof",
    "sr",
    "jr",
    "vs",
    "etc",
    "no",
    "inc",
    "ltd",
    "co",
    "mt",
    "gen",
    "col",
    "lt",
    "sgt",
    "capt",
    "rev",
    "hon",
    "gov",
    "sen",
    "rep",
    "e.g",
    "i.e",
    "a.m",
    "p.m",
    "u.s",
    "u.k",
    "ph.d",
    "fig",
    "approx",
    "dept",
    "est",
    "jan",
    "feb",
    "mar",
    "apr",
    "jun",
    "jul",
    "aug",
    "sep",
    "sept",
    "oct",
    "nov",
    "dec",
    "डॉ",
    "श्री",
    "रु",
    "नं",
    "प्रो",
    "rs",
];

fn is_abbreviation(word: &str, next_starts_lower: bool) -> bool {
    let lower = word.to_lowercase();
    let lower = lower.trim_start_matches(|c: char| !c.is_alphanumeric());
    if TITLES.contains(&lower) {
        // "No." and "est." and month names only count before a number or a lower-case continuation.
        return true;
    }
    // Single capital letter: an initial (J. Smith).
    let mut it = word.chars();
    if let (Some(c), None) = (it.next(), it.next()) {
        if c.is_uppercase() {
            return true;
        }
    }
    // Dotted letters: U.S.A
    if word.contains('.') && word.split('.').all(|p| p.chars().count() == 1) {
        return true;
    }
    next_starts_lower
}

fn is_opener(c: char) -> bool {
    matches!(
        c,
        '"' | '\'' | '(' | '[' | '{' | '“' | '‘' | '«' | '<' | '*' | '_' | '`' | '¿' | '¡'
    )
}

fn is_closer(c: char) -> bool {
    matches!(
        c,
        '"' | '\'' | ')' | ']' | '}' | '”' | '’' | '»' | '>' | '*' | '_' | '`'
    )
}

/// Split text into clauses of raw chunks.
///
/// `break_after_abbreviation` reproduces the reference behaviour of ending a
/// clause after a dotted title (`Dr. Smith` → two clauses).
pub fn clauses(text: &str) -> Vec<RawClause> {
    clauses_with(text, &OrdinalContext::default())
}

/// Words that make a number with a full stop after it an ordinal (lower
/// case), for languages that write ordinals so.
#[derive(Clone, Debug, Default)]
pub struct OrdinalContext {
    /// Words before the number ("am 3.", "der 3.").
    pub before: Vec<String>,
    /// Month names before the number, where a date ends with the day
    /// (Hungarian "május 15."): the full stop may end the sentence too.
    pub month_before: Vec<String>,
    /// Words after the number (month names: "3. Mai"). A capitalised word
    /// that is not one of these starts a new sentence ("iPhone 15. Am …").
    pub after: Vec<String>,
}

impl OrdinalContext {
    /// True when the language has no such ordinals.
    pub fn is_empty(&self) -> bool {
        self.before.is_empty() && self.after.is_empty() && self.month_before.is_empty()
    }
}

/// [`clauses`], where a number with a full stop after it does not end the
/// sentence when the word before it is one of `ordinal_context.before` or
/// the word after it one of `ordinal_context.after` ("am 3. Mai").
pub fn clauses_with(text: &str, ordinal_context: &OrdinalContext) -> Vec<RawClause> {
    let mut out: Vec<RawClause> = Vec::new();
    let mut cur = RawClause::default();
    let flush = |cur: &mut RawClause, out: &mut Vec<RawClause>, t: Terminator| {
        if !cur.chunks.is_empty() {
            cur.terminator = t;
            out.push(std::mem::take(cur));
        } else if t.ends_sentence() {
            // Punctuation with nothing before it: attach to the previous clause if it had none.
            if let Some(last) = out.last_mut() {
                if last.terminator == Terminator::None {
                    last.terminator = t;
                }
            }
        }
    };
    for (li, line) in text.split('\n').enumerate() {
        if li > 0 && line.trim().is_empty() {
            // A blank line ends the sentence.
            let had = !cur.chunks.is_empty();
            flush(&mut cur, &mut out, Terminator::Period);
            let _ = had;
            continue;
        }
        let words: Vec<&str> = line.split_whitespace().collect();
        for (wi, raw) in words.iter().enumerate() {
            let at = raw.as_ptr() as usize - line.as_ptr() as usize;
            let tight = line[..at]
                .chars()
                .next_back()
                .is_some_and(|c| matches!(c, '\u{a0}' | '\u{2007}' | '\u{2009}' | '\u{202f}'));
            // Dashes standing alone separate clauses without a terminator symbol.
            if raw.chars().all(|c| matches!(c, '-' | '–' | '—' | '―')) {
                flush(&mut cur, &mut out, Terminator::Comma);
                continue;
            }
            let mut s: &str = raw.trim_start_matches(is_opener);
            // Trailing run of closers and terminators.
            let mut term = Terminator::None;
            let mut end = s.len();
            let mut dots = 0usize;
            for (i, c) in s.char_indices().rev() {
                if is_closer(c) && !(c == '\'' && i > 0 && s[..i].ends_with(['s', 'S']) && false) {
                    end = i;
                } else if let Some(t) = Terminator::from_char(c) {
                    // The first terminator of the run decides the type.
                    term = t;
                    if c == '.' {
                        dots += 1;
                    }
                    end = i;
                } else {
                    break;
                }
            }
            s = &s[..end];
            if s.is_empty() {
                flush(&mut cur, &mut out, term);
                continue;
            }
            let mut dot = false;
            if term == Terminator::Period && dots == 1 && raw.ends_with('.') {
                let next = words.get(wi + 1);
                let next_lower =
                    next.is_some_and(|n| n.chars().next().is_some_and(|c| c.is_lowercase()));
                let in_context = |w: Option<&&str>, list: &[String]| {
                    w.is_some_and(|w| {
                        let w = w
                            .trim_matches(|c: char| !c.is_alphanumeric())
                            .to_lowercase();
                        list.contains(&w)
                    })
                };
                let prev = if wi > 0 { words.get(wi - 1) } else { None };
                let digits = !ordinal_context.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
                let month_before = digits && in_context(prev, &ordinal_context.month_before);
                let ordinal = digits
                    && (in_context(next, &ordinal_context.after)
                        || in_context(prev, &ordinal_context.before)
                        || month_before);
                // A date that ends with the day, before a capitalised word
                // that is not a month: the full stop ends the sentence too
                // ("május 15. A …").
                let ends = month_before
                    && !in_context(prev, &ordinal_context.before)
                    && !in_context(next, &ordinal_context.after)
                    && next.is_some_and(|n| n.chars().next().is_some_and(char::is_uppercase));
                if ends {
                    dot = true;
                } else if next.is_some() && (ordinal || is_abbreviation(s, next_lower)) {
                    term = Terminator::None;
                    dot = true;
                } else if next.is_none()
                    && s.contains('.')
                    && s.split('.').all(|p| p.chars().count() == 1)
                {
                    dot = true;
                }
            }
            // Em/en dashes inside a chunk separate words.
            for (pi, part) in s.split(['—', '–', '―']).enumerate() {
                if part.is_empty() {
                    continue;
                }
                if pi > 0 {
                    flush(&mut cur, &mut out, Terminator::Comma);
                }
                cur.chunks.push(Chunk {
                    text: part.to_string(),
                    dot,
                    tight: tight && pi == 0,
                });
            }
            if term != Terminator::None {
                flush(&mut cur, &mut out, term);
            }
        }
    }
    flush(&mut cur, &mut out, Terminator::None);
    out
}

/// `s` without the characters of `marks` that stand between two letters of
/// an Indic script (Dogri: the apostrophe of कि’यां, a zero-width space
/// inside आह्​ली), or `None` when it has none.
pub fn drop_inside_indic(s: &str, marks: &[char]) -> Option<String> {
    let cs: Vec<char> = s.chars().collect();
    let indic = |c: Option<&char>| c.is_some_and(|c| ('\u{0900}'..'\u{0e00}').contains(c));
    let hit =
        |i: usize| marks.contains(&cs[i]) && i > 0 && indic(cs.get(i - 1)) && indic(cs.get(i + 1));
    if !(0..cs.len()).any(hit) {
        return None;
    }
    Some((0..cs.len()).filter(|&i| !hit(i)).map(|i| cs[i]).collect())
}

/// Split a chunk into maximal runs of one script. Apostrophes and hyphens
/// between letters stay inside a Latin run; `,` and `.` between digits stay
/// inside a digit run.
pub fn runs(chunk: &str) -> Vec<(Script, &str)> {
    runs_with(chunk, false)
}

/// [`runs`]; with `indic_apostrophe`, an apostrophe between two letters of
/// an Indic script also stays inside their run (Dogri र’लाया).
pub fn runs_with(chunk: &str, indic_apostrophe: bool) -> Vec<(Script, &str)> {
    let mut out: Vec<(Script, &str)> = Vec::new();
    let mut start = 0usize;
    let mut cur: Option<Script> = None;
    let mut prev: Option<char> = None;
    let mut it = chunk.char_indices().peekable();
    while let Some((i, c)) = it.next() {
        let next = it.peek().map(|x| x.1);
        let mut sc = script_of(c);
        if matches!(c, '\u{200c}' | '\u{200d}')
            && matches!(cur, Some(Script::Devanagari | Script::Brahmic(_)))
        {
            sc = cur.unwrap();
        }
        if sc == Script::Other {
            let between = |f: fn(char) -> bool| prev.is_some_and(f) && next.is_some_and(f);
            if matches!(c, '\'' | '’')
                && cur == Some(Script::Latin)
                && (next.is_some_and(|n| script_of(n) == Script::Latin)
                    || prev.is_some_and(|p| p == 's' || p == 'S'))
            {
                sc = Script::Latin;
            } else if indic_apostrophe
                && matches!(c, '\'' | '’' | 'ʼ')
                && matches!(cur, Some(Script::Devanagari | Script::Brahmic(_)))
                && next.is_some_and(|n| cur == Some(script_of(n)))
            {
                sc = cur.unwrap();
            } else if matches!(c, ',' | '.' | ':')
                && between(|x| x.is_ascii_digit() || indic_digit(x).is_some())
            {
                sc = Script::Digit;
            }
        }
        if cur != Some(sc) || sc == Script::Other {
            if let Some(p) = cur {
                out.push((p, &chunk[start..i]));
            }
            start = i;
            cur = Some(sc);
        }
        prev = Some(c);
    }
    if let Some(p) = cur {
        out.push((p, &chunk[start..]));
    }
    out
}

/// The base letter(s) of a lower-case Latin letter with a diacritic or of
/// another Latin alphabet (`ř` r, `ł` l, `ø` o, `ß` ss); `None` for other
/// characters.
pub fn base_letter(c: char) -> Option<&'static str> {
    Some(match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'æ' | 'ā' | 'ă' | 'ą' | 'ǎ' | 'ǟ' | 'ǡ' | 'ǻ' | 'ȁ'
        | 'ȃ' | 'ȧ' | 'ḁ' | 'ạ' | 'ả' | 'ấ' | 'ầ' | 'ẩ' | 'ẫ' | 'ậ' | 'ắ' | 'ằ' | 'ẳ' | 'ẵ'
        | 'ặ' => "a",
        'ƀ' | 'ḃ' | 'ḅ' | 'ḇ' => "b",
        'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' | 'ƈ' | 'ȼ' | 'ḉ' => "c",
        'ð' | 'ď' | 'đ' | 'ȡ' | 'ḋ' | 'ḍ' | 'ḏ' | 'ḑ' | 'ḓ' => "d",
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' | 'ȅ' | 'ȇ' | 'ȩ' | 'ɇ' | 'ḕ' | 'ḗ'
        | 'ḙ' | 'ḛ' | 'ḝ' | 'ẹ' | 'ẻ' | 'ẽ' | 'ế' | 'ề' | 'ể' | 'ễ' | 'ệ' => {
            "e"
        }
        'ƒ' | 'ḟ' => "f",
        'ĝ' | 'ğ' | 'ġ' | 'ģ' | 'ǥ' | 'ǧ' | 'ǵ' | 'ḡ' => "g",
        'ĥ' | 'ħ' | 'ȟ' | 'ḣ' | 'ḥ' | 'ḧ' | 'ḩ' | 'ḫ' | 'ẖ' => "h",
        'ì' | 'í' | 'î' | 'ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' | 'ǐ' | 'ȉ' | 'ȋ' | 'ḭ' | 'ḯ' | 'ỉ'
        | 'ị' => "i",
        'ĳ' => "ij",
        'ĵ' | 'ǰ' | 'ȷ' | 'ɉ' => "j",
        'ķ' | 'ĸ' | 'ƙ' | 'ǩ' | 'ḱ' | 'ḳ' | 'ḵ' => "k",
        'ĺ' | 'ļ' | 'ľ' | 'ŀ' | 'ł' | 'ƚ' | 'ȴ' | 'ḷ' | 'ḹ' | 'ḻ' | 'ḽ' => "l",
        'ḿ' | 'ṁ' | 'ṃ' => "m",
        'ñ' | 'ń' | 'ņ' | 'ň' | 'ŉ' | 'ŋ' | 'ƞ' | 'ǹ' | 'ȵ' | 'ṅ' | 'ṇ' | 'ṉ' | 'ṋ' => {
            "n"
        }
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ŏ' | 'ő' | 'ơ' | 'ǒ' | 'ǫ' | 'ǭ' | 'ȍ' | 'ȏ'
        | 'ȫ' | 'ȭ' | 'ȯ' | 'ȱ' | 'ṍ' | 'ṏ' | 'ṑ' | 'ṓ' | 'ọ' | 'ỏ' | 'ố' | 'ồ' | 'ổ' | 'ỗ'
        | 'ộ' | 'ớ' | 'ờ' | 'ở' | 'ỡ' | 'ợ' => "o",
        'œ' => "oe",
        'ƥ' | 'ṕ' | 'ṗ' => "p",
        'ɋ' => "q",
        'ŕ' | 'ŗ' | 'ř' | 'ȑ' | 'ȓ' | 'ɍ' | 'ṙ' | 'ṛ' | 'ṝ' | 'ṟ' => "r",
        'ś' | 'ŝ' | 'ş' | 'š' | 'ſ' | 'ș' | 'ȿ' | 'ṡ' | 'ṣ' | 'ṥ' | 'ṧ' | 'ṩ' => {
            "s"
        }
        'ţ' | 'ť' | 'ŧ' | 'ƫ' | 'ƭ' | 'ț' | 'ȶ' | 'ṫ' | 'ṭ' | 'ṯ' | 'ṱ' | 'ẗ' => {
            "t"
        }
        'ß' => "ss",
        'þ' => "th",
        'ù' | 'ú' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' | 'ư' | 'ǔ' | 'ǖ' | 'ǘ' | 'ǚ'
        | 'ǜ' | 'ȕ' | 'ȗ' | 'ṳ' | 'ṵ' | 'ṷ' | 'ṹ' | 'ṻ' | 'ụ' | 'ủ' | 'ứ' | 'ừ' | 'ử' | 'ữ'
        | 'ự' => "u",
        'ṽ' | 'ṿ' => "v",
        'ŵ' | 'ẁ' | 'ẃ' | 'ẅ' | 'ẇ' | 'ẉ' | 'ẘ' => "w",
        'ẋ' | 'ẍ' => "x",
        'ý' | 'ÿ' | 'ŷ' | 'ƴ' | 'ȳ' | 'ɏ' | 'ẏ' | 'ẙ' | 'ỳ' | 'ỵ' | 'ỷ' | 'ỹ' => {
            "y"
        }
        'ź' | 'ż' | 'ž' | 'ƶ' | 'ɀ' | 'ẑ' | 'ẓ' | 'ẕ' => "z",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape(text: &str) -> Vec<(Vec<String>, Terminator)> {
        clauses(text)
            .into_iter()
            .map(|c| (c.chunks.into_iter().map(|k| k.text).collect(), c.terminator))
            .collect()
    }

    #[test]
    fn splits_clauses() {
        let s = shape("Hello, world. How are you? Fine! Yes: no; maybe");
        let terms: Vec<Terminator> = s.iter().map(|c| c.1).collect();
        assert_eq!(
            terms,
            [
                Terminator::Comma,
                Terminator::Period,
                Terminator::Question,
                Terminator::Exclamation,
                Terminator::Colon,
                Terminator::Semicolon,
                Terminator::None
            ]
        );
        assert_eq!(s[1].0, ["world"]);
    }

    #[test]
    fn abbreviations_and_decimals() {
        let s = shape("Dr. Smith paid $3.50 to J. Brown.");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].0, ["Dr", "Smith", "paid", "$3.50", "to", "J", "Brown"]);
        let s = shape("He said \"stop!\" (twice).");
        assert_eq!(s[0].0, ["He", "said", "stop"]);
        assert_eq!(s[0].1, Terminator::Exclamation);
        assert_eq!(s[1].0, ["twice"]);
    }

    #[test]
    fn hindi_danda() {
        let s = shape("राम आया। श्याम गया॥");
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].1, Terminator::Period);
    }

    #[test]
    fn runs_by_script() {
        let r = runs("मैंoffice5km");
        let kinds: Vec<Script> = r.iter().map(|x| x.0).collect();
        assert_eq!(
            kinds,
            [
                Script::Devanagari,
                Script::Latin,
                Script::Digit,
                Script::Latin
            ]
        );
        assert_eq!(runs("don't")[0], (Script::Latin, "don't"));
        assert_eq!(runs("1,234.5")[0], (Script::Digit, "1,234.5"));
        assert_eq!(runs("$5").len(), 2);
    }

    #[test]
    fn empty_and_punctuation_only() {
        assert!(clauses("").is_empty());
        assert!(clauses("...").is_empty());
        assert!(clauses(" ?! ").is_empty());
    }
}
