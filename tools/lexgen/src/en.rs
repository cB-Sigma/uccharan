//! ARPABET (CMUdict) → uccharan's English phoneme strings.
//!
//! The target notation is the IPA that espeak-style voices were trained on
//! (stress mark immediately before the vowel, `ɚ`, `ᵻ`, flap `ɾ`, …). Every
//! rule here was written by hand from the observable behaviour of the target
//! notation.

use crate::cmudict::{Entry, Seg};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Accent {
    Us,
    Gb,
}

fn consonant(p: &str) -> &'static str {
    match p {
        "B" => "b",
        "CH" => "tʃ",
        "D" => "d",
        "DH" => "ð",
        "F" => "f",
        "G" => "ɡ",
        "HH" => "h",
        "JH" => "dʒ",
        "K" => "k",
        "L" => "l",
        "M" => "m",
        "N" => "n",
        "NG" => "ŋ",
        "P" => "p",
        "R" => "ɹ",
        "S" => "s",
        "SH" => "ʃ",
        "T" => "t",
        "TH" => "θ",
        "V" => "v",
        "W" => "w",
        "Y" => "j",
        "Z" => "z",
        "ZH" => "ʒ",
        _ => "",
    }
}

/// One output phoneme with the bookkeeping the later passes need.
#[derive(Clone, Debug)]
pub struct Ph {
    pub s: String,
    pub stress: u8,
    pub vowel: bool,
    pub li: usize,
    /// First letter of the spelling (0 when unknown).
    pub g0: u8,
}

/// True when the phonemes after position `i` are exactly `pat` (to the end of
/// the word when `to_end`).
fn follows(segs: &[Seg], i: usize, pat: &[&str], to_end: bool) -> bool {
    let rest = &segs[(i + 1).min(segs.len())..];
    rest.len() >= pat.len()
        && rest.iter().zip(pat).all(|(s, p)| s.p == *p)
        && (!to_end || rest.len() == pat.len())
}

/// Reduced `i`: the near-close central `ᵻ` before a small set of suffixes
/// (-ity, -ify, -ible, -ization, -inate, -itate, -icate, -itive, -imate),
/// plain `ɪ` otherwise.
fn reduced_i(segs: &[Seg], i: usize) -> &'static str {
    let f = |pat: &[&str], end: bool| follows(segs, i, pat, end);
    if f(&["T", "IY"], true)
        || f(&["T", "IY", "Z"], true)
        || f(&["F", "AY"], false)
        || f(&["B", "AH", "L"], false)
        || f(&["Z", "EY", "SH"], false)
        || f(&["N", "EY"], false)
        || f(&["T", "EY"], false) && !f(&["T", "EY", "SH"], false)
        || f(&["K", "EY", "T"], false)
        || f(&["T", "IH", "V"], false)
        || f(&["T", "AH", "V"], false)
        || f(&["M", "AH", "T"], true)
    {
        "ᵻ"
    } else {
        "ɪ"
    }
}

