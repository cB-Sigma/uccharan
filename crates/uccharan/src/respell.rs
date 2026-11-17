//! Respelling: turn phonemes (espeak-style IPA) into text that a text-input
//! voice reads aloud the right way.
//!
//! For each language a writer produces candidate spellings (in the
//! language's own script, or a Latin respelling for English); every
//! candidate is read back with uccharan's own phonemiser and the one whose
//! phonemes are closest to the target wins. uccharan's reading stands in for
//! the voice's reading, so a respelling that uccharan reads correctly is a
//! good bet, not a guarantee.

use crate::text::Script;
use crate::{Lang, Mode, Options, Phonemizer};

/// How a Latin (English) respelling is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Style {
    /// The language's own script; for English, the plain style.
    #[default]
    Native,
    /// Syllables joined by hyphens, the stressed one in capitals ("kuh-KOR-oh").
    LatinHyphen,
    /// One lower-case word, no hyphens (for voices that lower-case text or
    /// read hyphens as pauses).
    LatinPlain,
}

impl Style {
    /// Parse `native`, `latin-hyphen` or `latin-plain`.
    pub fn parse(s: &str) -> Option<Style> {
        Some(match s {
            "" | "native" => Style::Native,
            "latin-hyphen" | "hyphen" => Style::LatinHyphen,
            "latin-plain" | "plain" => Style::LatinPlain,
            _ => return None,
        })
    }
}

/// A chosen respelling.
#[derive(Clone, Debug, PartialEq)]
pub struct Respelling {
    /// The text to give the voice.
    pub text: String,
    /// uccharan's reading of `text`.
    pub readback: String,
    /// Phoneme error rate of the reading against the target (0 = exact,
    /// after folding allophones and stress).
    pub distance: f32,
}

// ---------------------------------------------------------------------------
// Phones

/// One phone of a parsed IPA string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Phone {
    /// A vowel (possibly a diphthong), with length, nasality and stress.
    V {
        q: String,
        long: bool,
        nasal: bool,
        stress: u8,
    },
    /// A consonant, with aspiration.
    C { q: String, asp: bool },
    /// A word boundary.
    Space,
}

const VOWELS: &str = "aɑæɐəɚɝɛɜeɪiɨɔoɒʊuʌøyɵɤɯœɘᵻ";
const DIPHTHONGS: [&str; 9] = ["eɪ", "aɪ", "ɔɪ", "aʊ", "oʊ", "əʊ", "ɪə", "eə", "ʊə"];

fn is_vowel(c: char) -> bool {
    VOWELS.contains(c)
}

/// Parse espeak-style IPA into phones. Unknown symbols are skipped.
pub(crate) fn phones(ipa: &str) -> Vec<Phone> {
    let cs: Vec<char> = crate::profile::nfd(ipa).chars().collect();
    let mut out: Vec<Phone> = Vec::new();
    let mut stress = 0u8;
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        match c {
            'ˈ' => stress = 2,
            'ˌ' => stress = stress.max(1),
            ' ' | '-' | '_' | ',' | '.' | '?' | '!' | ';' | ':' | '|' => {
                if matches!(c, ' ' | '-' | '|') && !matches!(out.last(), Some(Phone::Space) | None)
                {
                    out.push(Phone::Space);
                }
                stress = 0;
            }
            'ː' | 'ˑ' => {
                if let Some(Phone::V { long, .. }) = out.last_mut() {
                    *long = true;
                }
            }
            '\u{0303}' => {
                if let Some(Phone::V { nasal, .. }) = out.last_mut() {
                    *nasal = true;
                }
            }
            'ʰ' | 'ʱ' => {
                if let Some(Phone::C { asp, .. }) = out.last_mut() {
                    *asp = true;
                }
            }
            '\u{0329}' | '\u{032a}' | '\u{0361}' | 'ʲ' | 'ʷ' | '\u{031e}' | '\u{031d}' | 'ʔ' => {
            }
            c if is_vowel(c) => {
                let mut q = c.to_string();
                if let Some(&n) = cs.get(i + 1) {
                    let pair: String = [c, n].iter().collect();
                    if DIPHTHONGS.contains(&pair.as_str()) {
                        q = pair;
                        i += 1;
                    }
                }
                match q.as_str() {
                    "ɚ" => {
                        out.push(Phone::V {
                            q: "ə".into(),
                            long: false,
                            nasal: false,
                            stress,
                        });
                        if cs.get(i + 1) != Some(&'ɹ') {
                            out.push(Phone::C {
                                q: "r".into(),
                                asp: false,
                            });
                        }
                    }
                    "ɝ" => {
                        out.push(Phone::V {
                            q: "ɜ".into(),
                            long: true,
                            nasal: false,
                            stress,
                        });
                        out.push(Phone::C {
                            q: "r".into(),
                            asp: false,
                        });
                    }
                    _ => out.push(Phone::V {
                        q,
                        long: false,
                        nasal: false,
                        stress,
                    }),
                }
                stress = 0;
            }
            c if c.is_alphabetic() => {
                let mut q = c.to_string();
                if let Some(&n) = cs.get(i + 1) {
                    let pair: String = [c, n].iter().collect();
                    if matches!(pair.as_str(), "tʃ" | "dʒ" | "tɕ" | "dʑ") {
                        q = pair;
                        i += 1;
                    } else if n == '\u{0361}' {
                        if let Some(&m) = cs.get(i + 2) {
                            q = [c, m].iter().collect();
                            i += 2;
                        }
                    }
                }
                let q = match q.as_str() {
                    "ɡ" => "g".to_string(),
                    "ɫ" => "l".to_string(),
                    other => other.to_string(),
                };
                out.push(Phone::C { q, asp: false });
            }
            _ => {}
        }
        i += 1;
    }
    while matches!(out.last(), Some(Phone::Space)) {
        out.pop();
    }
    out
}

