//! A small engine for ordered, context-sensitive rewrite rules, the building
//! block of the data-driven languages (see `docs/adding-a-language.md`).
//!
//! A rule set is written as text, one rule per line, in the notation of
//! phonological rules:
//!
//! ```text
//! # a comment line; `;` starts a comment after a rule
//! V = a e i o u á é í ó ú          ; a class (members may be several characters)
//! c -> θ / _ {E}                     ; c before a member of class E
//! qu -> k / _ {E}
//! x -> ks
//! n -> ŋ / _ {K}
//! s -> 0 / {V} _ #                    ; 0 deletes
//! ll -> ʎ
//! ```
//!
//! The text left of `->` is matched literally. The optional environment after
//! `/` is `LEFT _ RIGHT`, where each side is a sequence of:
//!
//! * a literal character (`\` escapes one of the special ones);
//! * `#`, a word boundary;
//! * `{Name}`, any member of a class; `{!Name}`, one character outside it;
//! * `.`, any one character;
//! * `*` after any of those: zero or more times.
//!
//! Several environments for the same rewrite can be given with `|`.
//!
//! Rules are tried at each position of the input, longest match first and,
//! for matches of the same length, in the order written; the first rule
//! whose environment holds is applied and the scan continues after the
//! matched text. Environments look at the *input* of the pass, not at what
//! earlier rules wrote (all rules of a pass apply "simultaneously"); chain
//! passes when one rule must see another's result. Characters no rule
//! matches are copied unchanged.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

/// A small multiplicative hasher for `char` keys (the standard hasher is
/// built to resist adversarial keys, which costs time here).
#[derive(Default, Clone, Copy)]
pub struct CharHasher(u64);

impl Hasher for CharHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0.rotate_left(5) ^ u64::from(b)).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
        }
    }
    fn write_u32(&mut self, n: u32) {
        self.0 = (self.0.rotate_left(5) ^ u64::from(n)).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
}

/// A `HashMap` keyed by `char` with [`CharHasher`].
pub type CharMap<V> = HashMap<char, V, BuildHasherDefault<CharHasher>>;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Atom {
    Lit(char),
    Class(u16, bool),
    Any,
    Boundary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Elem {
    atom: Atom,
    star: bool,
}

#[derive(Clone, Debug)]
struct Rule {
    pat: Vec<char>,
    /// A rule written `{Class} -> …` matches one member of the class; an
    /// output of `=` copies it.
    class: Option<u16>,
    out: String,
    /// Alternative environments: (left context, right context). Left
    /// contexts are stored in reading order.
    envs: Vec<(Vec<Elem>, Vec<Elem>)>,
}

/// A parse error with its line number (1-based).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleError {
    /// Line of the rule text.
    pub line: usize,
    /// What is wrong.
    pub message: String,
}

impl std::fmt::Display for RuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

/// Named classes shared by the passes of one language.
#[derive(Clone, Debug, Default)]
pub struct Classes {
    names: Vec<String>,
    /// Members of each class, longest first.
    members: Vec<Vec<Vec<char>>>,
}

impl Classes {
    /// Define (or redefine) a class.
    pub fn define(&mut self, name: &str, members: &[&str]) {
        let mut m: Vec<Vec<char>> = members.iter().map(|s| s.chars().collect()).collect();
        m.sort_by_key(|x| std::cmp::Reverse(x.len()));
        if let Some(i) = self.names.iter().position(|n| n == name) {
            self.members[i] = m;
        } else {
            self.names.push(name.to_string());
            self.members.push(m);
        }
    }

    fn index(&self, name: &str) -> Option<u16> {
        self.names.iter().position(|n| n == name).map(|i| i as u16)
    }

    /// True when `c` is a one-character member of the class.
    pub fn contains(&self, name: &str, c: char) -> bool {
        self.index(name).is_some_and(|i| {
            self.members[i as usize]
                .iter()
                .any(|m| m.len() == 1 && m[0] == c)
        })
    }

