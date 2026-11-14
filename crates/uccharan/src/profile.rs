//! Output profiles: pure rendering steps from analysed sentences to the
//! token streams each voice family was trained on.
//!
//! | Profile | Shape |
//! |---|---|
//! | [`Profile::EspeakIpa`] | one IPA string, words separated by spaces, punctuation optional |
//! | [`Profile::Piper`] | one NFD string per sentence with clause terminators; ids via a voice's `phoneme_id_map` |
//! | [`Profile::SherpaOnnx`] | one list of Unicode codepoints per sentence (`GenerationConfig.phoneme_codepoints`) |
//! | [`Profile::Kokoro`] | Kokoro token string (misaki symbols) and ids against the Kokoro-82M vocabulary |
//! | [`Profile::Kitten`], [`Profile::Matcha`] | StyleTTS2-style symbol ids with each model's framing |
//!
//! The definitions are documented, with sources, in `docs/profiles.md`.

use crate::{Clause, Lang, Sentence};

/// An output profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    /// Plain espeak-style IPA.
    EspeakIpa,
    /// Piper voices with `phoneme_type: espeak`.
    Piper,
    /// Kokoro-82M.
    Kokoro,
    /// Kitten TTS 0.8 ONNX models.
    Kitten,
    /// Matcha-TTS.
    Matcha,
    /// sherpa-onnx ≥ 2.0 `phoneme_codepoints`.
    SherpaOnnx,
}

impl Profile {
    /// Parse a profile name.
    pub fn parse(s: &str) -> Option<Profile> {
        match s.to_ascii_lowercase().replace('_', "-").as_str() {
            "espeak-ipa" | "espeak" | "ipa" => Some(Profile::EspeakIpa),
            "piper" => Some(Profile::Piper),
            "kokoro" => Some(Profile::Kokoro),
            "kitten" | "kittentts" | "kitten-tts" => Some(Profile::Kitten),
            "matcha" | "matcha-tts" => Some(Profile::Matcha),
            "sherpa-onnx" | "sherpa" | "sherpaonnx" => Some(Profile::SherpaOnnx),
            _ => None,
        }
    }

    /// Canonical name.
    pub fn name(self) -> &'static str {
        match self {
            Profile::EspeakIpa => "espeak-ipa",
            Profile::Piper => "piper",
            Profile::Kokoro => "kokoro",
            Profile::Kitten => "kitten",
            Profile::Matcha => "matcha",
            Profile::SherpaOnnx => "sherpa-onnx",
        }
    }

    /// All profiles.
    pub const ALL: [Profile; 6] = [
        Profile::EspeakIpa,
        Profile::Piper,
        Profile::Kokoro,
        Profile::Kitten,
        Profile::Matcha,
        Profile::SherpaOnnx,
    ];
}

fn clause_words(c: &Clause, out: &mut String, flags: Option<Lang>) {
    let mut first = true;
    let mut foreign = false;
    for w in &c.words {
        if !first && !w.glue {
            out.push(' ');
        }
        first = false;
        if let Some(base) = flags {
            let is_foreign = w.lang.flag() != base.flag();
            if is_foreign && !foreign {
                out.push('(');
                out.push_str(w.lang.flag());
                out.push(')');
            } else if !is_foreign && foreign {
                // The switch-back flag belongs to the previous word.
                let at = out.rfind(' ').unwrap_or(out.len());
                out.insert_str(at, &format!("({})", base.tag()));
            }
            foreign = is_foreign;
        }
        out.push_str(&w.ipa);
    }
    if foreign {
        if let Some(base) = flags {
            out.push_str(&format!("({})", base.tag()));
        }
    }
}

/// The "phonemizer shape": one string, words separated by single spaces.
/// With `punctuation`, each clause terminator follows its clause directly.
/// With `flags`, words from another language's rules are bracketed with
/// `(en)`…`(hi)`-style language-switch markers relative to that base language.
pub fn espeak_ipa(sentences: &[Sentence], punctuation: bool, flags: Option<Lang>) -> String {
    let mut out = String::new();
    for s in sentences {
        for c in &s.clauses {
            if !out.is_empty() {
                out.push(' ');
            }
            clause_words(c, &mut out, flags);
            if punctuation {
                if let Some(t) = c.terminator.as_char() {
                    out.push(t);
                }
            }
        }
    }
    out
}