/// Phones folded to coarse classes for comparing readings across
/// languages: length, stress and nasality are ignored, near vowels merge,
/// diphthongs become two vowels, rhotics, retroflex and dental stops merge.
fn fold(ps: &[Phone]) -> Vec<String> {
    let mut out = Vec::new();
    for p in ps {
        match p {
            Phone::Space => {}
            Phone::V { q, .. } => {
                let parts: &[&str] = match q.as_str() {
                    "eɪ" => &["e"],
                    "oʊ" | "əʊ" => &["o"],
                    "aɪ" => &["a", "i"],
                    "aʊ" => &["a", "u"],
                    "ɔɪ" => &["ɔ", "i"],
                    "ɪə" => &["i", "ə"],
                    "eə" => &["ɛ", "ə"],
                    "ʊə" => &["u", "ə"],
                    "ə" | "ɐ" | "ʌ" | "ɜ" | "ɘ" | "ɤ" | "ᵻ" => &["ə"],
                    "a" | "ɑ" => &["a"],
                    "æ" | "ɛ" => &["ɛ"],
                    "e" | "ø" | "œ" => &["e"],
                    "ɪ" | "i" | "ɨ" | "y" => &["i"],
                    "ʊ" | "u" | "ɯ" => &["u"],
                    "o" | "ɵ" => &["o"],
                    "ɔ" | "ɒ" => &["ɔ"],
                    _ => &["ə"],
                };
                out.extend(parts.iter().map(|s| s.to_string()));
            }
            Phone::C { q, asp } => {
                let base = match q.as_str() {
                    "ɹ" | "ɾ" | "r" | "ɽ" | "ɻ" | "ʀ" | "ʁ" => "r",
                    "ʋ" | "v" | "w" | "β" => "v",
                    "ɦ" | "h" => "h",
                    "ʈ" => "t",
                    "ɖ" => "d",
                    "ɳ" => "n",
                    "ʂ" | "ɕ" | "ç" => "ʃ",
                    "ɭ" => "l",
                    "tɕ" => "tʃ",
                    "dʑ" => "dʒ",
                    other => other,
                };
                // Aspiration counts only for the Indic distinction (kʰ vs k).
                let asp = *asp && !matches!(base, "r" | "v" | "l" | "h" | "m" | "n");
                out.push(if asp {
                    format!("{base}ʰ")
                } else {
                    base.to_string()
                });
            }
        }
    }
    out
}

