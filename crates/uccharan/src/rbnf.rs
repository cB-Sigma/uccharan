//! Number words from rule-based number format rules (Unicode CLDR
//! `common/rbnf`), for the written form of numbers in normalise-only mode.
//!
//! The files in `lang/numbers/` hold the `%spellout-numbering` and
//! `%spellout-cardinal` rule sets of a language and the private sets they
//! use, as CLDR writes them, after three header lines (`point:`, `minus:`,
//! `percent:`). A rule is `base: text;`. In the text `<<` stands for the
//! number divided by the rule's divisor (the highest power of ten not above
//! its base), `>>` for the remainder, `<%set<` and `>%set>` read those with
//! another rule set, `=%set=` reads the number itself with another set, and
//! `[a|b]` is `a` when the remainder is not zero and `b` otherwise.

use std::collections::HashMap;

#[derive(Debug)]
struct Rule {
    base: u64,
    radix: u64,
    text: String,
}

/// The rule sets of one language.
#[derive(Debug, Default)]
pub struct Rbnf {
    sets: Vec<Vec<Rule>>,
    names: HashMap<String, usize>,
    /// Word for the decimal sign.
    pub point: String,
    /// Word before a negative number.
    pub minus: String,
    /// Word for `%` after a number.
    pub percent: String,
}

/// The ordinal rule file of a language, when it has one.
pub fn ordinal_rules_for(tag: &str) -> Option<&'static str> {
    Some(match tag {
        "ca" => include_str!("../lang/numbers/ord-ca.txt"),
        "cs" => include_str!("../lang/numbers/ord-cs.txt"),
        "da" => include_str!("../lang/numbers/ord-da.txt"),
        "de" => include_str!("../lang/numbers/ord-de.txt"),
        "es" => include_str!("../lang/numbers/ord-es.txt"),
        "fi" => include_str!("../lang/numbers/ord-fi.txt"),
        "fr" | "fr-be" | "fr-ch" => include_str!("../lang/numbers/ord-fr.txt"),
        "hu" => include_str!("../lang/numbers/ord-hu.txt"),
        "it" => include_str!("../lang/numbers/ord-it.txt"),
        "nb" => include_str!("../lang/numbers/ord-nb.txt"),
        "nl" => include_str!("../lang/numbers/ord-nl.txt"),
        "pl" => include_str!("../lang/numbers/ord-pl.txt"),
        "pt" | "pt-br" => include_str!("../lang/numbers/ord-pt.txt"),
        "ro" => include_str!("../lang/numbers/ord-ro.txt"),
        "sv" => include_str!("../lang/numbers/ord-sv.txt"),
        "tr" => include_str!("../lang/numbers/ord-tr.txt"),
        _ => return None,
    })
}

/// The rule file of a language, when it has one.
pub fn rules_for(tag: &str) -> Option<&'static str> {
    Some(match tag {
        "bn" => include_str!("../lang/numbers/bn.txt"),
        "gu" => include_str!("../lang/numbers/gu.txt"),
        "kn" => include_str!("../lang/numbers/kn.txt"),
        "kok" => include_str!("../lang/numbers/kok.txt"),
        "ml" => include_str!("../lang/numbers/ml.txt"),
        "mr" => include_str!("../lang/numbers/mr.txt"),
        "ne" => include_str!("../lang/numbers/ne.txt"),
        "or" => include_str!("../lang/numbers/or.txt"),
        "pa" => include_str!("../lang/numbers/pa.txt"),
        "si" => include_str!("../lang/numbers/si.txt"),
        "ta" => include_str!("../lang/numbers/ta.txt"),
        "te" => include_str!("../lang/numbers/te.txt"),
        "ur" => include_str!("../lang/numbers/ur.txt"),
        _ => return None,
    })
}