/// Canonical decomposition for the handful of precomposed letters that can
/// occur in the notation.
pub fn nfd(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for c in s.chars() {
        match c {
            'ã' => out.push_str("a\u{303}"),
            'ẽ' => out.push_str("e\u{303}"),
            'ĩ' => out.push_str("i\u{303}"),
            'õ' => out.push_str("o\u{303}"),
            'ũ' => out.push_str("u\u{303}"),
            'ñ' => out.push_str("n\u{303}"),
            'ç' => out.push_str("c\u{327}"),
            'g' => out.push('ɡ'),
            c => out.push(c),
        }
    }
    out
}

/// The "Piper shape": one NFD string per sentence. Clause terminators are
/// kept, with a space after `,` `:` `;`. Language-switch flags never appear.
pub fn piper_sentences(sentences: &[Sentence]) -> Vec<String> {
    let mut out = Vec::with_capacity(sentences.len());
    for s in sentences {
        let mut line = String::new();
        let n = s.clauses.len();
        for (i, c) in s.clauses.iter().enumerate() {
            clause_words(c, &mut line, None);
            if let Some(t) = c.terminator.as_char() {
                line.push(t);
            }
            if i + 1 < n {
                line.push(' ');
            }
        }
        out.push(nfd(&line));
    }
    out
}

/// One list of Unicode codepoints per sentence, for sherpa-onnx ≥ 2.0
/// `GenerationConfig.phoneme_codepoints`. sherpa-onnx adds the model's own
/// BOS/EOS/padding.
pub fn sherpa_codepoints(sentences: &[Sentence]) -> Vec<Vec<u32>> {
    piper_sentences(sentences)
        .iter()
        .map(|s| s.chars().map(|c| c as u32).collect())
        .collect()
}

/// A Piper voice's `phoneme_id_map`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PhonemeIdMap {
    entries: Vec<(String, Vec<i64>)>,
}

impl PhonemeIdMap {
    /// Read the `phoneme_id_map` object from a Piper voice config
    /// (`*.onnx.json`). Accepts the whole config or just the object.
    pub fn from_json(json: &str) -> Option<PhonemeIdMap> {
        let body = match json.find("\"phoneme_id_map\"") {
            Some(i) => &json[i + "\"phoneme_id_map\"".len()..],
            None => json,
        };
        let start = body.find('{')?;
        let cs: Vec<char> = body[start + 1..].chars().collect();
        let mut entries = Vec::new();
        let mut i = 0;
        loop {
            while i < cs.len() && (cs[i].is_whitespace() || cs[i] == ',') {
                i += 1;
            }
            if i >= cs.len() || cs[i] == '}' {
                break;
            }
            if cs[i] != '"' {
                return None;
            }
            i += 1;
            let mut key = String::new();
            while i < cs.len() && cs[i] != '"' {
                if cs[i] == '\\' && i + 1 < cs.len() {
                    i += 1;
                    match cs[i] {
                        'u' if i + 4 < cs.len() => {
                            let hex: String = cs[i + 1..i + 5].iter().collect();
                            key.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                            i += 4;
                        }
                        'n' => key.push('\n'),
                        't' => key.push('\t'),
                        c => key.push(c),
                    }
                } else {
                    key.push(cs[i]);
                }
                i += 1;
            }
            i += 1;
            while i < cs.len() && cs[i] != '[' {
                i += 1;
            }
            i += 1;
            let mut ids = Vec::new();
            let mut num = String::new();
            while i < cs.len() && cs[i] != ']' {
                if cs[i].is_ascii_digit() || cs[i] == '-' {
                    num.push(cs[i]);
                } else if !num.is_empty() {
                    ids.push(num.parse().ok()?);
                    num.clear();
                }
                i += 1;
            }
            if !num.is_empty() {
                ids.push(num.parse().ok()?);
            }
            i += 1;
            entries.push((key, ids));
        }
        if entries.is_empty() {
            None
        } else {
            Some(PhonemeIdMap { entries })
        }
    }

    /// Ids of one symbol.
    pub fn get(&self, symbol: &str) -> Option<&[i64]> {
        self.entries
            .iter()
            .find(|e| e.0 == symbol)
            .map(|e| e.1.as_slice())
    }

