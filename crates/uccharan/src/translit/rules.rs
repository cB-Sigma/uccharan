//! A plain ITRANS-style rule converter: the baseline, and the fallback when
//! a pack has no model. Lower-case input, so the retroflex and long-vowel
//! capitals of real ITRANS never occur; a final virama is dropped.

/// Devanagari for a lower-case romanised word, read as ITRANS.
pub fn itrans_devanagari(word: &str) -> String {
    const CONS: &[(&str, &str)] = &[
        ("kSh", "क्ष"),
        ("ksh", "क्ष"),
        ("chh", "छ"),
        ("shh", "ष"),
        ("GY", "ज्ञ"),
        ("gy", "ज्ञ"),
        ("dny", "ज्ञ"),
        ("kh", "ख"),
        ("gh", "घ"),
        ("ch", "च"),
        ("jh", "झ"),
        ("th", "थ"),
        ("dh", "ध"),
        ("ph", "फ"),
        ("bh", "भ"),
        ("sh", "श"),
        ("k", "क"),
        ("g", "ग"),
        ("c", "च"),
        ("j", "ज"),
        ("t", "त"),
        ("d", "द"),
        ("n", "न"),
        ("p", "प"),
        ("f", "फ़"),
        ("b", "ब"),
        ("m", "म"),
        ("y", "य"),
        ("r", "र"),
        ("l", "ल"),
        ("v", "व"),
        ("w", "व"),
        ("s", "स"),
        ("h", "ह"),
        ("z", "ज़"),
        ("q", "क़"),
        ("x", "क्ष"),
    ];
    const VOWELS: &[(&str, &str, &str)] = &[
        ("aa", "आ", "ा"),
        ("ai", "ऐ", "ै"),
        ("au", "औ", "ौ"),
        ("ii", "ई", "ी"),
        ("ee", "ई", "ी"),
        ("uu", "ऊ", "ू"),
        ("oo", "ऊ", "ू"),
        ("a", "अ", ""),
        ("i", "इ", "ि"),
        ("u", "उ", "ु"),
        ("e", "ए", "े"),
        ("o", "ओ", "ो"),
    ];
    let w: String = word
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .collect::<String>()
        .to_lowercase();
    let mut out = String::new();
    let mut i = 0;
    let mut pending = false; // a consonant waits for its vowel
    while i < w.len() {
        let rest = &w[i..];
        if let Some((r, d)) = CONS.iter().find(|(r, _)| rest.starts_with(r)) {
            if pending {
                out.push('\u{94d}');
            }
            out.push_str(d);
            pending = true;
            i += r.len();
        } else if let Some((r, indep, sign)) = VOWELS.iter().find(|(r, _, _)| rest.starts_with(r)) {
            out.push_str(if pending { sign } else { indep });
            pending = false;
            i += r.len();
        } else {
            i += 1;
        }
    }
    out
}

/// The ITRANS reading moved to another Brahmic script by code-point offset
/// (`base` is the first code point of the script's block).
pub fn itrans(word: &str, base: u32) -> String {
    let d = itrans_devanagari(word);
    if base == 0x900 {
        return d;
    }
    let tamil = base == 0x0b80;
    let mut out = String::new();
    for c in d.chars() {
        let mut o = c as u32 - 0x900;
        if o == 0x3c && (tamil || base == 0x0c00 || base == 0x0d00) {
            continue; // no nukta
        }
        if tamil {
            // Tamil writes one letter per place of articulation.
            o = match o {
                0x16..=0x18 => 0x15,
                0x1b => 0x1a,
                0x1d => 0x1c,
                0x20..=0x22 => 0x1f,
                0x25..=0x27 => 0x24,
                0x2b..=0x2d => 0x2a,
                0x36 => 0x37,
                x => x,
            };
        }
        out.push(char::from_u32(base + o).unwrap_or(c));
    }
    out
}