pub fn convert(e: &Entry, accent: Accent) -> Vec<Ph> {
    let segs = &e.segs;
    let n = segs.len();
    let us = accent == Accent::Us;
    let mut out: Vec<Ph> = Vec::with_capacity(n + 2);

    // ---- stress pattern -------------------------------------------------
    let vowels: Vec<usize> = (0..n).filter(|&i| segs[i].is_vowel()).collect();
    let mut stress: Vec<u8> = vowels.iter().map(|&i| segs[i].stress).collect();
    if !stress.contains(&1) {
        if let Some(k) = stress.iter().position(|&s| s == 2) {
            stress[k] = 1;
        } else if let Some(s) = stress.first_mut() {
            *s = 1;
        }
    }
    // Only the first primary stress survives (nineteen, outside).
    if let Some(p) = stress.iter().position(|&s| s == 1) {
        for s in stress.iter_mut().skip(p + 1) {
            if *s == 1 {
                *s = 2;
            }
        }
    }
    // A secondary stress directly next to the primary one is not marked.
    let mut demoted = vec![false; stress.len()];
    if let Some(p) = stress.iter().position(|&s| s == 1) {
        for k in [p.wrapping_sub(1), p + 1] {
            if k < stress.len() && stress[k] == 2 {
                stress[k] = 0;
                demoted[k] = true;
            }
        }
    }
    // Word-final "happy" vowel is never stressed.
    if let (Some(&li), Some(s)) = (vowels.last(), stress.last_mut()) {
        if li + 1 == n && segs[li].p == "IY" && *s == 2 {
            *s = 0;
        }
    }
    let vindex = |i: usize| vowels.iter().position(|&x| x == i).unwrap_or(0);

    let mut skip = false;
    for (i, seg) in segs.iter().enumerate() {
        if skip {
            skip = false;
            continue;
        }
        let prev: Option<&Seg> = if i > 0 { segs.get(i - 1) } else { None };
        let next: Option<&Seg> = segs.get(i + 1);
        let next_p = next.map_or("", |s| s.p);
        let next2_p = segs.get(i + 2).map_or("", |s| s.p);
        let prev_p = prev.map_or("", |s| s.p);
        let last = i + 1 == n;
        let g = seg.g.as_str();
        let g0 = g.as_bytes().first().copied().unwrap_or(b' ');
        let next_is_vowel = next.is_some_and(|s| s.is_vowel());

        if !seg.is_vowel() {
            let mut c = consonant(seg.p).to_string();
            match seg.p {
                // n assimilates to a following velar: income, engaged, uncle.
                "N" if matches!(next_p, "K" | "G") => c = "ŋ".into(),
                // million, opinion, Australia: the glide spelled <i> is a short vowel.
                "Y" if g0 == b'i'
                    && i > 0
                    && matches!(prev_p, "L" | "N")
                    && next_p == "AH"
                    && next.is_some_and(|s| s.stress == 0) =>
                {
                    let tail_ok = matches!(next2_p, "N" | "L" | "") || i + 2 >= n;
                    out.push(Ph {
                        s: if tail_ok { "i" } else { "ɪ" }.into(),
                        stress: 0,
                        vowel: true,
                        li: seg.li,
                        g0,
                    });
                    continue;
                }
                _ => {}
            }
            out.push(Ph {
                s: c,
                stress: 9,
                vowel: false,
                li: seg.li,
                g0,
            });
            continue;
        }

        let k = vindex(i);
        let st = stress[k];
        let stressed = st != 0;
        let first_syl = k == 0;
        let was_secondary = demoted[k];
        let next_stressed = stress.get(k + 1).is_some_and(|&s| s != 0);
        let before_r = next_p == "R";
        let r_then_vowel = before_r && segs.get(i + 2).is_some_and(|s| s.is_vowel());

        let v: String = match seg.p {
            "AA" => {
                if last && !stressed && !was_secondary && g0 == b'a' && vowels.len() > 1 {
                    "ə".into()
                } else if before_r {
                    "ɑː".into()
                } else if matches!(g, "au" | "aw" | "augh" | "ough" | "ou" | "oa") {
                    "ɔː".into()
                } else if g0 == b'a'
                    && next_p == "L"
                    && (g.starts_with("al")
                        || seg_letters_after(e, seg.li).starts_with("ll")
                        || seg_letters_after(e, seg.li).starts_with("lk"))
                    && !seg_letters_after(e, seg.li).starts_with("lm")
                {
                    lot_a(e, seg, prev_p).into()
                } else if g0 == b'a' && prev_p == "W" {
                    lot_a(e, seg, prev_p).into()
                } else if g0 == b'o' && cloth(next_p, next2_p) {
                    "ɔ".into()
                } else if us || g0 != b'o' {
                    "ɑː".into()
                } else {
                    "ɒ".into()
                }
            }
            "AE" => {
                if !stressed
                    && i == 0
                    && seg_letters_after(e, seg.li).starts_with('d')
                    && next_stressed
                {
                    "ɐ".into()
                } else {
                    "æ".into()
                }
            }
            "AH" => {
                if stressed || was_secondary {
                    "ʌ".into()
                } else {
                    match g0 {
                        b'a' => {
                            if last {
                                "ə"
                            } else if next_p == "JH" && i + 2 == n {
                                "ɪ" // -age
                            } else if i == 0 {
                                "ɐ"
                            } else {
                                "ə"
                            }
                        }
                        b'i' if g == "i" || g == "ie" => {
                            if (next_p == "L" && i + 2 == n)
                                || next_p.is_empty()
                                || (next_p == "N" && g != "i")
                            {
                                "ə"
                            } else {
                                reduced_i(segs, i)
                            }
                        }
                        b'e' if g == "e" || g == "ed" || g == "es" => {
                            if (next_p == "Z" && i + 2 == n)
                                || (next_p == "D" && i + 2 == n && matches!(prev_p, "T" | "D"))
                                || follows(segs, i, &["D", "L", "IY"], true)
                            {
                                "ᵻ"
                            } else if follows(segs, i, &["S", "T"], true)
                                || follows(segs, i, &["T"], true)
                                || follows(segs, i, &["T", "S"], true)
                            {
                                "ɪ"
                            } else if first_syl && next_stressed {
                                "ᵻ"
                            } else if !first_syl
                                && !last
                                && i + 2 < n
                                && !matches!(next_p, "N" | "L" | "M" | "R" | "S")
                                && g == "e"
                            {
                                "ɪ"
                            } else {
                                "ə"
                            }
                        }
                        b'u' => {
                            if prev_p == "Y" {
                                if next_p == "L" {
                                    "ʊ"
                                } else {
                                    "uː"
                                }
                            } else if first_syl && g == "u" && i <= 2 && next_stressed && (i == 0) {
                                "ʌ"
                            } else {
                                "ə"
                            }
                        }
                        _ => "ə",
                    }
                    .into()
                }
            }
            "AO" => {
                if before_r {
                    let after = seg_letters_after(e, seg.li);
                    let short_gb = !us
                        && r_then_vowel
                        && g == "o"
                        && (after.starts_with("rr")
                            || after.starts_with("rit")
                            || after.starts_with("ric")
                            || after.starts_with("rid")
                            || after.starts_with("rest")
                            || after.starts_with("reig")
                            || after.starts_with("ran"));
                    if short_gb {
                        "ɒ".into()
                    } else if us && after.starts_with("rr") {
                        "ɔ".into()
                    } else {
                        "ɔː".into()
                    }
                } else if g0 == b'o' && !matches!(g, "ou" | "oa" | "ough") {
                    if cloth(next_p, next2_p) {
                        "ɔ".into()
                    } else if us {
                        "ɑː".into()
                    } else {
                        "ɒ".into()
                    }
                } else if g0 == b'a' && !matches!(g, "au" | "aw" | "augh") {
                    lot_a(e, seg, prev_p).into()
                } else if matches!(g, "au") && next_p == "L" && next2_p == "T" {
                    "ɔ".into()
                } else if g == "ough" && next_p == "F" {
                    "ɔ".into()
                } else {
                    "ɔː".into()
                }
            }
            "AW" => "aʊ".into(),
            "AY" => "aɪ".into(),
            "EH" => {
                if before_r && g0 == b'a' && r_then_vowel {
                    // carry, marry, Paris, character keep /æ/; Mary, parent, various have /ɛ/.
                    let after = seg_letters_after(e, seg.li);
                    if after.starts_with("rr")
                        || (first_syl
                            && (after.starts_with("ri")
                                || after.starts_with("ra")
                                || after.starts_with("ro"))
                            && !after.starts_with("ring")
                            && !after.starts_with("rie")
                            && !after.starts_with("rio")
                            && !after.starts_with("ria"))
                    {
                        "æ".into()
                    } else {
                        "ɛ".into()
                    }
                } else {
                    "ɛ".into()
                }
            }
            "ER" => {
                if (stressed || was_secondary) && !us && r_then_vowel_er(e, seg) {
                    "ʌ".into() // hurry, worry, courage (RP)
                } else if stressed || was_secondary {
                    "ɜː".into()
                } else if us {
                    "ɚ".into()
                } else {
                    "ə".into()
                }
            }
            "EY" => "eɪ".into(),
            "IH" => {
                if before_r {
                    // serious, period, hero: /iə/ before r + vowel inside a morpheme.
                    if stressed && r_then_vowel && matches!(g, "e" | "i" | "y") {
                        "iə".into()
                    } else {
                        "ɪ".into()
                    }
                } else if stressed {
                    "ɪ".into()
                } else if g0 == b'e' {
                    reduced_e(segs, i, first_syl, prev_p, next_p, n, !us).into()
                } else if g == "i" {
                    reduced_i(segs, i).into()
                } else {
                    "ɪ".into()
                }
            }
            "IY" => {
                if before_r {
                    if stressed && r_then_vowel && matches!(g, "e" | "i" | "y") {
                        "iə".into()
                    } else {
                        "ɪ".into()
                    }
                } else if st == 1 && next_p == "AH" && next.is_some_and(|s| s.stress == 0) {
                    // idea, museum, theater: one falling diphthong.
                    skip = true;
                    "iə".into()
                } else if stressed {
                    "iː".into()
                } else if g == "ay" {
                    "eɪ".into() // Monday, Friday
                } else if last {
                    "i".into()
                } else if next_is_vowel {
                    let nx = next.map_or(0, |s| s.stress);
                    if next_p == "AH" && nx == 0 && (i + 2 >= n || matches!(next2_p, "N" | "M")) {
                        "i".into()
                    } else if next_p == "AH"
                        && nx == 0
                        && next.is_some_and(|s| s.g.starts_with("ou"))
                    {
                        "i".into()
                    } else if next_p == "IH" && next2_p == "NG" {
                        "i".into()
                    } else if first_syl && g0 == b'e' && prev_p == "R" && i >= 2 {
                        "iː".into()
                    } else {
                        "ɪ".into()
                    }
                } else if g0 == b'e' && first_syl && !was_secondary {
                    reduced_e(segs, i, first_syl, prev_p, next_p, n, !us).into()
                } else if g0 == b'e' && first_syl && matches!(prev_p, "R" | "D" | "B") && i == 1 {
                    "ᵻ".into()
                } else if g == "y" && !last {
                    "ɪ".into()
                } else if was_secondary {
                    "iː".into()
                } else {
                    "i".into()
                }
            }
            "OW" => {
                if us {
                    "oʊ".into()
                } else {
                    "əʊ".into()
                }
            }
            "OY" => "ɔɪ".into(),
            "UH" => "ʊ".into(),
            "UW" => {
                let yod = !us
                    && (matches!(prev_p, "T" | "D" | "N")
                        || (prev_p == "S" && seg_letters_after(e, seg.li).starts_with("me"))
                        || (prev_p == "Z" && seg_letters_after(e, seg.li).starts_with("me")))
                    && (matches!(g, "u" | "ew" | "eu" | "ue" | "ui") || g.starts_with('u'))
                    && !matches!(
                        e.word.as_str(),
                        "suit" | "suits" | "do" | "to" | "two" | "noon"
                    );
                if yod {
                    out.push(Ph {
                        s: "j".into(),
                        stress: 9,
                        vowel: false,
                        li: seg.li,
                        g0,
                    });
                }
                "uː".into()
            }
            _ => String::new(),
        };
        // usual, eventually, graduate: /u/ + glide + vowel is a plain long /uː/.
        if !stressed
            && matches!(seg.p, "AH" | "UW")
            && next_p == "W"
            && next.is_some_and(|s| s.li == seg.li || s.g.starts_with('u') || s.g.is_empty())
            && g0 == b'u'
            && segs.get(i + 2).is_some_and(|s| s.is_vowel())
        {
            out.push(Ph {
                s: "uː".into(),
                stress: st,
                vowel: true,
                li: seg.li,
                g0,
            });
            skip = true;
            continue;
        }
        out.push(Ph {
            s: v,
            stress: st,
            vowel: true,
            li: seg.li,
            g0,
        });
    }
    if !us {
        // LOT after /w/: want, wash, watch, quality.
        for i in 1..out.len() {
            if out[i].s == "ɑː"
                && out[i].g0 == b'a'
                && out[i - 1].s == "w"
                && out.get(i + 1).is_none_or(|p| p.s != "ɹ")
            {
                let after = seg_letters_after(e, out[i].li);
                if !after.starts_with("lm") && !after.starts_with('h') {
                    out[i].s = "ɒ".into();
                }
            }
        }
    }
    post(&mut out, accent);
    if !us {
        post_gb(e, &mut out);
    }
    rhythm(&mut out);
    if us {
        for i in 1..out.len().saturating_sub(1) {
            if out[i].s == "ɾ" && out[i + 1].stress != 0 {
                out[i].s = "t".into();
            }
        }
    }
    out
}

