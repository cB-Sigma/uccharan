//! Optional WikiPron packs (`wikipron-en-us.ucd`, `wikipron-en-gb.ucd`,
//! `wikipron-hi.ucd`), built from `data/sources/wikipron` (Wiktionary text,
//! CC BY-SA). The packs are not part of the Apache-2.0 core.
//!
//! WikiPron's broad transcriptions carry no stress, mix several
//! transcription habits, and use plain IPA. Each pronunciation is rewritten
//! into the phoneme symbols uccharan prints for that language, without
//! stress marks. A `.` separates two vowels that belong to different
//! syllables, so that the run-time stress transfer (see
//! `uccharan::wiki`) can count syllables; it is removed on output.

use std::collections::BTreeMap;
use std::path::Path;
use uccharan::data::Builder;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Us,
    Gb,
    Hi,
}

fn strip_ties(seg: &str) -> String {
    seg.replace(['\u{361}', '\u{35c}'], "")
}

const EN_VOWELS: &[&str] = &[
    "a", "e", "i", "o", "u", "æ", "ɑ", "ɒ", "ɔ", "ə", "ɛ", "ɜ", "ɝ", "ɚ", "ɪ", "ʊ", "ʌ", "ɐ", "ʉ",
];

fn is_en_vowel(seg: &str) -> bool {
    let base: String = seg
        .chars()
        .filter(|c| !matches!(c, 'ː' | '\u{32f}' | '\u{303}' | '\u{329}'))
        .collect();
    EN_VOWELS.contains(&base.as_str())
}

