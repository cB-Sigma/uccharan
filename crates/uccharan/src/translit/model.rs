//! The joint-sequence n-gram transliteration model (run-time half).
//!
//! A word pair is a sequence of tokens, one per romanised letter: the letter
//! and the zero, one or two native units it writes (a unit is a character,
//! with its nukta when it has one). An interpolated Kneser-Ney n-gram model
//! over the tokens scores a transliteration; a beam search over the letters
//! of a romanised word lists the best ones. The model is trained by
//! `tools/lexgen` (`lexgen xlit`).

use super::pack::{lookup3, mix, table3_ok, Sections};
use std::ops::Range;

/// Token id of the word start.
pub const BOS: u16 = 1;
/// Token id of the word end.
pub const EOS: u16 = 2;
/// Bits per token in a packed n-gram.
pub const BITS: u32 = 12;
/// Mask of one token in a packed n-gram.
pub const MASK: u128 = (1 << BITS) - 1;
/// Quantisation step (nats) of stored log-probabilities.
pub const P_STEP: f32 = 0.06;
/// Quantisation step (nats) of stored back-off weights.
pub const B_STEP: f32 = 0.04;
/// Longest n-gram order a pack may have.
pub const MAX_ORDER: usize = 8;

/// Split a native-script word into units: characters, a nukta staying with
/// its letter.
pub fn units(word: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for c in word.chars() {
        let u = c as u32;
        let nukta = (0x0900..0x0d80).contains(&u) && (u & 0x7f) == 0x3c;
        match out.last_mut() {
            Some(last) if nukta => last.push(c),
            _ => out.push(c.to_string()),
        }
    }
    out
}

/// The table key of a packed n-gram of `order` tokens.
#[inline]
pub fn key(order: usize, packed: u128) -> u64 {
    mix((packed as u64) ^ mix(((packed >> 64) as u64) ^ ((order as u64) << 56) ^ 0x51ed_270b))
}

/// The loaded model: what is built once from the pack. The tables stay in
/// the pack; every method takes the pack's sections.
#[derive(Clone, Debug)]
pub struct Model {
    order: usize,
    units: Vec<String>,
    /// Tokens: (letter 1..=26, unit, unit).
    tokens: Vec<(u8, u8, u8)>,
    by_letter: Vec<Vec<u16>>,
    /// Tokens seen after each token, grouped by letter.
    succ: Vec<[Vec<u16>; 27]>,
    tables: Vec<Range<usize>>,
}

struct Hyp {
    ctx: u128,
    len: usize,
    score: f32,
    /// Index of the output so far in the arena.
    node: u32,
    /// Hash of the output so far.
    out: u64,
}

