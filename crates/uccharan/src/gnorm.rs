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

/// True when `t` is a currency sign of the language on its own.
pub fn is_currency_sign(t: &str, nums: &Numbers) -> bool {
    let mut cs = t.chars();
    matches!((cs.next(), cs.next()), (Some(c), None) if nums.currency.iter().any(|x| x.0 == c))
}

/// Chunks of an amount followed by a currency sign of its own ("25,50 €")
/// joined into one chunk ("25,50€"), for [`expand`] to read as money.
pub fn join_currency(chunks: &[Chunk], nums: &Numbers) -> Vec<Chunk> {
    let mut out: Vec<Chunk> = Vec::with_capacity(chunks.len());
    for c in chunks {
        if let Some(prev) = out.last_mut() {
            if is_currency_sign(&c.text, nums)
                && !prev.dot
                && prev.text.starts_with(|x: char| x.is_ascii_digit())
                && prev
                    .text
                    .bytes()
                    .all(|b| b.is_ascii_digit() || b == b',' || b == b'.')
            {
                prev.text.push_str(&c.text);
                continue;
            }
        }
        out.push(c.clone());
    }
    out
}

fn all_digits(s: &str, n: std::ops::RangeInclusive<usize>) -> bool {
    n.contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())
}

/// The chunks at the start of `texts` that write one clock time ("10",
/// "h", "30"; "10.30", "Uhr"), for a language that reads times
/// (`time…` lines): how many there are and the time as one chunk text
/// ("10h30", "10.30Uhr").
pub fn time_join(texts: &[&str], nums: &Numbers) -> Option<(usize, String)> {
    let t = nums.times.as_ref()?;
    let a = *texts.first()?;
    let b = texts.get(1).copied();
    let c = texts.get(2).copied();
    if t.has("h") && all_digits(a, 1..=2) && matches!(b, Some("h" | "H")) {
        return Some(match c.filter(|m| all_digits(m, 2..=2)) {
            Some(m) => (3, format!("{a}h{m}")),
            None => (2, format!("{a}h")),
        });
    }
    if t.has("h") {
        if let Some(h) = a.strip_suffix(['h', 'H']).filter(|h| all_digits(h, 1..=2)) {
            if let Some(m) = b.filter(|m| all_digits(m, 2..=2)) {
                return Some((2, format!("{h}h{m}")));
            }
        }
    }
    let w = t.after_word(b?)?;
    let clock = match a.split_once([':', '.']) {
        Some((h, m)) => all_digits(h, 1..=2) && all_digits(m, 2..=2),
        None => all_digits(a, 1..=2),
    };
    clock.then(|| (2, format!("{a}{w}")))
}

/// Chunks that write one clock time joined into one chunk each
/// ([`time_join`]).
pub fn join_times(chunks: &[Chunk], nums: &Numbers) -> Vec<Chunk> {
    if nums.times.is_none() {
        return chunks.to_vec();
    }
    let mut out: Vec<Chunk> = Vec::with_capacity(chunks.len());
    let mut i = 0;
    while i < chunks.len() {
        let texts: Vec<&str> = chunks[i..chunks.len().min(i + 3)]
            .iter()
            .map(|c| c.text.as_str())
            .collect();
        // A chunk with a full stop after it ends the time.
        let open = chunks[i..].iter().take_while(|c| !c.dot).count() + 1;
        match time_join(&texts, nums).filter(|(k, _)| *k <= open) {
            Some((k, text)) => {
                out.push(Chunk {
                    text,
                    dot: chunks[i + k - 1].dot,
                    tight: chunks[i].tight,
                });
                i += k;
            }
            None => {
                out.push(chunks[i].clone());
                i += 1;
            }
        }
    }
    out
}