/// One English pronunciation (space-separated WikiPron segments) in
/// uccharan's espeak-style symbols for the accent, without stress.
/// `None` when a segment is outside what the converter knows.
pub fn english(pron: &str, kind: Kind) -> Option<String> {
    let us = kind == Kind::Us;
    let segs: Vec<String> = pron.split_whitespace().map(strip_ties).collect();
    let mut out: Vec<(String, bool)> = Vec::new(); // (symbol, is vowel nucleus)
    let mut i = 0;
    while i < segs.len() {
        let s = segs[i].as_str();
        let next = segs.get(i + 1).map(String::as_str);
        let off = |n: Option<&str>, set: &[&str]| n.is_some_and(|x| set.contains(&x));
        let iy = ["ɪ", "ɪ̯", "i", "i̯", "j"];
        let uw = ["ʊ", "ʊ̯", "u", "u̯"];
        let schwa_off = ["ə", "ə̯", "ɐ"];
        // Diphthongs written as two segments.
        let diph: Option<&str> = match s {
            "a" | "ɑ" | "ʌ" | "aː" if off(next, &iy[..4]) => Some("aɪ"),
            "a" | "æ" | "ɑ" | "aː" if off(next, &uw) => Some("aʊ"),
            "e" | "ɛ" if off(next, &iy[..4]) => Some("eɪ"),
            "o" | "əʊ" if off(next, &uw) => Some(if us { "oʊ" } else { "əʊ" }),
            "ə" | "ɘ" | "ɵ" if off(next, &uw) => Some(if us { "oʊ" } else { "əʊ" }),
            "ɔ" | "ɔː" | "o" if off(next, &iy[..4]) => Some("ɔɪ"),
            "ɪ" | "i" if !us && off(next, &schwa_off) => Some("ɪə"),
            "ɛ" | "e" if !us && off(next, &schwa_off) => Some("eə"),
            "ʊ" | "u" if !us && off(next, &schwa_off) => Some("ʊə"),
            _ => None,
        };
        if let Some(d) = diph {
            out.push((d.to_string(), true));
            i += 2;
            continue;
        }
        let v: Option<&str> = match s {
            "i" => Some("i"), // length decided after stress
            "iː" | "ɪː" => Some("iː"),
            "ɪ" | "ɪ̯" | "ɨ" => Some("ɪ"),
            "e" | "eː" => Some(if us { "eɪ" } else { "ɛ" }),
            "ɛ" => Some("ɛ"),
            "ɛː" => Some(if us { "ɛ" } else { "eə" }),
            "æ" | "æː" => Some("æ"),
            "a" => Some(if us { "ɑː" } else { "æ" }),
            "aː" | "ɑ" | "ɑː" => Some("ɑː"),
            "ɒ" | "ɒː" => Some(if us { "ɑː" } else { "ɒ" }),
            "ɔ" | "ɔː" => Some("ɔː"),
            "o" | "oː" => Some(if us { "oʊ" } else { "əʊ" }),
            "ʌ" => Some("ʌ"),
            "ɐ" => Some(if us { "ə" } else { "ʌ" }),
            "ə" | "ə̯" | "ɘ" => Some("ə"),
            "ɚ" => Some(if us { "ɚ" } else { "ə" }),
            "ɝ" | "ɝː" | "ɜ" | "ɜː" | "əː" => Some("ɜː"),
            "ʊ" | "ʊ̯" | "ɵ" => Some("ʊ"),
            "u" | "uː" | "ʉ" | "ʉː" | "u̯" => Some("uː"),
            "ɛ̃" | "æ̃" => Some("ɑːn"),
            "ɑ̃" | "ɒ̃" | "ɔ̃" => Some("ɒn"),
            "œ" | "ø" | "øː" => Some("ɜː"),
            "y" | "yː" => Some("uː"),
            _ => None,
        };
        if let Some(v) = v {
            let v = if v == "ɒn" && us { "ɑːn" } else { v };
            if let Some(stem) = v.strip_suffix('n') {
                out.push((stem.to_string(), true));
                out.push(("n".to_string(), false));
            } else {
                out.push((v.to_string(), true));
            }
            i += 1;
            continue;
        }
        let c: &str = match s {
            "p" | "b" | "t" | "d" | "k" | "ɡ" | "f" | "v" | "θ" | "ð" | "s" | "z" | "ʃ" | "ʒ"
            | "h" | "m" | "n" | "ŋ" | "l" | "ɹ" | "j" | "w" | "tʃ" | "dʒ" => s,
            "g" => "ɡ",
            "r" | "ɾ" if !us => "ɹ",
            "r" | "ɻ" => "ɹ",
            "ɾ" => "ɾ",
            "ɫ" | "ɬ" => "l",
            "ʍ" => "w",
            "x" | "χ" => "k",
            "ʔ" => "t",
            "ts" => "ts",
            "dz" => "dz",
            "ɲ" => "nj",
            "ç" => "h",
            "ʎ" => "lj",
            "l̩" | "ɫ̩" => "əl",
            "n̩" => "ən",
            "m̩" => "əm",
            "ɹ̩" => {
                if us {
                    "ɚ"
                } else {
                    "ə"
                }
            }
            _ => return None,
        };
        match c {
            "əl" | "ən" | "əm" => {
                out.push(("ə".into(), true));
                out.push((c[2..].to_string(), false));
            }
            "ɚ" | "ə" => out.push((c.to_string(), true)),
            _ => out.push((c.to_string(), false)),
        }
        i += 1;
    }
    if !out.iter().any(|x| x.1) {
        return None;
    }
    // British non-rhotic: r only before a vowel.
    if !us {
        let mut k = 0;
        while k < out.len() {
            if out[k].0 == "ɹ" && !out.get(k + 1).is_some_and(|x| x.1) {
                // "ɑː" + r, "ɔː" + r keep the long vowel; short vowels lengthen.
                if k > 0 && out[k - 1].1 {
                    let p = &mut out[k - 1].0;
                    *p = match p.as_str() {
                        "ɪ" => "ɪə".into(),
                        "ɛ" | "e" => "eə".into(),
                        "ʊ" => "ʊə".into(),
                        "ɒ" | "ɔ" => "ɔː".into(),
                        "æ" | "a" => "ɑː".into(),
                        other => other.to_string(),
                    };
                }
                out.remove(k);
                continue;
            }
            k += 1;
        }
    }
    let mut s = String::new();
    for (k, (sym, vowel)) in out.iter().enumerate() {
        if *vowel && k > 0 && out[k - 1].1 {
            s.push('.');
        }
        s.push_str(sym);
    }
    let _ = is_en_vowel;
    Some(s)
}