impl Rbnf {
    /// Parse a rule file.
    pub fn parse(text: &str) -> Rbnf {
        let mut r = Rbnf::default();
        let mut cur: Option<usize> = None;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(v) = line.strip_prefix("point:") {
                r.point = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("minus:") {
                r.minus = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("percent:") {
                r.percent = v.trim().to_string();
            } else if line.starts_with('%') && line.ends_with(':') {
                let name = line.trim_end_matches(':').to_string();
                r.names.insert(name, r.sets.len());
                cur = Some(r.sets.len());
                r.sets.push(Vec::new());
            } else if let (Some(c), Some((head, body))) = (cur, line.split_once(':')) {
                // Rules for negative numbers, fractions and infinity are not used.
                let (base, radix) = match head.split_once('/') {
                    Some((b, x)) => (
                        b.trim().parse::<u64>(),
                        x.trim().parse::<u64>().unwrap_or(10),
                    ),
                    None => (head.trim().parse::<u64>(), 10),
                };
                let Ok(base) = base else { continue };
                let body = body.trim().trim_end_matches(';');
                let body = body.strip_prefix('\'').unwrap_or(body);
                r.sets[c].push(Rule {
                    base,
                    radix: radix.max(2),
                    text: body.to_string(),
                });
            }
        }
        for s in &mut r.sets {
            s.sort_by_key(|x| x.base);
        }
        r
    }

    /// The words for `n` ("%spellout-numbering").
    pub fn spell(&self, n: u64) -> String {
        let mut out = String::new();
        if let Some(&s) = self.names.get("%spellout-numbering") {
            self.format(s, n, &mut out, 0);
        }
        out.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// The words for `n` by the rule set `name` (`%spellout-ordinal`), if
    /// the rules have it.
    pub fn spell_in(&self, name: &str, n: u64) -> Option<String> {
        let &s = self.names.get(name)?;
        let mut out = String::new();
        self.format(s, n, &mut out, 0);
        let w = out.split_whitespace().collect::<Vec<_>>().join(" ");
        (!w.is_empty()).then_some(w)
    }

    /// True when the rules have the set `name`.
    pub fn has(&self, name: &str) -> bool {
        self.names.contains_key(name)
    }

    fn format(&self, set: usize, n: u64, out: &mut String, depth: u8) {
        let rules = &self.sets[set];
        let Some(mut i) = rules.iter().rposition(|r| r.base <= n) else {
            return;
        };
        if depth > 40 {
            return;
        }
        let divisor = |r: &Rule| {
            let mut d = 1u64;
            while d.checked_mul(r.radix).is_some_and(|x| x <= r.base) {
                d *= r.radix;
            }
            d
        };
        let mut d = divisor(&rules[i]);
        // A rule with both parts whose base is not a multiple of its divisor
        // gives way to the rule before it for exact multiples.
        let t = &rules[i].text;
        if i > 0 && t.contains('<') && t.contains('>') && n % d == 0 && rules[i].base % d != 0 {
            i -= 1;
            d = divisor(&rules[i]);
        }
        self.emit(&rules[i].text, set, n, d, out, depth);
    }

    fn emit(&self, text: &str, set: usize, n: u64, d: u64, out: &mut String, depth: u8) {
        let cs: Vec<char> = text.chars().collect();
        let mut i = 0;
        let named = |name: &str| -> usize {
            if name.is_empty() {
                set
            } else {
                self.names.get(name).copied().unwrap_or(set)
            }
        };
        while i < cs.len() {
            let c = cs[i];
            match c {
                '<' | '>' | '=' => {
                    let Some(j) = cs[i + 1..].iter().position(|&x| x == c) else {
                        break;
                    };
                    let inner: String = cs[i + 1..i + 1 + j].iter().collect();
                    i += j + 2;
                    // ">>>" is read as ">>".
                    while c == '>' && cs.get(i) == Some(&'>') {
                        i += 1;
                    }
                    match c {
                        '<' => self.format(named(&inner), n / d, out, depth + 1),
                        '>' => self.format(named(&inner), n % d, out, depth + 1),
                        _ if inner.starts_with('%') => {
                            self.format(named(&inner), n, out, depth + 1)
                        }
                        _ => out.push_str(&n.to_string()),
                    }
                    continue;
                }
                '[' => {
                    let j = cs[i..]
                        .iter()
                        .position(|&x| x == ']')
                        .unwrap_or(cs.len() - i - 1);
                    let inner: String = cs[i + 1..i + j].iter().collect();
                    let (a, b) = match inner.split_once('|') {
                        Some((a, b)) => (a.to_string(), b.to_string()),
                        None => (inner, String::new()),
                    };
                    self.emit(if n % d != 0 { &a } else { &b }, set, n, d, out, depth);
                    i += j + 1;
                    continue;
                }
                // `$(cardinal,one{million}other{millionen})$`: the form
                // for the count of the rule's divisor.
                '$' if cs.get(i + 1) == Some(&'(') => {
                    let rest: String = cs[i + 2..].iter().collect();
                    let Some(end) = rest.find(")$") else { break };
                    let q = n / d;
                    let cat = if q == 1 {
                        "one"
                    } else if (2..=4).contains(&(q % 10)) && !(12..=14).contains(&(q % 100)) {
                        "few"
                    } else {
                        "other"
                    };
                    let body = &rest[..end];
                    let pick = |c: &str| {
                        let at = body.find(&format!("{c}{{"))?;
                        let v = &body[at + c.len() + 1..];
                        Some(v[..v.find('}')?].to_string())
                    };
                    if let Some(w) = pick(cat).or_else(|| pick("other")) {
                        out.push_str(&w);
                    }
                    i += 2 + rest[..end].chars().count() + 2;
                    continue;
                }
                '\u{ad}' => {}
                _ => out.push(c),
            }
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules() {
        let r = Rbnf::parse(
            "point: point\n%spellout-numbering:\n0: =%spellout-cardinal=;\n%spellout-cardinal:\n\
             0: zero;\n1: one;\n2: two;\n3: three;\n4: four;\n5: five;\n20: twenty[->>];\n\
             100: << hundred[ >>|s];\n1000: << thousand[ >>];\n",
        );
        assert_eq!(r.spell(25), "twenty-five");
        assert_eq!(r.spell(300), "three hundreds");
        assert_eq!(r.spell(2105), "two thousand one hundred five");
    }

    #[test]
    fn languages() {
        let s = |tag: &str, n: u64| Rbnf::parse(rules_for(tag).unwrap()).spell(n);
        assert_eq!(s("ta", 25), "இருபத்து ஐந்து");
        assert_eq!(s("ta", 2024), "இரண்டாயிரத்து இருபத்து நான்கு");
        assert_eq!(s("te", 125), "నూట ఇరవై ఐదు");
        assert_eq!(s("mr", 125), "एकशे पंचवीस");
        assert_eq!(s("bn", 2345), "দুই হাজার তিনশো পঁয়তাল্লিশ");
        assert_eq!(s("ne", 3), "तीन");
        assert_eq!(s("kn", 125), "ನೂರ ಇಪ್ಪತ್ತೈದು");
    }
}
