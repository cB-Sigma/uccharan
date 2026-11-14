//! Pronunciation scoring: align the phonemes of a reference with what was
//! heard, with the matcher's weighted distance, and give every reference
//! word its share of the cost.
//!
//! ```
//! use uccharan::soundalike::score::align_words;
//!
//! let a = align_words(&["nəmˈʌsteː", "dˈʊnɪjaː"], &["nəmˈʌsteː", "dˈʊnɪjaː"], false, false);
//! assert_eq!(a.cost, 0.0);
//! let b = align_words(&["nəmˈʌsteː", "dˈʊnɪjaː"], &["nəmˈʌsteː", "tˈʊnɪjaː"], false, false);
//! assert_eq!(b.words[0].cost, 0.0);
//! assert!(b.words[1].cost > 0.0 && b.words[1].score() > 0.5);
//! ```
//!
//! Costs are in phones: 1.0 is a consonant dropped or exchanged for an
//! unrelated sound; close sounds (voicing, aspiration, vowel length, a weak
//! vowel dropped) cost less.

use std::ops::Range;

use super::{del_costs, is_vowel, parse_ipa, show, sub_cost, tables, tidy, Ph};

/// What happened to one phone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpKind {
    /// The same phone on both sides.
    Match,
    /// A reference phone heard as another phone.
    Substitute,
    /// A reference phone that was not heard.
    Delete,
    /// A heard phone that the reference does not have.
    Insert,
}

impl OpKind {
    /// Name used in the bindings.
    pub fn name(self) -> &'static str {
        match self {
            OpKind::Match => "match",
            OpKind::Substitute => "substitute",
            OpKind::Delete => "delete",
            OpKind::Insert => "insert",
        }
    }
}

/// One step of the alignment.
#[derive(Clone, Debug, PartialEq)]
pub struct Op {
    /// What happened.
    pub kind: OpKind,
    /// The reference phone (empty for an insertion), in the matcher's notation.
    pub reference: String,
    /// The heard phone (empty for a deletion).
    pub heard: String,
    /// Cost of the step, 0 to 1.
    pub cost: f32,
}

/// The score of one reference word.
#[derive(Clone, Debug, PartialEq)]
pub struct WordScore {
    /// Cost of the steps that belong to the word.
    pub cost: f32,
    /// Weight of the word: the cost of dropping all of it, or of inserting
    /// everything heard in its place when that is more.
    pub weight: f32,
    /// Number of phones in the reference word.
    pub phones: usize,
    /// The heard words the word was aligned with.
    pub heard: Range<usize>,
    /// The steps, in order.
    pub ops: Vec<Op>,
}

impl WordScore {
    /// Similarity of the word with what was heard for it, 0 to 1.
    pub fn score(&self) -> f32 {
        if self.weight <= 0.0 {
            return if self.cost > 0.0 { 0.0 } else { 1.0 };
        }
        (1.0 - self.cost / self.weight).clamp(0.0, 1.0)
    }
}

/// An alignment of a reference with what was heard.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Alignment {
    /// One entry per reference word.
    pub words: Vec<WordScore>,
    /// Total cost.
    pub cost: f32,
    /// Cost of dropping the whole reference.
    pub weight: f32,
    /// Phones in the reference.
    pub reference_phones: usize,
    /// Phones in what was heard.
    pub heard_phones: usize,
    /// Phones heard as another phone.
    pub substitutions: usize,
    /// Reference phones that were not heard.
    pub deletions: usize,
    /// Heard phones the reference does not have.
    pub insertions: usize,
    /// Heard words that were inserted whole (no phone of theirs stands
    /// for a reference phone). Their cost is in `cost` but in no word.
    pub extra: Vec<usize>,
}

impl Alignment {
    /// Weighted phoneme error rate: total cost over the reference weight.
    pub fn weighted_error(&self) -> f32 {
        if self.weight <= 0.0 {
            return if self.cost > 0.0 { 1.0 } else { 0.0 };
        }
        self.cost / self.weight
    }

    /// Plain phoneme error rate: (substitutions + deletions + insertions)
    /// over the number of reference phones.
    pub fn error_rate(&self) -> f32 {
        let e = (self.substitutions + self.deletions + self.insertions) as f32;
        if self.reference_phones == 0 {
            return if e > 0.0 { 1.0 } else { 0.0 };
        }
        e / self.reference_phones as f32
    }
}

fn phones(words: &[&str], english: bool) -> (Vec<Ph>, Vec<u32>) {
    let (mut seq, mut owner) = (Vec::new(), Vec::new());
    let mut one = Vec::new();
    for (w, ipa) in words.iter().enumerate() {
        one.clear();
        parse_ipa(ipa, english, &mut one);
        tidy(&mut one);
        owner.resize(owner.len() + one.len(), w as u32);
        seq.extend_from_slice(&one);
    }
    (seq, owner)
}

