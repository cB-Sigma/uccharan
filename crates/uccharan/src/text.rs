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
            ',' | '،' | '、' => Terminator::Comma,
            ':' => Terminator::Colon,
            ';' => Terminator::Semicolon,
            '.' | '।' | '॥' | '…' | '。' => Terminator::Period,
            '?' | '？' => Terminator::Question,
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
    /// ASCII or Devanagari digits.
    Digit,
    /// Anything else.
    Other,
}

/// Script of one character.
pub fn script_of(c: char) -> Script {
    match c {
        'a'..='z' | 'A'..='Z' | '\u{00c0}'..='\u{024f}' => Script::Latin,
        '\u{0966}'..='\u{096f}' | '0'..='9' => Script::Digit,
        '\u{0900}'..='\u{097f}' | '\u{200c}' | '\u{200d}' => Script::Devanagari,
        _ => Script::Other,
    }
}

/// A whitespace-delimited chunk with its surrounding punctuation removed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chunk {
    /// The chunk text (never empty).
    pub text: String,
    /// True when a full stop directly followed the chunk and was not used as a
    /// clause terminator (an abbreviation such as `Dr.`).
    pub dot: bool,
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
                if next.is_some() && is_abbreviation(s, next_lower) {
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

/// Split a chunk into maximal runs of one script. Apostrophes and hyphens
/// between letters stay inside a Latin run; `,` and `.` between digits stay
/// inside a digit run.
pub fn runs(chunk: &str) -> Vec<(Script, &str)> {
    let cs: Vec<(usize, char)> = chunk.char_indices().collect();
    let mut out: Vec<(Script, &str)> = Vec::new();
    let mut start = 0usize;
    let mut cur: Option<Script> = None;
    for (k, &(i, c)) in cs.iter().enumerate() {
        let prev = if k > 0 { Some(cs[k - 1].1) } else { None };
        let next = cs.get(k + 1).map(|x| x.1);
        let mut sc = script_of(c);
        if sc == Script::Other {
            let between = |f: fn(char) -> bool| prev.is_some_and(f) && next.is_some_and(f);
            if matches!(c, '\'' | '’')
                && cur == Some(Script::Latin)
                && (next.is_some_and(|n| script_of(n) == Script::Latin)
                    || prev.is_some_and(|p| p == 's' || p == 'S'))
            {
                sc = Script::Latin;
            } else if matches!(c, ',' | '.' | ':') && between(|x| x.is_ascii_digit()) {
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
    }
    if let Some(p) = cur {
        out.push((p, &chunk[start..]));
    }
    out
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