/// The letters of the word after letter index `li`.
fn seg_letters_after(e: &Entry, li: usize) -> String {
    crate::cmudict::letters_of(&e.word)
        .iter()
        .skip(li + 1)
        .map(|&b| b as char)
        .collect()
}

/// CLOTH words: <o> before a voiceless fricative or /ŋ/ (off, loss, cloth, long).
fn cloth(next_p: &str, next2_p: &str) -> bool {
    next_p == "F"
        || next_p == "NG"
        || (next_p == "S" && matches!(next2_p, "" | "T"))
        || (next_p == "TH" && next2_p.is_empty())
}

/// The vowel of <a> in all/also/walk (long) versus salt/want/quality (short).
fn lot_a(e: &Entry, seg: &Seg, prev_p: &str) -> &'static str {
    let after = seg_letters_after(e, seg.li);
    if after.starts_with("lt") || after.starts_with("lse") {
        "ɔ"
    } else if after.starts_with('l') || after.starts_with('w') || after.starts_with("ugh") {
        "ɔː"
    } else if prev_p == "W" {
        if after.starts_with('n') || after.starts_with("li") {
            "ɔ"
        } else if after.starts_with("te") || after.starts_with("r") {
            "ɔː"
        } else {
            "ɑː"
        }
    } else {
        "ɑː"
    }
}