fn edit_distance(a: &[String], b: &[String]) -> usize {
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

/// Phoneme error rate of `got` against `target` (both espeak-style IPA),
/// after folding (see the module notes).
pub fn ipa_distance(target: &str, got: &str) -> f32 {
    ipa_distance_in(target, got, None)
}

/// Vowels a language's script cannot write, mapped to the nearest it can
/// (so that a respelling is not blamed for what no spelling could do).
fn nearest_vowel(lang: Option<Lang>, v: String) -> String {
    let Some(l) = lang else { return v };
    let r = match (l, v.as_str()) {
        (Lang::Te | Lang::Kn | Lang::Ml | Lang::Ta | Lang::Si, "ə") => "a",
        (Lang::Te | Lang::Kn | Lang::Ml | Lang::Ta | Lang::Si, "ɔ") => "o",
        (Lang::Te | Lang::Kn | Lang::Ml | Lang::Ta | Lang::Si | Lang::Pa, "ɛ") => "e",
        (Lang::Bn | Lang::As | Lang::Or, "ə") => "ɔ",
        (Lang::Bn | Lang::As | Lang::Or, "ɛ") => "æ",
        _ => return v,
    };
    r.to_string()
}

/// [`ipa_distance`] for a respelling read in `lang`.
pub fn ipa_distance_in(target: &str, got: &str, lang: Option<Lang>) -> f32 {
    let t: Vec<String> = fold(&phones(target))
        .into_iter()
        .map(|v| nearest_vowel(lang, v))
        .collect();
    let g: Vec<String> = fold(&phones(got))
        .into_iter()
        .map(|v| nearest_vowel(lang, v))
        .collect();
    if t.is_empty() {
        return if g.is_empty() { 0.0 } else { 1.0 };
    }
    edit_distance(&t, &g) as f32 / t.len() as f32
}

/// Phones back to an IPA string.
pub(crate) fn unparse(ps: &[Phone]) -> String {
    let mut s = String::new();
    for p in ps {
        match p {
            Phone::Space => s.push(' '),
            Phone::C { q, asp } => {
                s.push_str(q);
                if *asp {
                    s.push('ʰ');
                }
            }
            Phone::V {
                q,
                long,
                nasal,
                stress,
            } => {
                match stress {
                    2 => s.push('ˈ'),
                    1 => s.push('ˌ'),
                    _ => {}
                }
                s.push_str(q);
                if *nasal {
                    s.push('\u{0303}');
                }
                if *long {
                    s.push('ː');
                }
            }
        }
    }
    s
}

/// `gb` (a non-rhotic reading) with the r sounds of `us` (a rhotic reading
/// of the same words) put back, by aligning the two phone strings.
pub fn restore_r(gb: &str, us: &str) -> String {
    let g = phones(gb);
    let u = phones(us);
    let fg = |p: &Phone| fold(std::slice::from_ref(p));
    let cost = |a: &Phone, b: &Phone| usize::from(a != b && fg(a) != fg(b));
    let (n, m) = (g.len(), u.len());
    let mut d = vec![vec![0usize; m + 1]; n + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            d[i][j] = (d[i - 1][j - 1] + cost(&g[i - 1], &u[j - 1]))
                .min(d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1);
        }
    }
    let is_r = |p: &Phone| matches!(p, Phone::C { q, .. } if q == "ɹ" || q == "r");
    let mut out: Vec<Phone> = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && d[i][j] == d[i - 1][j - 1] + cost(&g[i - 1], &u[j - 1]) {
            out.push(g[i - 1].clone());
            i -= 1;
            j -= 1;
        } else if j > 0 && d[i][j] == d[i][j - 1] + 1 {
            if is_r(&u[j - 1]) {
                out.push(Phone::C {
                    q: "r".into(),
                    asp: false,
                });
            }
            j -= 1;
        } else {
            out.push(g[i - 1].clone());
            i -= 1;
        }
    }
    out.reverse();
    unparse(&out)
}

// ---------------------------------------------------------------------------
// Writers: a phone sequence and a vector of choices -> text.

/// A writer produces text from the choice vector; it reports how many
/// alternatives each choice point has through `points`.
trait Writer {
    fn write(&self, choices: &[usize], points: &mut Vec<usize>) -> String;
}

/// Pick choice `k` among `n` alternatives, recording the choice point.
fn pick(choices: &[usize], points: &mut Vec<usize>, n: usize) -> usize {
    let k = points.len();
    points.push(n);
    choices
        .get(k)
        .copied()
        .unwrap_or(0)
        .min(n.saturating_sub(1))
}

struct Brahmic<'a> {
    ps: &'a [Phone],
    /// The IPA came from an Indic language (plain `t` is dental there).
    indic_source: bool,
    /// Write for a reader that deletes inherent vowels (Hindi-type).
    deletes_schwa: bool,
}

