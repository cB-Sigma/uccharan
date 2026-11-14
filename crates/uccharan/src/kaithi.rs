//! Kaithi script (U+11080–U+110CF), the historical script of Bhojpuri,
//! Maithili and Magahi, transliterated letter for letter to Devanagari
//! before the text is read. Kaithi has the same structure as Devanagari
//! (inherent vowel, vowel signs, virama), so the mapping is one to one; the
//! letters DDDHA and RHA are the flaps written ड़ and ढ़ in Devanagari.

/// Devanagari for one Kaithi code point (`None` outside the block or for
/// signs without an equivalent).
fn map(c: char) -> Option<&'static str> {
    const T: [&str; 0x43] = [
        "ँ", "ं", "ः", "अ", "आ", "इ", "ई", "उ", "ऊ", "ए", "ऐ", "ओ", "औ", // 11080–1108C
        "क", "ख", "ग", "घ", "ङ", "च", "छ", "ज", "झ", "ञ", "ट", "ठ", "ड", "ड़", "ढ", "ढ़",
        "ण", // 1108D–1109D
        "त", "थ", "द", "ध", "न", "प", "फ", "ब", "भ", "म", "य", "र", "ल", "व", "श", "ष", "स",
        "ह", // 1109E–110AF
        "ा", "ि", "ी", "ु", "ू", "े", "ै", "ो", "ौ", "्", "़", // 110B0–110BA
        "", "", "", "", "", "।", "॥",
        "ृ", // 110BB–110C2 (abbreviation, enumeration, number, section signs dropped)
    ];
    let u = c as u32;
    (0x11080..0x11080 + T.len() as u32)
        .contains(&u)
        .then(|| T[(u - 0x11080) as usize])
}

/// True when the text holds any Kaithi letter or sign.
pub fn has_kaithi(text: &str) -> bool {
    text.chars()
        .any(|c| ('\u{11080}'..='\u{110CF}').contains(&c))
}

/// The text with Kaithi letters written in Devanagari.
pub fn to_devanagari(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match map(c) {
            Some(d) => out.push_str(d),
            None if ('\u{11080}'..='\u{110CF}').contains(&c) => {}
            None => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transliterates() {
        // Kaithi "bhojapurī": BHA O JA PA U RA II.
        let k = "\u{110A6}\u{110B7}\u{11094}\u{110A3}\u{110B3}\u{110A9}\u{110B2}";
        assert!(has_kaithi(k));
        assert_eq!(to_devanagari(k), "भोजपुरी");
        assert_eq!(to_devanagari("\u{1109A}"), "ड़");
        assert_eq!(to_devanagari("abc"), "abc");
    }
}
