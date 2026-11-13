//! The compiled language-data container (`.ucd`) and its two section types.
//!
//! A `.ucd` file is a flat, little-endian, position-independent byte image that
//! is used in place: nothing is parsed into heap structures at load time, so the
//! same bytes can come from `include_bytes!`, a file read, a memory map or a
//! `fetch()` in the browser.
//!
//! ```text
//! "UCD1"  u32 section_count  { [u8;4] tag, u32 offset, u32 len } * section_count  payload...
//! ```
//!
//! Sections:
//!
//! * `LEXI` — a front-coded, block-indexed sorted string table: word → one or
//!   more pronunciations. Lookup is a binary search over blocks followed by a
//!   scan of at most [`BLOCK`] entries.
//! * `LTSM` — the letter-to-sound model: per-letter context tables with
//!   back-off, trained by `tools/lexgen` from the same permissive lexicon.
//! * `META` — UTF-8 `key=value` lines (language, build provenance).

use std::borrow::Cow;
use std::fmt;

/// Entries per front-coded block.
pub const BLOCK: usize = 16;

/// Error returned for a malformed data image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataError(pub &'static str);

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid uccharan data: {}", self.0)
    }
}

impl std::error::Error for DataError {}

fn rd_u32(b: &[u8], at: usize) -> Result<u32, DataError> {
    b.get(at..at + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or(DataError("truncated"))
}

fn rd_u16(b: &[u8], at: usize) -> Result<u16, DataError> {
    b.get(at..at + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .ok_or(DataError("truncated"))
}

/// A loaded data image. Cheap to clone when it borrows static bytes.
#[derive(Clone)]
pub struct Data {
    bytes: Cow<'static, [u8]>,
    lexi: Option<LexiHeader>,
    ltsm: Option<LtsHeader>,
    meta: (usize, usize),
}

#[derive(Clone, Copy)]
struct LexiHeader {
    entries: usize,
    blocks: usize,
    symbols: usize,
    symtab: usize,
    index: usize,
    blob: usize,
    blob_len: usize,
}

#[derive(Clone)]
struct LtsHeader {
    /// (left, right, count, keys offset, values offset)
    tables: Vec<(u8, u8, usize, usize, usize)>,
    outputs: Vec<(usize, usize)>,
}

impl fmt::Debug for Data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Data")
            .field("bytes", &self.bytes.len())
            .field("entries", &self.len())
            .finish()
    }
}

impl Data {
    /// Use a static byte image (for example from `include_bytes!`).
    pub fn from_static(bytes: &'static [u8]) -> Result<Self, DataError> {
        Self::new(Cow::Borrowed(bytes))
    }

    /// Take ownership of a byte image read from a file or the network.
    pub fn from_vec(bytes: Vec<u8>) -> Result<Self, DataError> {
        Self::new(Cow::Owned(bytes))
    }

    fn new(bytes: Cow<'static, [u8]>) -> Result<Self, DataError> {
        let b: &[u8] = &bytes;
        if b.len() < 8 || &b[..4] != b"UCD1" {
            return Err(DataError("bad magic"));
        }
        let n = rd_u32(b, 4)? as usize;
        if n > 64 {
            return Err(DataError("too many sections"));
        }
        let mut lexi = None;
        let mut ltsm = None;
        let mut meta = (0, 0);
        for i in 0..n {
            let at = 8 + i * 12;
            let tag = b.get(at..at + 4).ok_or(DataError("truncated"))?;
            let off = rd_u32(b, at + 4)? as usize;
            let len = rd_u32(b, at + 8)? as usize;
            let end = off.checked_add(len).ok_or(DataError("overflow"))?;
            if end > b.len() {
                return Err(DataError("section out of range"));
            }
            match tag {
                b"LEXI" => lexi = Some(Self::parse_lexi(b, off, len)?),
                b"LTSM" => ltsm = Some(Self::parse_lts(b, off, len)?),
                b"META" => meta = (off, len),
                _ => {}
            }
        }
        Ok(Data {
            bytes,
            lexi,
            ltsm,
            meta,
        })
    }

    fn parse_lexi(b: &[u8], off: usize, len: usize) -> Result<LexiHeader, DataError> {
        let entries = rd_u32(b, off)? as usize;
        let blocks = rd_u32(b, off + 4)? as usize;
        let symbols = rd_u32(b, off + 8)? as usize;
        let symtab = off + 12;
        let index = symtab + symbols * 4;
        let blob = index + blocks * 4;
        if blob > off + len || symbols > 254 || blocks != entries.div_ceil(BLOCK) {
            return Err(DataError("bad lexicon header"));
        }
        let blob_len = off + len - blob;
        for i in 0..blocks {
            if rd_u32(b, index + i * 4)? as usize >= blob_len.max(1) {
                return Err(DataError("bad block offset"));
            }
        }
        Ok(LexiHeader {
            entries,
            blocks,
            symbols,
            symtab,
            index,
            blob,
            blob_len,
        })
    }

