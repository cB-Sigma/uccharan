//! A compact lexicon section (`LEX2`): the sorted word → pronunciation table
//! of [`crate::data`], front-coded in blocks of a fixed number of entries and
//! entropy-coded with three canonical Huffman codes (shared-prefix length,
//! key bytes, phoneme symbols). Used in place: a lookup decodes the first key
//! of a dozen or so blocks (binary search) and then at most one block.
//! Larger blocks are smaller and slower; the block size is stored in the
//! header ([`DEFAULT_BLOCK`] unless the builder chooses another).
//!
//! A pronunciation can be stored literally or, when the image has a
//! letter-to-sound model, as an edit of that model's prediction for the key
//! (copy *n* symbols, skip *n*, insert symbols; the rest is copied at the
//! end). Most exceptions differ from the prediction in one or two places, so
//! this roughly halves the cost of the values.
//!
//! ```text
//! u32 entries, u32 entries per block, u32 blocks, u32 symbols, u32 codepoint * symbols,
//! u8 code length * 64 (prefix lengths), * 257 (key bytes + end), * (symbols + 24) (phonemes, '|', end, edit codes),
//! { u32 bit offset, u32 fence } * blocks, bit stream (MSB first)
//!
//! A block's fence is the first four bytes of its first key (big-endian,
//! zero-padded), so the binary search over blocks rarely decodes anything.
//! ```

use crate::data::DataError;

/// Entries per block unless the builder asks for another size.
pub const DEFAULT_BLOCK: usize = 8;
const MAX_LEN: u8 = 15;
const TABLE_BITS: u32 = 11;
const PREFIX_SYMS: usize = 64;
const KEY_SYMS: usize = 257;
const KEY_END: u16 = 256;
/// Value alphabet after the phoneme symbols: '|', end, then the edit codes.
const V_BAR: u16 = 0;
const V_END: u16 = 1;
const V_DIFF: u16 = 2;
const V_COPY: u16 = 3; // + (n - 1), n = 1..=16
const V_SKIP: u16 = 19; // + (n - 1), n = 1..=4
const V_DROP: u16 = 23; // drop the rest of the prediction
const V_EXTRA: usize = 24;

/// A canonical Huffman decoder.
#[derive(Clone, Debug, Default)]
struct Decoder {
    /// (symbol, length) for every TABLE_BITS-bit prefix whose code fits.
    table: Vec<(u16, u8)>,
    /// Canonical data for longer codes: per length, first code and first index.
    first_code: [u32; 16],
    first_index: [u32; 16],
    count: [u32; 16],
    sorted: Vec<u16>,
}

impl Decoder {
    fn new(lengths: &[u8]) -> Result<Decoder, DataError> {
        let mut count = [0u32; 16];
        for &l in lengths {
            if l > MAX_LEN {
                return Err(DataError("bad code length"));
            }
            if l > 0 {
                count[l as usize] += 1;
            }
        }
        let mut first_code = [0u32; 16];
        let mut first_index = [0u32; 16];
        let (mut code, mut index) = (0u32, 0u32);
        for l in 1..16 {
            code = (code + count[l - 1]) << 1;
            if l == 1 {
                code = 0;
            }
            first_code[l] = code;
            first_index[l] = index;
            index += count[l];
        }
        let mut sorted: Vec<u16> = Vec::with_capacity(index as usize);
        for l in 1..16u8 {
            for (s, &sl) in lengths.iter().enumerate() {
                if sl == l {
                    sorted.push(s as u16);
                }
            }
        }
        let mut table = vec![(0u16, 0u8); 1 << TABLE_BITS];
        for l in 1..=TABLE_BITS as usize {
            for k in 0..count[l] {
                let c = first_code[l] + k;
                let sym = sorted[(first_index[l] + k) as usize];
                let shift = TABLE_BITS as usize - l;
                let base = (c as usize) << shift;
                for t in table.iter_mut().skip(base).take(1 << shift) {
                    *t = (sym, l as u8);
                }
            }
        }
        Ok(Decoder {
            table,
            first_code,
            first_index,
            count,
            sorted,
        })
    }

