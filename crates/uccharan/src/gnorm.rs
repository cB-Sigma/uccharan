//! Text normalisation for the data-driven languages: a chunk is split into
//! runs of letters and digits; numbers are read with the language's number
//! words (`[numbers]` in its file); letters of other scripts are kept as
//! words for the router to hand to their own language.

use crate::langdef::{Fraction, Numbers};
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
fn push_number(
    out: &mut Vec<Item>,
    nums: &Numbers,
    s: &str,
    spaced: bool,
    compat: bool,
    strict: bool,
) {
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
    // With a decimal comma, a ',' here was a full stop in the text.
    let dots = nums.decimal_comma && int.contains(',') && !int.contains([':', '/']);
    // "1,00,000" and "1,000,000" are one number; times and dates are read part by part.
    let one_number = int.contains(',')
        && !int.contains([':', '/'])
        && groups.iter().skip(1).all(|g| g.len() == 2 || g.len() == 3)
        && !(strict && dots && nums.dot_no_groups);
    // Improved mode reads "2.5" in a language with a decimal comma as a
    // decimal number.
    let (int, frac, groups) =
        if !strict && dots && !one_number && frac.is_none() && groups.len() == 2 {
            (groups[0], Some(groups[1]), vec![groups[0]])
        } else {
            (int, frac, groups)
        };
    let _ = int;
    let parts: Vec<String> = if one_number {
        vec![groups.concat()]
    } else {
        groups.iter().map(|g| g.to_string()).collect()
    };
    let mut words: Vec<(String, bool)> = Vec::new();
    // Words that start a group of digits and keep their stress.
    let mut heads: Vec<usize> = Vec::new();
    for (pi, p) in parts.iter().enumerate() {
        // A full stop that does not group digits is read in compatible mode.
        if pi > 0 && dots && strict {
            words.extend(nums.dot_word.iter().map(|w| (w.clone(), false)));
            heads.push(words.len());
        }
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
        fraction(nums, f, &mut words, compat);
    }
    for (k, (w, glue)) in words.into_iter().enumerate() {
        let mut it = Item::word(&w);
        it.glue = glue;
        it.number_tail = k > 0 && !heads.contains(&k);
        out.push(it);
    }
}