/// Consonant spellings (Devanagari), most likely first.
fn deva_consonant(q: &str, asp: bool, indic: bool) -> &'static [&'static str] {
    match (q, asp) {
        ("p", false) => &["प"],
        ("p", true) => &["फ"],
        ("b", false) => &["ब"],
        ("b", true) => &["भ"],
        ("t", false) if indic => &["त", "ट"],
        ("t", false) => &["ट", "त"],
        ("t", true) if indic => &["थ", "ठ"],
        ("t", true) => &["ठ", "थ"],
        ("d", false) if indic => &["द", "ड"],
        ("d", false) => &["ड", "द"],
        ("d", true) => &["ध", "ढ"],
        ("ʈ", false) => &["ट"],
        ("ʈ", true) => &["ठ"],
        ("ɖ", false) => &["ड"],
        ("ɖ", true) => &["ढ"],
        ("k", false) => &["क"],
        ("k", true) => &["ख"],
        ("g", false) => &["ग"],
        ("g", true) => &["घ"],
        ("tʃ" | "tɕ" | "ts", false) => &["च"],
        ("tʃ" | "tɕ" | "ts", true) => &["छ"],
        ("dʒ" | "dʑ" | "dz", false) => &["ज"],
        ("dʒ" | "dʑ" | "dz", true) => &["झ"],
        ("m", _) => &["म"],
        ("n", _) => &["न"],
        ("ɳ", _) => &["ण", "न"],
        ("ŋ", _) => &["ङ", "न"],
        ("ɲ", _) => &["ञ", "न"],
        ("s", _) => &["स"],
        ("ʃ" | "ɕ" | "ç", _) => &["श"],
        ("ʂ", _) => &["ष", "श"],
        ("h" | "ɦ", _) => &["ह"],
        ("j", _) => &["य"],
        ("r" | "ɾ" | "ɹ" | "ɻ" | "ʀ" | "ʁ", _) => &["र"],
        ("ɽ", false) => &["ड़", "र"],
        ("ɽ", true) => &["ढ़"],
        ("l", _) => &["ल"],
        ("ɭ", _) => &["ळ", "ल"],
        ("ʋ" | "v" | "w" | "β", _) => &["व"],
        ("f" | "ɸ", _) => &["फ़", "फ"],
        ("z", _) => &["ज़", "ज"],
        ("ʒ", _) => &["ज़", "झ"],
        ("x" | "χ", _) => &["ख़", "ख"],
        ("ɣ" | "ʁ̞", _) => &["ग़", "ग"],
        ("q", _) => &["क़", "क"],
        ("θ", _) => &["थ", "त"],
        ("ð", _) => &["द", "ध"],
        _ => &[""],
    }
}

/// Vowel spellings as (independent letter, sign after a consonant); an
/// empty sign is the inherent vowel.
fn deva_vowel(q: &str, long: bool) -> &'static [(&'static str, &'static str)] {
    match (q, long) {
        ("ə" | "ɐ" | "ʌ" | "ɘ" | "ɤ", _) => &[("अ", "")],
        ("ɜ", _) => &[("अ", ""), ("अर", "र")],
        ("a" | "ɑ", _) => &[("आ", "ा")],
        ("æ", _) => &[("ऐ", "ै"), ("ए", "े"), ("आ", "ा")],
        ("ɛ", _) => &[("ए", "े"), ("ऐ", "ै")],
        ("e" | "eɪ" | "ø", _) => &[("ए", "े")],
        ("i" | "y", true) => &[("ई", "ी"), ("इ", "ि")],
        ("i" | "y" | "ɨ" | "ᵻ", false) => &[("इ", "ि"), ("ई", "ी")],
        ("ɪ", _) => &[("इ", "ि"), ("ई", "ी")],
        ("u", true) => &[("ऊ", "ू"), ("उ", "ु")],
        ("u" | "ɯ", false) => &[("उ", "ु"), ("ऊ", "ू")],
        ("ʊ", _) => &[("उ", "ु"), ("ऊ", "ू")],
        ("o" | "oʊ" | "əʊ" | "ɵ", _) => &[("ओ", "ो")],
        ("ɔ" | "ɒ", _) => &[("ऑ", "ॉ"), ("ओ", "ो"), ("औ", "ौ")],
        ("aɪ", _) => &[("आइ", "ाइ"), ("ऐ", "ै"), ("आय", "ाय")],
        ("aʊ", _) => &[("आउ", "ाउ"), ("औ", "ौ")],
        ("ɔɪ", _) => &[("ऑय", "ॉय"), ("ओइ", "ोइ")],
        ("ɪə", _) => &[("इअ", "िअ"), ("इय", "िय")],
        ("eə", _) => &[("एअ", "ेअ"), ("ए", "े")],
        ("ʊə", _) => &[("उअ", "ुअ")],
        _ => &[("अ", "")],
    }
}