impl Model {
    /// Read the model of a pack. `None` when the pack has none.
    pub fn load(pack: &Sections) -> Result<Option<Model>, String> {
        let t = pack.get(b"TOKS");
        let n = pack.get(b"NGRM");
        if t.is_empty() || n.is_empty() {
            return Ok(None);
        }
        let units: Vec<String> = std::str::from_utf8(pack.get(b"UNIT"))
            .map_err(|_| "bad unit table")?
            .split('\n')
            .map(str::to_string)
            .collect();
        let tokens: Vec<(u8, u8, u8)> = t.chunks_exact(3).map(|c| (c[0], c[1], c[2])).collect();
        if tokens.len() >= 1 << BITS
            || tokens
                .iter()
                .any(|t| t.0 > 26 || t.1 as usize >= units.len() || t.2 as usize >= units.len())
        {
            return Err("bad token table".into());
        }
        let mut by_letter = vec![Vec::new(); 27];
        for (i, &(l, _, _)) in tokens.iter().enumerate() {
            if l >= 1 && i > EOS as usize {
                by_letter[l as usize].push(i as u16);
            }
        }
        let order = n[0] as usize;
        if !(2..=MAX_ORDER).contains(&order) {
            return Err("bad model order".into());
        }
        let mut tables = Vec::new();
        let mut at = 1;
        for _ in 0..order {
            let len = n
                .get(at..at + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
                .ok_or("truncated model")?;
            let r = at + 4..at + 4 + len;
            if !n.get(r.clone()).is_some_and(table3_ok) {
                return Err("bad model table".into());
            }
            tables.push(r);
            at += 4 + len;
        }
        let mut succ: Vec<[Vec<u16>; 27]> = (0..tokens.len()).map(|_| Default::default()).collect();
        for c in pack.get(b"SUCC").chunks_exact(4) {
            let (a, b) = (
                u16::from_le_bytes([c[0], c[1]]) as usize,
                u16::from_le_bytes([c[2], c[3]]),
            );
            if a < tokens.len() && (b as usize) < tokens.len() {
                succ[a][tokens[b as usize].0 as usize].push(b);
            }
        }
        Ok(Some(Model {
            order,
            units,
            tokens,
            by_letter,
            succ,
            tables,
        }))
    }

    /// The n-gram order.
    pub fn order(&self) -> usize {
        self.order
    }

    #[inline]
    fn table<'a>(&self, ngrm: &'a [u8], k: usize) -> &'a [u8] {
        &ngrm[self.tables[k].clone()]
    }

    /// The back-off weights of a context for each of its lengths (index =
    /// number of tokens; 0 where the context is not stored). A hypothesis
    /// looks these up once, not once per token it is extended with.
    #[inline]
    fn backoffs(&self, ngrm: &[u8], ctx: u128, len: usize) -> [f32; MAX_ORDER] {
        let mut out = [0.0f32; MAX_ORDER];
        for (k, o) in out
            .iter_mut()
            .enumerate()
            .take(len.min(self.order - 1) + 1)
            .skip(1)
        {
            let c = ctx & ((1u128 << (BITS * k as u32)) - 1);
            if let Some([_, b]) = lookup3(self.table(ngrm, k - 1), key(k, c)) {
                *o = b as f32 * B_STEP;
            }
        }
        out
    }

    /// log P(token | context): `ctx` holds the previous tokens, the most
    /// recent in the low bits; `len` is how many of them are valid.
    #[inline]
    fn logp(&self, ngrm: &[u8], ctx: u128, len: usize, tok: u16, bows: &[f32; MAX_ORDER]) -> f32 {
        let mut acc = 0.0f32;
        let mut k = len.min(self.order - 1);
        loop {
            let c = if k == 0 {
                0
            } else {
                ctx & ((1u128 << (BITS * k as u32)) - 1)
            };
            let g = (c << BITS) | tok as u128;
            if let Some([p, _]) = lookup3(self.table(ngrm, k), key(k + 1, g)) {
                return acc - p as f32 * P_STEP;
            }
            if k == 0 {
                return acc - 14.0;
            }
            acc -= bows[k];
            k -= 1;
        }
    }

    /// The `nbest` best transliterations of a lower-case a–z word with
    /// their joint log-probabilities, best first. Hypotheses more than `gap`
    /// nats behind the best one are dropped at every letter.
    pub fn decode(
        &self,
        pack: &Sections,
        word: &str,
        beam: usize,
        nbest: usize,
        gap: f32,
    ) -> Vec<(String, f32)> {
        let ngrm = pack.get(b"NGRM");
        let letters: Vec<u8> = word
            .bytes()
            .filter(|b| b.is_ascii_lowercase())
            .map(|b| b - b'a' + 1)
            .collect();
        if letters.is_empty() || letters.len() > 48 || beam == 0 {
            return Vec::new();
        }
        let keep = (1u128 << (BITS * (self.order as u32 - 1))) - 1;
        // Outputs are chains in an arena: (parent, unit, unit).
        let mut arena: Vec<(u32, u8, u8)> = vec![(0, 0, 0)];
        let mut hyps = vec![Hyp {
            ctx: BOS as u128,
            len: 1,
            score: 0.0,
            node: 0,
            out: 0,
        }];
        // Candidates of the next letter: (score, context, parent hypothesis, token, output hash).
        let mut next: Vec<(f32, u128, u32, u16, u64)> = Vec::new();
        let mut seen: std::collections::HashSet<(u128, u64)> = Default::default();
        let mut prune = |next: &mut Vec<(f32, u128, u32, u16, u64)>| {
            next.sort_by(|a, b| b.0.total_cmp(&a.0));
            // Same context and same output: keep the best (the list is sorted).
            seen.clear();
            next.retain(|c| seen.insert((c.1, c.4)));
            next.truncate(beam);
        };
        for &l in &letters {
            next.clear();
            let mut worst = f32::NEG_INFINITY;
            let mut any = false;
            // Tokens seen after the previous one first; all tokens of the
            // letter only when no hypothesis has such a token.
            for pass in 0..2 {
                for (hi, h) in hyps.iter().enumerate() {
                    let prev = (h.ctx & MASK) as usize;
                    let list = if pass == 0 {
                        &self.succ[prev][l as usize]
                    } else {
                        &self.by_letter[l as usize]
                    };
                    if list.is_empty() {
                        continue;
                    }
                    let bows = self.backoffs(ngrm, h.ctx, h.len);
                    for &t in list {
                        any = true;
                        let s = h.score + self.logp(ngrm, h.ctx, h.len, t, &bows);
                        if s < worst {
                            continue;
                        }
                        let (_, a, b) = self.tokens[t as usize];
                        let mut out = h.out;
                        for u in [a, b] {
                            if u != 0 {
                                out = mix(out ^ (u as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
                            }
                        }
                        next.push((s, ((h.ctx << BITS) | t as u128) & keep, hi as u32, t, out));
                    }
                    if next.len() >= beam * 3 {
                        prune(&mut next);
                        worst = next.last().map_or(f32::NEG_INFINITY, |c| c.0);
                    }
                }
                if any {
                    break;
                }
            }
            prune(&mut next);
            let Some(best) = next.first().map(|c| c.0) else {
                return Vec::new();
            };
            let mut new: Vec<Hyp> = Vec::with_capacity(next.len());
            for &(score, ctx, parent, t, out) in &next {
                if score < best - gap {
                    break;
                }
                let h = &hyps[parent as usize];
                let (_, a, b) = self.tokens[t as usize];
                let node = if a == 0 && b == 0 {
                    h.node
                } else {
                    arena.push((h.node, a, b));
                    arena.len() as u32 - 1
                };
                new.push(Hyp {
                    ctx,
                    len: (h.len + 1).min(self.order - 1),
                    score,
                    node,
                    out,
                });
            }
            hyps = new;
        }
        let mut done: Vec<(u64, u32, f32)> = Vec::new();
        for h in &hyps {
            if h.node == 0 {
                continue;
            }
            let bows = self.backoffs(ngrm, h.ctx, h.len);
            let s = h.score + self.logp(ngrm, h.ctx, h.len, EOS, &bows);
            match done.iter_mut().find(|d| d.0 == h.out) {
                Some(d) => {
                    // Two alignments of the same output: add the probabilities.
                    let (a, b) = (d.2.max(s), d.2.min(s));
                    d.2 = a + (1.0 + (b - a).exp()).ln();
                }
                None => done.push((h.out, h.node, s)),
            }
        }
        done.sort_by(|a, b| b.2.total_cmp(&a.2));
        done.truncate(nbest);
        done.into_iter()
            .map(|(_, mut node, s)| {
                let mut us: Vec<u8> = Vec::new();
                while node != 0 {
                    let (parent, a, b) = arena[node as usize];
                    if b != 0 {
                        us.push(b);
                    }
                    if a != 0 {
                        us.push(a);
                    }
                    node = parent;
                }
                let text: String = us
                    .iter()
                    .rev()
                    .map(|&u| self.units[u as usize].as_str())
                    .collect();
                (text, s)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_keep_the_nukta_with_its_letter() {
        assert_eq!(units("ज़रा"), vec!["ज़", "र", "ा"]);
        assert_eq!(units("कल").len(), 2);
    }
}