/// One Hindi pronunciation in the improved-mode Hindi symbols, without stress.
pub fn hindi(pron: &str) -> Option<String> {
    let mut out: Vec<(String, bool)> = Vec::new();
    for seg in pron.split_whitespace() {
        let s = strip_ties(seg);
        // Aspiration (breathy voice on voiced stops) is written ʰ.
        let s = s.replace('ʱ', "ʰ").replace('\u{32a}', "");
        let (sym, vowel): (String, bool) = match s.as_str() {
            "ə" | "ə̯" => ("ə".into(), true),
            "ə̃" => ("ə̃".into(), true),
            "ɑː" | "aː" => ("aː".into(), true),
            "ɑ̃ː" | "ɑ̃" => ("ã".into(), true),
            "ɪ" | "i" => ("ɪ".into(), true),
            "ɪ̃" | "ĩ" | "ĩː" => ("ĩ".into(), true),
            "iː" => ("iː".into(), true),
            "ʊ" | "u" => ("ʊ".into(), true),
            "ʊ̃" | "ũ" | "ũː" => ("ũ".into(), true),
            "uː" => ("uː".into(), true),
            "eː" | "e" => ("eː".into(), true),
            "ẽː" | "ẽ" => ("ẽː".into(), true),
            "ɛː" | "ɛ" => ("ɛː".into(), true),
            "ɛ̃ː" | "ɛ̃" => ("ɛ̃".into(), true),
            "oː" | "o" => ("oː".into(), true),
            "õː" | "õ" => ("õ".into(), true),
            "ɔː" | "ɔ" => ("ɔː".into(), true),
            "ɔ̃ː" | "ɔ̃" => ("ɔ̃".into(), true),
            "dʒ" => ("ɟ".into(), false),
            "dʒʰ" => ("ɟʰ".into(), false),
            "tʃ" => ("c".into(), false),
            "tʃʰ" => ("cʰ".into(), false),
            "ɦ" | "h" => ("h".into(), false),
            "ɽʰ" => ("ɽʰ".into(), false),
            "g" => ("ɡ".into(), false),
            "gʰ" => ("ɡʰ".into(), false),
            "w" => ("ʋ".into(), false),
            "r" => ("ɾ".into(), false),
            x if x.ends_with('ᵊ') => {
                let c = x.trim_end_matches('ᵊ').to_string();
                out.push((c, false));
                ("ə".into(), true)
            }
            x => {
                let base = x.trim_end_matches(['ʰ', 'ː']);
                let ok = matches!(
                    base,
                    "p" | "b"
                        | "t"
                        | "d"
                        | "ʈ"
                        | "ɖ"
                        | "k"
                        | "ɡ"
                        | "q"
                        | "m"
                        | "n"
                        | "ɳ"
                        | "ŋ"
                        | "ɲ"
                        | "s"
                        | "ʃ"
                        | "ʂ"
                        | "z"
                        | "ʒ"
                        | "f"
                        | "x"
                        | "ɣ"
                        | "ɾ"
                        | "l"
                        | "ɭ"
                        | "ʋ"
                        | "j"
                        | "ɽ"
                );
                if !ok {
                    return None;
                }
                (x.to_string(), false)
            }
        };
        out.push((sym, vowel));
    }
    if !out.iter().any(|x| x.1) {
        return None;
    }
    let mut s = String::new();
    for (k, (sym, vowel)) in out.iter().enumerate() {
        if *vowel && k > 0 && out[k - 1].1 {
            s.push('.');
        }
        s.push_str(sym);
    }
    Some(s)
}

fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a
        .chars()
        .filter(|c| !matches!(c, 'ˈ' | 'ˌ' | '.'))
        .collect();
    let b: Vec<char> = b
        .chars()
        .filter(|c| !matches!(c, 'ˈ' | 'ˌ' | '.'))
        .collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, x) in a.iter().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, y) in b.iter().enumerate() {
            cur[j + 1] = (prev[j] + usize::from(x != y))
                .min(prev[j + 1] + 1)
                .min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Written final short i / u stay short (the Hindi convention uccharan
/// prints); Wiktionary writes them long.
fn hindi_final(word: &str, conv: String) -> String {
    if word.ends_with('\u{93f}') {
        if let Some(s) = conv.strip_suffix("iː") {
            return format!("{s}ɪ");
        }
    }
    if word.ends_with('\u{941}') {
        if let Some(s) = conv.strip_suffix("uː") {
            return format!("{s}ʊ");
        }
    }
    conv
}

/// Build one pack. Returns the image and (entries, skipped).
///
/// Where Wiktionary lists several pronunciations, the one closest to
/// uccharan's own improved-mode reading is kept (Wiktionary often lists a
/// colloquial variant first, for example ज़ read as j).
pub fn build(root: &Path, kind: Kind) -> (Vec<u8>, usize, usize) {
    let (file, lang) = match kind {
        Kind::Us => ("eng_latn_us_broad.tsv", "en-us"),
        Kind::Gb => ("eng_latn_uk_broad.tsv", "en-gb"),
        Kind::Hi => ("hin_deva_broad.tsv", "hi"),
    };
    let text = std::fs::read_to_string(root.join("data/sources/wikipron").join(file))
        .expect("WikiPron source file");
    let en_data = match kind {
        Kind::Us => Some("en-us.ucd"),
        Kind::Gb => Some("en-gb.ucd"),
        Kind::Hi => None,
    }
    .map(|f| {
        uccharan::data::Data::from_vec(
            std::fs::read(root.join("crates/uccharan/data").join(f)).expect("core data"),
        )
        .expect("core data")
    });
    let ulang = match kind {
        Kind::Us => uccharan::Lang::EnUs,
        Kind::Gb => uccharan::Lang::EnGb,
        Kind::Hi => uccharan::Lang::Hi,
    };
    let own = |word: &str| -> String {
        match &en_data {
            Some(d) => uccharan::en::English::new(d, ulang)
                .word(&word.to_lowercase(), uccharan::en::Hint::None),
            None => uccharan::hi::Hindi::new(None).word(word, false),
        }
    };
    let mut map: BTreeMap<String, String> = BTreeMap::new();
    let mut best: BTreeMap<String, usize> = BTreeMap::new();
    let mut skipped = 0usize;
    for line in text.lines() {
        let Some((word, pron)) = line.split_once('\t') else {
            continue;
        };
        if word.contains(' ') || word.starts_with('-') || word.ends_with('-') {
            continue;
        }
        let conv = match kind {
            Kind::Hi => hindi(pron).map(|c| hindi_final(word, c)),
            k => english(pron, k),
        };
        let Some(conv) = conv else {
            skipped += 1;
            continue;
        };
        let key = if kind == Kind::Hi {
            word.to_string()
        } else {
            let lower = word.to_lowercase();
            let capital = word.chars().next().is_some_and(char::is_uppercase);
            // Capitalised spellings (mostly names) go under "^name".
            if capital {
                format!("^{lower}")
            } else {
                lower
            }
        };
        let d = distance(&conv, &own(word));
        if best.get(&key).is_none_or(|&b| d < b) {
            best.insert(key.clone(), d);
            map.insert(key, conv);
        }
    }
    let mut b = Builder::new();
    for (k, v) in &map {
        b.entry(k, v);
    }
    b.meta("lang", &format!("wikipron-{lang}"))
        .meta("license", "CC-BY-SA-4.0")
        .meta(
            "source",
            "Wiktionary contributors via WikiPron d282e848 (data/sources/wikipron)",
        )
        .meta("format", "unstressed; '.' separates syllable nuclei");
    (b.build().expect("build"), map.len(), skipped)
}
