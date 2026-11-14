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
//! * `LEX2` — the lexicon: word → one or more pronunciations, entropy-coded
//!   (see [`crate::lex2`]).
//! * `LTS2` — the letter-to-sound model: per-letter context tables with
//!   back-off, trained by `tools/lexgen` from the same permissive lexicon.
//!   Each table is a hash table: the top bits of a context hash pick a
//!   bucket and a 16-bit fingerprint (the low bits) picks the entry in it.
//! * `KNWN` — the words left out of the lexicon because the letter-to-sound
//!   model reproduces them (see [`crate::known`]).
//! * `META` — UTF-8 `key=value` lines (language, build provenance).

use std::borrow::Cow;
use std::fmt;

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
    id: usize,
    lex2: Option<crate::lex2::Lex2>,
    namc: (usize, usize),
    knwn: (usize, usize),
    ltsm: Option<LtsHeader>,
    meta: (usize, usize),
}

#[derive(Clone, Copy)]
struct LtsShape {
    left: u8,
    right: u8,
    /// log2 of the bucket count
    bits: u8,
    /// bytes per output id (1 or 2)
    width: u8,
    count: usize,
    /// u16 start of each bucket, plus one end marker
    starts: usize,
    /// u16 fingerprint per entry
    prints: usize,
    /// output id per entry
    vals: usize,
}

#[derive(Clone)]
struct LtsHeader {
    tables: Vec<LtsShape>,
    /// Some tables give the stress mark of a letter's vowel (flag 0x40).
    stress: bool,
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
        let mut lex2 = None;
        let mut namc = (0, 0);
        let mut knwn = (0, 0);
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
                b"LEX2" => lex2 = Some(crate::lex2::Lex2::parse(b, off, len)?),
                b"LTS2" => ltsm = Some(Self::parse_lts(b, off, len)?),
                b"META" => meta = (off, len),
                b"NAMC" => namc = (off, len),
                b"KNWN" => {
                    crate::known::validate(&b[off..end])?;
                    knwn = (off, len);
                }
                _ => {}
            }
        }
        static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
        Ok(Data {
            bytes,
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            lex2,
            namc,
            knwn,
            ltsm,
            meta,
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
            let hdr = b.get(at..at + 8).ok_or(DataError("truncated"))?;
            let (left, right, bits, width) = (hdr[0], hdr[1], hdr[2], hdr[3]);
            let count = rd_u32(b, at + 4)? as usize;
            if bits > 16 || !(1..=2).contains(&width) || count > 65_535 {
                return Err(DataError("bad lts table"));
            }
            let starts = at + 8;
            let prints = starts + ((1usize << bits) + 1) * 2;
            let vals = prints + count * 2;
            let next = vals + count * width as usize;
            if next > end || rd_u16(b, prints - 2)? as usize != count {
                return Err(DataError("lts table out of range"));
            }
            tables.push(LtsShape {
                left,
                right,
                bits,
                width,
                count,
                starts,
                prints,
                vals,
            });
            at = next;
        }
        let stress = tables.iter().any(|t| t.left & 0x40 != 0);
        Ok(LtsHeader {
            tables,
            stress,
            outputs,
        })
    }

    /// Number of lexicon entries.
    pub fn len(&self) -> usize {
        self.lex2.as_ref().map_or(0, |h| h.entries)
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

    /// The name-origin model section (`NAMC`), empty when absent.
    pub fn name_model(&self) -> &[u8] {
        &self.bytes[self.namc.0..self.namc.0 + self.namc.1]
    }

    /// An identity for caches: unique per loaded image (clones share it).
    pub(crate) fn id(&self) -> usize {
        self.id
    }

    /// True when `key` is a word the lexicon leaves out because the
    /// letter-to-sound model reproduces it (the `KNWN` section).
    pub fn known(&self, key: &str) -> bool {
        self.knwn.1 > 0
            && crate::known::contains(&self.bytes[self.knwn.0..self.knwn.0 + self.knwn.1], key)
    }

    /// A `key=value` entry from the `META` section.
    pub fn meta(&self, key: &str) -> Option<&str> {
        let s = std::str::from_utf8(&self.bytes[self.meta.0..self.meta.0 + self.meta.1]).ok()?;
        s.lines()
            .find_map(|l| l.split_once('=').filter(|(k, _)| *k == key).map(|(_, v)| v))
    }

    /// Look a word up. `key` must already be in lexicon form (lower case).
    /// Writes the pronunciation variants (joined by `|`) to `out`.
    pub fn lookup(&self, key: &str, out: &mut String) -> bool {
        // Running text repeats its words: remember recent answers.
        let hit = memo(self.id * 2, key, || {
            let mut s = String::new();
            self.lookup_uncached(key, &mut s).then_some(s)
        });
        match hit {
            Some(s) => {
                out.clear();
                out.push_str(&s);
                true
            }
            None => false,
        }
    }

    /// [`Data::lookup`] without the cache of recent answers.
    pub(crate) fn lookup_uncached(&self, key: &str, out: &mut String) -> bool {
        match self.lex2.as_ref() {
            Some(l) => l.lookup(&self.bytes, key, out, |k| self.predict(k)),
            None => false,
        }
    }

    /// The letter-to-sound prediction that compact lexicon entries are stored
    /// relative to (letters of the key only, lower case).
    pub(crate) fn predict(&self, key: &str) -> String {
        let letters: Vec<u8> = key
            .bytes()
            .filter(u8::is_ascii_alphabetic)
            .map(|b| b.to_ascii_lowercase())
            .collect();
        let mut out = String::new();
        if letters.is_empty() || !self.lts(&letters, &mut out) {
            out.clear();
        }
        out
    }

    /// Predict a pronunciation for an out-of-lexicon letter string with the
    /// letter-to-sound model. Returns `false` when the image has no model.
    pub fn lts(&self, letters: &[u8], out: &mut String) -> bool {
        let Some(h) = self.ltsm.as_ref() else {
            return false;
        };
        let b: &[u8] = &self.bytes;
        out.clear();
        let mut state = 0u8;
        let text = |v: usize| {
            h.outputs.get(v).map_or("", |&(o, l)| {
                std::str::from_utf8(&b[o..o + l]).unwrap_or("")
            })
        };
        let pick = |i: usize, stress: bool, state: u32| {
            h.tables
                .iter()
                .filter(|t| (t.left & 0x40 != 0) == stress)
                .find_map(|t| Self::lts_probe(b, t, shape_hash(letters, i, t.left, t.right, state)))
                .map(text)
        };
        for i in 0..letters.len() {
            let Some(seg) = pick(i, false, state as u32) else {
                continue;
            };
            let from = out.len();
            match seg.char_indices().find(|&(_, c)| is_vowel(c)) {
                Some((at, _)) if h.stress => {
                    out.push_str(&seg[..at]);
                    out.push_str(pick(i, true, stress_key(state, seg)).unwrap_or(""));
                    out.push_str(&seg[at..]);
                }
                _ => out.push_str(seg),
            }
            state = stress_state(state, &out[from..]);
        }
        true
    }
}