/// Reduced <e>: `ɛ` in ex-/en-/em-/es-, `ᵻ` in be-/de-/re-/se- and in -ed/-es,
/// `ɪ` in -est/-et and a few prefix + consonant combinations.
fn reduced_e(
    segs: &[Seg],
    i: usize,
    first_syl: bool,
    prev_p: &str,
    next_p: &str,
    n: usize,
    gb: bool,
) -> &'static str {
    let after_td = matches!(prev_p, "T" | "D");
    if (next_p == "Z" && i + 2 == n)
        || (next_p == "D" && i + 2 == n && after_td)
        || follows(segs, i, &["D", "L", "IY"], true)
    {
        return "ᵻ";
    }
    if !first_syl {
        return if next_p == "N" { "ə" } else { "ɪ" };
    }
    if i == 0 && gb {
        return "ɪ";
    }
    if i == 0 {
        return match next_p {
            "K" | "G" | "N" | "S" => "ɛ",
            "M" if matches!(segs.get(2).map_or("", |s| s.p), "B" | "P") => "ɛ",
            "L" => "ᵻ",
            _ => "ɪ",
        };
    }
    match (prev_p, next_p) {
        ("B", "G") | ("D", "T" | "V") | (_, "Z") => "ɪ",
        ("R", _) if i >= 2 && segs[i - 2].p == "P" => "ɪ",
        _ => "ᵻ",
    }
}

