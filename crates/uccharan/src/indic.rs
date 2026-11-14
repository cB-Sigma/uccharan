//! Brahmic-script languages (Bengali, Marathi, and the rest of the Indic
//! family) as data: one decoder for every script block, parameterised by a
//! language file (`crates/uccharan/lang/<tag>.lang`).
//!
//! All Indic scripts share Unicode's parallel layout (128 code points per
//! block, the same offsets for "the same" letter), so a letter's class comes
//! from its offset and the language file only gives sounds. The pipeline for
//! one word:
//!
//! 1. letter-level rewrite rules (`[pre]`), e.g. conjuncts read as other letters;
//! 2. akshara decoding into consonants and vowels, with an inherent vowel
//!    after every consonant that has no vowel sign or virama;
//! 3. geminates, stress (one of the schemes in [`Stress`]) and inherent-vowel
//!    deletion (one of [`Deletion`]);
//! 4. rendering, then phoneme-level rewrite rules (`[post]`).
//!
//! Hindi still has its own module (`hi`), which predates this engine.

use std::collections::HashMap;

use crate::langdef::{self, LangFile, Numbers};
use crate::rules::{Classes, RuleSet};

/// How a letter behaves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Consonant,
    /// A consonant that never takes the inherent vowel (ৎ, chillu letters).
    Dead,
    Vowel,
    Sign,
    Virama,
    Nukta,
    Anusvara,
    Candrabindu,
    Visarga,
    Ignore,
}

/// A letter name whose first vowel is ə or ʌ (it protects a schwa before it).
fn starts_with_schwa(s: &str) -> bool {
    s.chars()
        .find(|c| "aeiouɪʊəʌɛɔ".contains(*c))
        .is_some_and(|c| c == 'ə' || c == 'ʌ')
}

/// The first syllable of a letter name is unstressed.
fn name_starts_unstressed(s: &str) -> bool {
    for c in s.chars() {
        if c == 'ˈ' || c == 'ˌ' {
            return false;
        }
        if "aeiouɪʊəʌɛɔ".contains(c) {
            return true;
        }
    }
    false
}

fn default_kind(off: u32) -> Option<Kind> {
    Some(match off {
        0x01 => Kind::Candrabindu,
        0x02 => Kind::Anusvara,
        0x03 => Kind::Visarga,
        0x04..=0x14 | 0x60 | 0x61 => Kind::Vowel,
        0x15..=0x39 | 0x58..=0x5f => Kind::Consonant,
        0x3c => Kind::Nukta,
        0x3d => Kind::Ignore,
        0x3e..=0x4c | 0x4e | 0x4f | 0x55..=0x57 | 0x62 | 0x63 => Kind::Sign,
        0x4d => Kind::Virama,
        _ => return None,
    })
}

#[derive(Clone, Debug)]
struct Letter {
    kind: Kind,
    /// The vowel of this letter attracts primary stress (`stressed` in the file).
    attract: bool,
    /// Sounds, space-separated in the file (a sign such as ृ can be "ɾ ɪ").
    sym: Vec<String>,
}

/// Where primary stress goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stress {
    /// Heaviest non-final syllable, the later one on a tie; a doubly heavy
    /// final syllable when nothing earlier is doubly heavy (Hindi type).
    Weight,
    /// First syllable.
    Initial,
    /// Penultimate syllable.
    Penult,
    /// First syllable, but the second when the first is light and the second long.
    InitialLightHeavy,
    /// Classical Sanskrit: the penult if heavy, else the antepenult if
    /// heavy, else the fourth syllable from the end.
    Sanskrit,
}

/// What happens to inherent vowels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Deletion {
    /// Word-final and medial (V C _ C V) deletion of unstressed inherent
    /// vowels, left to right (Hindi type).
    Medial,
    /// Word-final only, after a single consonant that follows a vowel.
    Final,
    /// Word-final, then a medial one right before the final consonant
    /// (V C _ C#), both unstressed (Gujarati type).
    Prefinal,
    /// Never.
    None,
}

/// What the anusvara becomes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Anusvara {
    /// After an inherent vowel a nasal homorganic with the next consonant;
    /// after a written vowel a nasal vowel.
    Split,
    /// Always the velar nasal.
    Velar,
    /// A nasal consonant homorganic with what follows (`m` at the end).
    Homorganic,
    /// Always `n`.
    N,
    /// Always `m`.
    M,
}

/// What a visarga inside a word does (at the end it is always `h`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Visarga {
    H,
    /// Doubles the next consonant.
    Geminate,
    Silent,
}

#[derive(Clone, Debug)]
struct Ph {
    s: String,
    vowel: bool,
    schwa: bool,
    long: bool,
    nasal: bool,
    stress: u8,
    conjunct: bool,
    independent: bool,
    attract: bool,
}

impl Ph {
    fn c(s: &str) -> Ph {
        Ph {
            s: s.to_string(),
            vowel: false,
            schwa: false,
            long: false,
            nasal: false,
            stress: 0,
            conjunct: false,
            independent: false,
            attract: false,
        }
    }
    fn v(s: &str) -> Ph {
        let long = s.ends_with('ː');
        Ph {
            s: s.trim_end_matches('ː').to_string(),
            vowel: true,
            long,
            ..Ph::c("")
        }
    }
}

