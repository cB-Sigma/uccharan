//! The pack file and its hash tables.
//!
//! A transliteration pack is a `.ucd` file (`UCD1`, a section table, the
//! sections) whose `META` section says `lang=xlit-<language>`. It has no
//! lexicon section; its sections are listed in the parent module.
//!
//! The tables are open-addressed arrays of fixed-width slots: a non-zero
//! fingerprint of the key followed by the value bytes. Keys are not stored,
//! so a lookup of an absent key answers "present" about once in
//! 2^(fingerprint bits) / (probe length) lookups.

/// The sections of a loaded pack.
#[derive(Clone, Debug, Default)]
pub struct Sections {
    bytes: Vec<u8>,
    index: Vec<([u8; 4], usize, usize)>,
}

fn rd_u32(b: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?) as usize)
}

impl Sections {
    /// Read the section table of a `.ucd` image.
    pub fn parse(bytes: Vec<u8>) -> Result<Sections, String> {
        if bytes.len() < 8 || &bytes[..4] != b"UCD1" {
            return Err("not a .ucd file".into());
        }
        let n = rd_u32(&bytes, 4).unwrap_or(0);
        if n > 64 {
            return Err("too many sections".into());
        }
        let mut index = Vec::with_capacity(n);
        for i in 0..n {
            let at = 8 + i * 12;
            let (Some(tag), Some(off), Some(len)) = (
                bytes.get(at..at + 4),
                rd_u32(&bytes, at + 4),
                rd_u32(&bytes, at + 8),
            ) else {
                return Err("truncated pack".into());
            };
            if off.checked_add(len).is_none_or(|end| end > bytes.len()) {
                return Err("section out of range".into());
            }
            index.push(([tag[0], tag[1], tag[2], tag[3]], off, len));
        }
        Ok(Sections { bytes, index })
    }

    /// The bytes of a section, empty when absent.
    pub fn get(&self, tag: &[u8; 4]) -> &[u8] {
        match self.index.iter().find(|s| &s.0 == tag) {
            Some(&(_, off, len)) => &self.bytes[off..off + len],
            None => &[],
        }
    }

    /// A `key=value` line of the `META` section.
    pub fn meta(&self, key: &str) -> Option<&str> {
        meta_line(self.get(b"META"), key)
    }

    /// Section tags and sizes.
    pub fn sizes(&self) -> Vec<(String, usize)> {
        self.index
            .iter()
            .map(|(t, _, l)| (String::from_utf8_lossy(t).into_owned(), *l))
            .collect()
    }

    /// Size of the whole image.
    pub fn byte_len(&self) -> usize {
        self.bytes.len()
    }
}

fn meta_line<'a>(meta: &'a [u8], key: &str) -> Option<&'a str> {
    std::str::from_utf8(meta)
        .ok()?
        .lines()
        .find_map(|l| l.split_once('=').filter(|(k, _)| *k == key).map(|(_, v)| v))
}

/// The language of a transliteration pack image (`lang=xlit-<tag>` in its
/// metadata), without loading it. `None` for any other file.
pub fn pack_language(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 8 || &bytes[..4] != b"UCD1" {
        return None;
    }
    let n = rd_u32(bytes, 4)?.min(64);
    for i in 0..n {
        let at = 8 + i * 12;
        if bytes.get(at..at + 4)? == b"META" {
            let (off, len) = (rd_u32(bytes, at + 4)?, rd_u32(bytes, at + 8)?);
            let meta = bytes.get(off..off.checked_add(len)?)?;
            return meta_line(meta, "lang")?
                .strip_prefix("xlit-")
                .map(str::to_string);
        }
    }
    None
}

/// Serialise sections as a `.ucd` image (used by the pack builder).
pub fn write_container(sections: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut out: Vec<u8> = b"UCD1".to_vec();
    out.extend((sections.len() as u32).to_le_bytes());
    let mut off = 8 + sections.len() * 12;
    for (tag, body) in sections {
        out.extend(tag);
        out.extend((off as u32).to_le_bytes());
        out.extend((body.len() as u32).to_le_bytes());
        off += body.len();
    }
    for (_, body) in sections {
        out.extend(body);
    }
    out
}