    /// Number of symbols.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Phoneme ids for one sentence string in Piper's framing:
    /// `[BOS, PAD, p1, PAD, p2, PAD, …, EOS]`. Returns the ids and the symbols
    /// that are not in the map (which Piper would skip).
    pub fn ids(&self, sentence: &str) -> (Vec<i64>, Vec<char>) {
        let pad = self.get("_").unwrap_or(&[0]).to_vec();
        let mut ids: Vec<i64> = Vec::with_capacity(sentence.len() * 2 + 4);
        let mut missing = Vec::new();
        if let Some(b) = self.get("^") {
            ids.extend_from_slice(b);
            ids.extend_from_slice(&pad);
        }
        let mut buf = [0u8; 4];
        for c in nfd(sentence).chars() {
            match self.get(c.encode_utf8(&mut buf)) {
                Some(v) => {
                    ids.extend_from_slice(v);
                    ids.extend_from_slice(&pad);
                }
                None => missing.push(c),
            }
        }
        if let Some(e) = self.get("$") {
            ids.extend_from_slice(e);
        }
        (ids, missing)
    }
}

/// Kokoro-82M vocabulary (`config.json`, Apache-2.0): symbol → id.
pub const KOKORO_VOCAB: &[(char, u16)] = &[
    (';', 1),
    (':', 2),
    (',', 3),
    ('.', 4),
    ('!', 5),
    ('?', 6),
    ('—', 9),
    ('…', 10),
    ('"', 11),
    ('(', 12),
    (')', 13),
    ('“', 14),
    ('”', 15),
    (' ', 16),
    ('\u{303}', 17),
    ('ʣ', 18),
    ('ʥ', 19),
    ('ʦ', 20),
    ('ʨ', 21),
    ('ᵝ', 22),
    ('ꭧ', 23),
    ('A', 24),
    ('I', 25),
    ('O', 31),
    ('Q', 33),
    ('S', 35),
    ('T', 36),
    ('W', 39),
    ('Y', 41),
    ('ᵊ', 42),
    ('a', 43),
    ('b', 44),
    ('c', 45),
    ('d', 46),
    ('e', 47),
    ('f', 48),
    ('h', 50),
    ('i', 51),
    ('j', 52),
    ('k', 53),
    ('l', 54),
    ('m', 55),
    ('n', 56),
    ('o', 57),
    ('p', 58),
    ('q', 59),
    ('r', 60),
    ('s', 61),
    ('t', 62),
    ('u', 63),
    ('v', 64),
    ('w', 65),
    ('x', 66),
    ('y', 67),
    ('z', 68),
    ('ɑ', 69),
    ('ɐ', 70),
    ('ɒ', 71),
    ('æ', 72),
    ('β', 75),
    ('ɔ', 76),
    ('ɕ', 77),
    ('ç', 78),
    ('ɖ', 80),
    ('ð', 81),
    ('ʤ', 82),
    ('ə', 83),
    ('ɚ', 85),
    ('ɛ', 86),
    ('ɜ', 87),
    ('ɟ', 90),
    ('ɡ', 92),
    ('ɥ', 99),
    ('ɨ', 101),
    ('ɪ', 102),
    ('ʝ', 103),
    ('ɯ', 110),
    ('ɰ', 111),
    ('ŋ', 112),
    ('ɳ', 113),
    ('ɲ', 114),
    ('ɴ', 115),
    ('ø', 116),
    ('ɸ', 118),
    ('θ', 119),
    ('œ', 120),
    ('ɹ', 123),
    ('ɾ', 125),
    ('ɻ', 126),
    ('ʁ', 128),
    ('ɽ', 129),
    ('ʂ', 130),
    ('ʃ', 131),
    ('ʈ', 132),
    ('ʧ', 133),
    ('ʊ', 135),
    ('ʋ', 136),
    ('ʌ', 138),
    ('ɣ', 139),
    ('ɤ', 140),
    ('χ', 142),
    ('ʎ', 143),
    ('ʒ', 147),
    ('ʔ', 148),
    ('ˈ', 156),
    ('ˌ', 157),
    ('ː', 158),
    ('ʰ', 162),
    ('ʲ', 164),
    ('↓', 169),
    ('→', 171),
    ('↗', 172),
    ('↘', 173),
    ('ᵻ', 177),
];