    /// Decode one symbol at bit position `pos`; returns (symbol, bits used).
    #[inline]
    fn decode(&self, bits: &[u8], pos: usize) -> Option<(u16, usize)> {
        let peek = peek16(bits, pos);
        let (s, l) = self.table[(peek >> (16 - TABLE_BITS)) as usize];
        if l > 0 {
            return Some((s, l as usize));
        }
        for l in (TABLE_BITS as usize + 1)..16 {
            let c = peek >> (16 - l);
            if self.count[l] > 0
                && c >= self.first_code[l]
                && c - self.first_code[l] < self.count[l]
            {
                return self
                    .sorted
                    .get((self.first_index[l] + c - self.first_code[l]) as usize)
                    .map(|&s| (s, l));
            }
        }
        None
    }
}

/// The first four bytes of a key, big-endian, zero-padded.
fn fence_of(k: &[u8]) -> u32 {
    let mut f = [0u8; 4];
    for (d, s) in f.iter_mut().zip(k) {
        *d = *s;
    }
    u32::from_be_bytes(f)
}

#[inline]
fn peek16(bits: &[u8], pos: usize) -> u32 {
    let byte = pos / 8;
    let b0 = *bits.get(byte).unwrap_or(&0) as u32;
    let b1 = *bits.get(byte + 1).unwrap_or(&0) as u32;
    let b2 = *bits.get(byte + 2).unwrap_or(&0) as u32;
    let word = (b0 << 16) | (b1 << 8) | b2;
    (word >> (8 - (pos % 8))) & 0xffff
}

/// A parsed `LEX2` header (offsets into the image).
#[derive(Clone, Debug)]
pub struct Lex2 {
    /// Number of entries.
    pub entries: usize,
    block: usize,
    blocks: usize,
    symbols: Vec<char>,
    index: usize,
    stream: usize,
    stream_len: usize,
    prefix: Decoder,
    key: Decoder,
    value: Decoder,
}