const VIRAMA: &str = "\u{094d}";

impl Writer for Brahmic<'_> {
    fn write(&self, choices: &[usize], points: &mut Vec<usize>) -> String {
        let mut out = String::new();
        // True when the last written letter is a consonant still carrying
        // its inherent vowel.
        let mut open = false;
        let n = self.ps.len();
        for (i, p) in self.ps.iter().enumerate() {
            let next = self.ps.get(i + 1);
            match p {
                Phone::Space => {
                    if open {
                        // Word-final consonant: bare, or with a virama.
                        if pick(choices, points, 2) == 1 {
                            out.push_str(VIRAMA);
                        }
                    }
                    open = false;
                    out.push(' ');
                }
                Phone::C { q, asp } => {
                    let nasal_before_c = matches!(q.as_str(), "m" | "n" | "ŋ" | "ɲ" | "ɳ")
                        && matches!(next, Some(Phone::C { q: nq, .. }) if !matches!(nq.as_str(), "j" | "r" | "ɾ" | "ɹ" | "h" | "ɦ" | "v" | "ʋ" | "w"))
                        && i > 0
                        && matches!(self.ps[i - 1], Phone::V { .. });
                    if open {
                        // A cluster: virama, or leave the inherent vowel for
                        // the reader to delete.
                        let k = if self.deletes_schwa {
                            pick(choices, points, 2)
                        } else {
                            0
                        };
                        if k == 0 {
                            out.push_str(VIRAMA);
                        }
                    }
                    if nasal_before_c && pick(choices, points, 2) == 1 {
                        out.push('\u{0902}');
                        open = false;
                        continue;
                    }
                    let opts = deva_consonant(q, *asp, self.indic_source);
                    let k = pick(choices, points, opts.len());
                    out.push_str(opts[k]);
                    open = !opts[k].is_empty();
                }
                Phone::V { q, long, nasal, .. } => {
                    let opts = deva_vowel(q, *long);
                    let k = pick(choices, points, opts.len());
                    let (ind, sign) = opts[k];
                    let was_open = open;
                    if was_open {
                        out.push_str(sign);
                    } else {
                        out.push_str(ind);
                    }
                    if *nasal {
                        out.push_str(if pick(choices, points, 2) == 0 {
                            "\u{0901}"
                        } else {
                            "\u{0902}"
                        });
                    }
                    open = false;
                    // A vowel written with a trailing consonant letter
                    // ("अर", "आय") leaves that consonant open.
                    if (sign.ends_with('र') || sign.ends_with('य')) && i + 1 < n {
                        open = true;
                    }
                }
            }
        }
        if open && pick(choices, points, 2) == 1 {
            out.push_str(VIRAMA);
        }
        out
    }
}

struct Latin<'a> {
    ps: &'a [Phone],
    hyphen: bool,
}

fn latin_consonant(q: &str, asp: bool) -> &'static [&'static str] {
    match (q, asp) {
        ("p", false) => &["p"],
        ("p", true) => &["ph", "p"],
        ("b", false) => &["b"],
        ("b", true) => &["bh", "b"],
        ("t" | "ʈ", false) => &["t"],
        ("t" | "ʈ", true) => &["th", "t"],
        ("d" | "ɖ", false) => &["d"],
        ("d" | "ɖ", true) => &["dh", "d"],
        ("k" | "q", false) => &["k", "c"],
        ("k", true) => &["kh", "k"],
        ("g", false) => &["g"],
        ("g", true) => &["gh", "g"],
        ("tʃ" | "tɕ", false) => &["ch"],
        ("tʃ" | "tɕ", true) => &["chh", "ch"],
        ("dʒ" | "dʑ", false) => &["j"],
        ("dʒ" | "dʑ", true) => &["jh", "j"],
        ("ts", _) => &["ts"],
        ("dz", _) => &["dz"],
        ("m", _) => &["m"],
        ("n" | "ɳ", _) => &["n"],
        ("ŋ", _) => &["ng", "n"],
        ("ɲ", _) => &["ny", "n"],
        ("f" | "ɸ", _) => &["f"],
        ("v" | "ʋ" | "β", _) => &["v", "w"],
        ("w", _) => &["w"],
        ("θ", _) => &["th"],
        ("ð", _) => &["th", "dh"],
        ("s", _) => &["s", "ss"],
        ("z", _) => &["z"],
        ("ʃ" | "ʂ" | "ɕ" | "ç", _) => &["sh"],
        ("ʒ", _) => &["zh"],
        ("h" | "ɦ", _) => &["h"],
        ("j", _) => &["y"],
        ("r" | "ɾ" | "ɹ" | "ɻ" | "ɽ" | "ʀ" | "ʁ", _) => &["r"],
        ("l" | "ɭ", _) => &["l"],
        ("x" | "χ", _) => &["kh", "h"],
        ("ɣ", _) => &["gh", "g"],
        _ => &[""],
    }
}