    /// Parse a `Name = a b c` line; `None` when the line is not a class definition.
    pub fn parse_line(&mut self, line: &str) -> Option<()> {
        let (name, rest) = line.split_once('=')?;
        let name = name.trim();
        if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return None;
        }
        let ms: Vec<&str> = rest.split_whitespace().collect();
        self.define(name, &ms);
        Some(())
    }
}

/// One pass of rewrite rules.
#[derive(Clone, Debug, Default)]
pub struct RuleSet {
    rules: Vec<Rule>,
    /// Rules by first character of the match, in priority order.
    index: CharMap<Vec<u16>>,
}

fn parse_side(s: &str, classes: &Classes, line: usize) -> Result<Vec<Elem>, RuleError> {
    let err = |m: &str| RuleError {
        line,
        message: m.to_string(),
    };
    let mut out: Vec<Elem> = Vec::new();
    let cs: Vec<char> = s.chars().filter(|c| !c.is_whitespace()).collect();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        let atom = match c {
            '*' => {
                let last = out
                    .last_mut()
                    .ok_or_else(|| err("`*` with nothing before it"))?;
                last.star = true;
                i += 1;
                continue;
            }
            '#' => Atom::Boundary,
            '.' => Atom::Any,
            '\\' => {
                i += 1;
                Atom::Lit(*cs.get(i).ok_or_else(|| err("`\\` at end"))?)
            }
            '{' => {
                let end = cs[i..]
                    .iter()
                    .position(|&x| x == '}')
                    .ok_or_else(|| err("unclosed `{`"))?;
                let inner: String = cs[i + 1..i + end].iter().collect();
                let (neg, name) = match inner.strip_prefix('!') {
                    Some(n) => (true, n),
                    None => (false, inner.as_str()),
                };
                let idx = classes
                    .index(name)
                    .ok_or_else(|| err(&format!("unknown class {{{name}}}")))?;
                i += end;
                Atom::Class(idx, neg)
            }
            c => Atom::Lit(c),
        };
        out.push(Elem { atom, star: false });
        i += 1;
    }
    Ok(out)
}

impl RuleSet {
    /// Parse rules (class definitions may be mixed in; they are added to `classes`).
    pub fn parse(text: &str, classes: &mut Classes) -> Result<RuleSet, RuleError> {
        let mut rs = RuleSet::default();
        for (n, raw) in text.lines().enumerate() {
            let line = n + 1;
            let l = strip_comment(raw).trim();
            if l.is_empty() {
                continue;
            }
            let Some((lhs, rhs)) = l.split_once("->") else {
                if classes.parse_line(l).is_some() {
                    continue;
                }
                return Err(RuleError {
                    line,
                    message: format!("expected `match -> output` or `Class = …`: {l}"),
                });
            };
            let lhs_t = lhs.trim();
            let class = match lhs_t.strip_prefix('{').and_then(|x| x.strip_suffix('}')) {
                Some(name) => Some(classes.index(name).ok_or_else(|| RuleError {
                    line,
                    message: format!("unknown class {{{name}}}"),
                })?),
                None => None,
            };
            let pat: Vec<char> = if class.is_some() {
                vec!['\0']
            } else {
                unescape(lhs_t).chars().collect()
            };
            if pat.is_empty() {
                return Err(RuleError {
                    line,
                    message: "empty match".into(),
                });
            }
            let (out, env) = match rhs.split_once('/') {
                Some((o, e)) => (o.trim(), Some(e)),
                None => (rhs.trim(), None),
            };
            let out = if out == "0" {
                String::new()
            } else {
                unescape(out)
            };
            let mut envs = Vec::new();
            match env {
                None => envs.push((Vec::new(), Vec::new())),
                Some(e) => {
                    for alt in e.split('|') {
                        let Some((l, r)) = alt.split_once('_') else {
                            return Err(RuleError {
                                line,
                                message: "environment needs `_`".into(),
                            });
                        };
                        envs.push((parse_side(l, classes, line)?, parse_side(r, classes, line)?));
                    }
                }
            }
            rs.rules.push(Rule {
                pat,
                class,
                out,
                envs,
            });
        }
        rs.build_index(classes);
        Ok(rs)
    }