/// English in espeak-style notation → Kokoro (misaki) symbols.
fn kokoro_english(ipa: &str, gb: bool) -> String {
    let mut s = ipa.replace("ʔn\u{329}", "tᵊn");
    // Syllabic consonants: a small schwa before the consonant.
    while let Some(i) = s.find('\u{329}') {
        let start = s[..i].char_indices().next_back().map_or(0, |x| x.0);
        let cons: String = s[start..i].to_string();
        s.replace_range(start..i + '\u{329}'.len_utf8(), &format!("ᵊ{cons}"));
    }
    // The syllabic l ("little", "apple"): a schwa + l at the end of a word or before a consonant.
    let cs: Vec<char> = s.chars().collect();
    let mut t = String::with_capacity(s.len());
    for (i, &c) in cs.iter().enumerate() {
        let syllabic = c == 'ə'
            && cs.get(i + 1) == Some(&'l')
            && cs
                .get(i + 2)
                .is_none_or(|n| !crate::ipa::is_vowel(*n) && !crate::ipa::is_stress(*n))
            && i > 0;
        t.push(if syllabic { 'ᵊ' } else { c });
    }
    s = t;
    for (a, b) in [
        ("aɪ", "I"),
        ("aʊ", "W"),
        ("dʒ", "ʤ"),
        ("eɪ", "A"),
        ("tʃ", "ʧ"),
        ("ɔɪ", "Y"),
        ("ɚɹ", "əɹ"),
        ("ɚ", "əɹ"),
        ("ɐ", "ə"),
        ("ʔ", "t"),
    ] {
        s = s.replace(a, b);
    }
    if gb {
        s = s
            .replace("eə", "ɛː")
            .replace("iə", "ɪə")
            .replace("əʊ", "Q")
            .replace('æ', "a");
    } else {
        s = s
            .replace("oʊ", "O")
            .replace("ɜːɹ", "ɜɹ")
            .replace("ɜː", "ɜɹ")
            .replace("ɪə", "iə")
            .replace('ː', "")
            .replace('ɾ', "T");
    }
    s
}

/// Hindi (and embedded English) in espeak-style notation → Kokoro symbols:
/// the published espeak → Kokoro mapping for non-English languages.
fn kokoro_espeak(ipa: &str, english_word: bool) -> String {
    let mut s = ipa.to_string();
    if english_word {
        for (a, b) in [
            ("aɪ", "I"),
            ("aʊ", "W"),
            ("dʒ", "ʤ"),
            ("eɪ", "A"),
            ("oʊ", "O"),
            ("əʊ", "Q"),
            ("tʃ", "ʧ"),
            ("ɔɪ", "Y"),
        ] {
            s = s.replace(a, b);
        }
    }
    s.replace('-', "")
}

/// Kokoro token string for the analysed text. `lang` is the request language.
pub fn kokoro_tokens(sentences: &[Sentence], lang: Lang) -> String {
    let mut out = String::new();
    for s in sentences {
        for c in &s.clauses {
            if !out.is_empty() {
                out.push(' ');
            }
            let mut first = true;
            for w in &c.words {
                if !first && !w.glue {
                    out.push(' ');
                }
                first = false;
                let t = if lang.is_english() && w.lang.is_english() {
                    kokoro_english(&w.ipa, lang == Lang::EnGb)
                } else {
                    kokoro_espeak(&w.ipa, w.lang.is_english())
                };
                out.push_str(&t);
            }
            if let Some(t) = c.terminator.as_char() {
                out.push(t);
            }
        }
    }
    // Anything outside the vocabulary is dropped, as the model's tokenizer does.
    nfd(&out)
        .chars()
        .filter(|c| KOKORO_VOCAB.iter().any(|v| v.0 == *c))
        .collect()
}

/// Kokoro input ids: `[0, ids…, 0]`, at most 510 symbols.
pub fn kokoro_ids(tokens: &str) -> Vec<i64> {
    let mut ids = vec![0i64];
    for c in tokens.chars().take(510) {
        if let Some(v) = KOKORO_VOCAB.iter().find(|v| v.0 == c) {
            ids.push(v.1 as i64);
        }
    }
    ids.push(0);
    ids
}

const STYLETTS_PUNCT_KITTEN: &str = ";:,.!?¡¿—…\"«»\"\" ";
const STYLETTS_PUNCT_MATCHA: &str = ";:,.!?¡¿—…\"«»“” ";
const STYLETTS_LETTERS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const STYLETTS_IPA: &str = "ɑɐɒæɓʙβɔɕçɗɖðʤəɘɚɛɜɝɞɟʄɡɠɢʛɦɧħɥʜɨɪʝɭɬɫɮʟɱɯɰŋɳɲɴøɵɸθœɶʘɹɺɾɻʀʁɽʂʃʈʧʉʊʋⱱʌɣɤʍχʎʏʑʐʒʔʡʕʢǀǁǂǃˈˌːˑʼʴʰʱʲʷˠˤ˞↓↑→↗↘'\u{329}'ᵻ";