/// The bytes of a word for a letter-to-sound model of a language whose
/// alphabet goes beyond ASCII: ASCII letters, the apostrophe and the hyphen
/// stand for themselves, the letters of `extra` are numbered from 0x80.
/// `None` when the word has any other character.
pub fn encode_letters(word: &str, extra: &str) -> Option<Vec<u8>> {
    word.chars()
        .map(|c| {
            if c.is_ascii_lowercase() || c == '\'' || c == '-' {
                Some(c as u8)
            } else if c.is_ascii() {
                None
            } else {
                extra.chars().position(|x| x == c).map(|i| 0x80 + i as u8)
            }
        })
        .collect()
}

impl Data {
    /// The model's reading of a lower-case word, for images whose `META`
    /// section names the model's letters (`letters=`).
    pub fn model_word(&self, word: &str) -> Option<String> {
        self.ltsm.as_ref()?;
        let letters = encode_letters(word, self.meta("letters")?)?;
        let mut out = String::new();
        (self.lts(&letters, &mut out) && !out.is_empty()).then_some(out)
    }

    fn lts_probe(b: &[u8], t: &LtsShape, h: u32) -> Option<usize> {
        let bucket = if t.bits == 0 {
            0
        } else {
            (h >> (32 - t.bits as u32)) as usize
        };
        let lo = rd_u16(b, t.starts + bucket * 2).ok()? as usize;
        let hi = (rd_u16(b, t.starts + bucket * 2 + 2).ok()? as usize).min(t.count);
        let fp = h as u16;
        for i in lo..hi {
            let f = rd_u16(b, t.prints + i * 2).ok()?;
            if f == fp {
                return Some(if t.width == 1 {
                    *b.get(t.vals + i)? as usize
                } else {
                    rd_u16(b, t.vals + i * 2).ok()? as usize
                });
            }
            if f > fp {
                return None;
            }
        }
        None
    }
}