/// A clock time ("10h30", "10:30", "10.30", "10.30Uhr" from
/// [`time_join`]) read with the language's `time…` lines: the hours, the
/// hour word, the minutes ("dix heures trente", "zehn Uhr dreißig",
/// "kymmenen kolmekymmentä").
fn time(text: &str, nums: &Numbers, prev: Option<&str>) -> Option<Vec<Item>> {
    let t = nums.times.as_ref()?;
    let hd = text.len() - text.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    if hd == 0 || hd > 2 {
        return None;
    }
    let rest = &text[hd..];
    let (form, rest) = match rest.chars().next() {
        Some('h' | 'H') => ("h", &rest[1..]),
        Some(':') => ("colon", &rest[1..]),
        Some('.') => ("dot", &rest[1..]),
        _ => ("", rest),
    };
    let md = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    let tail = &rest[md..];
    let after = if tail.is_empty() {
        None
    } else {
        Some(t.after_word(tail)?)
    };
    let minutes = match (form, md) {
        (_, 2) => rest[..2].parse::<u64>().ok()?,
        ("h" | "", 0) => 0,
        _ => return None,
    };
    let h: u64 = text[..hd].parse().ok()?;
    if h > 24 || minutes > 59 {
        return None;
    }
    let context = prev.is_some_and(|p| t.context.contains(&p.to_lowercase()));
    let is_time = match form {
        "h" => t.has("h"),
        "colon" => t.has("colon") || after.is_some(),
        "dot" => t.has("dot") || (t.has("dot-context") && (after.is_some() || context)),
        _ => after.is_some(),
    };
    if !is_time {
        return None;
    }
    let hour_word = match (after, &t.hour) {
        (Some(w), _) => Some(w.to_string()),
        (None, Some((one, more))) if form == "h" || t.hour_always || minutes == 0 => {
            Some(if h == 1 { one } else { more }.clone())
        }
        _ => None,
    };
    let mut out: Vec<Item> = Vec::new();
    let plain = |out: &mut Vec<Item>, w: &str| {
        out.extend(w.split_whitespace().map(|x| {
            let mut it = Item::word(x);
            it.generated = false;
            it
        }))
    };
    match t.hours.get(&h) {
        Some(w) => plain(&mut out, w),
        None => push_number(&mut out, nums, &h.to_string(), false, false, false),
    }
    if let Some(w) = &hour_word {
        plain(&mut out, w);
    }
    if minutes > 0 {
        if hour_word.is_some() {
            out.extend(t.and.iter().map(|w| Item::word(w)));
        } else if minutes < 10 {
            let mut z = Vec::new();
            nums.digits("0", &mut z);
            out.extend(z.iter().map(|(w, _)| Item::word(w)));
        }
        push_number(&mut out, nums, &minutes.to_string(), false, false, false);
    }
    Some(out)
}

/// True when `text` is a clock time the language reads as one
/// (normalise-only mode: the kind of the span).
pub fn is_time(text: &str, nums: &Numbers, prev: Option<&str>) -> bool {
    time(text, nums, prev).is_some()
}

/// The whole units of an amount written with digits ("1.250,50" 1250).
fn whole_amount(s: &str, nums: &Numbers) -> Option<u64> {
    let (dec, group) = if nums.decimal_comma {
        (',', '.')
    } else {
        ('.', ',')
    };
    let whole = s.split(dec).next()?.replace([group, ' ', GROUPED], "");
    if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    whole.parse().ok()
}

/// The words for the currency `sign` after the whole amount `n` (natural
/// readings: the plural where the language has one), with the word that
/// goes between them (`currency-of`).
fn currency_words(nums: &Numbers, sign: char, words: &[String], n: Option<u64>) -> Vec<Item> {
    let mut out: Vec<Item> = Vec::new();
    let Some(n) = n else {
        return words.iter().map(|w| Item::word(w)).collect();
    };
    if nums.money.of_after(n) {
        out.extend(nums.money.of.iter().map(|w| Item::word(w)));
    }
    match nums.money.word(sign, n) {
        Some(p) => out.extend(p.iter().map(|w| {
            let mut it = Item::word(w);
            it.generated = w.contains(['#', '¤']);
            it
        })),
        None => out.extend(words.iter().map(|w| Item::word(w))),
    }
    out
}