fn latin_vowel(q: &str, long: bool) -> &'static [&'static str] {
    match (q, long) {
        ("ə" | "ɐ" | "ɘ", _) => &["uh", "a", "u"],
        ("ʌ", _) => &["u", "uh", "o"],
        ("ɜ", _) => &["ur", "er"],
        ("æ", _) => &["a"],
        ("a" | "ɑ", _) => &["ah", "aa", "a"],
        ("ɒ", _) => &["o"],
        ("ɔ", _) => &["aw", "o"],
        ("ɛ", _) => &["e", "eh"],
        ("e" | "eɪ", _) => &["ay", "e"],
        ("ɪ", _) => &["i", "ih"],
        ("i" | "y" | "ɨ" | "ᵻ", _) => &["ee", "i"],
        ("ʊ", _) => &["oo", "u"],
        ("u" | "ɯ", _) => &["oo"],
        ("o" | "oʊ" | "əʊ" | "ɵ", _) => &["oh", "o"],
        ("aɪ", _) => &["eye", "y", "igh", "ai", "ie"],
        ("aʊ", _) => &["ow"],
        ("ɔɪ", _) => &["oy"],
        ("ɪə", _) => &["eer"],
        ("eə", _) => &["air"],
        ("ʊə", _) => &["oor"],
        _ => &["uh"],
    }
}