fn is_vowel_ph(p: &Ph) -> bool {
    p.vowel
}

/// True for stressed <ur>/<our>/<or> + <r> + vowel spellings that RP reads with /ʌ/.
fn r_then_vowel_er(e: &Entry, seg: &Seg) -> bool {
    let ls = crate::cmudict::letters_of(&e.word);
    let after: String = ls.iter().skip(seg.li).map(|&b| b as char).collect();
    (after.starts_with("urr") || after.starts_with("orr") || after.starts_with("our"))
        && after.len() > 3
        && matches!(after.as_bytes()[3], b'a' | b'e' | b'i' | b'o' | b'y' | b'u')
        && !after.starts_with("urri")
        && !after.starts_with("urry")
        || after.starts_with("urry") && !e.word.starts_with('f') && !e.word.starts_with("bl")
}

/// Was this vowel spelled with an <r> (so that a linking r belongs after it)?
fn er_spelled(e: &Entry, p: &Ph) -> bool {
    let ls = crate::cmudict::letters_of(&e.word);
    ls.iter().skip(p.li).take(3).any(|&b| b == b'r')
}

/// BATH words: /æ/ in General American, /ɑː/ in RP. A hand-built list of stems.
const BATH: &[&str] = &[
    "ask",
    "bask",
    "basket",
    "cask",
    "casket",
    "flask",
    "mask",
    "task",
    "rascal",
    "last",
    "past",
    "fast",
    "vast",
    "cast",
    "caste",
    "mast",
    "blast",
    "aghast",
    "ghastly",
    "contrast",
    "broadcast",
    "forecast",
    "outcast",
    "repast",
    "master",
    "plaster",
    "disaster",
    "nasty",
    "pastor",
    "pasture",
    "castle",
    "fasten",
    "class",
    "glass",
    "grass",
    "pass",
    "brass",
    "bypass",
    "surpass",
    "trespass",
    "path",
    "bath",
    "lath",
    "laugh",
    "draught",
    "after",
    "craft",
    "draft",
    "shaft",
    "raft",
    "daft",
    "graft",
    "staff",
    "half",
    "calf",
    "behalf",
    "dance",
    "chance",
    "france",
    "glance",
    "lance",
    "prance",
    "stance",
    "trance",
    "advance",
    "enhance",
    "answer",
    "plant",
    "grant",
    "slant",
    "chant",
    "can't",
    "shan't",
    "aunt",
    "advantage",
    "branch",
    "ranch",
    "avalanche",
    "blanch",
    "example",
    "sample",
    "demand",
    "command",
    "reprimand",
    "slander",
    "chancellor",
    "banana",
    "tomato",
    "rather",
    "lather",
    "clasp",
    "gasp",
    "grasp",
    "rasp",
    "raspberry",
    "telegraph",
    "photograph",
    "autograph",
    "paragraph",
    "giraffe",
    "moustache",
    "pyjama",
    "drama",
    "llama",
    "soprano",
    "sultana",
    "piano",
];

