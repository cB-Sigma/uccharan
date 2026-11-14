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

fn push_number(out: &mut Vec<Item>, nums: &Numbers, s: &str) {
    let ascii: String = s
        .chars()
        .map(|c| digit(c).map_or(c, |d| char::from(b'0' + d)))
        .collect();
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
    for p in &parts {
        if p.len() > 1 && p.starts_with('0') || p.len() > 15 {
            nums.digits(p, &mut words);
        } else if let Ok(n) = p.parse::<u64>() {
            nums.cardinal(n, &mut words);
        }
    }
    if let Some(f) = frac {
        words.extend(nums.point.iter().map(|w| (w.clone(), false)));
        nums.digits(f, &mut words);
    }
    for (k, (w, glue)) in words.into_iter().enumerate() {
        let mut it = Item::word(&w);
        it.glue = glue;
        it.number_tail = k > 0;
        out.push(it);
    }
}

/// Expand one chunk for a language whose number words are `nums`. With
/// `ascii_english`, runs of ASCII digits are read as English numbers (as the
/// reference's Nepali voice does) and later routed to English.
pub fn expand(chunk: &Chunk, nums: &Numbers, ascii_english: bool) -> Vec<Item> {
    let mut out: Vec<Item> = Vec::new();
    // An apostrophe between two Indic letters is ignored (Assamese ল'ৰা).
    let cs: Vec<char> = chunk.text.chars().collect();
    let indic = |c: Option<&char>| c.is_some_and(|c| ('\u{0900}'..'\u{0e00}').contains(c));
    let text: String = cs
        .iter()
        .enumerate()
        .filter(|&(i, c)| {
            !(matches!(c, '\'' | '’') && i > 0 && indic(cs.get(i - 1)) && indic(cs.get(i + 1)))
        })
        .map(|(_, c)| *c)
        .collect();
    let rs = runs(&text);
    let n = rs.len();
    for (i, (sc, r)) in rs.iter().enumerate() {
        match sc {
            Script::Latin | Script::Devanagari | Script::Brahmic(_) => {
                let t = r.replace('\u{200d}', "");
                if !t.is_empty() {
                    let mut it = Item::word(&t);
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
                };
                out.extend(crate::norm::expand(
                    &c,
                    crate::Lang::EnGb,
                    crate::Normalize::EspeakLike,
                ));
            }
            Script::Digit => push_number(&mut out, nums, r),
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