impl Writer for Latin<'_> {
    fn write(&self, choices: &[usize], points: &mut Vec<usize>) -> String {
        // Syllables: each vowel is a nucleus; of the consonants between two
        // vowels, the last starts the next syllable.
        let mut words: Vec<String> = Vec::new();
        let mut word: Vec<(String, bool)> = Vec::new();
        let mut cur = String::new();
        let mut cur_stress = false;
        let mut pending: Vec<String> = Vec::new();
        let flush_word = |word: &mut Vec<(String, bool)>, words: &mut Vec<String>| {
            if word.is_empty() {
                return;
            }
            let w = if self.hyphen {
                word.iter()
                    .map(|(s, st)| {
                        if *st && word.len() > 1 {
                            s.to_uppercase()
                        } else {
                            s.clone()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("-")
            } else {
                word.iter().map(|x| x.0.as_str()).collect::<String>()
            };
            words.push(w);
            word.clear();
        };
        let mut seen_vowel = false;
        for p in self.ps {
            match p {
                Phone::Space => {
                    cur.push_str(&pending.concat());
                    pending.clear();
                    if !cur.is_empty() {
                        word.push((std::mem::take(&mut cur), cur_stress));
                    }
                    cur_stress = false;
                    seen_vowel = false;
                    flush_word(&mut word, &mut words);
                }
                Phone::C { q, asp } => {
                    let opts = latin_consonant(q, *asp);
                    let k = pick(choices, points, opts.len());
                    pending.push(opts[k].to_string());
                }
                Phone::V {
                    q, long, stress, ..
                } => {
                    if seen_vowel {
                        // Close the previous syllable with all but the last consonant.
                        let onset = pending.pop();
                        cur.push_str(&pending.concat());
                        pending.clear();
                        word.push((std::mem::take(&mut cur), cur_stress));
                        if let Some(o) = onset {
                            cur.push_str(&o);
                        }
                    } else {
                        cur.push_str(&pending.concat());
                        pending.clear();
                    }
                    cur_stress = *stress == 2;
                    let opts = latin_vowel(q, *long);
                    let k = pick(choices, points, opts.len());
                    cur.push_str(opts[k]);
                    seen_vowel = true;
                }
            }
        }
        cur.push_str(&pending.concat());
        if !cur.is_empty() {
            word.push((cur, cur_stress));
        }
        flush_word(&mut word, &mut words);
        words.join(" ")
    }
}

// ---------------------------------------------------------------------------
// Scripts other than Devanagari: the Devanagari candidate moved to the
// script's block, with letters the script lacks replaced first.

fn transpose(deva: &str, lang: Lang) -> Option<String> {
    let base = match lang.script() {
        Script::Devanagari => return Some(deva.to_string()),
        Script::Brahmic(b) => b,
        _ => return None,
    };
    if lang == Lang::Si {
        return None;
    }
    let tamil = base == 0x0b80;
    let no_nukta = matches!(base, 0x0c00 | 0x0c80 | 0x0d00 | 0x0b80);
    let has_candra = base == 0x0a80;
    let mut s = String::new();
    for c in deva.chars() {
        let rep: Option<&str> = match c {
            'ॅ' if !has_candra => Some("े"),
            'ॉ' if !has_candra => Some("ो"),
            'ऍ' if !has_candra => Some("ए"),
            'ऑ' if !has_candra => Some("ओ"),
            '\u{093c}' if no_nukta => Some(""),
            'व' if base == 0x0980 => Some("ब"),
            'ष' if base == 0x0a00 => Some("श"),
            'ऋ' if base == 0x0a00 || tamil => Some("रि"),
            'ळ' if base == 0x0980 => Some("ल"),
            'ं' if tamil => Some("न्"),
            'ँ' if tamil => Some(""),
            _ => None,
        };
        let rep = rep.or_else(|| {
            if !tamil {
                return None;
            }
            // Tamil writes one stop letter per place of articulation.
            Some(match c {
                'ख' | 'ग' | 'घ' => "क",
                'छ' | 'झ' => "च",
                'ठ' | 'ड' | 'ढ' => "ट",
                'थ' | 'द' | 'ध' => "त",
                'फ' | 'ब' | 'भ' => "प",
                'ङ' => "ङ",
                _ => return None,
            })
        });
        match rep {
            Some(r) => s.push_str(r),
            None => s.push(c),
        }
    }
    let mut out = String::new();
    for c in s.chars() {
        let u = c as u32;
        if (0x0900..0x0980).contains(&u) && !matches!(u, 0x0964 | 0x0965) {
            let t = char::from_u32(u - 0x0900 + base)?;
            out.push(t);
        } else {
            out.push(c);
        }
    }
    // Gurmukhi writes the flap as its own letter.
    if base == 0x0a00 {
        out = out.replace("\u{0a21}\u{0a3c}", "\u{0a5c}");
    }
    Some(out)
}

/// The language used to read a respelling back.
fn reader(lang: Lang) -> Lang {
    lang
}

fn readback(ph: &Phonemizer, text: &str, lang: Lang) -> String {
    let mut opt = Options::new(reader(lang));
    opt.mode = Mode::Improved;
    ph.phonemize(text, &opt)
}

/// Respell `ipa` (espeak-style IPA, from a language `source`) for a text
/// voice reading language `lang`. Returns `None` for a language without a
/// writer (Sinhala for now) or empty IPA.
pub fn respell(
    ph: &Phonemizer,
    ipa: &str,
    source: Option<Lang>,
    lang: Lang,
    style: Style,
) -> Option<Respelling> {
    let ps = phones(ipa);
    if ps.is_empty() {
        return None;
    }
    let indic_source = source.is_some_and(|s| !s.is_english());
    if lang.is_english() || matches!(style, Style::LatinHyphen | Style::LatinPlain) {
        let w = Latin {
            ps: &ps,
            hyphen: style == Style::LatinHyphen,
        };
        let read_lang = if lang.is_english() { lang } else { Lang::EnUs };
        return Some(search(read_lang, &w, ipa, |t| {
            let r = if style == Style::LatinHyphen {
                t.replace('-', " ").to_lowercase()
            } else {
                t.to_string()
            };
            (t.to_string(), readback(ph, &r, read_lang))
        }));
    }
    transpose("", lang)?;
    let w = Brahmic {
        ps: &ps,
        indic_source,
        deletes_schwa: lang.hindi_like()
            || matches!(
                lang,
                Lang::Mr
                    | Lang::Ne
                    | Lang::Gu
                    | Lang::Pa
                    | Lang::Bn
                    | Lang::As
                    | Lang::Kok
                    | Lang::Doi
            ),
    };
    Some(search(lang, &w, ipa, |t| {
        let text = transpose(t, lang).unwrap_or_default();
        let r = readback(ph, &text, lang);
        (text, r)
    }))
}

/// Coordinate descent over the writer's choice points.
fn search<W: Writer>(
    lang: Lang,
    w: &W,
    target: &str,
    eval: impl Fn(&str) -> (String, String),
) -> Respelling {
    let score = |choices: &[usize]| -> (f32, Respelling, Vec<usize>) {
        let mut points = Vec::new();
        let raw = w.write(choices, &mut points);
        let (text, back) = eval(&raw);
        let d = ipa_distance_in(target, &back, Some(lang));
        (
            d,
            Respelling {
                text,
                readback: back,
                distance: d,
            },
            points,
        )
    };
    let mut choices: Vec<usize> = Vec::new();
    let (mut best_d, mut best, mut points) = score(&choices);
    for _pass in 0..3 {
        let mut k = 0;
        let mut improved = false;
        while k < points.len() {
            let n = points[k];
            let keep = choices.get(k).copied().unwrap_or(0);
            for alt in 0..n {
                if alt == keep {
                    continue;
                }
                let mut c = choices.clone();
                c.resize(points.len().max(k + 1), 0);
                c[k] = alt;
                let (d, r, p) = score(&c);
                if d + 1e-6 < best_d {
                    best_d = d;
                    best = r;
                    choices = c;
                    points = p;
                    improved = true;
                }
            }
            k += 1;
        }
        if !improved || best_d == 0.0 {
            break;
        }
    }
    best
}

/// English letter names (en-US IPA), for spelling Latin acronyms out in
/// another script.
pub fn letter_ipa(c: char) -> Option<&'static str> {
    Some(match c.to_ascii_lowercase() {
        'a' => "ˈeɪ",
        'b' => "bˈiː",
        'c' => "sˈiː",
        'd' => "dˈiː",
        'e' => "ˈiː",
        'f' => "ˈɛf",
        'g' => "dʒˈiː",
        'h' => "ˈeɪtʃ",
        'i' => "ˈaɪ",
        'j' => "dʒˈeɪ",
        'k' => "kˈeɪ",
        'l' => "ˈɛl",
        'm' => "ˈɛm",
        'n' => "ˈɛn",
        'o' => "ˈoʊ",
        'p' => "pˈiː",
        'q' => "kjˈuː",
        'r' => "ˈɑːɹ",
        's' => "ˈɛs",
        't' => "tˈiː",
        'u' => "jˈuː",
        'v' => "vˈiː",
        'w' => "dˈʌbəljuː",
        'x' => "ˈɛks",
        'y' => "wˈaɪ",
        'z' => "zˈɛd",
        _ => return None,
    })
}

/// Common Devanagari spellings of the English letter names.
pub fn letter_devanagari(c: char) -> Option<&'static str> {
    Some(match c.to_ascii_lowercase() {
        'a' => "ए",
        'b' => "बी",
        'c' => "सी",
        'd' => "डी",
        'e' => "ई",
        'f' => "एफ़",
        'g' => "जी",
        'h' => "एच",
        'i' => "आई",
        'j' => "जे",
        'k' => "के",
        'l' => "एल",
        'm' => "एम",
        'n' => "एन",
        'o' => "ओ",
        'p' => "पी",
        'q' => "क्यू",
        'r' => "आर",
        's' => "एस",
        't' => "टी",
        'u' => "यू",
        'v' => "वी",
        'w' => "डब्ल्यू",
        'x' => "एक्स",
        'y' => "वाई",
        'z' => "ज़ेड",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_phones() {
        let p = phones("kəmpjˈuːtə");
        assert_eq!(p.len(), 8);
        assert!(matches!(&p[5], Phone::V { q, long: true, stress: 2, .. } if q == "u"));
        let p = phones("tʃˈaɪ");
        assert!(matches!(&p[0], Phone::C { q, .. } if q == "tʃ"));
        assert!(matches!(&p[1], Phone::V { q, .. } if q == "aɪ"));
        assert_eq!(ipa_distance("kˈat", "kat"), 0.0);
        assert!(ipa_distance("kˈat", "bat") > 0.3);
    }
}