/// Align the IPA of reference words with the IPA of heard words.
/// `*_english` selects the reading of `ɾ` on that side (the flapped t of
/// en-US output). Stress marks and symbols the matcher does not know are
/// ignored.
pub fn align_words(
    reference: &[&str],
    heard: &[&str],
    reference_english: bool,
    heard_english: bool,
) -> Alignment {
    let t = tables();
    let (a, wa) = phones(reference, reference_english);
    let (b, wb) = phones(heard, heard_english);
    let (mut da, mut db) = (Vec::new(), Vec::new());
    del_costs(&a, &mut da);
    del_costs(&b, &mut db);
    let (n, m) = (a.len(), b.len());
    let w = m + 1;
    // Full table: the path is needed, and a text is short.
    let mut d = vec![0u32; (n + 1) * w];
    for j in 0..m {
        d[j + 1] = d[j] + db[j] as u32;
    }
    for i in 0..n {
        d[(i + 1) * w] = d[i * w] + da[i] as u32;
        for j in 0..m {
            let v = (d[i * w + j] + sub_cost(t, a[i], b[j]))
                .min(d[i * w + j + 1] + da[i] as u32)
                .min(d[(i + 1) * w + j] + db[j] as u32);
            d[(i + 1) * w + j + 1] = v;
        }
    }
    // Walk back. Steps: (reference index, heard index).
    let mut steps: Vec<(Option<usize>, Option<usize>)> = Vec::with_capacity(n + m);
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        let here = d[i * w + j];
        if i > 0 && j > 0 && here == d[(i - 1) * w + j - 1] + sub_cost(t, a[i - 1], b[j - 1]) {
            steps.push((Some(i - 1), Some(j - 1)));
            i -= 1;
            j -= 1;
        } else if i > 0 && here == d[(i - 1) * w + j] + da[i - 1] as u32 {
            steps.push((Some(i - 1), None));
            i -= 1;
        } else {
            steps.push((None, Some(j - 1)));
            j -= 1;
        }
    }
    steps.reverse();

    let mut out = Alignment {
        words: (0..reference.len())
            .map(|_| WordScore {
                cost: 0.0,
                weight: 0.0,
                phones: 0,
                heard: 0..0,
                ops: Vec::new(),
            })
            .collect(),
        reference_phones: n,
        heard_phones: m,
        ..Alignment::default()
    };
    if out.words.is_empty() {
        out.insertions = m;
        out.cost = db.iter().map(|c| *c as f32 / 100.0).sum();
        out.extra = (0..heard.len())
            .filter(|&k| wb.contains(&(k as u32)))
            .collect();
        return out;
    }
    // A heard word whose phones are all insertions is an extra word: it is
    // reported as such and charged to no reference word.
    let mut matched = vec![false; heard.len()];
    let mut has_phones = vec![false; heard.len()];
    for &(ri, hj) in &steps {
        if let Some(j) = hj {
            has_phones[wb[j] as usize] = true;
            if ri.is_some() {
                matched[wb[j] as usize] = true;
            }
        }
    }
    out.extra = (0..heard.len())
        .filter(|&k| has_phones[k] && !matched[k])
        .collect();
    let mut ins_weight = vec![0u32; reference.len()];
    let mut heard_span: Vec<Option<(usize, usize)>> = vec![None; reference.len()];
    for (k, &(ri, hj)) in steps.iter().enumerate() {
        if let (None, Some(j)) = (ri, hj) {
            if !matched[wb[j] as usize] {
                out.insertions += 1;
                out.cost += db[j] as f32 / 100.0;
                continue;
            }
        }
        // An inserted phone belongs to the word of the reference phone
        // before it; at the very start, to the word of the next one. When
        // the heard word changes with the insertion and the next reference
        // phone starts a word, it belongs to that next word.
        let word = match ri {
            Some(i) => wa[i] as usize,
            None => {
                let prev = steps[..k].iter().rev().find_map(|s| s.0);
                let next = steps[k + 1..].iter().find_map(|s| s.0);
                let j = hj.unwrap_or(0);
                match (prev, next) {
                    (Some(p), Some(nx)) if wa[p] != wa[nx] => {
                        // The side whose heard phones share the word.
                        let with_next = steps[k + 1..]
                            .iter()
                            .find(|s| s.0.is_some() && s.1.is_some())
                            .is_some_and(|s| wb[s.1.unwrap_or(0)] == wb[j]);
                        let with_prev = steps[..k]
                            .iter()
                            .rev()
                            .find(|s| s.0.is_some() && s.1.is_some())
                            .is_some_and(|s| wb[s.1.unwrap_or(0)] == wb[j]);
                        if with_next && !with_prev {
                            wa[nx] as usize
                        } else {
                            wa[p] as usize
                        }
                    }
                    (Some(p), _) => wa[p] as usize,
                    (None, Some(nx)) => wa[nx] as usize,
                    (None, None) => 0,
                }
            }
        };
        let (kind, cost) = match (ri, hj) {
            (Some(i), Some(j)) => {
                let c = sub_cost(t, a[i], b[j]);
                if c == 0 {
                    (OpKind::Match, 0)
                } else {
                    out.substitutions += 1;
                    (OpKind::Substitute, c)
                }
            }
            (Some(i), None) => {
                out.deletions += 1;
                (OpKind::Delete, da[i] as u32)
            }
            (None, Some(j)) => {
                out.insertions += 1;
                (OpKind::Insert, db[j] as u32)
            }
            (None, None) => unreachable!(),
        };
        let ws = &mut out.words[word];
        if let Some(i) = ri {
            ws.weight += da[i] as f32 / 100.0;
            ws.phones += 1;
        }
        if let Some(j) = hj {
            ins_weight[word] += db[j] as u32;
            let hw = wb[j] as usize;
            heard_span[word] = Some(match heard_span[word] {
                Some((lo, hi)) => (lo.min(hw), hi.max(hw + 1)),
                None => (hw, hw + 1),
            });
        }
        ws.cost += cost as f32 / 100.0;
        ws.ops.push(Op {
            kind,
            reference: ri.map(|i| show(&a[i..=i])).unwrap_or_default(),
            heard: hj.map(|j| show(&b[j..=j])).unwrap_or_default(),
            cost: cost as f32 / 100.0,
        });
    }
    let mut at = 0usize;
    for (k, ws) in out.words.iter_mut().enumerate() {
        out.weight += ws.weight;
        out.cost += ws.cost;
        ws.weight = ws.weight.max(ins_weight[k] as f32 / 100.0);
        ws.heard = match heard_span[k] {
            Some((lo, hi)) => {
                at = hi;
                lo..hi
            }
            None => at..at,
        };
    }
    out
}

