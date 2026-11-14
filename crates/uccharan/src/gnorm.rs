//! Text normalisation for the data-driven languages: a chunk is split into
//! runs of letters and digits; numbers are read with the language's number
//! words (`[numbers]` in its file); letters of other scripts are kept as
//! words for the router to hand to their own language.

use crate::langdef::Numbers;
use crate::norm::Item;
use crate::text::{indic_digit, runs, Chunk, Script};

fn digit(c: char) -> Option<u8> {
    if c.is_ascii_digit() {
        Some(c as u8 - b'0')
    } else {
        indic_digit(c)
    }
}

/// Between the groups of a number written with spaces ("1 500").
pub const GROUPED: char = '\u{2063}';

/// Chunks with numbers written in groups of three digits ("1 250 000")
/// joined into one chunk each. A group that carries a decimal part ends
/// the number ("12 500,50"). `first_max` is the most digits the first
/// group may have (the reference voices also join "2020 100"). A joined
/// chunk carries [`GROUPED`] between its groups. `plain_only`: only an
/// ordinary space separates groups; the reference voices read groups after
/// a no-break or thin space as a number of their own.
pub fn join_groups(chunks: &[Chunk], first_max: usize, plain_only: bool) -> Vec<Chunk> {
    let digits = |s: &str, n: std::ops::RangeInclusive<usize>| {
        n.contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())
    };
    let mut out: Vec<Chunk> = Vec::with_capacity(chunks.len());
    let mut open = false;
    for c in chunks {
        let head = c.text.split([',', '.']).next().unwrap_or("");
        let whole = head.len() == c.text.len();
        if open
            && digits(head, 3..=3)
            && !(plain_only && c.tight)
            && !out.last().is_some_and(|p| p.dot)
        {
            if let Some(prev) = out.last_mut() {
                prev.text.push(GROUPED);
                prev.text.push_str(&c.text);
                prev.dot = c.dot;
                open = whole;
                continue;
            }
        }
        open = whole && digits(&c.text, 1..=first_max) && !c.text.starts_with('0');
        out.push(c.clone());
    }
    out
}

/// `compat`: a group of three digits with a leading zero is read as the
/// reference voices read it ("000" zero-zero zero, "012" zero twelve).
fn push_number(out: &mut Vec<Item>, nums: &Numbers, s: &str, spaced: bool, compat: bool) {
    let ascii: String = s
        .chars()
        .map(|c| digit(c).map_or(c, |d| char::from(b'0' + d)))
        .collect();
    // With a decimal comma the two signs swap their roles.
    let ascii: String = if nums.decimal_comma {
        ascii
            .chars()
            .map(|c| match c {
                ',' => '.',
                '.' => ',',
                c => c,
            })
            .collect()
    } else {
        ascii
    };
    let (int, frac) = match ascii.split_once('.') {
        Some((a, b)) if !b.is_empty() && b.bytes().all(|x| x.is_ascii_digit()) => (a, Some(b)),
        _ => (ascii.as_str(), None),
    };
    let groups: Vec<&str> = int
        .split([',', ':', '/'])
        .filter(|g| !g.is_empty())
        .collect();
    // "1,00,000" and "1,000,000" are one number; times and dates are read part by part.
    let one_number = int.contains(',')
        && !int.contains([':', '/'])
        && groups.iter().skip(1).all(|g| g.len() == 2 || g.len() == 3);
    let parts: Vec<String> = if one_number {
        vec![groups.concat()]
    } else {
        groups.iter().map(|g| g.to_string()).collect()
    };
    let mut words: Vec<(String, bool)> = Vec::new();
    // Words that start a group of digits and keep their stress.
    let mut heads: Vec<usize> = Vec::new();
    for p in &parts {
        if compat && p.len() == 3 && p.starts_with('0') && nums.speller.is_none() {
            if let Some(last) = p.strip_prefix("00") {
                let at = words.len();
                nums.digits("00", &mut words);
                if let Some(w) = words.get_mut(at + 1) {
                    w.1 = true;
                }
                nums.digits(last, &mut words);
            } else {
                nums.digits(&p[..1], &mut words);
                if let Ok(n) = p[1..].parse::<u64>() {
                    nums.cardinal(n, &mut words);
                }
            }
        } else if p.len() > 1 && p.starts_with('0') || p.len() > 15 {
            nums.digits(p, &mut words);
        } else if let Ok(n) = p.parse::<u64>() {
            let base = words.len();
            heads.extend(nums.group_starts(n, spaced).into_iter().map(|k| base + k));
            nums.cardinal(n, &mut words);
        }
    }
    if let Some(f) = frac {
        words.extend(nums.point.iter().map(|w| (w.clone(), false)));
        match f.parse::<u64>() {
            Ok(v) if nums.fraction_number && !f.starts_with('0') => nums.cardinal(v, &mut words),
            _ => nums.digits(f, &mut words),
        }
    }
    for (k, (w, glue)) in words.into_iter().enumerate() {
        let mut it = Item::word(&w);
        it.glue = glue;
        it.number_tail = k > 0 && !heads.contains(&k);
        out.push(it);
    }
}