/// A Brahmic-script language.
#[derive(Debug)]
pub struct Indic {
    /// Language tag.
    pub tag: String,
    base: u32,
    letters: crate::rules::CharMap<Letter>,
    /// Consonant + nukta, keyed by the base consonant.
    nukta: crate::rules::CharMap<Vec<String>>,
    inherent: (String, String),
    stress: Stress,
    deletion: Deletion,
    anusvara: Anusvara,
    final_secondary: bool,
    /// Consonants that merge into one long consonant when doubled through a virama.
    geminate: Vec<String>,
    /// Unaspirated stops that merge with their aspirated twin (त्थ → tʰː).
    geminate_aspirate: Vec<String>,
    /// Consonants before which a written र + virama is the trill `r`.
    trill: Vec<String>,
    /// Independent vowels after which the anusvara is a nasal consonant.
    anusvara_consonant_after: Vec<String>,
    /// Anusvara after a written vowel is the consonant n (not a nasal vowel).
    anusvara_n_after_vowel: bool,
    /// Consonants after which a word-final inherent vowel stays.
    final_keep_after: Vec<String>,
    /// ASCII digits are read as English numbers (the Nepali voice).
    pub ascii_digits_english: bool,
    /// Consonants written differently as the first member of a cluster
    /// (`ɾ=rr`: र् prints rr).
    cluster_first: Vec<(String, String)>,
    /// Second members of a final cluster after which the inherent vowel goes.
    final_drop_after_cluster: Vec<String>,
    /// A syllable right after a stressed one takes secondary stress when a
    /// doubled consonant separates them, or a cluster starting with one of
    /// these (`same` alone enables only the doubled case).
    secondary_after_cluster: Vec<String>,
    /// Vowels that do not take a secondary stress when a later non-final
    /// syllable can.
    secondary_avoid: Vec<String>,
    /// Letter names for spelling a word (`[spell]`).
    spell: crate::rules::CharMap<String>,
    /// In compatible mode, a word containing one of these is spelled out
    /// letter by letter (the reference's Assamese reading of nukta letters).
    spell_if: Vec<char>,
    /// Letter sequences that make the reference spell a word out.
    spell_if_seq: Vec<String>,
    /// A spell-out letter at the start of a word does not trigger spelling.
    spell_skip_initial: bool,
    /// Written vowels after which the anusvara is a nasal consonant before
    /// another consonant (and a nasal vowel at the end of a word).
    anusvara_consonant_after_sign: Vec<String>,
    /// Candrabindu after an inherent vowel is the consonant n.
    candrabindu_n: bool,
    /// Final deletion needs only some earlier vowel, not one right before
    /// the last consonant (`final-after = any`).
    final_after_any: bool,
    /// Word-final inherent vowels go before stress is assigned.
    delete_first: bool,
    /// Unstressed (function) words print the inherent vowel in its
    /// unstressed form.
    reduce_unstressed: bool,
    /// Syllabic consonants that never take secondary stress (the next
    /// syllable takes it instead).
    unstressable: Vec<String>,
    /// Words containing a letter name with its own stress marks keep only
    /// those, plus a secondary stress on the first syllable.
    explicit_stress: bool,
    /// Consonants whose (long) inherent vowel blocks the deletion of the
    /// next schwa.
    keep_after_schwa_of: Vec<String>,
    /// Vowels (letter names) after which a final consonant keeps its vowel.
    final_keep_after_vowel: Vec<String>,
    /// In compatible mode a word breaks after "virama + ZWJ".
    split_zwj: bool,
    /// Nasal vowels that do not make a syllable heavy.
    light_nasal: Vec<String>,
    /// An independent inherent vowel (अ) always takes at least secondary stress.
    independent_stressed: bool,
    /// The anusvara after an inherent vowel is always `n` (not homorganic).
    anusvara_plain_n: bool,
    /// The anusvara after the independent inherent vowel (अं) is a nasal vowel.
    anusvara_independent_nasal: bool,
    /// Voiceless stops that lengthen before a conjunct y.
    lengthen_before_y: Vec<String>,
    /// Second members of a final conjunct that keep the inherent vowel.
    final_release: Vec<String>,
    /// First members that block that release.
    final_release_not_after: Vec<String>,
    keep_schwa_near: Vec<String>,
    y_final: bool,
    visarga: Visarga,
    classes: Classes,
    pre: RuleSet,
    pre_compat: RuleSet,
    pre_improved: RuleSet,
    post: RuleSet,
    post_compat: RuleSet,
    post_improved: RuleSet,
    words: HashMap<String, (String, String)>,
    unstressed: HashMap<String, (u8, u8)>,
    /// Number words.
    pub numbers: Numbers,
}

fn list(meta: &HashMap<String, String>, k: &str) -> Vec<String> {
    meta.get(k)
        .map(|v| v.split_whitespace().map(str::to_string).collect())
        .unwrap_or_default()
}

