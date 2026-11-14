//! The word data of a pack: native word counts, the spelling lexicon,
//! word-pair association, English word counts, two letter models and the
//! homograph list. All of it is read in place from the pack.

use super::pack::{hash2, hash_bytes, lookup, Sections};

/// Quantisation step of stored log counts.
pub const F_STEP: f32 = 1.0 / 16.0;
/// Quantisation step of stored spelling shares.
pub const SHARE_STEP: f32 = 0.03;
/// Quantisation step of the letter models.
pub const CLM_STEP: f32 = 0.04;
/// Size of a letter-trigram model: 27^3 bytes (a–z and the word boundary).
pub const CLM_LEN: usize = 27 * 27 * 27;
/// ln(1,000,000): turns ln(count / total) into ln(count per million).
const LN_MILLION: f32 = 13.8155;

/// Average log-probability per letter of a lower-case a–z word under a
/// letter-trigram model.
pub fn charlm_score(lm: &[u8], w: &str) -> f32 {
    if lm.len() != CLM_LEN {
        return -3.0;
    }
    let (mut a, mut b, mut s, mut n) = (0usize, 0usize, 0.0f32, 0);
    for x in w
        .bytes()
        .filter(|x| x.is_ascii_lowercase())
        .map(|x| (x - b'a' + 1) as usize)
        .chain([0])
    {
        s -= lm[(a * 27 + b) * 27 + x] as f32 * CLM_STEP;
        a = b;
        b = x;
        n += 1;
    }
    s / n as f32
}

/// A view of the word data of a pack.
#[derive(Clone, Copy)]
pub struct Lex<'a> {
    nfrq: &'a [u8],
    lexo: &'a [u8],
    lexw: &'a [u8],
    lexe: &'a [u8],
    lexr: &'a [u8],
    bigr: &'a [u8],
    enfq: &'a [u8],
    homo: &'a [u8],
    /// Letter model of romanised words.
    pub clm_indic: &'a [u8],
    /// Letter model of English words.
    pub clm_en: &'a [u8],
    ln_total: f32,
    ln_en_total: f32,
}

impl<'a> Lex<'a> {
    /// The word data of `pack`.
    pub fn load(pack: &'a Sections) -> Lex<'a> {
        let t = |k: &str| {
            pack.meta(k)
                .and_then(|v| v.parse::<f64>().ok())
                .filter(|v| *v >= 1.0)
                .unwrap_or(1e6)
                .ln() as f32
        };
        Lex {
            nfrq: pack.get(b"NFRQ"),
            lexo: pack.get(b"LEXO"),
            lexw: pack.get(b"LEXW"),
            lexe: pack.get(b"LEXE"),
            lexr: pack.get(b"LEXR"),
            bigr: pack.get(b"BIGR"),
            enfq: pack.get(b"ENFQ"),
            homo: pack.get(b"HOMO"),
            clm_indic: pack.get(b"CLMI"),
            clm_en: pack.get(b"CLME"),
            ln_total: t("total"),
            ln_en_total: t("en_total"),
        }
    }

    /// ln(count) of a native word in the corpus.
    pub fn native_ln(&self, w: &str) -> Option<f32> {
        lookup::<1>(self.nfrq, hash_bytes(w.as_bytes())).map(|[q]| q as f32 * F_STEP)
    }

    /// ln(count per million) of a native word.
    pub fn native_cpm(&self, w: &str) -> Option<f32> {
        self.native_ln(w).map(|l| l - self.ln_total + LN_MILLION)
    }

    /// ln(count per million) of an English word in the English list.
    pub fn english_cpm(&self, w: &str) -> Option<f32> {
        lookup::<1>(self.enfq, hash_bytes(w.as_bytes()))
            .map(|[q]| q as f32 * F_STEP - self.ln_en_total + LN_MILLION)
    }

    /// Listed native words of a romanised spelling:
    /// (word, ln(1 + attestations), ln(share of the word's spellings)).
    pub fn spellings(&self, roman: &str) -> Vec<(&'a str, f32, f32)> {
        let Some([a, b, c]) = lookup::<3>(self.lexr, hash_bytes(roman.as_bytes())) else {
            return Vec::new();
        };
        let mut at = a as usize | (b as usize) << 8 | (c as usize) << 16;
        let Some(&n) = self.lexe.get(at) else {
            return Vec::new();
        };
        at += 1;
        let offset = |i: usize| -> Option<usize> {
            let b = self.lexo.get(i * 4..i * 4 + 4)?;
            Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
        };
        let mut out = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let Some(e) = self.lexe.get(at..at + 5) else {
                break;
            };
            let id = e[0] as usize | (e[1] as usize) << 8 | (e[2] as usize) << 16;
            let word = match (offset(id), offset(id + 1)) {
                (Some(s), Some(t)) => self
                    .lexw
                    .get(s..t)
                    .and_then(|b| std::str::from_utf8(b).ok())
                    .unwrap_or(""),
                _ => "",
            };
            if !word.is_empty() {
                out.push((word, e[3] as f32 * F_STEP, -(e[4] as f32) * SHARE_STEP));
            }
            at += 5;
        }
        out
    }

    /// For a listed homograph of an English word: the hash of the native
    /// word meant.
    pub fn homograph(&self, roman: &str) -> Option<u32> {
        lookup::<4>(self.homo, hash_bytes(roman.as_bytes())).map(u32::from_le_bytes)
    }

    /// Association of two adjacent native words (pointwise mutual
    /// information, nats), when stored.
    pub fn pmi(&self, a: &str, b: &str) -> Option<f32> {
        lookup::<1>(
            self.bigr,
            hash2(hash_bytes(a.as_bytes()), hash_bytes(b.as_bytes())),
        )
        .map(|[q]| q as i8 as f32 / 10.0)
    }
}