/// Id of a symbol in the StyleTTS2-style table (pad, punctuation, letters,
/// IPA). Later duplicates win, as in the reference table construction.
fn styletts_id(c: char, matcha: bool) -> Option<i64> {
    let punct = if matcha {
        STYLETTS_PUNCT_MATCHA
    } else {
        STYLETTS_PUNCT_KITTEN
    };
    let mut found = None;
    for (i, s) in punct
        .chars()
        .chain(STYLETTS_LETTERS.chars())
        .chain(STYLETTS_IPA.chars())
        .enumerate()
    {
        if s == c {
            found = Some(i as i64 + 1);
        }
    }
    found
}

/// Kitten TTS 0.8 input ids: `[0, ids…, 10, 0]` (10 is `…`). Unknown symbols
/// are dropped.
pub fn kitten_ids(phonemes: &str) -> Vec<i64> {
    let mut ids = vec![0i64];
    ids.extend(phonemes.chars().filter_map(|c| styletts_id(c, false)));
    ids.push(10);
    ids.push(0);
    ids
}

/// Matcha-TTS input ids: symbol ids interspersed with the blank id 0
/// (`add_blank: True`).
pub fn matcha_ids(phonemes: &str) -> Vec<i64> {
    let mut ids = vec![0i64];
    for id in phonemes.chars().filter_map(|c| styletts_id(c, true)) {
        ids.push(id);
        ids.push(0);
    }
    ids
}

/// Symbols of `text` that a profile's fixed table cannot represent.
pub fn unmapped(text: &str, profile: Profile) -> Vec<char> {
    text.chars()
        .filter(|&c| match profile {
            Profile::Kokoro => !KOKORO_VOCAB.iter().any(|v| v.0 == c),
            Profile::Kitten => styletts_id(c, false).is_none(),
            Profile::Matcha => styletts_id(c, true).is_none(),
            _ => false,
        })
        .collect()
}

/// A profile's output as one line of text (used by the command-line tools).
///
/// * `espeak-ipa`, `kitten`, `matcha`: the phoneme string.
/// * `piper`: sentence strings joined by a space.
/// * `kokoro`: the token string.
/// * `sherpa-onnx`: codepoints separated by spaces, sentences by ` | `.
pub fn render_text(
    sentences: &[Sentence],
    profile: Profile,
    lang: Lang,
    punctuation: bool,
) -> String {
    match profile {
        Profile::EspeakIpa => espeak_ipa(sentences, punctuation, None),
        Profile::Kitten | Profile::Matcha => espeak_ipa(sentences, true, None),
        Profile::Piper => piper_sentences(sentences).join(" "),
        Profile::Kokoro => kokoro_tokens(sentences, lang),
        Profile::SherpaOnnx => sherpa_codepoints(sentences)
            .iter()
            .map(|s| {
                s.iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>()
            .join(" | "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styletts_table_positions() {
        assert_eq!(styletts_id(' ', false), Some(16));
        assert_eq!(styletts_id('A', false), Some(17));
        assert_eq!(styletts_id('a', true), Some(43));
        assert_eq!(styletts_id('ɑ', false), Some(69));
        assert_eq!(styletts_id('ᵻ', false), Some(177));
        assert_eq!(styletts_id('\'', false), Some(176));
        assert_eq!(styletts_id('\u{329}', false), Some(175));
        assert_eq!(styletts_id('"', false), Some(15));
        assert_eq!(styletts_id('"', true), Some(11));
        assert_eq!(styletts_id('”', true), Some(15));
        assert_eq!(kitten_ids("a"), [0, 43, 10, 0]);
        assert_eq!(matcha_ids("ab"), [0, 43, 0, 44, 0]);
    }

    #[test]
    fn kokoro_mappings() {
        assert_eq!(kokoro_english("bˈʌɾɚ", false), "bˈʌTəɹ");
        assert_eq!(kokoro_english("bˈʌʔn̩", false), "bˈʌtᵊn");
        assert_eq!(kokoro_english("tʃˈɜːtʃ", false), "ʧˈɜɹʧ");
        assert_eq!(kokoro_english("ɡˈəʊ", true), "ɡˈQ");
        assert_eq!(kokoro_ids("a."), [0, 43, 4, 0]);
    }

    #[test]
    fn piper_id_map() {
        let m = PhonemeIdMap::from_json(r#"{"phoneme_id_map": {"_": [0], "^": [1], "$": [2], " ": [3], "a": [14], "ˈ": [120]}}"#).unwrap();
        assert_eq!(m.len(), 6);
        let (ids, missing) = m.ids("ˈa ax");
        assert_eq!(ids, [1, 0, 120, 0, 14, 0, 3, 0, 14, 0, 2]);
        assert_eq!(missing, ['x']);
    }
}