impl Indic {
    /// Build a language from its file.
    pub fn from_text(text: &str) -> Result<Indic, String> {
        let f = LangFile::parse(text);
        let meta = f.meta();
        let tag = meta.get("tag").cloned().ok_or("missing tag")?;
        let base = u32::from_str_radix(meta.get("block").ok_or("missing block")?, 16)
            .map_err(|e| e.to_string())?;
        let mut letters = crate::rules::CharMap::default();
        let mut nukta = crate::rules::CharMap::default();
        for l in f.lines("letters") {
            let mut parts = l.split_whitespace();
            let Some(key) = parts.next() else { continue };
            let mut rest: Vec<String> = parts.map(str::to_string).collect();
            let attract = rest.last().is_some_and(|x| x == "stressed");
            if attract {
                rest.pop();
            }
            let (sym, kind_word): (Vec<String>, Option<&str>) =
                match rest.last().map(String::as_str) {
                    Some(
                        k @ ("dead" | "vowel" | "sign" | "consonant" | "ignore" | "anusvara"
                        | "virama" | "visarga" | "candrabindu"),
                    ) => (rest[..rest.len() - 1].to_vec(), Some(k)),
                    _ => (rest.clone(), None),
                };
            let cs: Vec<char> = key.chars().collect();
            if cs.len() == 2 && (cs[1] as u32).wrapping_sub(base) == 0x3c {
                nukta.insert(cs[0], sym);
                continue;
            }
            let c = cs[0];
            let kind = match kind_word {
                Some("dead") => Kind::Dead,
                Some("vowel") => Kind::Vowel,
                Some("sign") => Kind::Sign,
                Some("consonant") => Kind::Consonant,
                Some("ignore") => Kind::Ignore,
                Some("anusvara") => Kind::Anusvara,
                Some("virama") => Kind::Virama,
                Some("visarga") => Kind::Visarga,
                Some("candrabindu") => Kind::Candrabindu,
                _ => default_kind((c as u32).wrapping_sub(base))
                    .ok_or_else(|| format!("{tag}: no default class for {c:?}"))?,
            };
            letters.insert(c, Letter { kind, attract, sym });
        }
        for off in [0x01, 0x02, 0x03, 0x3c, 0x4d] {
            if let Some(c) = char::from_u32(base + off) {
                letters.entry(c).or_insert(Letter {
                    kind: default_kind(off).unwrap(),
                    attract: false,
                    sym: Vec::new(),
                });
            }
        }
        let inh = list(&meta, "inherent");
        let inherent = (
            inh.first().cloned().unwrap_or_else(|| "ə".into()),
            inh.get(1)
                .or(inh.first())
                .cloned()
                .unwrap_or_else(|| "ə".into()),
        );
        let stress = match meta.get("stress").map(String::as_str) {
            Some("weight") => Stress::Weight,
            Some("penult") => Stress::Penult,
            Some("initial-light-heavy") => Stress::InitialLightHeavy,
            Some("sanskrit") => Stress::Sanskrit,
            _ => Stress::Initial,
        };
        let deletion = match meta.get("deletion").map(String::as_str) {
            Some("medial") => Deletion::Medial,
            Some("final") => Deletion::Final,
            Some("prefinal") => Deletion::Prefinal,
            _ => Deletion::None,
        };
        let anusvara = match meta.get("anusvara").map(String::as_str) {
            Some("velar") => Anusvara::Velar,
            Some("homorganic") => Anusvara::Homorganic,
            Some("n") => Anusvara::N,
            Some("m") => Anusvara::M,
            _ => Anusvara::Split,
        };
        let mut classes = f.classes();
        let pre = f.rules("pre", &mut classes)?;
        let pre_compat = f.rules("pre.compat", &mut classes)?;
        let pre_improved = f.rules("pre.improved", &mut classes)?;
        let post = f.rules("post", &mut classes)?;
        let post_compat = f.rules("post.compat", &mut classes)?;
        let post_improved = f.rules("post.improved", &mut classes)?;
        Ok(Indic {
            tag,
            base,
            letters,
            nukta,
            inherent,
            stress,
            deletion,
            anusvara,
            final_secondary: meta.get("final-secondary").is_none_or(|v| v != "no"),
            geminate: list(&meta, "geminate"),
            geminate_aspirate: list(&meta, "geminate-aspirate"),
            trill: list(&meta, "trill"),
            anusvara_consonant_after: list(&meta, "anusvara-consonant-after"),
            anusvara_n_after_vowel: meta.get("anusvara-after-vowel").is_some_and(|v| v == "n"),
            final_keep_after: list(&meta, "final-keep-after"),
            ascii_digits_english: meta.get("ascii-digits").is_some_and(|v| v == "en"),
            cluster_first: list(&meta, "cluster-first")
                .iter()
                .filter_map(|x| {
                    x.split_once('=')
                        .map(|(a, b)| (a.to_string(), b.to_string()))
                })
                .collect(),
            final_drop_after_cluster: list(&meta, "final-drop-after-cluster"),
            secondary_after_cluster: list(&meta, "secondary-after-cluster"),
            secondary_avoid: list(&meta, "secondary-avoid"),
            spell: f
                .lines("spell")
                .iter()
                .filter_map(|l| {
                    let mut it = l.splitn(2, char::is_whitespace);
                    let c = it.next()?.chars().next()?;
                    Some((c, it.next()?.trim().to_string()))
                })
                .collect(),
            spell_if: meta
                .get("spell-if")
                .map(|v| v.chars().filter(|c| !c.is_whitespace()).collect())
                .unwrap_or_default(),
            spell_if_seq: list(&meta, "spell-if-sequence"),
            spell_skip_initial: meta.get("spell-skip-initial").is_some_and(|v| v == "yes"),
            anusvara_consonant_after_sign: list(&meta, "anusvara-consonant-after-sign"),
            candrabindu_n: meta
                .get("candrabindu-after-inherent")
                .is_some_and(|v| v == "n"),
            final_after_any: meta.get("final-after").is_some_and(|v| v == "any"),
            light_nasal: list(&meta, "light-nasal"),
            split_zwj: meta.get("split-zwj").is_some_and(|v| v == "yes"),
            final_keep_after_vowel: list(&meta, "final-keep-after-vowel"),
            keep_after_schwa_of: list(&meta, "keep-after-schwa-of"),
            explicit_stress: meta.get("explicit-stress").is_some_and(|v| v == "yes"),
            unstressable: list(&meta, "unstressable"),
            delete_first: meta.get("delete-before-stress").is_some_and(|v| v == "yes"),
            reduce_unstressed: meta
                .get("unstressed-inherent")
                .is_some_and(|v| v == "reduce"),
            independent_stressed: meta
                .get("independent-inherent")
                .is_some_and(|v| v == "stressed"),
            anusvara_plain_n: meta.get("anusvara-place").is_some_and(|v| v == "n"),
            anusvara_independent_nasal: meta
                .get("anusvara-after-independent")
                .is_some_and(|v| v == "nasal"),
            lengthen_before_y: list(&meta, "lengthen-before-y"),
            final_release: list(&meta, "final-release"),
            final_release_not_after: list(&meta, "final-release-not-after"),
            keep_schwa_near: list(&meta, "keep-schwa-after"),
            y_final: meta.get("y-final").is_some_and(|v| v == "yes"),
            visarga: match meta.get("visarga").map(String::as_str) {
                Some("geminate") => Visarga::Geminate,
                Some("silent") => Visarga::Silent,
                _ => Visarga::H,
            },
            classes,
            pre,
            pre_compat,
            pre_improved,
            post,
            post_compat,
            post_improved,
            words: langdef::words(&f),
            unstressed: langdef::unstressed(&f),
            numbers: Numbers::parse(&f),
        })
    }