/// The phones of an IPA string in the matcher's broad notation, one string
/// per phone (stress and unknown symbols dropped).
pub fn broad_phones(ipa: &str, english: bool) -> Vec<String> {
    let mut v = Vec::new();
    parse_ipa(ipa, english, &mut v);
    tidy(&mut v);
    v.iter().map(|p| show(std::slice::from_ref(p))).collect()
}

/// True when the phone (in the matcher's notation) is a vowel.
pub fn is_vowel_phone(phone: &str) -> bool {
    let mut v = Vec::new();
    parse_ipa(phone, false, &mut v);
    v.first().is_some_and(|p| is_vowel(*p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_texts_cost_nothing() {
        let a = align_words(&["bʰˈaːɾət", "mẽː"], &["bʰˈaːɾət", "mẽː"], false, false);
        assert_eq!(a.cost, 0.0);
        assert_eq!(a.error_rate(), 0.0);
        assert_eq!(a.words.len(), 2);
        assert_eq!(a.words[0].heard, 0..1);
        assert_eq!(a.words[1].heard, 1..2);
        assert!(a.words.iter().all(|w| w.score() == 1.0));
    }

    #[test]
    fn close_sounds_are_cheap_and_stay_in_their_word() {
        // Aspiration lost in the first word, last word unrelated.
        let a = align_words(
            &["bʰˈaːɾət", "mẽː", "ɡʰˈoːɽeː"],
            &["bˈaːɾət", "mẽː", "kˈɪtaːb"],
            false,
            false,
        );
        assert!(a.words[0].cost > 0.0 && a.words[0].cost < 0.3);
        assert_eq!(a.words[1].cost, 0.0);
        assert!(a.words[2].score() < 0.6);
        assert_eq!(a.words[2].heard, 2..3);
        assert!(a.substitutions + a.deletions + a.insertions > 0);
        let total: f32 = a.words.iter().map(|w| w.cost).sum();
        assert!((total - a.cost).abs() < 1e-4);
    }

    #[test]
    fn missing_and_extra_words() {
        let a = align_words(&["ˈeːk", "dˈoː", "tˈiːn"], &["ˈeːk", "tˈiːn"], false, false);
        assert_eq!(a.words[0].cost, 0.0);
        assert_eq!(a.words[1].score(), 0.0);
        assert!(a.words[1].heard.is_empty());
        assert_eq!(a.words[2].cost, 0.0);
        assert_eq!(a.deletions, 2);

        let b = align_words(&["ˈeːk", "tˈiːn"], &["ˈeːk", "dˈoː", "tˈiːn"], false, false);
        assert_eq!(b.insertions, 2);
        assert!(b.cost > 1.0);
        assert_eq!(b.extra, [1]);
        assert!(b.words.iter().all(|w| w.cost == 0.0));
        assert_eq!(b.words[1].heard, 2..3);
        assert!(b.error_rate() > 0.0);

        let c = align_words(&[], &["ˈeːk"], false, false);
        assert!(c.words.is_empty() && c.insertions == 2);
        let d = align_words(&["ˈeːk"], &[], false, false);
        assert_eq!(d.words[0].score(), 0.0);
        assert_eq!(d.weighted_error(), 1.0);
    }

    #[test]
    fn broad_notation() {
        assert_eq!(broad_phones("bʰˈaːɾət", false), ["bʰ", "aː", "r", "ə", "t"]);
        assert!(is_vowel_phone("aː") && !is_vowel_phone("bʰ"));
    }
}