fn is_bath(word: &str) -> bool {
    let w = word.trim_end_matches("'s");
    BATH.iter().any(|b| {
        w == *b
            || (w.len() > b.len()
                && w.starts_with(b)
                && matches!(
                    &w[b.len()..],
                    "s" | "es"
                        | "ed"
                        | "d"
                        | "ing"
                        | "er"
                        | "ers"
                        | "est"
                        | "ly"
                        | "y"
                        | "ful"
                        | "less"
                        | "ness"
                        | "room"
                        | "rooms"
                        | "work"
                ))
            || (b.ends_with('e')
                && w.len() > b.len()
                && w.starts_with(&b[..b.len() - 1])
                && matches!(&w[b.len() - 1..], "ing" | "ed" | "er" | "ers"))
    }) || [
        "after", "class", "glass", "grass", "pass", "master", "fast", "craft", "path", "bath",
        "staff", "half", "castle",
    ]
    .iter()
    .any(|b| {
        w.starts_with(b)
            && w.len() >= b.len() + 3
            && !w.starts_with("passi")
            && !w.starts_with("passa")
            && !w.starts_with("passe")
            && !w.starts_with("classi")
            && !w.starts_with("fasti")
            && !w.starts_with("pathe")
            && !w.starts_with("patho")
            && !w.starts_with("bathe")
            && !w.starts_with("halfp")
    })
}