fn rd(b: &[u8], at: usize) -> Result<u32, DataError> {
    b.get(at..at + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or(DataError("truncated"))
}

impl Lex2 {
    /// Parse the section at `off..off + len` of image `b`.
    pub fn parse(b: &[u8], off: usize, len: usize) -> Result<Lex2, DataError> {
        let end = off + len;
        let entries = rd(b, off)? as usize;
        let block = rd(b, off + 4)? as usize;
        let blocks = rd(b, off + 8)? as usize;
        let nsym = rd(b, off + 12)? as usize;
        if nsym > 4000 || !(1..=1024).contains(&block) || blocks != entries.div_ceil(block) {
            return Err(DataError("bad lexicon header"));
        }
        let mut at = off + 16;
        let mut symbols = Vec::with_capacity(nsym);
        for _ in 0..nsym {
            symbols.push(char::from_u32(rd(b, at)?).ok_or(DataError("bad symbol"))?);
            at += 4;
        }
        let take = |at: &mut usize, n: usize| -> Result<&[u8], DataError> {
            let s = b.get(*at..*at + n).ok_or(DataError("truncated"))?;
            *at += n;
            Ok(s)
        };
        let prefix = Decoder::new(take(&mut at, PREFIX_SYMS)?)?;
        let key = Decoder::new(take(&mut at, KEY_SYMS)?)?;
        let value = Decoder::new(take(&mut at, nsym + V_EXTRA)?)?;
        let index = at;
        let stream = index + blocks * 8;
        if stream > end {
            return Err(DataError("bad lexicon index"));
        }
        let stream_len = end - stream;
        for i in 0..blocks {
            if rd(b, index + i * 8)? as usize > stream_len * 8 {
                return Err(DataError("bad block offset"));
            }
        }
        Ok(Lex2 {
            entries,
            block,
            blocks,
            symbols,
            index,
            stream,
            stream_len,
            prefix,
            key,
            value,
        })
    }

    /// Look `key` up; on success `out` holds the pronunciation(s), variants
    /// separated by `|`.
    pub fn lookup(
        &self,
        b: &[u8],
        key: &str,
        out: &mut String,
        predict: impl FnOnce(&str) -> String,
    ) -> bool {
        if self.blocks == 0 || key.is_empty() || key.len() > 255 {
            return false;
        }
        let bits = &b[self.stream..self.stream + self.stream_len];
        let kb = key.as_bytes();
        let mut buf = [0u8; 255];
        let block_start = |i: usize| rd(b, self.index + i * 8).unwrap_or(0) as usize;
        let fence = |i: usize| rd(b, self.index + i * 8 + 4).unwrap_or(0);
        let want = fence_of(kb);
        // Binary search for the last block whose first key <= key: on the
        // fences, decoding a first key only when the fences tie.
        let (mut lo, mut hi) = (0usize, self.blocks);
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            let f = fence(mid);
            let le = if f != want {
                f < want
            } else {
                let mut pos = block_start(mid);
                let mut n = 0;
                if self.read_key(bits, &mut pos, &mut buf, &mut n).is_none() {
                    return false;
                }
                &buf[..n] <= kb
            };
            if le {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let mut pos = block_start(lo);
        let mut n = 0usize;
        let in_block = self.block.min(self.entries - lo * self.block);
        for _ in 0..in_block {
            if self.read_key(bits, &mut pos, &mut buf, &mut n).is_none() {
                return false;
            }
            match buf[..n].cmp(kb) {
                std::cmp::Ordering::Equal => {
                    out.clear();
                    return self
                        .read_value(bits, &mut pos, Some((out, Some(predict), key)))
                        .is_some();
                }
                std::cmp::Ordering::Greater => return false,
                std::cmp::Ordering::Less => {
                    if self
                        .read_value::<fn(&str) -> String>(bits, &mut pos, None)
                        .is_none()
                    {
                        return false;
                    }
                }
            }
        }
        false
    }

    fn read_key(
        &self,
        bits: &[u8],
        pos: &mut usize,
        buf: &mut [u8; 255],
        n: &mut usize,
    ) -> Option<()> {
        let (p, l) = self.prefix.decode(bits, *pos)?;
        *pos += l;
        let mut len = (p as usize).min(*n);
        loop {
            let (s, l) = self.key.decode(bits, *pos)?;
            *pos += l;
            if s == KEY_END {
                break;
            }
            if len >= 255 {
                return None;
            }
            buf[len] = s as u8;
            len += 1;
        }
        *n = len;
        Some(())
    }

    fn read_value<F: FnOnce(&str) -> String>(
        &self,
        bits: &[u8],
        pos: &mut usize,
        mut out: Option<(&mut String, Option<F>, &str)>,
    ) -> Option<()> {
        let nsym = self.symbols.len() as u16;
        let mut pred: Vec<char> = Vec::new();
        let mut at = 0usize;
        let mut diff = false;
        loop {
            let (s, l) = self.value.decode(bits, *pos)?;
            *pos += l;
            if s < nsym {
                if let Some((o, ..)) = out.as_mut() {
                    o.push(*self.symbols.get(s as usize)?);
                }
                continue;
            }
            match s - nsym {
                V_BAR => {
                    if let Some((o, ..)) = out.as_mut() {
                        o.push('|');
                    }
                }
                V_END => {
                    if let Some((o, ..)) = out.as_mut() {
                        if diff {
                            o.extend(pred.iter().skip(at));
                        }
                    }
                    return Some(());
                }
                V_DIFF => {
                    diff = true;
                    if let Some((_, f, key)) = out.as_mut() {
                        if let Some(f) = f.take() {
                            pred = f(key).chars().collect();
                        }
                    }
                }
                c if (V_COPY..V_SKIP).contains(&c) => {
                    let n = (c - V_COPY + 1) as usize;
                    if let Some((o, ..)) = out.as_mut() {
                        o.extend(pred.iter().skip(at).take(n));
                    }
                    at += n;
                }
                c if (V_SKIP..V_DROP).contains(&c) => at += (c - V_SKIP + 1) as usize,
                V_DROP => at = usize::MAX / 2,
                _ => return None,
            }
        }
    }
}

/// Edit codes turning `pred` into `target` (`None` when a literal is shorter).
fn diff_codes(
    pred: &[char],
    target: &[char],
    nsym: u16,
    sym: impl Fn(char) -> u16,
) -> Option<Vec<u16>> {
    let (n, m) = (pred.len(), target.len());
    if n == 0 {
        return None;
    }
    // Levenshtein table with operations: copy (equal), insert, delete.
    let mut d = vec![vec![0u32; m + 1]; n + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i as u32;
    }
    for (j, x) in d[0].iter_mut().enumerate() {
        *x = j as u32;
    }
    for i in 1..=n {
        for j in 1..=m {
            let same = if pred[i - 1] == target[j - 1] {
                d[i - 1][j - 1]
            } else {
                u32::MAX
            };
            d[i][j] = same.min(d[i - 1][j] + 1).min(d[i][j - 1] + 1);
        }
    }
    // Walk back to a list of ops: 'c' copy, 'd' delete, 'i'(char) insert.
    let mut ops: Vec<(u8, char)> = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && pred[i - 1] == target[j - 1] && d[i][j] == d[i - 1][j - 1] {
            ops.push((b'c', ' '));
            i -= 1;
            j -= 1;
        } else if j > 0 && d[i][j] == d[i][j - 1] + 1 {
            ops.push((b'i', target[j - 1]));
            j -= 1;
        } else {
            ops.push((b'd', ' '));
            i -= 1;
        }
    }
    ops.reverse();
    // Trailing copies are implicit; trailing deletes become one drop code.
    let ends_with_copy = ops.last().is_some_and(|o| o.0 == b'c');
    while ops.last().is_some_and(|o| o.0 == b'c') {
        ops.pop();
    }
    let mut codes = vec![nsym + V_DIFF];
    let mut k = 0;
    let trailing_deletes = if ends_with_copy {
        0
    } else {
        ops.iter().rev().take_while(|o| o.0 == b'd').count()
    };
    let body = ops.len() - trailing_deletes;
    while k < body {
        let op = ops[k].0;
        let mut run = 1;
        while k + run < body && ops[k + run].0 == op && op != b'i' {
            run += 1;
        }
        match op {
            b'c' => {
                let mut r = run;
                while r > 0 {
                    let t = r.min(16);
                    codes.push(nsym + V_COPY + t as u16 - 1);
                    r -= t;
                }
            }
            b'd' => {
                let mut r = run;
                while r > 0 {
                    let t = r.min(4);
                    codes.push(nsym + V_SKIP + t as u16 - 1);
                    r -= t;
                }
            }
            _ => codes.push(sym(ops[k].1)),
        }
        k += run;
    }
    if trailing_deletes > 0 {
        codes.push(nsym + V_DROP);
    }
    // Worth it only when shorter than the literal.
    (codes.len() < target.len()).then_some(codes)
}

// ---------------------------------------------------------------------------
// Encoder.

/// Huffman code lengths (at most 15 bits) for the given frequencies.
fn code_lengths(freq: &[u64]) -> Vec<u8> {
    let mut f: Vec<u64> = freq.to_vec();
    loop {
        let mut nodes: Vec<(u64, Vec<usize>)> = f
            .iter()
            .enumerate()
            .filter(|(_, &c)| c > 0)
            .map(|(i, &c)| (c, vec![i]))
            .collect();
        let mut lengths = vec![0u8; f.len()];
        if nodes.len() == 1 {
            lengths[nodes[0].1[0]] = 1;
            return lengths;
        }
        while nodes.len() > 1 {
            nodes.sort_by_key(|a| std::cmp::Reverse(a.0));
            let (ca, a) = nodes.pop().unwrap_or_default();
            let (cb, bb) = nodes.pop().unwrap_or_default();
            for &s in a.iter().chain(&bb) {
                lengths[s] += 1;
            }
            let mut m = a;
            m.extend(bb);
            nodes.push((ca + cb, m));
        }
        if lengths.iter().all(|&l| l <= MAX_LEN) {
            return lengths;
        }
        // Flatten the distribution and try again.
        for c in f.iter_mut().filter(|c| **c > 0) {
            *c = (*c >> 1) + 1;
        }
    }
}

fn canonical_codes(lengths: &[u8]) -> Vec<(u32, u8)> {
    let mut count = [0u32; 16];
    for &l in lengths {
        if l > 0 {
            count[l as usize] += 1;
        }
    }
    let mut next = [0u32; 16];
    let mut code = 0u32;
    for l in 1..16 {
        code = (code + count[l - 1]) << 1;
        if l == 1 {
            code = 0;
        }
        next[l] = code;
    }
    let mut codes = vec![(0u32, 0u8); lengths.len()];
    for l in 1..16u8 {
        for (s, &sl) in lengths.iter().enumerate() {
            if sl == l {
                codes[s] = (next[l as usize], l);
                next[l as usize] += 1;
            }
        }
    }
    codes
}

struct BitWriter {
    bytes: Vec<u8>,
    bits: usize,
}

impl BitWriter {
    fn put(&mut self, code: u32, len: u8) {
        for i in (0..len).rev() {
            if self.bits % 8 == 0 {
                self.bytes.push(0);
            }
            if (code >> i) & 1 == 1 {
                let last = self.bytes.len() - 1;
                self.bytes[last] |= 0x80 >> (self.bits % 8);
            }
            self.bits += 1;
        }
    }
}

/// Encode sorted, de-duplicated `(key, pronunciation)` entries.
pub fn encode(
    entries: &[(String, String)],
    block: usize,
    predict: impl Fn(&str) -> Option<String>,
) -> Result<Vec<u8>, DataError> {
    if !(1..=1024).contains(&block) {
        return Err(DataError("bad block size"));
    }
    let mut symbols: Vec<char> = Vec::new();
    for (_, v) in entries {
        for c in v.chars() {
            if c != '|' && !symbols.contains(&c) {
                symbols.push(c);
            }
        }
    }
    symbols.sort_unstable();
    let nsym = symbols.len();
    // Split every entry into (prefix length, key suffix, value ids).
    let mut items: Vec<(usize, &[u8], Vec<u16>)> = Vec::with_capacity(entries.len());
    let mut prev: &[u8] = &[];
    for (i, (k, v)) in entries.iter().enumerate() {
        let kb = k.as_bytes();
        if kb.is_empty() || kb.len() > 255 {
            return Err(DataError("key length out of range"));
        }
        let p = if i % block == 0 {
            0
        } else {
            prev.iter()
                .zip(kb)
                .take_while(|(a, b)| a == b)
                .count()
                .min(PREFIX_SYMS - 1)
        };
        let sym = |c: char| symbols.binary_search(&c).map_or(0, |x| x as u16);
        let literal: Vec<u16> = v
            .chars()
            .map(|c| {
                if c == '|' {
                    nsym as u16 + V_BAR
                } else {
                    sym(c)
                }
            })
            .collect();
        let target: Vec<char> = v.chars().collect();
        let ids = if v.contains('|') {
            literal
        } else {
            match predict(k).map(|p| p.chars().collect::<Vec<char>>()) {
                Some(p) if p.iter().all(|c| *c == '|' || true) => {
                    diff_codes(&p, &target, nsym as u16, sym).unwrap_or(literal)
                }
                _ => literal,
            }
        };
        items.push((p, &kb[p..], ids));
        prev = kb;
    }
    let mut fp = vec![0u64; PREFIX_SYMS];
    let mut fk = vec![0u64; KEY_SYMS];
    let mut fv = vec![0u64; nsym + V_EXTRA];
    for (p, suffix, ids) in &items {
        fp[*p] += 1;
        for &c in *suffix {
            fk[c as usize] += 1;
        }
        fk[KEY_END as usize] += 1;
        for &s in ids {
            fv[s as usize] += 1;
        }
        fv[nsym + V_END as usize] += 1;
    }
    let (lp, lk, lv) = (code_lengths(&fp), code_lengths(&fk), code_lengths(&fv));
    let (cp, ck, cv) = (
        canonical_codes(&lp),
        canonical_codes(&lk),
        canonical_codes(&lv),
    );
    let mut w = BitWriter {
        bytes: Vec::new(),
        bits: 0,
    };
    let mut index: Vec<(u32, u32)> = Vec::new();
    for (i, (p, suffix, ids)) in items.iter().enumerate() {
        if i % block == 0 {
            index.push((w.bits as u32, fence_of(suffix)));
        }
        w.put(cp[*p].0, cp[*p].1);
        for &c in *suffix {
            w.put(ck[c as usize].0, ck[c as usize].1);
        }
        w.put(ck[KEY_END as usize].0, ck[KEY_END as usize].1);
        for &s in ids {
            w.put(cv[s as usize].0, cv[s as usize].1);
        }
        w.put(cv[nsym + V_END as usize].0, cv[nsym + V_END as usize].1);
    }
    let mut out: Vec<u8> = Vec::new();
    out.extend((entries.len() as u32).to_le_bytes());
    out.extend((block as u32).to_le_bytes());
    out.extend((index.len() as u32).to_le_bytes());
    out.extend((nsym as u32).to_le_bytes());
    for c in &symbols {
        out.extend((*c as u32).to_le_bytes());
    }
    out.extend(&lp);
    out.extend(&lk);
    out.extend(&lv);
    for (o, f) in &index {
        out.extend(o.to_le_bytes());
        out.extend(f.to_le_bytes());
    }
    out.extend(w.bytes);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_codes_round_trip() {
        let syms: Vec<char> = "abcdefghijklmnopqrstuvwxyzˈˌ".chars().collect();
        let nsym = syms.len() as u16;
        let sym = |c: char| syms.iter().position(|&x| x == c).unwrap() as u16;
        for (p, t) in [
            ("ˈaɪfoʊnz", "ˈaɪfoʊn"),
            ("abcdefg", "abxdefg"),
            ("abcdefgh", "abdefgh"),
            ("abcd", "abcdxyz"),
            ("hello", "ˈhello"),
            ("abcdef", "ab"),
        ] {
            let p: Vec<char> = p
                .chars()
                .map(|c| if syms.contains(&c) { c } else { 'z' })
                .collect();
            let t: Vec<char> = t
                .chars()
                .map(|c| if syms.contains(&c) { c } else { 'z' })
                .collect();
            let Some(codes) = diff_codes(&p, &t, nsym, sym) else {
                continue;
            };
            // decode
            let (mut out, mut at) = (String::new(), 0usize);
            for &c in &codes[1..] {
                if c < nsym {
                    out.push(syms[c as usize]);
                } else if (nsym + V_COPY..nsym + V_SKIP).contains(&c) {
                    let n = (c - nsym - V_COPY + 1) as usize;
                    out.extend(p.iter().skip(at).take(n));
                    at += n;
                } else if (nsym + V_SKIP..nsym + V_DROP).contains(&c) {
                    at += (c - nsym - V_SKIP + 1) as usize;
                } else if c == nsym + V_DROP {
                    at = usize::MAX / 2;
                }
            }
            out.extend(p.iter().skip(at));
            assert_eq!(out, t.iter().collect::<String>());
        }
    }
}
