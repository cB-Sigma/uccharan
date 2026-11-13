//! Letter ↔ phoneme alignment by expectation-maximisation.
//!
//! Each letter emits zero, one or two phonemes. The probabilities
//! `P(phonemes | letter)` start uniform and are re-estimated with
//! forward-backward; the final alignment is the Viterbi path.

use std::collections::hash_map::DefaultHasher;
use std::hash::BuildHasherDefault;

/// A hash map with a fixed hasher, so that iteration order (and therefore the
/// built data) is the same on every run.
type HashMap<K, V> = std::collections::HashMap<K, V, BuildHasherDefault<DefaultHasher>>;

type Unit = (u8, u8); // phoneme ids, 0 = none

pub struct Aligner {
    prob: HashMap<(u8, Unit), f64>,
}

const FLOOR: f64 = 1e-7;

impl Aligner {
    fn p(&self, letter: u8, unit: Unit) -> f64 {
        self.prob.get(&(letter, unit)).copied().unwrap_or(FLOOR)
    }

    /// Train on `(letters, phoneme ids)` pairs.
    /// `allowed(letter, first phoneme of the unit)` restricts the initial
    /// model; everything else keeps a tiny floor probability so that every
    /// word stays alignable.
    pub fn train(
        data: &[(Vec<u8>, Vec<u8>)],
        iterations: usize,
        allowed: impl Fn(u8, u8) -> bool,
    ) -> Self {
        let mut a = Aligner {
            prob: HashMap::default(),
        };
        // Initial guess: any unit is possible; pairs and deletions are less likely.
        for (ls, ps) in data {
            for &l in ls {
                a.prob.entry((l, (0, 0))).or_insert(0.05);
                for (j, &p) in ps.iter().enumerate() {
                    if !allowed(l, p) {
                        continue;
                    }
                    a.prob.entry((l, (p, 0))).or_insert(0.1);
                    if j + 1 < ps.len() {
                        a.prob.entry((l, (p, ps[j + 1]))).or_insert(0.002);
                    }
                }
            }
        }
        for _ in 0..iterations {
            let mut counts: HashMap<(u8, Unit), f64> = HashMap::default();
            for (ls, ps) in data {
                a.expect(ls, ps, &mut counts);
            }
            let mut totals: HashMap<u8, f64> = HashMap::default();
            for (&(l, _), &c) in &counts {
                *totals.entry(l).or_insert(0.0) += c;
            }
            // Only units that were possible in the initial model may gain probability.
            let keep = &a.prob;
            let next: HashMap<(u8, Unit), f64> = counts
                .into_iter()
                .filter(|(k, _)| keep.contains_key(k))
                .map(|((l, u), c)| ((l, u), (c / totals[&l]).max(FLOOR)))
                .collect();
            a.prob = next;
        }
        a
    }

    fn expect(&self, ls: &[u8], ps: &[u8], counts: &mut HashMap<(u8, Unit), f64>) {
        let (n, m) = (ls.len(), ps.len());
        if m > 2 * n {
            return;
        }
        let w = m + 1;
        let mut fwd = vec![0.0f64; (n + 1) * w];
        let mut bwd = vec![0.0f64; (n + 1) * w];
        fwd[0] = 1.0;
        for i in 0..n {
            for j in 0..=m {
                let f = fwd[i * w + j];
                if f == 0.0 {
                    continue;
                }
                fwd[(i + 1) * w + j] += f * self.p(ls[i], (0, 0));
                if j < m {
                    fwd[(i + 1) * w + j + 1] += f * self.p(ls[i], (ps[j], 0));
                }
                if j + 1 < m {
                    fwd[(i + 1) * w + j + 2] += f * self.p(ls[i], (ps[j], ps[j + 1]));
                }
            }
        }
        let z = fwd[n * w + m];
        if z <= 0.0 {
            return;
        }
        bwd[n * w + m] = 1.0;
        for i in (0..n).rev() {
            for j in 0..=m {
                let mut b = bwd[(i + 1) * w + j] * self.p(ls[i], (0, 0));
                if j < m {
                    b += bwd[(i + 1) * w + j + 1] * self.p(ls[i], (ps[j], 0));
                }
                if j + 1 < m {
                    b += bwd[(i + 1) * w + j + 2] * self.p(ls[i], (ps[j], ps[j + 1]));
                }
                bwd[i * w + j] = b;
            }
        }
        for i in 0..n {
            for j in 0..=m {
                let f = fwd[i * w + j];
                if f == 0.0 {
                    continue;
                }
                let mut add = |u: Unit, nj: usize| {
                    let c = f * self.p(ls[i], u) * bwd[(i + 1) * w + nj] / z;
                    if c > 0.0 {
                        *counts.entry((ls[i], u)).or_insert(0.0) += c;
                    }
                };
                add((0, 0), j);
                if j < m {
                    add((ps[j], 0), j + 1);
                }
                if j + 1 < m {
                    add((ps[j], ps[j + 1]), j + 2);
                }
            }
        }
    }

    /// Viterbi alignment: for each letter, the number of phonemes it emits.
    pub fn align(&self, ls: &[u8], ps: &[u8]) -> Option<Vec<u8>> {
        let (n, m) = (ls.len(), ps.len());
        if m > 2 * n {
            return None;
        }
        let w = m + 1;
        let neg = f64::NEG_INFINITY;
        let mut best = vec![neg; (n + 1) * w];
        let mut back = vec![0u8; (n + 1) * w];
        best[0] = 0.0;
        for i in 0..n {
            for j in 0..=m {
                let s = best[i * w + j];
                if s == neg {
                    continue;
                }
                let mut relax = |k: usize, u: Unit| {
                    let v = s + self.p(ls[i], u).ln();
                    let at = (i + 1) * w + j + k;
                    if v > best[at] {
                        best[at] = v;
                        back[at] = k as u8;
                    }
                };
                relax(0, (0, 0));
                if j < m {
                    relax(1, (ps[j], 0));
                }
                if j + 1 < m {
                    relax(2, (ps[j], ps[j + 1]));
                }
            }
        }
        if best[n * w + m] == neg {
            return None;
        }
        let mut out = vec![0u8; n];
        let mut j = m;
        for i in (0..n).rev() {
            let k = back[(i + 1) * w + j];
            out[i] = k;
            j -= k as usize;
        }
        Some(out)
    }
}