/// The items of the amount `n` (from `at` on) replaced by the language's
/// words for it before the currency `sign` (`currency-count`: "un euro").
fn count_words(out: &mut Vec<Item>, at: usize, nums: &Numbers, sign: char, n: Option<u64>) {
    let Some(w) = n.and_then(|n| nums.money.count_words(sign, n)) else {
        return;
    };
    out.truncate(at);
    out.extend(w.iter().map(|x| {
        let mut it = Item::word(x);
        it.generated = false;
        it
    }));
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
    // "1,00,000" and "1,000,000" are one number; times and dates are read
    // part by part. A full stop groups only three digits ("10.30" is not
    // a thousand and thirty).
    let one_number = int.contains(',')
        && !int.contains([':', '/'])
        && groups
            .iter()
            .skip(1)
            .all(|g| g.len() == 3 || (g.len() == 2 && !dots))
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

/// An amount of money with cents and a currency sign before or after it
/// ("25,50 €", "€25,50"): the whole units, the currency word, the cents
/// ("fünfundzwanzig Euro fünfzig").
fn money(text: &str, nums: &Numbers) -> Option<Vec<Item>> {
    let (sign, words, amount) = nums.currency.iter().find_map(|(c, w)| {
        let a = text
            .strip_suffix(*c)
            .or_else(|| text.strip_prefix(*c))?
            .trim_end_matches('\u{a0}');
        Some((*c, w, a))
    })?;
    let sep = if nums.decimal_comma { ',' } else { '.' };
    let (whole, cents) = amount.split_once(sep)?;
    let whole = whole.replace([if nums.decimal_comma { '.' } else { ',' }, ' '], "");
    if cents.len() != 2
        || whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !cents.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let n: u64 = whole.parse().ok()?;
    let mut w = Vec::new();
    nums.cardinal(n, &mut w);
    let item = |(t, g): &(String, bool)| {
        let mut it = Item::word(t);
        it.glue = *g;
        it
    };
    let mut out: Vec<Item> = w.iter().map(item).collect();
    count_words(&mut out, 0, nums, sign, Some(n));
    out.extend(currency_words(nums, sign, words, Some(n)));
    let c: u64 = cents.parse().ok()?;
    if c > 0 {
        let mut w = Vec::new();
        nums.cardinal(c, &mut w);
        out.extend(w.iter().map(item));
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
        if let Some(items) = date(&chunk.text, nums, prev)
            .or_else(|| money(&chunk.text, nums))
            .or_else(|| time(&chunk.text, nums, prev).filter(|_| !chunk.dot))
        {
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
    let ordinal_items = |words: &str| -> Vec<Item> {
        words
            .split_whitespace()
            .map(|w| {
                let mut it = Item::word(w);
                // Number keys (`ord3#`) stay generated words.
                it.generated = w.ends_with('#');
                it
            })
            .collect()
    };
    if let Some(words) = ordinal {
        return ordinal_items(&words);
    }
    // An ordinal before a hyphen and a word ("1er-mai", "2e-étage"): the
    // ordinal, then the rest as words of their own after the hyphen.
    if let Some((head, rest)) = text.split_once('-') {
        let words = (nums.ordinal_words.is_some()
            && head.starts_with(|c: char| c.is_ascii_digit())
            && rest.starts_with(char::is_alphabetic))
        .then(|| nums.suffix_ordinal(head, strict))
        .flatten();
        if let Some(words) = words {
            let mut out = ordinal_items(&words);
            let c = Chunk {
                text: rest.to_string(),
                dot: chunk.dot,
                tight: false,
            };
            let mut more = expand(&c, nums, ascii_english, compat, natural, None);
            if let Some(first) = more.first_mut() {
                first.hyphen = true;
            }
            out.append(&mut more);
            return out;
        }
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
    // That sign, for the agreement of its word with the amount.
    let mut sign_before: Option<char> = None;
    // Where the items of the last number start (`percent-before`), and
    // its whole units.
    let mut number_at = 0usize;
    let mut amount: Option<u64> = None;
    for (i, (sc, r)) in rs.iter().enumerate() {
        if i > 0 && rs[i - 1].0 == Script::Digit && !after_amount.is_empty() {
            if let (Some(c), true) = (sign_before.take(), natural) {
                let words: Vec<String> = after_amount.iter().map(|x| x.text.clone()).collect();
                after_amount = currency_words(nums, c, &words, amount);
                count_words(&mut out, number_at, nums, c, amount);
            }
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
                amount = whole_amount(r, nums);
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
                        let after = i > 0 && rs[i - 1].0 == Script::Digit;
                        if !compat && before && r.chars().count() == 1 {
                            after_amount.extend(items);
                            sign_before = Some(c);
                        } else if natural && after {
                            // The word agrees with the amount ("25 euros").
                            count_words(&mut out, number_at, nums, c, amount);
                            out.extend(currency_words(nums, c, words, amount));
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