/// Received Pronunciation realisation of the General-American-shaped list.
fn post_gb(e: &Entry, ph: &mut Vec<Ph>) {
    // BATH broadening.
    if is_bath(&e.word) {
        if let Some(p) = ph.iter_mut().find(|p| p.s == "æ" && p.stress == 1).or(None) {
            p.s = "ɑː".into();
        } else if e.word.starts_with("tomato") {
            if let Some(p) = ph.iter_mut().find(|p| p.s == "eɪ" && p.stress == 1) {
                p.s = "ɑː".into();
            }
        }
    }
    let mut i = 0;
    while i < ph.len() {
        let next_vowel = ph.get(i + 1).is_some_and(|p| p.vowel);
        if ph[i].s == "ɹ" && i > 0 && ph[i - 1].vowel {
            let prev = ph[i - 1].s.clone();
            let stressed = ph[i - 1].stress != 0;
            let new_prev = match prev.as_str() {
                // very, America keep the short vowel; care, various, there have the diphthong.
                "ɛ" if ph[i - 1].g0 == b'e'
                    && next_vowel
                    && !e.word.starts_with("where")
                    && !e.word.starts_with("there") =>
                {
                    "ɛ"
                }
                "ɛ" => "eə",
                "ɪ" if stressed => "iə",
                "ʊ" => {
                    if stressed || !next_vowel {
                        "ɔː"
                    } else {
                        "ʊ"
                    }
                }
                "aɪ" if !next_vowel => "aɪə",
                "aʊ" if !next_vowel => "aʊə",
                other => other,
            }
            .to_string();
            ph[i - 1].s = new_prev;
            if !next_vowel {
                ph.remove(i);
                continue;
            }
        }
        i += 1;
    }
    // An r-coloured vowel before another vowel keeps its r: parade, hurry, stirring.
    let mut i = 0;
    while i + 1 < ph.len() {
        if matches!(ph[i].s.as_str(), "ə" | "ɜː")
            && ph[i].g0 != 0
            && ph[i + 1].vowel
            && er_spelled(e, &ph[i])
        {
            let li = ph[i].li;
            ph.insert(
                i + 1,
                Ph {
                    s: "ɹ".into(),
                    stress: 9,
                    vowel: false,
                    li,
                    g0: 0,
                },
            );
            i += 1;
        }
        i += 1;
    }
    let n = ph.len();
    let next_is_vowel: Vec<bool> = (0..n)
        .map(|i| ph.get(i + 1).is_some_and(|p| p.vowel))
        .collect();
    let prev_s: Vec<String> = (0..n)
        .map(|i| {
            if i > 0 {
                ph[i - 1].s.clone()
            } else {
                String::new()
            }
        })
        .collect();
    for (i, p) in ph.iter_mut().enumerate() {
        match p.s.as_str() {
            "ᵻ" => p.s = "ɪ".into(),
            "i" if !next_is_vowel[i] => p.s = "ɪ".into(),
            "ɔ" => p.s = "ɒ".into(),
            "ɚ" => p.s = "ə".into(),
            _ => {}
        }
        if i + 1 == n && p.s == "ə" && prev_s[i] != "aɪ" {
            p.s = "ɐ".into();
        }
    }
    // -ary / -ory after the stress: territory, necessary.
    let vs: Vec<usize> = (0..ph.len()).filter(|&i| ph[i].vowel).collect();
    if vs.len() >= 3 {
        let k = vs[vs.len() - 2];
        let last = vs[vs.len() - 1];
        if last + 1 == ph.len()
            && ph[last].s == "ɪ"
            && ph.get(k + 1).is_some_and(|p| p.s == "ɹ")
            && k + 2 == last
            && matches!(ph[k].s.as_str(), "eə" | "ɔː")
            && ph[k].stress != 1
            && ph[vs[vs.len() - 3]].stress == 0
        {
            ph[k].s = "ə".into();
            ph[k].stress = 0;
        }
    }
}

