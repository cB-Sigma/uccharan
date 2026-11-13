//! CMUdict reader and aligned-entry preparation.

use crate::align::Aligner;

pub const PHONES: [&str; 39] = [
    "AA", "AE", "AH", "AO", "AW", "AY", "B", "CH", "D", "DH", "EH", "ER", "EY", "F", "G", "HH",
    "IH", "IY", "JH", "K", "L", "M", "N", "NG", "OW", "OY", "P", "R", "S", "SH", "T", "TH", "UH",
    "UW", "V", "W", "Y", "Z", "ZH",
];

/// One phoneme of an aligned entry.
#[derive(Clone, Debug)]
pub struct Seg {
    /// ARPABET symbol without the stress digit.
    pub p: &'static str,
    /// 0 none, 1 primary, 2 secondary; 9 for consonants.
    pub stress: u8,
    /// The letters this phoneme is spelled with ("" when unaligned).
    pub g: String,
    /// Index of the emitting letter.
    pub li: usize,
}

impl Seg {
    pub fn is_vowel(&self) -> bool {
        self.stress != 9
    }
}

pub struct Entry {
    pub word: String,
    pub segs: Vec<Seg>,
    pub aligned: bool,
    /// Variant number: 1 for the first pronunciation.
    pub variant: u8,
}

fn phone_id(p: &str) -> Option<u8> {
    PHONES.iter().position(|x| *x == p).map(|i| i as u8 + 1)
}

pub fn letters_of(word: &str) -> Vec<u8> {
    word.bytes().filter(|b| b.is_ascii_lowercase()).collect()
}

pub fn load(text: &str) -> Vec<Entry> {
    let mut raw: Vec<(String, u8, Vec<(u8, u8)>)> = Vec::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let mut it = line.split_whitespace();
        let Some(head) = it.next() else { continue };
        let (word, variant) = match head.split_once('(') {
            Some((w, v)) => (w, v.trim_end_matches(')').parse::<u8>().unwrap_or(2)),
            None => (head, 1),
        };
        let mut ph = Vec::new();
        let mut ok = true;
        for p in it {
            let (base, stress) = match p.as_bytes().last() {
                Some(d @ b'0'..=b'2') => (&p[..p.len() - 1], d - b'0'),
                _ => (p, 9),
            };
            match phone_id(base) {
                Some(id) => ph.push((id, stress)),
                None => ok = false,
            }
        }
        if ok
            && !ph.is_empty()
            && word
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b == b'\'' || b == b'-' || b == b'.')
        {
            raw.push((word.to_string(), variant, ph));
        }
    }
    let train: Vec<(Vec<u8>, Vec<u8>)> = raw
        .iter()
        .map(|(w, _, ph)| (letters_of(w), ph.iter().map(|x| x.0).collect()))
        .collect();
    let is_vowel_phone = |p: u8| {
        matches!(
            PHONES[p as usize - 1],
            "AA" | "AE"
                | "AH"
                | "AO"
                | "AW"
                | "AY"
                | "EH"
                | "ER"
                | "EY"
                | "IH"
                | "IY"
                | "OW"
                | "OY"
                | "UH"
                | "UW"
        )
    };
    let aligner = Aligner::train(&train, 8, |letter, phone| {
        let vowel_letter = matches!(letter, b'a' | b'e' | b'i' | b'o' | b'u' | b'y');
        match (vowel_letter, is_vowel_phone(phone)) {
            (true, true) | (false, false) => true,
            // u → W (quick), i/y → Y (onion), o → W (one), u → Y UW (use)
            (true, false) => matches!(PHONES[phone as usize - 1], "W" | "Y"),
            // syllabic consonants and vowel-less spellings: rhythm, little, mcdonald, w as a vowel
            (false, true) => matches!(letter, b'l' | b'm' | b'n' | b'w' | b'h'),
        }
    });
    let mut out = Vec::with_capacity(raw.len());
    for ((word, variant, ph), (ls, ps)) in raw.into_iter().zip(train) {
        let al = aligner.align(&ls, &ps);
        let mut segs: Vec<Seg> = Vec::with_capacity(ph.len());
        match &al {
            Some(al) => {
                let mut j = 0;
                let mut pending = String::new();
                for (i, &k) in al.iter().enumerate() {
                    if k == 0 {
                        // A silent vowel letter right before a letter that spells a vowel
                        // belongs to that vowel ("oo", "ea", "er"); other silent letters
                        // belong to the phoneme before them ("ck", "tt", final "e").
                        let vowel_letter =
                            |c: u8| matches!(c, b'a' | b'e' | b'i' | b'o' | b'u' | b'y');
                        let forward =
                            vowel_letter(ls[i]) && al.get(i + 1).is_some_and(|&k2| k2 > 0) && {
                                let j2 = j; // next phoneme to be emitted
                                ph.get(j2).is_some_and(|x| x.1 != 9)
                            };
                        match segs.last_mut() {
                            Some(s) if !forward && pending.is_empty() => s.g.push(ls[i] as char),
                            _ => pending.push(ls[i] as char),
                        }
                        continue;
                    }
                    for _ in 0..k {
                        let (id, stress) = ph[j];
                        let mut g = std::mem::take(&mut pending);
                        g.push(ls[i] as char);
                        segs.push(Seg {
                            p: PHONES[id as usize - 1],
                            stress,
                            g,
                            li: i,
                        });
                        j += 1;
                    }
                }
            }
            None => {
                for (id, stress) in &ph {
                    segs.push(Seg {
                        p: PHONES[*id as usize - 1],
                        stress: *stress,
                        g: String::new(),
                        li: 0,
                    });
                }
            }
        }
        // A letter that emitted two phonemes (x → K S) spells both of them.
        for i in 1..segs.len() {
            if segs[i].li == segs[i - 1].li && !segs[i].g.is_empty() {
                let longer = if segs[i].g.len() >= segs[i - 1].g.len() {
                    segs[i].g.clone()
                } else {
                    segs[i - 1].g.clone()
                };
                segs[i].g = longer.clone();
                segs[i - 1].g = longer;
            }
        }
        out.push(Entry {
            word,
            segs,
            aligned: al.is_some(),
            variant,
        });
    }
    out
}