    /// True for a letter of this language's script block.
    pub fn owns(&self, c: char) -> bool {
        let o = (c as u32).wrapping_sub(self.base);
        o < 0x80
    }

    fn kind(&self, c: char) -> Option<Kind> {
        self.letters.get(&c).map(|l| l.kind)
    }

    fn homorganic(&self, next: Option<&Ph>) -> &'static str {
        let Some(n) = next.filter(|p| !p.vowel) else {
            return "n";
        };
        match n.s.chars().next().unwrap_or(' ') {
            'k' | 'ɡ' | 'x' | 'ɣ' | 'q' => "ŋ",
            'c' | 'ɟ' | 'j' => "ɲ",
            'ʈ' | 'ɖ' | 'ɽ' => "ɳ",
            'p' | 'b' | 'm' | 'f' => "m",
            _ => "n",
        }
    }

    /// Underlying sounds of a word.
    fn decode(&self, word: &[char]) -> Vec<Ph> {
        let mut out: Vec<Ph> = Vec::with_capacity(word.len() + 4);
        let mut i = 0;
        let mut after_virama = false;
        // Anusvara positions resolved once the next consonant is known.
        let mut pending_nasal: Vec<(usize, bool)> = Vec::new();
        while i < word.len() {
            let c = word[i];
            let Some(letter) = self.letters.get(&c) else {
                i += 1;
                after_virama = false;
                continue;
            };
            match letter.kind {
                Kind::Consonant | Kind::Dead => {
                    i += 1;
                    let mut sym = &letter.sym;
                    if self.kind(word.get(i).copied().unwrap_or(' ')) == Some(Kind::Nukta) {
                        if let Some(s) = self.nukta.get(&c) {
                            sym = s;
                        }
                        i += 1;
                    }
                    for (k, s) in sym.iter().enumerate() {
                        let mut p = Ph::c(s);
                        p.conjunct = after_virama || k > 0;
                        out.push(p);
                    }
                    after_virama = false;
                    if letter.kind == Kind::Dead {
                        continue;
                    }
                    match word.get(i).and_then(|n| self.letters.get(n)) {
                        Some(l) if l.kind == Kind::Virama => {
                            after_virama = true;
                            i += 1;
                        }
                        Some(l) if l.kind == Kind::Sign => {
                            self.push_sign(&l.sym, l.attract, &mut out);
                            i += 1;
                        }
                        _ => {
                            let mut p = Ph::v(&self.inherent.0);
                            p.schwa = true;
                            out.push(p);
                        }
                    }
                }
                Kind::Vowel | Kind::Sign => {
                    after_virama = false;
                    let start = out.len();
                    self.push_sign(&letter.sym, letter.attract, &mut out);
                    if letter.kind == Kind::Vowel {
                        for p in out[start..].iter_mut().filter(|p| p.vowel) {
                            p.independent = true;
                            // The independent inherent vowel (अ) behaves like one.
                            p.schwa = p.s == self.inherent.0
                                || p.s == self.inherent.1
                                || (self.explicit_stress && starts_with_schwa(&p.s));
                        }
                    }
                    i += 1;
                }
                Kind::Anusvara => {
                    after_virama = false;
                    if out.last().is_some_and(|p| p.vowel) {
                        pending_nasal.push((out.len(), true));
                        out.push(Ph::c("ŋ"));
                    }
                    i += 1;
                }
                Kind::Candrabindu => {
                    after_virama = false;
                    // The vowel, or the vowel before a glide (ঔ + ঁ).
                    let n = out.len();
                    let at = if n >= 2
                        && !out[n - 1].vowel
                        && !out[n - 1].conjunct
                        && out[n - 2].vowel
                    {
                        n - 2
                    } else {
                        n.wrapping_sub(1)
                    };
                    if let Some(v) = out.get_mut(at).filter(|p| p.vowel) {
                        if v.schwa && self.candrabindu_n {
                            // After the inherent vowel: a nasal consonant.
                            out.push(Ph::c("n"));
                        } else {
                            v.nasal = true;
                        }
                    }
                    i += 1;
                }
                Kind::Visarga => {
                    after_virama = false;
                    let at_end = i + 1 == word.len();
                    let next_cons = word
                        .get(i + 1)
                        .and_then(|n| self.letters.get(n))
                        .filter(|l| l.kind == Kind::Consonant);
                    if at_end || self.visarga == Visarga::H {
                        out.push(Ph::c("h"));
                    } else if self.visarga == Visarga::Silent {
                    } else if let Some(l) = next_cons {
                        let mut p = Ph::c(&l.sym[0]);
                        p.conjunct = false;
                        out.push(p);
                        after_virama = true;
                    }
                    i += 1;
                }
                Kind::Virama => {
                    after_virama = true;
                    i += 1;
                }
                Kind::Nukta | Kind::Ignore => i += 1,
            }
        }
        // Resolve anusvara now that the following sound is known.
        for (at, _) in pending_nasal.into_iter().rev() {
            let next = out.get(at + 1).cloned();
            let prev_schwa = at > 0 && out[at - 1].schwa;
            let prev_independent_short = at > 0 && out[at - 1].independent;
            let _ = prev_independent_short;
            match self.anusvara {
                Anusvara::Velar => {}
                Anusvara::N => out[at].s = "n".into(),
                Anusvara::M => out[at].s = "m".into(),
                Anusvara::Homorganic => {
                    out[at].s = if next.is_none() {
                        "m".into()
                    } else {
                        self.homorganic(next.as_ref()).into()
                    };
                }
                Anusvara::Split => {
                    // After the inherent vowel, or a short independent vowel
                    // (उं), a nasal consonant; after other vowels a nasal vowel.
                    let short_indep = at > 0
                        && out[at - 1].independent
                        && self
                            .anusvara_consonant_after
                            .iter()
                            .any(|v| *v == out[at - 1].s);
                    let sign_cons = at > 0
                        && next.as_ref().is_some_and(|n| !n.vowel)
                        && self
                            .anusvara_consonant_after_sign
                            .iter()
                            .any(|v| *v == out[at - 1].s);
                    let indep_nasal = self.anusvara_independent_nasal
                        && at > 0
                        && out[at - 1].independent
                        && out[at - 1].schwa;
                    let prev_schwa = (prev_schwa || short_indep || sign_cons) && !indep_nasal;
                    if prev_schwa && next.as_ref().is_some_and(|n| !n.vowel) {
                        out[at].s = if self.anusvara_plain_n {
                            "n".into()
                        } else {
                            self.homorganic(next.as_ref()).into()
                        };
                    } else if prev_schwa {
                        out[at].s = "n".into();
                    } else if self.anusvara_n_after_vowel {
                        out[at].s = self.homorganic(next.as_ref()).into();
                    } else {
                        out.remove(at);
                        out[at - 1].nasal = true;
                    }
                }
            }
        }
        out
    }

    fn push_sign(&self, sym: &[String], attract: bool, out: &mut Vec<Ph>) {
        // Symbols before the vowel are consonants (ृ = "ɾ ɪ"); symbols written
        // with a leading `+` follow the vowel as a glide (ঐ = "o +j").
        let vowel_at = sym.iter().rposition(|s| !s.starts_with('+')).unwrap_or(0);
        for (k, s) in sym.iter().enumerate() {
            if k == vowel_at {
                let mut v = Ph::v(s);
                v.attract = attract;
                out.push(v);
            } else {
                let mut p = Ph::c(s.trim_start_matches('+'));
                p.conjunct = k < vowel_at;
                out.push(p);
            }
        }
    }

    fn geminates(&self, ph: &mut Vec<Ph>) {
        let mut i = 0;
        while i + 1 < ph.len() {
            if !ph[i].vowel && !ph[i + 1].vowel && ph[i + 1].conjunct {
                let (a, b) = (ph[i].s.clone(), ph[i + 1].s.clone());
                let same = a == b && self.geminate.contains(&a);
                let aspirated =
                    b.strip_suffix('ʰ') == Some(a.as_str()) && self.geminate_aspirate.contains(&a);
                if same || aspirated {
                    ph[i].s = b;
                    ph[i].long = true;
                    ph.remove(i + 1);
                    continue;
                }
                if b == "j" && !ph[i].long && self.lengthen_before_y.contains(&a) {
                    ph[i].long = true;
                }
            }
            i += 1;
        }
    }

    fn assign_stress(&self, ph: &mut [Ph]) {
        let vs: Vec<usize> = (0..ph.len()).filter(|&i| ph[i].vowel).collect();
        if vs.is_empty() {
            return;
        }
        let n = vs.len();
        if self.explicit_stress && vs.iter().any(|&i| ph[i].s.contains('ˈ')) {
            // Letter names keep their own marks (stress 3: nothing printed);
            // the other syllables alternate around them.
            for &i in &vs {
                if ph[i].s.contains('ˈ') || ph[i].s.contains('ˌ') {
                    ph[i].stress = 3;
                }
            }
            for k in 0..n {
                let prev = k > 0 && ph[vs[k - 1]].stress != 0;
                let next = k + 1 < n
                    && ph[vs[k + 1]].stress != 0
                    && !(ph[vs[k + 1]].stress == 3 && name_starts_unstressed(&ph[vs[k + 1]].s));
                if ph[vs[k]].stress == 0 && !prev && !next {
                    ph[vs[k]].stress = 2;
                }
            }
            return;
        }
        let weight = |k: usize| -> u8 {
            let i = vs[k];
            let mut w = 0u8;
            let diphthong = ph[i]
                .s
                .chars()
                .filter(|c| !('\u{300}'..='\u{36f}').contains(c))
                .count()
                > 1;
            let nasal = ph[i].nasal && !self.light_nasal.contains(&ph[i].s);
            if ph[i].long || nasal || diphthong {
                w += 1;
            }
            let end = vs.get(k + 1).copied().unwrap_or(ph.len());
            let cons = &ph[i + 1..end];
            let closed =
                cons.len() >= 2 || cons.iter().any(|c| c.long) || (k + 1 == n && !cons.is_empty());
            if closed {
                w += 1;
            }
            w
        };
        let primary = match self.stress {
            _ if vs.iter().any(|&i| ph[i].attract) => {
                vs.iter().position(|&i| ph[i].attract).unwrap_or(0)
            }
            Stress::Initial => 0,
            Stress::Sanskrit => {
                if n >= 2 && weight(n - 2) > 0 {
                    n - 2
                } else if n >= 3 && weight(n - 3) > 0 {
                    n - 3
                } else {
                    n.saturating_sub(4)
                }
            }
            Stress::Penult => n.saturating_sub(2),
            Stress::InitialLightHeavy => {
                if n > 1 && !ph[vs[0]].long && ph[vs[1]].long {
                    1
                } else {
                    0
                }
            }
            Stress::Weight => {
                let mut primary = 0usize;
                if n > 1 {
                    let mut best = 0u8;
                    for k in 0..n - 1 {
                        let w = weight(k);
                        if w > best || (w == best && w > 0) {
                            best = w;
                            primary = k;
                        }
                    }
                    if weight(n - 1) >= 2 && best < 2 {
                        primary = n - 1;
                    }
                }
                primary
            }
        };
        ph[vs[primary]].stress = 1;
        if self.independent_stressed {
            for &i in &vs {
                if ph[i].independent && ph[i].schwa && !ph[i].nasal && ph[i].stress == 0 {
                    ph[i].stress = 2;
                }
            }
        }
        for k in 0..n {
            if k == primary || (k + 1 == n && !self.final_secondary) {
                continue;
            }
            // A doubled consonant (or a cluster starting with one of the
            // listed consonants) after a stressed syllable.
            let cluster_before = k > 0 && {
                let between = &ph[vs[k - 1] + 1..vs[k]];
                let listed = |c: &str| self.secondary_after_cluster.iter().any(|x| x == c);
                between.len() == 2
                    && ((between[0].s == between[1].s && (listed("same") || listed(&between[0].s)))
                        || (between[0].s != between[1].s && listed(&between[0].s)))
                    || between.iter().any(|c| c.long)
            };
            let prev = k > 0
                && ph[vs[k - 1]].stress != 0
                && !(!self.secondary_after_cluster.is_empty() && cluster_before);
            let next = k + 1 < n && ph[vs[k + 1]].stress != 0;
            // Some voices pass over certain vowels when a later, non-final
            // syllable can take the secondary stress instead.
            let avoid = self.secondary_avoid.iter().any(|v| *v == ph[vs[k]].s)
                && k + 2 < n
                && !(k > 0 && ph[vs[k - 1]].stress != 0);
            let unstressable = self.unstressable.iter().any(|v| *v == ph[vs[k]].s);
            if !prev && !next && !avoid && !unstressable {
                ph[vs[k]].stress = 2;
            }
        }
    }

    fn final_release(&self, c1: &Ph, c2: &Ph) -> bool {
        let nasal = matches!(c2.s.as_str(), "n" | "m" | "ɲ" | "ŋ");
        self.final_release.contains(&c2.s)
            && !(nasal && self.final_release_not_after.contains(&c1.s))
    }

    fn delete(&self, ph: &mut Vec<Ph>) {
        let total = ph.iter().filter(|p| p.vowel).count();
        match self.deletion {
            Deletion::None => {}
            Deletion::Final => {
                let n = ph.len();
                if n >= 3
                    && ph[n - 1].schwa
                    && !ph[n - 1].independent
                    && ph[n - 2].conjunct
                    && self
                        .final_drop_after_cluster
                        .iter()
                        .any(|k| *k == ph[n - 2].s)
                {
                    ph.pop();
                    return;
                }
                if n >= 3
                    && ph[n - 1].schwa
                    && !ph[n - 1].independent
                    && !ph[n - 2].vowel
                    && (!ph[n - 2].conjunct || self.final_after_any)
                    && (ph[n - 3].vowel
                        || (self.final_after_any && ph[..n - 2].iter().any(|p| p.vowel)))
                    && ph[n - 1].stress != 1
                    && !self.final_keep_after.iter().any(|k| *k == ph[n - 2].s)
                {
                    ph.pop();
                }
            }
            Deletion::Prefinal => {
                let n = ph.len();
                if n >= 2
                    && ph[n - 1].schwa
                    && !ph[n - 1].independent
                    && ph[n - 1].stress != 1
                    && total > 1
                {
                    let release =
                        n >= 3 && ph[n - 2].conjunct && self.final_release(&ph[n - 3], &ph[n - 2]);
                    if !release {
                        ph.pop();
                        let n = ph.len();
                        if n >= 4
                            && ph[n - 2].schwa
                            && !ph[n - 2].independent
                            && ph[n - 2].stress != 1
                            && !ph[n - 1].vowel
                            && !ph[n - 3].vowel
                            && ph[n - 4].vowel
                        {
                            ph.remove(n - 2);
                        }
                    }
                }
            }
            Deletion::Medial => {
                let mut i = 0;
                while i < ph.len() {
                    if !ph[i].schwa || ph[i].stress == 1 || ph[i].independent {
                        i += 1;
                        continue;
                    }
                    if i + 1 == ph.len() {
                        let release = i >= 2
                            && ph[i - 1].conjunct
                            && self.final_release(&ph[i - 2], &ph[i - 1]);
                        let y_glide =
                            i >= 2 && ph[i - 1].s == "j" && !ph[i - 1].conjunct && ph[i - 2].schwa;
                        let keep = i >= 1
                            && !ph[i - 1].conjunct
                            && self.final_keep_after.iter().any(|k| *k == ph[i - 1].s)
                            || i >= 2
                                && self
                                    .final_keep_after_vowel
                                    .iter()
                                    .any(|k| *k == ph[i - 2].s)
                                && self.final_release.contains(&ph[i - 1].s);
                        if total > 1 && !release && !(self.y_final && y_glide) && !keep {
                            ph.remove(i);
                            continue;
                        }
                        i += 1;
                        continue;
                    }
                    if i > 0 && self.keep_schwa_near.iter().any(|s| *s == ph[i - 1].s) {
                        i += 1;
                        continue;
                    }
                    let protected = i >= 3
                        && ph[i - 2].schwa
                        && self.keep_after_schwa_of.iter().any(|k| *k == ph[i - 3].s);
                    let vc = i >= 2 && ph[i - 2].vowel && !ph[i - 1].vowel && !protected;
                    let next_vowel = ph[i + 1..].iter().find(|p| p.vowel);
                    let next_is_schwa = next_vowel.is_some_and(|p| p.schwa);
                    if vc && !next_is_schwa && next_vowel.is_some() {
                        ph.remove(i);
                        continue;
                    }
                    i += 1;
                }
            }
        }
    }

    fn render(&self, ph: &[Ph]) -> String {
        let mut out = String::with_capacity(ph.len() * 3);
        // A lone consonant letter keeps the unstressed form of its vowel.
        let single = ph.len() == 2 && ph[1].schwa;
        for (i, p) in ph.iter().enumerate() {
            if p.vowel {
                match p.stress {
                    1 => out.push('ˈ'),
                    2 => out.push('ˌ'),
                    _ => {}
                }
                if p.schwa && !p.independent {
                    out.push_str(if p.stress == 1 && !single {
                        &self.inherent.1
                    } else {
                        &self.inherent.0
                    });
                } else if p.schwa && p.stress == 1 {
                    // The independent inherent vowel (अ) under primary stress.
                    out.push_str(&self.inherent.1);
                } else {
                    out.push_str(&p.s);
                }
                if p.nasal {
                    out.push('\u{303}');
                }
                if p.long {
                    out.push('ː');
                }
            } else {
                let trill = (p.s == "ɾ" || p.s == "ɹ")
                    && ph
                        .get(i + 1)
                        .is_some_and(|q| !q.vowel && q.conjunct && self.trill.contains(&q.s));
                let first = ph.get(i + 1).is_some_and(|q| !q.vowel && q.conjunct);
                let mapped = if first {
                    self.cluster_first
                        .iter()
                        .find(|(a, _)| *a == p.s)
                        .map(|x| x.1.as_str())
                } else {
                    None
                };
                out.push_str(if trill { "r" } else { mapped.unwrap_or(&p.s) });
                if p.long {
                    out.push('ː');
                }
            }
        }
        out
    }

    /// Pronunciation of one word (letters of this script only).
    pub fn word(&self, word: &str, compat: bool) -> String {
        self.word_as(word, compat, false)
    }

    /// [`Indic::word`]; `number` says the word was written as digits, which
    /// selects `@word` entries of the exceptions lexicon first.
    pub fn word_as(&self, word: &str, compat: bool, number: bool) -> String {
        if number {
            if let Some((c, i)) = self.words.get(&format!("@{word}")) {
                return if compat { c } else { i }.clone();
            }
        }
        // Running text repeats its words: remember recent answers.
        let space = (self as *const Indic as usize) ^ usize::from(compat);
        crate::data::memo(space, word, || Some(self.word_uncached(word, compat)))
            .unwrap_or_default()
    }

    fn word_uncached(&self, word: &str, compat: bool) -> String {
        if compat && self.split_zwj {
            // The reference breaks a word after "consonant + virama + ZWJ".
            if let Some(at) = word.find("\u{200d}") {
                let (a, b) = word.split_at(at);
                let b = &b[3..];
                if a.chars().last().and_then(|c| self.kind(c)) == Some(Kind::Virama)
                    && !b.is_empty()
                {
                    let left = self.word_uncached(a, compat);
                    let right = self.word_uncached(b, compat);
                    return format!("{left} {right}").trim().to_string();
                }
            }
        }
        if let Some((c, i)) = self.words.get(word) {
            let w = if compat { c } else { i };
            if !w.is_empty() {
                return w.clone();
            }
        }
        let cs: Vec<char> = word
            .chars()
            .filter(|c| !matches!(c, '\u{200c}' | '\u{200d}'))
            .collect();
        let mut letters = String::with_capacity(word.len());
        self.pre.apply_to(&cs, &self.classes, &mut letters);
        let mode = if compat {
            &self.pre_compat
        } else {
            &self.pre_improved
        };
        if !mode.is_empty() {
            letters = mode.apply(&letters, &self.classes);
        }
        let cs: Vec<char> = letters.chars().collect();
        let mut ph = self.decode(&cs);
        if ph.is_empty() {
            return String::new();
        }
        self.geminates(&mut ph);
        let n = ph.len();
        let y_final = self.y_final
            && n >= 4
            && ph[n - 1].schwa
            && ph[n - 2].s == "j"
            && !ph[n - 2].conjunct
            && ph[n - 3].schwa
            && !ph[n - 3].independent;
        if self.delete_first {
            self.delete(&mut ph);
        }
        self.assign_stress(&mut ph);
        if y_final {
            let v = ph.last_mut().unwrap();
            v.s = "ɛ".into();
            v.schwa = false;
        }
        if !self.delete_first {
            self.delete(&mut ph);
        }
        if y_final {
            let n = ph.len();
            ph.swap(n - 1, n - 2);
        }
        let raw = self.render(&ph);
        let mut s = self.post.apply(&raw, &self.classes);
        let mode = if compat {
            &self.post_compat
        } else {
            &self.post_improved
        };
        if !mode.is_empty() {
            s = mode.apply(&s, &self.classes);
        }
        s
    }

    /// Phonemes for a run of words of this language inside one clause.
    pub fn span(
        &self,
        items: &[crate::norm::Item],
        clause_final: bool,
        compat: bool,
        lang: crate::Lang,
    ) -> Vec<crate::WordOut> {
        let n = items.len();
        let mut out: Vec<crate::WordOut> = Vec::with_capacity(n);
        for (i, it) in items.iter().enumerate() {
            let spelled = self
                .spell_if_seq
                .iter()
                .any(|q| it.text.contains(q.as_str()))
                || if self.spell_skip_initial {
                    it.text.chars().skip(1).any(|c| self.spell_if.contains(&c))
                } else {
                    it.text.chars().any(|c| self.spell_if.contains(&c))
                };
            if compat && spelled {
                // Spelled out letter by letter, each name read as words.
                for c in it.text.chars() {
                    let name = self.spell.get(&c).cloned().unwrap_or_else(|| c.to_string());
                    for w in name.split_whitespace() {
                        // "/ipa": phonemes as written.
                        if let Some(ipa) = w.strip_prefix('/') {
                            out.push(crate::WordOut::new(w, ipa.to_string(), lang, false));
                            continue;
                        }
                        // "a+b": read as parts, joined, all but the last demoted.
                        let parts: Vec<&str> = w.split('+').collect();
                        let mut ipa = String::new();
                        for (k, part) in parts.iter().enumerate() {
                            let p = self.word_as(part, compat, false);
                            if k + 1 < parts.len() {
                                ipa.push_str(&p.replace('ˈ', "ˌ"));
                            } else {
                                ipa.push_str(&p);
                            }
                        }
                        if !ipa.is_empty() {
                            out.push(crate::WordOut::new(w, ipa, lang, false));
                        }
                    }
                }
                continue;
            }
            let mut ipa = self.word_as(&it.text, compat, it.generated);
            if it.number_tail && self.numbers.demote_tail {
                ipa = ipa.replace('ˈ', "ˌ");
            }
            if let Some(&(mid, fin)) = self.unstressed.get(it.text.as_str()) {
                let level = if i + 1 == n && clause_final { fin } else { mid };
                if self.reduce_unstressed && level != b'p' {
                    let stressed = format!("ˈ{}", self.inherent.1);
                    ipa = ipa.replace(&stressed, &format!("ˈ{}", self.inherent.0));
                }
                ipa = match level {
                    b'0' => crate::ipa::strip_stress(&ipa),
                    b's' => ipa.replace('ˈ', "ˌ"),
                    _ => ipa,
                };
            }
            // A word-initial glide ʲ merges into a preceding i, ɪ or j.
            if ipa.trim_start_matches(['ˈ', 'ˌ']).starts_with('ʲ') {
                let prev_front = out.last().is_some_and(|w: &crate::WordOut| {
                    matches!(
                        w.ipa.trim_end_matches('ː').chars().last(),
                        Some('i' | 'ɪ' | 'j')
                    )
                });
                if prev_front {
                    ipa = ipa.replacen('ʲ', "", 1);
                }
            }
            out.push(crate::WordOut::new(&it.text, ipa, lang, it.glue));
        }
        out
    }

    /// The ASCII digit value of a digit of this script.
    pub fn digit(&self, c: char) -> Option<u8> {
        let o = (c as u32).wrapping_sub(self.base);
        (0x66..=0x6f).contains(&o).then(|| (o - 0x66) as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoder_basics() {
        let l = Indic::from_text(
            "[meta]\ntag = xx\nblock = 0900\nstress = initial\ndeletion = final\ninherent = ɔ ɔ\n\
             [letters]\nक k\nम m\nल l\nा a\nि i\n",
        )
        .unwrap();
        assert_eq!(l.word("कमल", true), "kˈɔmɔl");
        assert_eq!(l.word("कमला", true), "kˈɔmɔlˌa");
        assert_eq!(l.word("क", true), "kˈɔ");
    }
}