    fn parse_lts(b: &[u8], off: usize, len: usize) -> Result<LtsHeader, DataError> {
        let end = off + len;
        let n_out = rd_u32(b, off)? as usize;
        let n_tab = rd_u32(b, off + 4)? as usize;
        if n_out > 65_535 || n_tab > 32 {
            return Err(DataError("bad lts header"));
        }
        let mut at = off + 8;
        let mut outputs = Vec::with_capacity(n_out);
        for _ in 0..n_out {
            let l = *b.get(at).ok_or(DataError("truncated"))? as usize;
            if at + 1 + l > end || std::str::from_utf8(&b[at + 1..at + 1 + l]).is_err() {
                return Err(DataError("bad lts output"));
            }
            outputs.push((at + 1, l));
            at += 1 + l;
        }
        let mut tables = Vec::with_capacity(n_tab);
        for _ in 0..n_tab {
            let left = *b.get(at).ok_or(DataError("truncated"))?;
            let right = *b.get(at + 1).ok_or(DataError("truncated"))?;
            let count = rd_u32(b, at + 2)? as usize;
            let keys = at + 6;
            let vals = keys + count * 4;
            let next = vals + count * 2;
            if next > end {
                return Err(DataError("lts table out of range"));
            }
            tables.push((left, right, count, keys, vals));
            at = next;
        }
        Ok(LtsHeader { tables, outputs })
    }

    /// Number of lexicon entries.
    pub fn len(&self) -> usize {
        self.lexi.map_or(0, |h| h.entries)
    }

    /// True when the image has no lexicon entries.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Size of the image in bytes.
    pub fn byte_len(&self) -> usize {
        self.bytes.len()
    }

    /// True when the image carries a letter-to-sound model.
    pub fn has_lts(&self) -> bool {
        self.ltsm.is_some()
    }

    /// A `key=value` entry from the `META` section.
    pub fn meta(&self, key: &str) -> Option<&str> {
        let s = std::str::from_utf8(&self.bytes[self.meta.0..self.meta.0 + self.meta.1]).ok()?;
        s.lines()
            .find_map(|l| l.split_once('=').filter(|(k, _)| *k == key).map(|(_, v)| v))
    }

    fn symbol(&self, h: &LexiHeader, id: u8) -> char {
        if id == 0 || id as usize > h.symbols {
            return '\u{fffd}';
        }
        let cp = rd_u32(&self.bytes, h.symtab + (id as usize - 1) * 4).unwrap_or(0xfffd);
        char::from_u32(cp).unwrap_or('\u{fffd}')
    }