const MEMO_SLOTS: usize = 4096;

type MemoSlot = Option<(usize, Box<str>, Option<Box<str>>)>;

thread_local! {
    static MEMO: std::cell::RefCell<Vec<MemoSlot>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// A small per-thread, direct-mapped cache of lexicon answers, keyed by the
/// image (and a namespace) and the word. Bounded: at most [`MEMO_SLOTS`] words.
pub(crate) fn memo(space: usize, key: &str, f: impl FnOnce() -> Option<String>) -> Option<String> {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ space as u64;
    for b in key.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let slot = (h ^ h >> 29) as usize % MEMO_SLOTS;
    let hit = MEMO.try_with(|c| {
        let c = c.borrow();
        match c.get(slot) {
            Some(Some((s, k, v))) if *s == space && **k == *key => {
                Some(v.as_deref().map(str::to_string))
            }
            _ => None,
        }
    });
    if let Ok(Some(v)) = hit {
        return v;
    }
    let v = f();
    let _ = MEMO.try_with(|c| {
        let mut c = c.borrow_mut();
        if c.is_empty() {
            c.resize(MEMO_SLOTS, None);
        }
        c[slot] = Some((space, key.into(), v.as_deref().map(Box::from)));
    });
    v
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

/// A vowel letter of the phonetic alphabet.
pub fn is_vowel(c: char) -> bool {
    matches!(
        c,
        'a' | 'e'
            | 'i'
            | 'o'
            | 'u'
            | 'y'
            | 'ø'
            | 'œ'
            | 'ɛ'
            | 'ɔ'
            | 'ə'
            | 'ɪ'
            | 'ʊ'
            | 'ʏ'
            | 'ɑ'
            | 'ɐ'
            | 'æ'
            | 'ɜ'
            | 'ɨ'
            | 'ʉ'
            | 'ɯ'
            | 'ɤ'
            | 'ʌ'
            | 'ɒ'
            | 'ɵ'
            | 'ɘ'
            | 'ɚ'
    )
}

/// The stress state after the output `out` of one letter: bits 0-1 count
/// the vowels since the last stress mark (at most 3), bits 2-3 are that mark
/// (1 primary, 2 secondary), bit 4 says a primary stress was written.
pub fn stress_state(state: u8, out: &str) -> u8 {
    let mut s = state;
    for c in out.chars() {
        match c {
            'ˈ' => s = 0b1_01_00,
            'ˌ' => s = (s & 0b1_00_00) | 0b10_00,
            c if is_vowel(c) && s & 3 < 3 => s += 1,
            _ => {}
        }
    }
    s
}

/// The stress state together with a hash of the segments just chosen for
/// a letter: the key of the stress tables.
pub fn stress_key(state: u8, seg: &str) -> u32 {
    let h = seg.bytes().fold(0x9dc5u32, |h, b| {
        (h ^ b as u32).wrapping_mul(0x0193) & 0xffff
    });
    state as u32 | h << 8
}

/// [`context_hash`] for a table shape with flags: bit 7 of `left` adds the
/// stress state of the output so far, bits 4-6 of `right` give a number of
/// letters at the end of the word that are part of the context. Without
/// flags this is `context_hash`.
pub fn shape_hash(letters: &[u8], i: usize, left: u8, right: u8, state: u32) -> u32 {
    let mut h = context_hash(letters, i, (left & 15) as usize, (right & 15) as usize);
    if left & 0x80 == 0 && right & 0x70 == 0 {
        return h;
    }
    let mut step = |c: u8| {
        h ^= c as u32;
        h = h.wrapping_mul(0x0100_0193);
    };
    step(left);
    step(right);
    if left & 0x80 != 0 {
        step(state as u8);
    }
    if left & 0x40 != 0 {
        // A stress table also sees the segments chosen for the letter.
        step((state >> 8) as u8);
        step((state >> 16) as u8);
    }
    let e = ((right >> 4) & 7) as usize;
    for k in (1..=e).rev() {
        step(if letters.len() >= k {
            letters[letters.len() - k]
        } else {
            b'#'
        });
    }
    h
}

/// Mean entries per letter-to-sound hash bucket.
const LTS_BUCKET: usize = 8;

/// One letter-to-sound table: `(letters to the left, letters to the right, sorted (context hash, output id))`.
pub type LtsTable = (u8, u8, Vec<(u32, u16)>);

/// Builds a `.ucd` image. Used by `tools/lexgen` and by tests.
#[derive(Default)]
pub struct Builder {
    entries: Vec<(String, String)>,
    lts_tables: Vec<LtsTable>,
    lts_outputs: Vec<String>,
    meta: String,
    extra: Vec<([u8; 4], Vec<u8>)>,
    lex_block: usize,
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

    /// The serialised `LTS2` section (empty when there is no model).
    fn lts_section(&mut self) -> Result<Vec<u8>, DataError> {
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
            let width: u8 = if self.lts_outputs.len() <= 256 { 1 } else { 2 };
            for (l, r, t) in &self.lts_tables {
                let bits = (0..=16u8)
                    .find(|&b| (t.len() >> b) <= LTS_BUCKET)
                    .unwrap_or(16);
                let bucket = |h: u32| {
                    if bits == 0 {
                        0
                    } else {
                        (h >> (32 - bits as u32)) as usize
                    }
                };
                // Order by (bucket, fingerprint); a fingerprint seen twice in a
                // bucket keeps its first (most frequent context's) entry.
                let mut e: Vec<(usize, u16, u16)> =
                    t.iter().map(|&(h, v)| (bucket(h), h as u16, v)).collect();
                e.sort_by_key(|x| (x.0, x.1));
                e.dedup_by_key(|x| (x.0, x.1));
                if e.len() > 65_535 {
                    return Err(DataError("lts table too large"));
                }
                ltsm.extend([*l, *r, bits, width]);
                ltsm.extend((e.len() as u32).to_le_bytes());
                let mut at = 0usize;
                for k in 0..=(1usize << bits) {
                    while at < e.len() && e[at].0 < k {
                        at += 1;
                    }
                    ltsm.extend((at as u16).to_le_bytes());
                }
                for x in &e {
                    ltsm.extend(x.1.to_le_bytes());
                }
                for x in &e {
                    if width == 1 {
                        ltsm.push(x.2 as u8);
                    } else {
                        ltsm.extend(x.2.to_le_bytes());
                    }
                }
            }
        }
        Ok(ltsm)
    }

    /// Entries per lexicon block (smaller is faster to look up, larger is
    /// smaller); [`crate::lex2::DEFAULT_BLOCK`] unless set.
    pub fn block_size(&mut self, n: usize) -> &mut Self {
        self.lex_block = n;
        self
    }

    /// Add a raw section (for example `NAMC`, the name-origin model).
    pub fn section(&mut self, tag: [u8; 4], body: Vec<u8>) -> &mut Self {
        self.extra.push((tag, body));
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
        let ltsm_only = self.lts_section()?;
        let predictor = if ltsm_only.is_empty() {
            None
        } else {
            let mut img = Vec::new();
            img.extend(b"UCD1");
            img.extend(1u32.to_le_bytes());
            img.extend(b"LTS2");
            img.extend(20u32.to_le_bytes());
            img.extend((ltsm_only.len() as u32).to_le_bytes());
            img.extend(&ltsm_only);
            Some(Data::from_vec(img)?)
        };
        let block = if self.lex_block == 0 {
            crate::lex2::DEFAULT_BLOCK
        } else {
            self.lex_block
        };
        let lex2 = crate::lex2::encode(&self.entries, block, |k| {
            predictor
                .as_ref()
                .map(|d| d.predict(k))
                .filter(|p| !p.is_empty())
        })?;
        let ltsm = ltsm_only;

        let mut sections: Vec<(&[u8; 4], &[u8])> =
            vec![(b"LEX2", &lex2), (b"META", self.meta.as_bytes())];
        if !ltsm.is_empty() {
            sections.push((b"LTS2", &ltsm));
        }
        for (tag, body) in &self.extra {
            sections.push((tag, body));
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