fn post(ph: &mut Vec<Ph>, accent: Accent) {
    let us = accent == Accent::Us;
    // tire, hour: the r after a closing diphthong is a rhotic schwa.
    let mut i = 0;
    while us && i < ph.len() {
        if ph[i].s == "ɹ"
            && i > 0
            && matches!(ph[i - 1].s.as_str(), "aɪ" | "aʊ")
            && (i + 1 == ph.len() || !ph[i + 1].vowel)
        {
            ph[i].s = "ɚ".into();
            ph[i].vowel = true;
            ph[i].stress = 0;
        }
        i += 1;
    }
    if us {
        // Linking r is written twice after a rhotic vowel: hurry, during, starry.
        let mut i = 0;
        while i + 1 < ph.len() {
            let cur = ph[i].s.clone();
            if matches!(cur.as_str(), "ɚ" | "ɜː") && ph[i + 1].vowel {
                ph.insert(
                    i + 1,
                    Ph {
                        s: "ɹ".into(),
                        stress: 9,
                        vowel: false,
                        li: ph[i].li,
                        g0: 0,
                    },
                );
            } else if cur == "ɹ"
                && i > 0
                && ph[i + 1].vowel
                && ((ph[i - 1].s == "ʊ" && ph[i - 1].stress != 0)
                    || (ph[i - 1].s == "ɑː"
                        && ph[i - 1].stress != 0
                        && ph[i + 1].stress == 0
                        && false))
            {
                ph.insert(
                    i + 1,
                    Ph {
                        s: "ɹ".into(),
                        stress: 9,
                        vowel: false,
                        li: ph[i].li,
                        g0: 0,
                    },
                );
                i += 1;
            }
            i += 1;
        }
        // t before a syllabic n: written, button, certain.
        let mut i = 1;
        while i + 2 <= ph.len().saturating_sub(1) + 1 && i + 1 < ph.len() {
            if ph[i].s == "t"
                && ph[i - 1].vowel
                && ph[i - 1].stress != 0
                && matches!(ph[i + 1].s.as_str(), "ə" | "ɪ")
                && ph[i + 1].stress == 0
                && ph.get(i + 2).is_some_and(|p| p.s == "n")
                && ph.get(i + 3).is_none_or(|p| !p.vowel)
                && ph[i + 1].s == "ə"
                && matches!(ph[i + 1].g0, b'e' | b'o')
                && !matches!(ph[i - 1].s.as_str(), "eɪ" | "aʊ" | "oʊ")
            {
                ph[i].s = "ʔ".into();
                ph[i + 2].s = "n̩".into();
                ph.remove(i + 1);
            }
            i += 1;
        }
        // Flap: /t/ after a vowel or /ɹ/ and before an unstressed vowel.
        for i in 1..ph.len().saturating_sub(1) {
            if ph[i].s == "t"
                && (ph[i - 1].vowel || ph[i - 1].s == "ɹ")
                && ph[i + 1].vowel
                && ph[i + 1].stress == 0
            {
                let before_n = ph.get(i + 2).is_some_and(|p| p.s == "n")
                    && matches!(ph[i + 1].s.as_str(), "ə" | "ɪ")
                    && ph.get(i + 3).is_none_or(|p| !p.vowel);
                if !before_n {
                    ph[i].s = "ɾ".into();
                }
            }
        }
    }
    let _ = is_vowel_ph;
}

pub fn render(ph: &[Ph]) -> String {
    let mut s = String::new();
    for p in ph {
        match p.stress {
            1 => s.push('ˈ'),
            2 => s.push('ˌ'),
            _ => {}
        }
        s.push_str(&p.s);
    }
    s
}

/// Secondary stress after the primary: a full vowel two or more syllables after
/// it, whose neighbours are both unstressed, is marked (ˈɛnɪθˌɪŋ, pˈɑːzᵻtˌɪv).
fn rhythm(ph: &mut [Ph]) {
    let vs: Vec<usize> = (0..ph.len()).filter(|&i| ph[i].vowel).collect();
    let Some(p) = vs.iter().position(|&i| ph[i].stress == 1) else {
        return;
    };
    for k in p + 2..vs.len() {
        let i = vs[k];
        if ph[i].stress != 0
            || ph[vs[k - 1]].stress != 0
            || vs.get(k + 1).is_some_and(|&j| ph[j].stress != 0)
        {
            continue;
        }
        let next = ph.get(i + 1).map_or("", |p| p.s.as_str());
        let next2 = ph.get(i + 2).map_or("", |p| p.s.as_str());
        let full = match ph[i].s.as_str() {
            "ə" | "ɚ" | "ᵻ" | "ɐ" | "i" | "n̩" | "ʊ" => false,
            "ɪ" => matches!(
                (next, next2),
                ("k", _) | ("v", _) | ("n", "") | ("s", "t") | ("z", "ə") | ("ɹ", "")
            ),
            "oʊ" | "əʊ" => next.is_empty() || next == "z",
            _ => true,
        };
        if full {
            ph[i].stress = 2;
        }
    }
}