/// 64-bit mix (the splitmix64 finaliser).
#[inline]
pub fn mix(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// Hash of a byte string (FNV-1a, then mixed).
pub fn hash_bytes(b: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &c in b {
        h ^= c as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    mix(h)
}

/// Hash of an ordered pair of hashes.
pub fn hash2(a: u64, b: u64) -> u64 {
    mix(a ^ b.rotate_left(29).wrapping_mul(0x9e37_79b9_7f4a_7c15))
}

#[inline]
fn fingerprint(h: u64) -> u32 {
    (h as u32).max(1)
}

/// Build a table with a 4-byte fingerprint and `V` value bytes per key
/// (used by the pack builder).
pub fn build_table<const V: usize>(items: &[(u64, [u8; V])]) -> Vec<u8> {
    let slots = ((items.len() as f64 / 0.7) as usize + 1).max(2);
    let w = 4 + V;
    let mut out = vec![0u8; 4 + slots * w];
    out[..4].copy_from_slice(&(slots as u32).to_le_bytes());
    for (h, v) in items {
        let fp = fingerprint(*h);
        let mut i = (h >> 32) as usize % slots;
        loop {
            let at = 4 + i * w;
            if out[at..at + 4] == [0, 0, 0, 0] {
                out[at..at + 4].copy_from_slice(&fp.to_le_bytes());
                out[at + 4..at + w].copy_from_slice(v);
                break;
            }
            i += 1;
            if i == slots {
                i = 0;
            }
        }
    }
    out
}

/// Look a key up in a table built by [`build_table`].
#[inline]
pub fn lookup<const V: usize>(table: &[u8], h: u64) -> Option<[u8; V]> {
    let slots = rd_u32(table, 0)?;
    let w = 4 + V;
    if slots == 0 || table.len() < 4 + slots * w {
        return None;
    }
    let fp = fingerprint(h).to_le_bytes();
    let mut i = (h >> 32) as usize % slots;
    for _ in 0..slots {
        let at = 4 + i * w;
        let s = &table[at..at + w];
        if s[..4] == fp {
            return s[4..].try_into().ok();
        }
        if s[..4] == [0, 0, 0, 0] {
            return None;
        }
        i += 1;
        if i == slots {
            i = 0;
        }
    }
    None
}

#[inline]
fn fingerprint3(h: u64) -> [u8; 3] {
    let f = [h as u8, (h >> 8) as u8, (h >> 16) as u8];
    if f == [0, 0, 0] {
        [1, 0, 0]
    } else {
        f
    }
}

/// Slots per bucket of the n-gram tables.
const BUCKET: usize = 4;

#[inline]
fn buckets_of(h: u64, n: usize) -> (usize, usize) {
    ((h >> 32) as usize % n, (mix(h) >> 32) as usize % n)
}

/// Build a table with a 3-byte fingerprint and two value bytes per key: the
/// n-gram tables (used by the pack builder). A key lives in one of two
/// buckets of four 5-byte slots (cuckoo hashing), so a lookup reads at most
/// eight slots whether the key is there or not. A stray hit gives a stray
/// score, which costs at most a rank.
pub fn build_table3(items: &[(u64, [u8; 2])]) -> Vec<u8> {
    let mut n = (items.len() as f64 / (BUCKET as f64 * 0.9)) as usize + 1;
    'size: loop {
        // Slot contents during the build: index of the item, or none.
        let mut slots: Vec<u32> = vec![u32::MAX; n * BUCKET];
        let mut rng: u64 = 0x2545_f491_4f6c_dd1d;
        for start in 0..items.len() {
            let mut item = start as u32;
            let mut bucket = buckets_of(items[start].0, n).0;
            let mut placed = false;
            for kick in 0..2000 {
                let (b1, b2) = buckets_of(items[item as usize].0, n);
                // The first bucket is tried first, so that an empty slot in
                // it proves a key absent.
                let tries = if kick == 0 {
                    [b1, b2]
                } else {
                    [bucket, bucket]
                };
                for b in tries {
                    if let Some(free) = (0..BUCKET).find(|i| slots[b * BUCKET + i] == u32::MAX) {
                        slots[b * BUCKET + free] = item;
                        placed = true;
                        break;
                    }
                }
                if placed {
                    break;
                }
                // Both full: take the place of a resident of one of them and
                // move that one to its other bucket.
                rng ^= rng << 13;
                rng ^= rng >> 7;
                rng ^= rng << 17;
                let from = match (kick, rng & 16) {
                    (0, 0) => b1,
                    (0, _) => b2,
                    _ => bucket,
                };
                let at = from * BUCKET + (rng as usize % BUCKET);
                let out = std::mem::replace(&mut slots[at], item);
                let (o1, o2) = buckets_of(items[out as usize].0, n);
                bucket = if o1 == from { o2 } else { o1 };
                item = out;
            }
            if !placed {
                n += n / 32 + 1;
                continue 'size;
            }
        }
        let mut out = vec![0u8; 4 + n * BUCKET * 5];
        out[..4].copy_from_slice(&(n as u32).to_le_bytes());
        for (i, &it) in slots.iter().enumerate() {
            if it != u32::MAX {
                let (h, v) = items[it as usize];
                let at = 4 + i * 5;
                out[at..at + 3].copy_from_slice(&fingerprint3(h));
                out[at + 3..at + 5].copy_from_slice(&v);
            }
        }
        return out;
    }
}

/// Look a key up in a table built by [`build_table3`]. The table must have
/// been checked with [`table3_ok`].
#[inline]
pub fn lookup3(table: &[u8], h: u64) -> Option<[u8; 2]> {
    let n = (table.len() - 4) / (BUCKET * 5);
    let fp = fingerprint3(h);
    let (b1, b2) = buckets_of(h, n);
    let at = 4 + b1 * BUCKET * 5;
    let mut full = true;
    for s in table[at..at + BUCKET * 5].chunks_exact(5) {
        if s[..3] == fp {
            return Some([s[3], s[4]]);
        }
        full &= s[..3] != [0, 0, 0];
    }
    if !full {
        return None;
    }
    let at = 4 + b2 * BUCKET * 5;
    for s in table[at..at + BUCKET * 5].chunks_exact(5) {
        if s[..3] == fp {
            return Some([s[3], s[4]]);
        }
    }
    None
}

/// True when `table` is a well-formed [`build_table3`] table.
pub fn table3_ok(table: &[u8]) -> bool {
    rd_u32(table, 0).is_some_and(|n| n >= 1 && table.len() == 4 + n * BUCKET * 5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_find_what_was_stored() {
        let items: Vec<(u64, [u8; 2])> = (0..1000u64)
            .map(|i| (mix(i), [i as u8, (i >> 8) as u8]))
            .collect();
        let t = build_table::<2>(&items);
        let t3 = build_table3(&items);
        assert!(table3_ok(&t3));
        for (h, v) in &items {
            assert_eq!(lookup::<2>(&t, *h), Some(*v));
            assert_eq!(lookup3(&t3, *h), Some(*v));
        }
        assert_eq!(lookup::<2>(&t, mix(5000)), None);
        assert_eq!(lookup::<2>(&[], 1), None);
    }

    #[test]
    fn container_round_trip() {
        let img = write_container(&[
            (*b"META", b"lang=xlit-hi\nformat=1\n".to_vec()),
            (*b"UNIT", b"x".to_vec()),
        ]);
        assert_eq!(pack_language(&img).as_deref(), Some("hi"));
        let s = Sections::parse(img).unwrap();
        assert_eq!(s.get(b"UNIT"), b"x");
        assert_eq!(s.meta("format"), Some("1"));
        assert_eq!(s.get(b"NONE"), b"");
        assert!(Sections::parse(b"UCD1\x01\0\0\0".to_vec()).is_err());
    }
}
