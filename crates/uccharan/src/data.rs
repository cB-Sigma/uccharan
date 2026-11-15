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
//! * `BLM1` — a Bloom filter over the lexicon's keys (optional): a word that
//!   fails it is not in the lexicon, so most misses decode nothing.

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
    /// The key filter (`BLM1`): offset and length of its bits, hash count.
    bloom: (usize, usize, u32),
    /// `letters=` of `META`: the model's letters beyond ASCII.
    letters: Option<Box<[char]>>,
    /// `stress=latent` in `META`.
    stress_latent: bool,
    /// `input=rules` in `META`: the model reads the letter rules' reading
    /// of a word, not its spelling.
    reads_rules: bool,
}

#[derive(Clone, Copy)]
struct LtsShape {
    left: u8,
    right: u8,
    /// 32 less the log2 of the bucket count: the top bits of a hash that
    /// are left number the bucket
    shift: u8,
    /// bytes per output id (1 or 2)
    width: u8,
    count: usize,
    /// u16 start of each bucket, plus one end marker
    starts: usize,
    /// u16 fingerprint per entry
    prints: usize,
    /// output id per entry
    vals: usize,
    /// first word of the table's part of the presence bits
    seen: usize,
    /// 32 less the log2 of the number of its presence bits
    seen_shift: u8,
}

impl LtsShape {
    /// The presence bit of the entry with fingerprint `fp` in `bucket`:
    /// word and mask.
    #[inline(always)]
    fn seen_bit(&self, bucket: usize, fp: u16) -> (usize, u64) {
        let key = ((bucket as u32) << 16 | fp as u32).wrapping_mul(0x9e37_79b1);
        let bit = (key >> self.seen_shift) as usize;
        (self.seen + bit / 64, 1 << (bit % 64))
    }
}

#[derive(Clone)]
struct LtsHeader {
    /// The segment tables, most specific first.
    tables: Vec<LtsShape>,
    /// The tables that give the stress mark of a letter's vowel (flag 0x40).
    marks: Vec<LtsShape>,
    /// One bit per (bucket, fingerprint) that some entry has, set on first
    /// use: most searches for a context that a table does not have end
    /// there. `seen_words` words of 64 bits.
    seen: std::sync::OnceLock<Box<[u64]>>,
    seen_words: usize,
    /// Some tables give the stress mark of a letter's vowel.
    stress: bool,
    /// Both chains have the usual shapes ([`USUAL`], [`USUAL_MARKS`]).
    usual: bool,
    outputs: Vec<LtsOut>,
    /// The output strings, one after the other.
    text: String,
    /// What the outputs do to the stress state, one code per stress mark
    /// or vowel (see [`LtsOut`]).
    ops: Vec<u8>,
}