    /// Look a word up. `key` must already be in lexicon form (lower case).
    /// Calls `f` with the decoded pronunciation variants, in stored order.
    pub fn lookup(&self, key: &str, out: &mut String) -> bool {
        let Some(h) = self.lexi.as_ref() else {
            return false;
        };
        if h.blocks == 0 || key.is_empty() || key.len() > 255 {
            return false;
        }
        let b: &[u8] = &self.bytes;
        let blob = &b[h.blob..h.blob + h.blob_len];
        let kb = key.as_bytes();
        // Binary search for the last block whose first key <= key.
        let first_key = |blk: usize| -> &[u8] {
            let off = rd_u32(b, h.index + blk * 4).unwrap_or(0) as usize;
            let len = *blob.get(off + 1).unwrap_or(&0) as usize;
            blob.get(off + 2..off + 2 + len).unwrap_or(&[])
        };
        let (mut lo, mut hi) = (0usize, h.blocks);
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if first_key(mid) <= kb {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let mut at = rd_u32(b, h.index + lo * 4).unwrap_or(0) as usize;
        let in_block = BLOCK.min(h.entries - lo * BLOCK);
        let mut cur = [0u8; 255];
        let mut cur_len = 0usize;
        for _ in 0..in_block {
            let (Some(&p), Some(&s)) = (blob.get(at), blob.get(at + 1)) else {
                return false;
            };
            let (p, s) = (p as usize, s as usize);
            let Some(suffix) = blob.get(at + 2..at + 2 + s) else {
                return false;
            };
            if p > cur_len || p + s > 255 {
                return false;
            }
            cur[p..p + s].copy_from_slice(suffix);
            cur_len = p + s;
            let Some(&vl) = blob.get(at + 2 + s) else {
                return false;
            };
            let vstart = at + 3 + s;
            let Some(val) = blob.get(vstart..vstart + vl as usize) else {
                return false;
            };
            match cur[..cur_len].cmp(kb) {
                std::cmp::Ordering::Equal => {
                    out.clear();
                    for &id in val {
                        out.push(if id == 255 { '|' } else { self.symbol(h, id) });
                    }
                    return true;
                }
                std::cmp::Ordering::Greater => return false,
                std::cmp::Ordering::Less => {}
            }
            at = vstart + vl as usize;
        }
        false
    }

    /// Predict a pronunciation for an out-of-lexicon letter string with the
    /// letter-to-sound model. Returns `false` when the image has no model.
    pub fn lts(&self, letters: &[u8], out: &mut String) -> bool {
        let Some(h) = self.ltsm.as_ref() else {
            return false;
        };
        let b: &[u8] = &self.bytes;
        out.clear();
        for i in 0..letters.len() {
            for &(left, right, count, keys, vals) in &h.tables {
                let k = context_hash(letters, i, left as usize, right as usize);
                let (mut lo, mut hi) = (0usize, count);
                while lo < hi {
                    let mid = (lo + hi) / 2;
                    if rd_u32(b, keys + mid * 4).unwrap_or(u32::MAX) < k {
                        lo = mid + 1;
                    } else {
                        hi = mid;
                    }
                }
                if lo < count && rd_u32(b, keys + lo * 4) == Ok(k) {
                    let v = rd_u16(b, vals + lo * 2).unwrap_or(0) as usize;
                    if let Some(&(o, l)) = h.outputs.get(v) {
                        out.push_str(std::str::from_utf8(&b[o..o + l]).unwrap_or(""));
                    }
                    break;
                }
            }
        }
        true
    }
}

/// Hash of the letter at `i` with `left`/`right` letters of context.
/// `#` pads beyond the word edges. FNV-1a, 32 bit, salted with the shape.
pub fn context_hash(letters: &[u8], i: usize, left: usize, right: usize) -> u32 {
    let mut h: u32 = 0x811c_9dc5 ^ ((left as u32) << 8 | right as u32);
    let mut step = |c: u8| {
        h ^= c as u32;
        h = h.wrapping_mul(0x0100_0193);
    };
    for k in (1..=left).rev() {
        step(if i >= k { letters[i - k] } else { b'#' });
    }
    step(b'|');
    step(letters[i]);
    step(b'|');
    for k in 1..=right {
        step(if i + k < letters.len() {
            letters[i + k]
        } else {
            b'#'
        });
    }
    h
}

/// One letter-to-sound table: `(letters to the left, letters to the right, sorted (context hash, output id))`.
pub type LtsTable = (u8, u8, Vec<(u32, u16)>);

/// Builds a `.ucd` image. Used by `tools/lexgen` and by tests.
#[derive(Default)]
pub struct Builder {
    entries: Vec<(String, String)>,
    lts_tables: Vec<LtsTable>,
    lts_outputs: Vec<String>,
    meta: String,
}

impl Builder {
    /// A new, empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a word. Several variants are joined with `|`.
    pub fn entry(&mut self, key: &str, pron: &str) -> &mut Self {
        self.entries.push((key.to_string(), pron.to_string()));
        self
    }

    /// Add a metadata line.
    pub fn meta(&mut self, key: &str, value: &str) -> &mut Self {
        self.meta.push_str(key);
        self.meta.push('=');
        self.meta.push_str(value);
        self.meta.push('\n');
        self
    }

    /// Set the letter-to-sound model: output strings and, most specific first,
    /// `(left, right, sorted (hash, output index))` tables.
    pub fn lts(&mut self, outputs: Vec<String>, tables: Vec<LtsTable>) -> &mut Self {
        self.lts_outputs = outputs;
        self.lts_tables = tables;
        self
    }