/// Expand one chunk for a language whose number words are `nums`. With
/// `ascii_english`, runs of ASCII digits are read as English numbers (as the
/// reference's Nepali voice does) and later routed to English.
/// `compat`: reference-compatible readings (see [`push_number`]).
pub fn expand(chunk: &Chunk, nums: &Numbers, ascii_english: bool, compat: bool) -> Vec<Item> {
    let mut out: Vec<Item> = Vec::new();
    // An apostrophe between two Indic letters is ignored (Assamese ল'ৰা).
    let owned;
    let text: &str = if chunk.text.contains(['\'', '’']) && !chunk.text.is_ascii() {
        let cs: Vec<char> = chunk.text.chars().collect();
        let indic = |c: Option<&char>| c.is_some_and(|c| ('\u{0900}'..'\u{0e00}').contains(c));
        owned = cs
            .iter()
            .enumerate()
            .filter(|&(i, c)| {
                !(matches!(c, '\'' | '’') && i > 0 && indic(cs.get(i - 1)) && indic(cs.get(i + 1)))
            })
            .map(|(_, c)| *c)
            .collect::<String>();
        &owned
    } else {
        &chunk.text
    };
    let spaced = text.contains(GROUPED);
    let owned_grouped;
    let text: &str = if spaced {
        owned_grouped = text.replace(GROUPED, "");
        &owned_grouped
    } else {
        text
    };
    let rs = runs(text);
    let n = rs.len();
    for (i, (sc, r)) in rs.iter().enumerate() {
        match sc {
            Script::Latin | Script::Devanagari | Script::Brahmic(_) => {
                // ZWJ stays inside Brahmic words (the reader decides).
                let owned_run;
                let t: &str = if matches!(sc, Script::Brahmic(_)) || !r.contains('\u{200d}') {
                    r
                } else {
                    owned_run = r.replace('\u{200d}', "");
                    &owned_run
                };
                if !t.is_empty() {
                    let mut it = Item::word(t);
                    it.generated = false;
                    if i >= 2 && rs[i - 1].1 == "-" {
                        it.hyphen = true;
                    }
                    out.push(it);
                }
            }
            Script::Digit
                if ascii_english && r.bytes().next().is_some_and(|b| b.is_ascii_digit()) =>
            {
                let c = Chunk {
                    text: r.to_string(),
                    dot: false,
                    tight: false,
                };
                out.extend(crate::norm::expand(
                    &c,
                    crate::Lang::EnGb,
                    crate::Normalize::EspeakLike,
                ));
            }
            Script::Digit => push_number(&mut out, nums, r, spaced, compat),
            Script::Other => {
                let c = r.chars().next().unwrap_or(' ');
                match c {
                    '-' if i == 0 && n > 1 && rs[1].0 == Script::Digit => {
                        out.extend(nums.minus.iter().map(|w| Item::word(w)));
                    }
                    '%' => out.extend(nums.percent.iter().map(|w| Item::word(w))),
                    _ => {}
                }
            }
        }
    }
    out
}
