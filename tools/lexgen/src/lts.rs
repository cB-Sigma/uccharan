//! Letter-to-sound model: per-letter context tables with back-off.
//!
//! For every letter of every aligned lexicon word the trainer records which
//! phoneme string that letter produced, keyed by the letter and a window of
//! neighbouring letters. Seven window shapes are kept, from wide to narrow;
//! an entry is stored only when it disagrees with what the narrower shapes
//! would already predict, so the tables stay small. The model is derived
//! from the permissive lexicon only.

use std::collections::hash_map::DefaultHasher;
use std::hash::BuildHasherDefault;

/// A hash map with a fixed hasher, so that iteration order (and therefore the
/// built data) is the same on every run.
type HashMap<K, V> = std::collections::HashMap<K, V, BuildHasherDefault<DefaultHasher>>;
use uccharan::data::{shape_hash, stress_state};

pub const SHAPES: [(u8, u8); 8] = [
    (4, 4),
    (3, 3),
    (2, 3),
    (2, 2),
    (1, 2),
    (1, 1),
    (0, 1),
    (0, 0),
];

pub struct Model {
    pub outputs: Vec<String>,
    pub tables: Vec<(u8, u8, Vec<(u32, u16)>)>,
}

/// `examples`: for each word, its letters and one output string per letter.
pub fn train(examples: &[(Vec<u8>, Vec<String>)], min_count: u32) -> Model {
    train_shapes(examples, min_count, &SHAPES)
}

/// [`train`] with a chosen back-off chain, most specific shape first. The
/// shape bytes may carry the flags of [`shape_hash`].
pub fn train_shapes(
    examples: &[(Vec<u8>, Vec<String>)],
    min_count: u32,
    shapes: &[(u8, u8)],
) -> Model {
    train_full(examples, None, min_count, shapes)
}

/// A letter that the model does not decide (see [`train_full`]).
pub const SKIP: &str = "\u{1}";

/// [`train_shapes`] with the stress states given (`None`: from the outputs
/// themselves). Letters whose output is [`SKIP`] are not learned.
pub fn train_full(
    examples: &[(Vec<u8>, Vec<String>)],
    given: Option<&[Vec<u32>]>,
    min_count: u32,
    shapes: &[(u8, u8)],
) -> Model {
    let mut out_ids: HashMap<String, u16> = HashMap::default();
    let mut outputs: Vec<String> = Vec::new();
    let mut id_of = |s: &str| -> u16 {
        if let Some(&i) = out_ids.get(s) {
            return i;
        }
        let i = outputs.len() as u16;
        outputs.push(s.to_string());
        out_ids.insert(s.to_string(), i);
        i
    };
    // The stress state before each letter, from the outputs to its left.
    let own: Vec<Vec<u32>> = examples
        .iter()
        .map(|(_, outs)| {
            let mut s = 0u8;
            outs.iter()
                .map(|o| {
                    let before = s as u32;
                    s = stress_state(s, o);
                    before
                })
                .collect()
        })
        .collect();
    let states: &[Vec<u32>] = given.unwrap_or(&own);
    // counts[shape][context hash][output] = n
    let mut counts: Vec<HashMap<u32, HashMap<u16, u32>>> =
        shapes.iter().map(|_| HashMap::default()).collect();
    for ((letters, outs), st) in examples.iter().zip(states) {
        for (i, o) in outs.iter().enumerate() {
            if o == SKIP {
                continue;
            }
            let oid = id_of(o);
            for (si, &(l, r)) in shapes.iter().enumerate() {
                let h = shape_hash(letters, i, l, r, st[i]);
                *counts[si].entry(h).or_default().entry(oid).or_insert(0) += 1;
            }
        }
    }
    // Best output per context.
    let best: Vec<HashMap<u32, (u16, u32)>> = counts
        .iter()
        .map(|m| {
            m.iter()
                .map(|(&h, c)| {
                    let (&o, &n) = c
                        .iter()
                        .max_by_key(|(&o, &n)| (n, std::cmp::Reverse(o)))
                        .unwrap();
                    (h, (o, n))
                })
                .collect()
        })
        .collect();
    // Prune from narrow to wide: keep an entry only if the narrower chain disagrees.
    let mut kept: Vec<HashMap<u32, u16>> = shapes.iter().map(|_| HashMap::default()).collect();
    let last = shapes.len() - 1;
    for (&h, &(o, _)) in &best[last] {
        kept[last].insert(h, o);
    }
    for si in (0..last).rev() {
        let (l, r) = shapes[si];
        let wide = (l & 15) + (r & 15) >= 4;
        // Re-walk the examples so that the back-off prediction can be computed per context.
        let mut decided: HashMap<u32, bool> = HashMap::default();
        for ((letters, outs), st) in examples.iter().zip(states) {
            for i in 0..outs.len() {
                if outs[i] == SKIP {
                    continue;
                }
                let h = shape_hash(letters, i, l, r, st[i]);
                if decided.contains_key(&h) {
                    continue;
                }
                let (o, n) = best[si][&h];
                let mut fallback = None;
                for sj in si + 1..shapes.len() {
                    let (l2, r2) = shapes[sj];
                    let h2 = shape_hash(letters, i, l2, r2, st[i]);
                    if let Some(&v) = kept[sj].get(&h2) {
                        fallback = Some(v);
                        break;
                    }
                }
                let keep = fallback != Some(o) && (!wide || n >= min_count);
                decided.insert(h, keep);
                if keep {
                    kept[si].insert(h, o);
                }
            }
        }
    }
    let tables = shapes
        .iter()
        .zip(kept)
        .map(|(&(l, r), m)| {
            let mut v: Vec<(u32, u16)> = m.into_iter().collect();
            v.sort_unstable();
            (l, r, v)
        })
        .collect::<Vec<_>>();
    // A table holds at most 65 535 contexts: the best supported ones stay.
    let tables = tables
        .into_iter()
        .zip(&best)
        .map(|((l, r, mut v), b)| {
            if v.len() > 65_535 {
                v.sort_by_key(|&(h, _)| (std::cmp::Reverse(b[&h].1), h));
                v.truncate(65_535);
                v.sort_unstable();
            }
            (l, r, v)
        })
        .collect();
    Model { outputs, tables }
}