/// An output string of the model, with what the run-time needs to know
/// about it worked out once.
#[derive(Clone, Copy)]
struct LtsOut {
    off: u32,
    len: u8,
    /// Byte offset of the first vowel (255: none).
    vowel_at: u8,
    /// Its codes in `LtsHeader::ops`: 1 primary stress, 2 secondary, 3 vowel.
    ops: u32,
    ops_len: u8,
    /// How many of them stand before the first vowel.
    ops_before: u8,
    /// The hash of the string that [`stress_key`] uses.
    key: u16,
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
        let mut bloom = (0, 0, 0);
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
                b"BLM1" if len > 4 => bloom = (off + 4, len - 4, rd_u32(b, off)?.min(16)),
                b"NAMC" => namc = (off, len),
                b"KNWN" => {
                    crate::known::validate(&b[off..end])?;
                    knwn = (off, len);
                }
                _ => {}
            }
        }
        let meta_line = |key: &str| {
            std::str::from_utf8(&b[meta.0..meta.0 + meta.1])
                .ok()?
                .lines()
                .find_map(|l| l.split_once('=').filter(|(k, _)| *k == key).map(|(_, v)| v))
        };
        let letters = meta_line("letters").map(|v| v.chars().collect());
        let stress_latent = meta_line("stress") == Some("latent");
        let reads_rules = meta_line("input") == Some("rules");
        static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
        Ok(Data {
            bytes,
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            lex2,
            namc,
            knwn,
            ltsm,
            meta,
            bloom,
            letters,
            stress_latent,
            reads_rules,
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
        let mut ops: Vec<u8> = Vec::new();
        let mut all = String::new();
        for _ in 0..n_out {
            let l = *b.get(at).ok_or(DataError("truncated"))? as usize;
            let text = b
                .get(at + 1..at + 1 + l)
                .filter(|_| at + 1 + l <= end)
                .and_then(|t| std::str::from_utf8(t).ok())
                .ok_or(DataError("bad lts output"))?;
            let mut o = LtsOut {
                off: all.len() as u32,
                len: l as u8,
                vowel_at: 255,
                ops: ops.len() as u32,
                ops_len: 0,
                ops_before: 0,
                key: (stress_key(0, text) >> 8) as u16,
            };
            for (k, c) in text.char_indices() {
                let code = match c {
                    'ˈ' => 1,
                    'ˌ' => 2,
                    c if is_vowel(c) => 3,
                    _ => continue,
                };
                if code == 3 && o.vowel_at == 255 {
                    o.vowel_at = k as u8;
                    o.ops_before = o.ops_len;
                }
                ops.push(code);
                o.ops_len += 1;
            }
            if o.vowel_at == 255 {
                o.ops_before = o.ops_len;
            }
            all.push_str(text);
            outputs.push(o);
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
                shift: 32 - bits,
                seen: 0,
                seen_shift: 0,
                width,
                count,
                starts,
                prints,
                vals,
            });
            at = next;
        }
        // About sixteen presence bits per entry.
        let mut seen_words = 0usize;
        for t in &mut tables {
            let bits = (t.count * 16).next_power_of_two().max(64);
            t.seen = seen_words;
            t.seen_shift = 32 - bits.trailing_zeros() as u8;
            seen_words += bits / 64;
        }
        let (marks, tables): (Vec<LtsShape>, Vec<LtsShape>) =
            tables.into_iter().partition(|t| t.left & 0x40 != 0);
        let shaped = |chain: &[LtsShape], usual: &[(u8, u8)]| {
            chain.len() == usual.len()
                && chain
                    .iter()
                    .zip(usual)
                    .all(|(t, u)| (t.left & 15, t.right) == *u)
        };
        let usual = shaped(&tables, &USUAL) && (marks.is_empty() || shaped(&marks, &USUAL_MARKS));
        Ok(LtsHeader {
            seen: std::sync::OnceLock::new(),
            seen_words,
            usual,
            stress: !marks.is_empty(),
            tables,
            marks,
            outputs,
            text: all,
            ops,
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

    /// The letters beyond ASCII that the letter-to-sound model was trained
    /// with (`letters=` of `META`); empty without such a model.
    pub fn model_letters(&self) -> &[char] {
        self.letters.as_deref().unwrap_or(&[])
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
        if !self.may_have(key) {
            return false;
        }
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
        let present: &[u64] = h.seen.get_or_init(|| {
            let mut bits = vec![0u64; h.seen_words];
            for t in h.tables.iter().chain(&h.marks) {
                let rd = |at: usize| rd_u16(b, at).map_or(0, |v| v as usize);
                for bucket in 0..1usize << (32 - t.shift) {
                    let lo = rd(t.starts + bucket * 2);
                    let hi = rd(t.starts + bucket * 2 + 2).min(t.count);
                    for i in lo..hi {
                        let (word, mask) = t.seen_bit(bucket, rd(t.prints + i * 2) as u16);
                        if let Some(w) = bits.get_mut(word) {
                            *w |= mask;
                        }
                    }
                }
            }
            bits.into_boxed_slice()
        });
        // The word between two runs of the edge sign, so that a context is
        // one slice.
        let n = letters.len();
        let mut small = [b'#'; 96];
        let large;
        let padded: &[u8] = if n + 2 * PAD <= small.len() {
            small[PAD..PAD + n].copy_from_slice(letters);
            &small[..n + 2 * PAD]
        } else {
            let mut v = vec![b'#'; n + 2 * PAD];
            v[PAD..PAD + n].copy_from_slice(letters);
            large = v;
            &large
        };
        // The first table of a chain that has the context of letter `i`
        // answers (`usize::MAX`: none does). `seen` keeps the hashes of
        // the letter's contexts for its second chain.
        let pick = |i: usize, marks: bool, state: u32, seen: &mut [u32; 8]| -> usize {
            let chain: &[LtsShape] = if marks { &h.marks } else { &h.tables };
            if !h.usual {
                return chain
                    .iter()
                    .map(|t| {
                        Self::lts_find(
                            b,
                            present,
                            t,
                            padded_hash(padded, i, t.left, t.right, state),
                        )
                    })
                    .find(|&v| v != usize::MAX)
                    .unwrap_or(usize::MAX);
            }
            if !marks {
                // The letter with its neighbours as the hashes read them.
                let p = PAD + i;
                let mut ctx = [b'|'; 2 * NEAR + 3];
                ctx[..NEAR].copy_from_slice(&padded[p - NEAR..p]);
                ctx[NEAR + 1] = padded[p];
                ctx[NEAR + 3..].copy_from_slice(&padded[p + 1..p + 1 + NEAR]);
                *seen = usual_hashes(&ctx);
            }
            // The hashes do not depend on each other: the processor works
            // on them together.
            let mut hs = *seen;
            if marks {
                hs[6] = hs[7];
            }
            for (x, t) in hs.iter_mut().zip(chain) {
                *x = flag_hash(*x, t.left, t.right, state);
            }
            for (t, x) in chain.iter().zip(hs) {
                let v = Self::lts_find(b, present, t, x);
                if v != usize::MAX {
                    return v;
                }
            }
            usize::MAX
        };
        let step = |state: u8, ops: &[u8]| {
            ops.iter().fold(state, |s, &op| match op {
                1 => 0b1_01_00,
                2 => (s & 0b1_00_00) | 0b10_00,
                _ if s & 3 < 3 => s + 1,
                _ => s,
            })
        };
        let ops_of = |o: &LtsOut| {
            h.ops
                .get(o.ops as usize..o.ops as usize + o.ops_len as usize)
                .unwrap_or(&[])
        };
        let text_of = |o: &LtsOut| {
            h.text
                .get(o.off as usize..o.off as usize + o.len as usize)
                .unwrap_or("")
        };
        for i in 0..n {
            let mut seen = [0u32; 8];
            let Some(seg) = h.outputs.get(pick(i, false, state as u32, &mut seen)) else {
                continue;
            };
            let (text, ops) = (text_of(seg), ops_of(seg));
            if h.stress && seg.vowel_at != 255 {
                // The stress mark of the letter's vowel stands before it.
                let at = (seg.vowel_at as usize).min(text.len());
                let before = (seg.ops_before as usize).min(ops.len());
                let key = state as u32 | (seg.key as u32) << 8;
                out.push_str(&text[..at]);
                state = step(state, &ops[..before]);
                if let Some(mark) = h.outputs.get(pick(i, true, key, &mut seen)) {
                    out.push_str(text_of(mark));
                    state = step(state, ops_of(mark));
                }
                out.push_str(&text[at..]);
                state = step(state, &ops[before..]);
            } else {
                out.push_str(text);
                state = step(state, ops);
            }
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
        let extra = self.letters.as_deref()?;
        let mut small = [0u8; 64];
        let mut large = Vec::new();
        let mut n = 0usize;
        for c in word.chars() {
            let b = if c.is_ascii_lowercase() || c == '\'' || c == '-' {
                c as u8
            } else if c.is_ascii() {
                return None;
            } else {
                0x80 + extra.iter().position(|&x| x == c)? as u8
            };
            if n < small.len() {
                small[n] = b;
            } else {
                if n == small.len() {
                    large.extend_from_slice(&small);
                }
                large.push(b);
            }
            n += 1;
        }
        let letters: &[u8] = if large.is_empty() {
            &small[..n]
        } else {
            &large
        };
        let mut out = String::with_capacity(n * 3 + 8);
        (self.lts(letters, &mut out) && !out.is_empty()).then_some(out)
    }

    /// True when the model's stress marks are the ones the language's
    /// stress rule cannot derive (`stress=latent` in `META`).
    pub fn stress_latent(&self) -> bool {
        self.stress_latent
    }

    /// True when the model corrects the reading of the language's letter
    /// rules instead of reading the spelling (`input=rules` in `META`).
    pub fn reads_rules(&self) -> bool {
        self.reads_rules
    }

    /// False when the key filter says `key` is not in the lexicon; true
    /// when it may be, and for images without a filter.
    pub fn may_have(&self, key: &str) -> bool {
        let (off, len, k) = self.bloom;
        if len == 0 {
            return true;
        }
        let bits = &self.bytes[off..off + len];
        let (h1, h2) = bloom_hashes(key);
        let m = len as u64 * 8;
        (0..k as u64).all(|i| {
            let at = (h1.wrapping_add(i.wrapping_mul(h2)) % m) as usize;
            bits[at / 8] & (1 << (at % 8)) != 0
        })
    }

    /// The output of the entry of table `t` for the context hash `h`, or
    /// `usize::MAX`. The entries of a bucket are sorted by fingerprint.
    #[inline(always)]
    fn lts_find(b: &[u8], seen: &[u64], t: &LtsShape, h: u32) -> usize {
        let bucket = ((h as u64) >> t.shift) as usize;
        let (word, mask) = t.seen_bit(bucket, h as u16);
        if seen.get(word).is_some_and(|w| w & mask == 0) {
            return usize::MAX;
        }
        let Some(s) = b.get(t.starts + bucket * 2..t.starts + bucket * 2 + 4) else {
            return usize::MAX;
        };
        let lo = u16::from_le_bytes([s[0], s[1]]) as usize;
        let hi = (u16::from_le_bytes([s[2], s[3]]) as usize).min(t.count);
        let Some(prints) = b.get(t.prints + lo * 2..t.prints + hi * 2) else {
            return usize::MAX;
        };
        if prints.is_empty() {
            return usize::MAX;
        }
        let fp = h as u16;
        let print = |i: usize| u16::from_le_bytes([prints[i * 2], prints[i * 2 + 1]]);
        // The last entry whose fingerprint is not larger.
        let (mut i, mut size) = (0usize, hi - lo);
        while size > 1 {
            let half = size / 2;
            i = if print(i + half) <= fp { i + half } else { i };
            size -= half;
        }
        if print(i) != fp {
            return usize::MAX;
        }
        let i = lo + i;
        if t.width == 1 {
            b.get(t.vals + i).map_or(usize::MAX, |&v| v as usize)
        } else {
            b.get(t.vals + i * 2..t.vals + i * 2 + 2)
                .map_or(usize::MAX, |p| u16::from_le_bytes([p[0], p[1]]) as usize)
        }
    }
}

/// The widest context, in letters to one side, of the usual shapes.
const NEAR: usize = 4;

/// The usual chain of shapes (letters to the left and to the right), most
/// specific first.
const USUAL: [(u8, u8); 8] = [
    (4, 4),
    (3, 3),
    (2, 3),
    (2, 2),
    (1, 2),
    (1, 1),
    (0, 1),
    (0, 0),
];

/// The usual chain of the stress tables.
const USUAL_MARKS: [(u8, u8); 8] = [
    (4, 4),
    (3, 3),
    (2, 3),
    (2, 2),
    (1, 2),
    (1, 1),
    (0, 0),
    (0, 0),
];

/// One step of the context hash.
#[inline(always)]
fn fnv(h: u32, c: u8) -> u32 {
    (h ^ c as u32).wrapping_mul(0x0100_0193)
}

/// The hash of the middle `L + R + 3` bytes of a context.
#[inline(always)]
fn fixed_hash<const L: usize, const R: usize>(ctx: &[u8; 2 * NEAR + 3]) -> u32 {
    let mut h: u32 = 0x811c_9dc5 ^ ((L as u32) << 8 | R as u32);
    for &c in &ctx[NEAR - L..NEAR + 3 + R] {
        h = fnv(h, c);
    }
    h
}

/// [`context_hash`] for each shape of [`USUAL`]. `ctx` holds the letters to
/// the left, `|`, the letter, `|` and the letters to the right.
#[inline(always)]
fn usual_hashes(ctx: &[u8; 2 * NEAR + 3]) -> [u32; 8] {
    [
        fixed_hash::<4, 4>(ctx),
        fixed_hash::<3, 3>(ctx),
        fixed_hash::<2, 3>(ctx),
        fixed_hash::<2, 2>(ctx),
        fixed_hash::<1, 2>(ctx),
        fixed_hash::<1, 1>(ctx),
        fixed_hash::<0, 1>(ctx),
        fixed_hash::<0, 0>(ctx),
    ]
}

/// What the flags of a shape add to its [`context_hash`] `h` (see
/// [`shape_hash`]), for shapes that do not look at the end of the word.
#[inline(always)]
fn flag_hash(mut h: u32, left: u8, right: u8, state: u32) -> u32 {
    if left & 0x80 == 0 && right & 0x70 == 0 {
        return h;
    }
    h = fnv(fnv(h, left), right);
    if left & 0x80 != 0 {
        h = fnv(h, state as u8);
    }
    if left & 0x40 != 0 {
        h = fnv(fnv(h, (state >> 8) as u8), (state >> 16) as u8);
    }
    h
}

/// Edge signs on each side of a word in [`padded_hash`].
const PAD: usize = 16;

/// The two hashes of a key for the key filter.
fn bloom_hashes(key: &str) -> (u64, u64) {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in key.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let h1 = h ^ (h >> 32);
    let h2 = (h.wrapping_mul(0x9e37_79b9_7f4a_7c15) >> 17) | 1;
    (h1, h2)
}

/// A key filter for `keys` with `bits_per_key` bits each: the body of a
/// `BLM1` section.
pub fn bloom_section<'a>(
    keys: impl Iterator<Item = &'a str> + Clone,
    bits_per_key: usize,
) -> Vec<u8> {
    let n = keys.clone().count().max(1);
    let bytes = (n * bits_per_key).div_ceil(8).max(8);
    let k = ((bits_per_key as f64 * 0.69).round() as u32).clamp(1, 16);
    let mut out = vec![0u8; 4 + bytes];
    out[..4].copy_from_slice(&k.to_le_bytes());
    let m = bytes as u64 * 8;
    for key in keys {
        let (h1, h2) = bloom_hashes(key);
        for i in 0..k as u64 {
            let at = (h1.wrapping_add(i.wrapping_mul(h2)) % m) as usize;
            out[4 + at / 8] |= 1 << (at % 8);
        }
    }
    out
}

/// [`shape_hash`] of letter `i` of a word that stands in `padded` between
/// two runs of [`PAD`] edge signs.
#[inline]
fn padded_hash(padded: &[u8], i: usize, left: u8, right: u8, state: u32) -> u32 {
    let (l, r) = ((left & 15) as usize, (right & 15) as usize);
    let mut h: u32 = 0x811c_9dc5 ^ ((l as u32) << 8 | r as u32);
    let mut step = |c: u8| {
        h ^= c as u32;
        h = h.wrapping_mul(0x0100_0193);
    };
    let p = PAD + i;
    for &c in &padded[p - l..p] {
        step(c);
    }
    step(b'|');
    step(padded[p]);
    step(b'|');
    for &c in &padded[p + 1..p + 1 + r] {
        step(c);
    }
    if left & 0x80 == 0 && right & 0x70 == 0 {
        return h;
    }
    step(left);
    step(right);
    if left & 0x80 != 0 {
        step(state as u8);
    }
    if left & 0x40 != 0 {
        step((state >> 8) as u8);
        step((state >> 16) as u8);
    }
    let e = ((right >> 4) & 7) as usize;
    let end = padded.len() - PAD;
    for &c in &padded[end - e..end] {
        step(c);
    }
    h
}

const MEMO_SLOTS: usize = 4096;

type MemoSlot = Option<((usize, usize), Box<str>, Option<Box<str>>)>;

thread_local! {
    static MEMO: std::cell::RefCell<Vec<MemoSlot>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// A small per-thread, direct-mapped cache of lexicon answers, keyed by the
/// image (and a namespace) and the word. Bounded: at most [`MEMO_SLOTS`] words.
pub(crate) fn memo(space: usize, key: &str, f: impl FnOnce() -> Option<String>) -> Option<String> {
    memo2((space, 0), key, f)
}

/// [`memo`] with a namespace of two numbers.
pub(crate) fn memo2(
    space: (usize, usize),
    key: &str,
    f: impl FnOnce() -> Option<String>,
) -> Option<String> {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ space.0 as u64;
    for b in key.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    if space.1 != 0 {
        h ^= space.1 as u64;
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
    bloom_bits: usize,
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

    /// Add a key filter (`BLM1`) with this many bits per entry (0: none).
    /// It makes a lookup of a word the lexicon does not have cheap.
    pub fn key_filter(&mut self, bits_per_key: usize) -> &mut Self {
        self.bloom_bits = bits_per_key;
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
        let filter = if self.bloom_bits > 0 && !self.entries.is_empty() {
            bloom_section(self.entries.iter().map(|e| e.0.as_str()), self.bloom_bits)
        } else {
            Vec::new()
        };
        if !filter.is_empty() {
            sections.push((b"BLM1", &filter));
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
    fn padded_hash_is_shape_hash() {
        let words: [&[u8]; 5] = [b"a", b"ab", b"strasse", b"x\x80y\x81-z'", &[b'q'; 120]];
        for w in words {
            let mut padded = vec![b'#'; w.len() + 2 * PAD];
            padded[PAD..PAD + w.len()].copy_from_slice(w);
            for i in 0..w.len() {
                for (l, r) in [
                    (4u8, 4u8),
                    (0, 0),
                    (2, 3),
                    (15, 15),
                    (0xc4, 4),
                    (0xc0, 0),
                    (0x40, 0),
                    (0x82, 0x32),
                    (1, 0x71),
                ] {
                    for st in [0u32, 0x12_34_15] {
                        assert_eq!(
                            padded_hash(&padded, i, l, r, st),
                            shape_hash(w, i, l, r, st)
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn usual_hashes_are_shape_hashes() {
        let words: [&[u8]; 4] = [b"a", b"ab", b"strasse", b"x\x80y\x81-z'"];
        for w in words {
            let mut padded = vec![b'#'; w.len() + 2 * PAD];
            padded[PAD..PAD + w.len()].copy_from_slice(w);
            for i in 0..w.len() {
                let p = PAD + i;
                let mut ctx = [b'|'; 2 * NEAR + 3];
                ctx[..NEAR].copy_from_slice(&padded[p - NEAR..p]);
                ctx[NEAR + 1] = padded[p];
                ctx[NEAR + 3..].copy_from_slice(&padded[p + 1..p + 1 + NEAR]);
                let hs = usual_hashes(&ctx);
                for (k, (l, r)) in USUAL.into_iter().enumerate() {
                    for flags in [0u8, 0x80, 0xc0, 0x40] {
                        for st in [0u32, 0x12_34_15] {
                            assert_eq!(
                                flag_hash(hs[k], l | flags, r, st),
                                shape_hash(w, i, l | flags, r, st)
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn key_filter() {
        let mut b = Builder::new();
        for i in 0..500 {
            b.entry(&format!("word{i}"), "a");
        }
        b.key_filter(8);
        let d = Data::from_vec(b.build().unwrap()).unwrap();
        let mut out = String::new();
        assert!((0..500)
            .all(|i| d.may_have(&format!("word{i}"))
                && d.lookup_uncached(&format!("word{i}"), &mut out)));
        let wrong = (0..2000)
            .filter(|i| d.may_have(&format!("other{i}")))
            .count();
        assert!(wrong < 150, "{wrong}");
        assert!(!d.lookup_uncached("other1", &mut out));
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