/// A date written with digits (15/08/2024, 15.08.2024, 15-08-2024,
/// 2024-08-15), read with the language's `[dates]`.
fn date(text: &str, nums: &Numbers, prev: Option<&str>) -> Option<Vec<Item>> {
    let d = nums.dates.as_ref()?;
    let sep = ['/', '.', '-']
        .into_iter()
        .find(|&c| text.matches(c).count() == 2)?;
    let p: Vec<&str> = text.split(sep).collect();
    if p.iter()
        .any(|x| x.is_empty() || !x.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    let (y, m, day) = if p[0].len() == 4 && p[1].len() <= 2 && p[2].len() <= 2 {
        (p[0], p[1], p[2])
    } else if p[2].len() == 4 && p[0].len() <= 2 && p[1].len() <= 2 {
        (p[2], p[1], p[0])
    } else {
        return None;
    };
    let (y, m, day): (u64, usize, u64) = (y.parse().ok()?, m.parse().ok()?, day.parse().ok()?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&day) {
        return None;
    }
    let mut out: Vec<Item> = Vec::new();
    let words = |out: &mut Vec<Item>, w: &[(String, bool)]| {
        out.extend(w.iter().map(|(t, g)| {
            let mut it = Item::word(t);
            it.glue = *g;
            it
        }))
    };
    let plain = |out: &mut Vec<Item>, w: &str| {
        out.extend(w.split_whitespace().map(|t| {
            let mut it = Item::word(t);
            it.generated = false;
            it
        }))
    };
    for tok in &d.pattern {
        match tok.as_str() {
            "D" => {
                if let (1, Some(w)) = (day, &d.day1) {
                    plain(&mut out, w);
                    continue;
                }
                let ord = nums.ordinal_words.as_ref();
                match d.day.0.as_str() {
                    "ordinal" | "possessive" => {
                        // The form after the word before the date ("am").
                        let set = ord
                            .and_then(|o| o.after_words(prev, day))
                            .filter(|_| d.day.0 == "ordinal")
                            .or_else(|| ord.and_then(|o| o.words(&d.day.1, day)));
                        let mut w = set?;
                        if d.day.0 == "possessive" {
                            w = possessive(&w);
                        }
                        plain(&mut out, &w);
                    }
                    _ => {
                        let mut w = Vec::new();
                        nums.cardinal(day, &mut w);
                        words(&mut out, &w);
                    }
                }
            }
            "M" => plain(&mut out, &d.months[m - 1]),
            "Y" => match d.year.0.as_str() {
                "ordinal" => {
                    let w = nums.ordinal_words.as_ref()?.words(&d.year.1, y)?;
                    plain(&mut out, &w);
                }
                "hundreds" => {
                    let mut w = Vec::new();
                    nums.cardinal_year(y, &mut w);
                    words(&mut out, &w);
                }
                _ => {
                    let mut w = Vec::new();
                    nums.cardinal(y, &mut w);
                    words(&mut out, &w);
                }
            },
            lit => plain(&mut out, lit),
        }
    }
    Some(out)
}

/// The Hungarian day of a date: the ordinal with its possessive ending
/// ("tizenötödik" tizenötödike, "harmadik" harmadika, "első" elseje).
fn possessive(w: &str) -> String {
    if let Some(stem) = w.strip_suffix("első") {
        return format!("{stem}elseje");
    }
    let Some(stem) = w.strip_suffix("ik") else {
        return w.to_string();
    };
    let back = stem
        .chars()
        .rev()
        .find(|c| "aáeéiíoóöőuúüű".contains(*c))
        .is_some_and(|c| "aáoóuú".contains(c));
    format!("{w}{}", if back { "a" } else { "e" })
}

/// The digits `f` after a decimal sign, as the language reads them.
fn fraction(nums: &Numbers, f: &str, words: &mut Vec<(String, bool)>, compat: bool) {
    let start = words.len();
    let value = f.parse::<u64>().ok();
    // Words of leading zeros, and whether the rest became one number.
    let (mut zeros, mut number) = (0, false);
    match (nums.fraction, value) {
        (Fraction::Number(max), Some(_)) => {
            let rest = f.trim_start_matches('0');
            nums.digits(&f[..f.len() - rest.len()], words);
            zeros = words.len() - start;
            match rest.parse::<u64>() {
                Ok(v) if rest.len() <= max => {
                    nums.cardinal(v, words);
                    number = v >= 100;
                }
                _ => nums.digits(rest, words),
            }
        }
        (Fraction::Romanian, Some(v)) if f.len() <= 4 && !f.starts_with('0') => {
            nums.cardinal(v, words);
            number = v >= 100;
        }
        (Fraction::Suffix(zero_only), Some(v)) if f.len() <= 5 => {
            let suffix = nums.fraction_suffix.get(&f.len());
            if zero_only && !f.starts_with('0') {
                nums.cardinal(v, words);
                number = v >= 100;
            } else if let Some(sfx) = suffix {
                nums.cardinal(v, words);
                words.push((sfx.clone(), true));
                number = v >= 100;
            } else {
                nums.digits(f, words);
            }
        }
        _ => nums.digits(f, words),
    }
    // The reference voices join the zeros, and a number of hundreds read
    // as one number, to the word for the decimal sign.
    if compat {
        let n = zeros + usize::from(number);
        for w in words[start..].iter_mut().take(n) {
            w.1 = true;
        }
    }
}

/// Expand one chunk for a language whose number words are `nums`. With
/// `ascii_english`, runs of ASCII digits are read as English numbers (as the
/// reference's Nepali voice does) and later routed to English.
/// `compat`: reference-compatible readings (see [`push_number`]).
/// `natural`: dates and ordinals are read in full (improved mode and
/// normalise-only mode); `prev`: the chunk before this one, which selects
/// the form of some ordinals ("am 3." dritten).
pub fn expand(
    chunk: &Chunk,
    nums: &Numbers,
    ascii_english: bool,
    compat: bool,
    natural: bool,
    prev: Option<&str>,
) -> Vec<Item> {
    if natural {
        if let Some(items) = date(&chunk.text, nums, prev) {
            return items;
        }
    }
    // Ordinals: "1er", "2ª", "3:e", and "3." where the language writes them so.
    let strict = compat && !natural;
    let text = chunk.text.as_str();
    let plain = match text.split_once(['.']) {
        Some((a, b))
            if natural && a.bytes().all(|c| c.is_ascii_digit()) && matches!(b, "º" | "ª") =>
        {
            std::borrow::Cow::Owned(format!("{a}{b}"))
        }
        _ => std::borrow::Cow::Borrowed(text),
    };
    let ordinal = (nums.ordinal_words.is_some())
        .then(|| nums.suffix_ordinal(&plain, strict))
        .flatten()
        .or_else(|| {
            let n = text.parse::<u64>().ok().filter(|_| chunk.dot)?;
            nums.dot_ordinal(n, prev, strict)
        });
    if let Some(words) = ordinal {
        return words
            .split_whitespace()
            .map(|w| {
                let mut it = Item::word(w);
                // Number keys (`ord3#`) stay generated words.
                it.generated = w.ends_with('#');
                it
            })
            .collect();
    }
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
    // Improved mode says a currency sign written before the amount after it.
    let mut after_amount: Vec<Item> = Vec::new();
    // Where the items of the last number start (`percent-before`).
    let mut number_at = 0usize;
    for (i, (sc, r)) in rs.iter().enumerate() {
        if i > 0 && rs[i - 1].0 == Script::Digit && !after_amount.is_empty() {
            out.append(&mut after_amount);
        }
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
            Script::Digit => {
                number_at = out.len();
                push_number(&mut out, nums, r, spaced, compat, strict)
            }
            Script::Other => {
                let c = r.chars().next().unwrap_or(' ');
                match c {
                    '-' if i == 0
                        && n > 1
                        && rs[1].0 == Script::Digit
                        && !(compat && nums.minus_silent_compat) =>
                    {
                        out.extend(nums.minus.iter().map(|w| Item::word(w)));
                    }
                    '%' if !nums.percent_before.is_empty() => {
                        let words = nums.percent_before.iter().map(|w| Item::word(w));
                        if i > 0 && rs[i - 1].0 == Script::Digit {
                            out.splice(number_at..number_at, words);
                        } else {
                            out.extend(words);
                        }
                    }
                    '%' => out.extend(nums.percent.iter().map(|w| Item::word(w))),
                    _ => {
                        let Some((_, words)) = nums.currency.iter().find(|(s, _)| *s == c) else {
                            continue;
                        };
                        let items = words.iter().map(|w| Item::word(w));
                        let before = rs.get(i + 1).is_some_and(|x| x.0 == Script::Digit);
                        if !compat && before && r.chars().count() == 1 {
                            after_amount.extend(items);
                        } else {
                            out.extend(items);
                        }
                    }
                }
            }
        }
    }
    out.append(&mut after_amount);
    out
}