    fn build_index(&mut self, classes: &Classes) {
        let mut order: Vec<usize> = (0..self.rules.len()).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(self.rules[i].pat.len()));
        self.index.clear();
        for i in order {
            match self.rules[i].class {
                Some(c) => {
                    for m in &classes.members[c as usize] {
                        if m.len() == 1 {
                            self.index.entry(m[0]).or_default().push(i as u16);
                        }
                    }
                }
                None => self
                    .index
                    .entry(self.rules[i].pat[0])
                    .or_default()
                    .push(i as u16),
            }
        }
    }

    /// Number of rules.
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// True when the set has no rules.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Rewrite `input` (one word, without boundaries) and append to `out`.
    pub fn apply_to(&self, input: &[char], classes: &Classes, out: &mut String) {
        let mut i = 0;
        'pos: while i < input.len() {
            if let Some(cands) = self.index.get(&input[i]) {
                for &r in cands {
                    let rule = &self.rules[r as usize];
                    let n = rule.pat.len();
                    if rule.class.is_none()
                        && (i + n > input.len() || input[i..i + n] != rule.pat[..])
                    {
                        continue;
                    }
                    let ok = rule.envs.iter().any(|(l, r)| {
                        match_left(l, input, i, classes) && match_right(r, input, i + n, classes)
                    });
                    if ok {
                        if rule.class.is_some() && rule.out == "=" {
                            out.push(input[i]);
                        } else {
                            out.push_str(&rule.out);
                        }
                        i += n;
                        continue 'pos;
                    }
                }
            }
            out.push(input[i]);
            i += 1;
        }
    }

    /// Rewrite a word.
    pub fn apply(&self, input: &str, classes: &Classes) -> String {
        let cs: Vec<char> = input.chars().collect();
        let mut out = String::with_capacity(input.len() + 8);
        self.apply_to(&cs, classes, &mut out);
        out
    }
}

fn strip_comment(l: &str) -> &str {
    // A line whose first character is `#` is a comment; elsewhere `;` starts a
    // comment (`#` inside a rule is a word boundary).
    if l.trim_start().starts_with('#') {
        return "";
    }
    let b = l.as_bytes();
    for (i, &c) in b.iter().enumerate() {
        if c == b';' && (i == 0 || b[i - 1] != b'\\') {
            return &l[..i];
        }
    }
    l
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut esc = false;
    for c in s.chars() {
        if esc {
            out.push(c);
            esc = false;
        } else if c == '\\' {
            esc = true;
        } else if !c.is_whitespace() {
            out.push(c);
        }
    }
    out
}

/// Match one atom at `pos` going forward and call `k` with each end
/// position until it returns true.
fn atom_fwd(
    a: &Atom,
    input: &[char],
    pos: usize,
    classes: &Classes,
    k: &mut dyn FnMut(usize) -> bool,
) -> bool {
    match a {
        Atom::Boundary => pos == input.len() && k(pos),
        Atom::Any => pos < input.len() && k(pos + 1),
        Atom::Lit(c) => input.get(pos) == Some(c) && k(pos + 1),
        Atom::Class(i, neg) => {
            let ms = &classes.members[*i as usize];
            if *neg {
                pos < input.len()
                    && !ms.iter().any(|m| m.len() == 1 && m[0] == input[pos])
                    && k(pos + 1)
            } else {
                let rest = &input[pos.min(input.len())..];
                ms.iter().any(|m| rest.starts_with(m) && k(pos + m.len()))
            }
        }
    }
}