    /// Serialise. Entries are sorted and de-duplicated (first wins).
    pub fn build(&mut self) -> Result<Vec<u8>, DataError> {
        self.entries
            .sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
        self.entries.dedup_by(|b, a| a.0 == b.0);
        let mut symbols: Vec<char> = Vec::new();
        for (_, v) in &self.entries {
            for c in v.chars() {
                if c != '|' && !symbols.contains(&c) {
                    symbols.push(c);
                }
            }
        }
        symbols.sort_unstable();
        if symbols.len() > 254 {
            return Err(DataError("too many distinct phoneme symbols"));
        }
        let mut lexi: Vec<u8> = Vec::new();
        let n = self.entries.len();
        let blocks = n.div_ceil(BLOCK);
        lexi.extend((n as u32).to_le_bytes());
        lexi.extend((blocks as u32).to_le_bytes());
        lexi.extend((symbols.len() as u32).to_le_bytes());
        for c in &symbols {
            lexi.extend((*c as u32).to_le_bytes());
        }
        let mut index: Vec<u32> = Vec::with_capacity(blocks);
        let mut blob: Vec<u8> = Vec::new();
        let mut prev: &[u8] = &[];
        for (i, (k, v)) in self.entries.iter().enumerate() {
            let kb = k.as_bytes();
            if kb.is_empty() || kb.len() > 255 {
                return Err(DataError("key length out of range"));
            }
            let p = if i % BLOCK == 0 {
                index.push(blob.len() as u32);
                0
            } else {
                prev.iter().zip(kb).take_while(|(a, b)| a == b).count()
            };
            blob.push(p as u8);
            blob.push((kb.len() - p) as u8);
            blob.extend(&kb[p..]);
            let ids: Vec<u8> = v
                .chars()
                .map(|c| {
                    if c == '|' {
                        255
                    } else {
                        symbols.binary_search(&c).map_or(0, |x| x as u8 + 1)
                    }
                })
                .collect();
            if ids.len() > 255 {
                return Err(DataError("pronunciation too long"));
            }
            blob.push(ids.len() as u8);
            blob.extend(ids);
            prev = kb;
        }
        for o in &index {
            lexi.extend(o.to_le_bytes());
        }
        lexi.extend(blob);

        let mut ltsm: Vec<u8> = Vec::new();
        if !self.lts_tables.is_empty() {
            ltsm.extend((self.lts_outputs.len() as u32).to_le_bytes());
            ltsm.extend((self.lts_tables.len() as u32).to_le_bytes());
            for o in &self.lts_outputs {
                if o.len() > 255 {
                    return Err(DataError("lts output too long"));
                }
                ltsm.push(o.len() as u8);
                ltsm.extend(o.as_bytes());
            }
            for (l, r, t) in &mut self.lts_tables {
                t.sort_unstable();
                t.dedup_by_key(|e| e.0);
                ltsm.push(*l);
                ltsm.push(*r);
                ltsm.extend((t.len() as u32).to_le_bytes());
                for (k, _) in t.iter() {
                    ltsm.extend(k.to_le_bytes());
                }
                for (_, v) in t.iter() {
                    ltsm.extend(v.to_le_bytes());
                }
            }
        }

        let mut sections: Vec<(&[u8; 4], &[u8])> =
            vec![(b"LEXI", &lexi), (b"META", self.meta.as_bytes())];
        if !ltsm.is_empty() {
            sections.push((b"LTSM", &ltsm));
        }
        let mut out: Vec<u8> = Vec::new();
        out.extend(b"UCD1");
        out.extend((sections.len() as u32).to_le_bytes());
        let mut off = 8 + sections.len() * 12;
        for (tag, body) in &sections {
            out.extend(*tag);
            out.extend((off as u32).to_le_bytes());
            out.extend((body.len() as u32).to_le_bytes());
            off += body.len();
        }
        for (_, body) in &sections {
            out.extend(*body);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let mut b = Builder::new();
        let words: Vec<String> = (0..100).map(|i| format!("w{i:03}")).collect();
        for w in &words {
            b.entry(w, &format!("ˈ{w}|x"));
        }
        b.entry("zebra", "zˈiːbɹə").meta("lang", "test");
        let d = Data::from_vec(b.build().unwrap()).unwrap();
        assert_eq!(d.len(), 101);
        assert_eq!(d.meta("lang"), Some("test"));
        let mut s = String::new();
        for w in &words {
            assert!(d.lookup(w, &mut s), "{w}");
            assert_eq!(s, format!("ˈ{w}|x"));
        }
        assert!(d.lookup("zebra", &mut s));
        assert_eq!(s, "zˈiːbɹə");
        assert!(!d.lookup("w1000", &mut s));
        assert!(!d.lookup("a", &mut s));
        assert!(!d.lookup("zzz", &mut s));
    }

    #[test]
    fn rejects_garbage() {
        assert!(Data::from_vec(vec![1, 2, 3]).is_err());
        assert!(Data::from_vec(b"UCD1\xff\xff\xff\xff".to_vec()).is_err());
        let mut img = Builder::new().entry("a", "b").build().unwrap();
        img.truncate(img.len() - 2);
        assert!(Data::from_vec(img).is_err());
    }
}
