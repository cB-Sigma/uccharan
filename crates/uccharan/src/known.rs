//! The `KNWN` section: which words the lexicon knows although it stores no
//! entry for them, because the letter-to-sound model reproduces their
//! pronunciation exactly. Morphology and the name path ask whether a word is
//! a known word; this set answers for the left-out ones.
//!
//! It is a Golomb-coded set: each word is hashed into `0..n << k`, the sorted
//! hashes are delta-coded (Rice code, parameter `k`) in blocks of [`BLOCK`].
//! A word that is not in the set is reported as known with probability about
//! `2^-k`.
//!
//! ```text
//! u32 n, u32 k, u32 blocks, { u32 first hash, u32 bit offset } * blocks, bits (MSB first)
//! ```

use crate::data::DataError;

/// Hashes per block.
const BLOCK: usize = 64;

/// Rice parameter used by [`encode`]: false positives about 1 in 1 024.
pub const RICE_K: u32 = 10;

fn hash(word: &str, range: u64) -> u32 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in word.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    (h % range.max(1)) as u32
}

/// Serialise the set of `words`.
pub fn encode(words: &[&str], k: u32) -> Result<Vec<u8>, DataError> {
    let n = words.len() as u64;
    let range = n << k;
    if range >= 1 << 32 {
        return Err(DataError("known-word set too large"));
    }
    let mut hs: Vec<u32> = words.iter().map(|w| hash(w, range)).collect();
    hs.sort_unstable();
    hs.dedup();
    let blocks = hs.len().div_ceil(BLOCK);
    let mut index: Vec<(u32, u32)> = Vec::with_capacity(blocks);
    let mut bits: Vec<u8> = Vec::new();
    let mut len = 0usize;
    let put = |bit: bool, bits: &mut Vec<u8>, len: &mut usize| {
        if *len % 8 == 0 {
            bits.push(0);
        }
        if bit {
            *bits.last_mut().unwrap() |= 0x80 >> (*len % 8);
        }
        *len += 1;
    };
    for (i, &h) in hs.iter().enumerate() {
        if i % BLOCK == 0 {
            index.push((h, len as u32));
            continue;
        }
        let d = h - hs[i - 1];
        for _ in 0..(d >> k) {
            put(true, &mut bits, &mut len);
        }
        put(false, &mut bits, &mut len);
        for j in (0..k).rev() {
            put(d >> j & 1 == 1, &mut bits, &mut len);
        }
    }
    let mut out = Vec::with_capacity(12 + blocks * 8 + bits.len());
    out.extend((n as u32).to_le_bytes());
    out.extend(k.to_le_bytes());
    out.extend((blocks as u32).to_le_bytes());
    for (h, at) in index {
        out.extend(h.to_le_bytes());
        out.extend(at.to_le_bytes());
    }
    out.extend(bits);
    Ok(out)
}

fn rd(b: &[u8], at: usize) -> Option<u32> {
    b.get(at..at + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// Check a `KNWN` section (`b` is the section's bytes).
pub fn validate(b: &[u8]) -> Result<(), DataError> {
    let (Some(n), Some(k), Some(blocks)) = (rd(b, 0), rd(b, 4), rd(b, 8)) else {
        return Err(DataError("truncated known-word set"));
    };
    if k > 24 || (n as u64) << k >= 1 << 32 || 12 + blocks as usize * 8 > b.len() {
        return Err(DataError("bad known-word set"));
    }
    Ok(())
}

/// True when `word` is (probably) in the set stored in section bytes `b`.
pub fn contains(b: &[u8], word: &str) -> bool {
    let (Some(n), Some(k), Some(blocks)) = (rd(b, 0), rd(b, 4), rd(b, 8)) else {
        return false;
    };
    let blocks = blocks as usize;
    if blocks == 0 {
        return false;
    }
    let v = hash(word, (n as u64) << k);
    let first = |i: usize| rd(b, 12 + i * 8).unwrap_or(u32::MAX);
    // Last block whose first hash <= v.
    let (mut lo, mut hi) = (0usize, blocks);
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if first(mid) <= v {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let mut h = first(lo);
    if h >= v {
        return h == v;
    }
    let bits = &b[(12 + blocks * 8).min(b.len())..];
    let total = bits.len() * 8;
    let mut pos = rd(b, 12 + lo * 8 + 4).unwrap_or(u32::MAX) as usize;
    let end = if lo + 1 < blocks {
        rd(b, 12 + (lo + 1) * 8 + 4).unwrap_or(0) as usize
    } else {
        total
    }
    .min(total);
    // 32 bits starting at bit `p` (zeros past the end).
    let peek = |p: usize| -> u64 {
        let i = p / 8;
        let mut w = [0u8; 8];
        for (j, x) in w.iter_mut().enumerate() {
            *x = *bits.get(i + j).unwrap_or(&0);
        }
        (u64::from_be_bytes(w) << (p % 8)) >> 32
    };
    while pos < end {
        // Unary quotient: count the leading ones (several words for a long run).
        let mut q = 0u32;
        loop {
            let ones = ((peek(pos) as u32).leading_ones()).min((end - pos) as u32);
            q += ones;
            pos += ones as usize;
            if ones < 32 || pos >= end {
                break;
            }
        }
        pos += 1;
        if pos + k as usize > total {
            return false;
        }
        let r = if k == 0 {
            0
        } else {
            (peek(pos) >> (32 - k)) as u32
        };
        pos += k as usize;
        h = h.saturating_add(q << k | r);
        if h >= v {
            return h == v;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn members_found_and_others_mostly_not() {
        let words: Vec<String> = (0..5000).map(|i| format!("word{i}")).collect();
        let refs: Vec<&str> = words.iter().map(String::as_str).collect();
        let b = encode(&refs, RICE_K).unwrap();
        validate(&b).unwrap();
        assert!(refs.iter().all(|w| contains(&b, w)));
        let false_hits = (0..20000)
            .filter(|i| contains(&b, &format!("other{i}")))
            .count();
        assert!(false_hits < 60, "{false_hits}");
        assert!(!contains(&encode(&[], RICE_K).unwrap(), "x"));
    }
}