/// Match one atom ending at `end` (exclusive) going backward and call `k`
/// with each start position until it returns true.
fn atom_bwd(
    a: &Atom,
    input: &[char],
    end: usize,
    classes: &Classes,
    k: &mut dyn FnMut(usize) -> bool,
) -> bool {
    match a {
        Atom::Boundary => end == 0 && k(0),
        Atom::Any => end > 0 && k(end - 1),
        Atom::Lit(c) => end > 0 && input[end - 1] == *c && k(end - 1),
        Atom::Class(i, neg) => {
            let ms = &classes.members[*i as usize];
            if *neg {
                end > 0 && !ms.iter().any(|m| m.len() == 1 && m[0] == input[end - 1]) && k(end - 1)
            } else {
                let head = &input[..end];
                ms.iter().any(|m| head.ends_with(m) && k(end - m.len()))
            }
        }
    }
}

fn match_right(elems: &[Elem], input: &[char], pos: usize, classes: &Classes) -> bool {
    let Some((e, rest)) = elems.split_first() else {
        return true;
    };
    if e.star {
        // Zero occurrences, or one more and try again.
        if match_right(rest, input, pos, classes) {
            return true;
        }
        if e.atom == Atom::Boundary {
            return false;
        }
        return atom_fwd(&e.atom, input, pos, classes, &mut |p| {
            p > pos && match_right(elems, input, p, classes)
        });
    }
    atom_fwd(&e.atom, input, pos, classes, &mut |p| {
        match_right(rest, input, p, classes)
    })
}

fn match_left(elems: &[Elem], input: &[char], end: usize, classes: &Classes) -> bool {
    let Some((e, rest)) = elems.split_last() else {
        return true;
    };
    if e.star {
        if match_left(rest, input, end, classes) {
            return true;
        }
        if e.atom == Atom::Boundary {
            return false;
        }
        return atom_bwd(&e.atom, input, end, classes, &mut |p| {
            p < end && match_left(elems, input, p, classes)
        });
    }
    atom_bwd(&e.atom, input, end, classes, &mut |p| {
        match_left(rest, input, p, classes)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(rules: &str, w: &str) -> String {
        let mut cl = Classes::default();
        let rs = RuleSet::parse(rules, &mut cl).unwrap();
        rs.apply(w, &cl)
    }

    #[test]
    fn literal_context_and_order() {
        let r = "E = e i é í\nc -> θ / _ {E}\nc -> k\nch -> tʃ\nqu -> k / _ {E}\n";
        assert_eq!(run(r, "cinco"), "θinko");
        assert_eq!(run(r, "chico"), "tʃiko");
        assert_eq!(run(r, "queso"), "keso");
        assert_eq!(run(r, "cuando"), "kuando");
    }

    #[test]
    fn boundaries_negation_star_and_deletion() {
        let r = "V = a e i o u\nC = b d g k l m n p r s t\n\
                 s -> 0 / {V} _ #\n\
                 r -> ɾ / {V} _ {V}\n\
                 n -> ŋ / _ {!V} {C}* #\n\
                 a -> A / # {C}* _   ; first vowel after initial consonants\n";
        assert_eq!(run(r, "basas"), "bAsa");
        assert_eq!(run(r, "para"), "pAɾa");
        assert_eq!(run(r, "bank"), "bAŋk");
        assert_eq!(run(r, "strand"), "strAŋd");
        assert_eq!(run(r, "nana"), "nAna");
    }

    #[test]
    fn multi_char_classes_and_alternatives() {
        let r = "L = aː iː uː\nh -> ʰ / {L} _ | _ #\n";
        assert_eq!(run(r, "aːhb"), "aːʰb");
        assert_eq!(run(r, "ahb"), "ahb");
        assert_eq!(run(r, "bah"), "baʰ");
    }

    #[test]
    fn class_matches() {
        let r = "M = ा ि\n{M} -> 0 / ड़ _\n";
        assert_eq!(run(r, "पड़ा"), "पड़");
        assert_eq!(run(r, "पढ़ा"), "पढ़ा");
    }

    #[test]
    fn errors_have_line_numbers() {
        let mut cl = Classes::default();
        let e = RuleSet::parse("a -> b\nc -> d / _ {Nope}\n", &mut cl).unwrap_err();
        assert_eq!(e.line, 2);
    }
}
