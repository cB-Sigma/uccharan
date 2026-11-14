//! Letter-to-sound models for the word tables of the data-driven languages.
//!
//! The readings of a word list are aligned letter by letter with
//! [`crate::align`], and [`crate::lts`] learns from them which phonemes a
//! letter gives in which neighbourhood. The model then reads the words the
//! list does not have, and the table only needs the words the model gets
//! wrong.

use crate::align::Aligner;
use crate::lts;
use uccharan::data::encode_letters;

/// Split a phoneme string into units: a base character with its length and
/// diacritic marks. The stress mark before a unit is returned apart.
pub fn units(ipa: &str) -> Vec<(char, String)> {
    let mut out: Vec<(char, String)> = Vec::new();
    let mut stress = ' ';
    let mut tie = false;
    for c in ipa.chars() {
        if matches!(c, 'ˈ' | 'ˌ' | '°') {
            stress = c;
            continue;
        }
        let modifier =
            matches!(c, 'ː' | 'ˑ' | 'ʰ' | 'ʲ' | 'ʷ' | 'ˤ') || ('\u{300}'..='\u{36f}').contains(&c);
        if (modifier || tie) && !out.is_empty() && stress == ' ' {
            out.last_mut().unwrap().1.push(c);
        } else {
            out.push((stress, c.to_string()));
            stress = ' ';
        }
        tie = c == '\u{361}';
    }
    out
}

/// The letters outside ASCII of a word list, most frequent first (at most 120).
pub fn alphabet(words: &[(String, String)]) -> String {
    let mut n: std::collections::BTreeMap<char, usize> = Default::default();
    for (w, _) in words {
        for c in w.chars().filter(|c| !c.is_ascii()) {
            *n.entry(c).or_default() += 1;
        }
    }
    let mut v: Vec<(char, usize)> = n.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v.into_iter().take(120).map(|(c, _)| c).collect()
}

/// Train a model on `(word, reading)` rows. Words that cannot be aligned
/// are left out. Returns the extra letters and the model.
pub fn train(
    rows: &[(String, String)],
    min_count: u32,
    shapes: &[(u8, u8)],
    latent: Option<&uccharan::latin::Latin>,
) -> (String, lts::Model) {
    let extra = alphabet(rows);
    // Unit ids, the most frequent 255.
    let mut freq: std::collections::BTreeMap<String, usize> = Default::default();
    let parsed: Vec<Option<(Vec<u8>, Vec<(char, String)>)>> = rows
        .iter()
        .map(|(w, r)| {
            let ls = encode_letters(w, &extra)?;
            let us = units(r);
            for (_, u) in &us {
                *freq.entry(u.clone()).or_default() += 1;
            }
            Some((ls, us))
        })
        .collect();
    let mut by: Vec<(&String, &usize)> = freq.iter().collect();
    by.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    let ids: std::collections::BTreeMap<&str, u8> = by
        .iter()
        .take(255)
        .enumerate()
        .map(|(i, (u, _))| (u.as_str(), i as u8 + 1))
        .collect();
    let mut data: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    let mut keep: Vec<usize> = Vec::new();
    for (k, p) in parsed.iter().enumerate() {
        let Some((ls, us)) = p else { continue };
        let Some(ps) = us
            .iter()
            .map(|(_, u)| ids.get(u.as_str()).copied())
            .collect::<Option<Vec<u8>>>()
        else {
            continue;
        };
        if ls.is_empty() || ps.len() > 2 * ls.len() {
            continue;
        }
        data.push((ls.clone(), ps));
        keep.push(k);
    }
    let aligner = Aligner::train(&data, 6, |_, _| true);
    let mut examples: Vec<(Vec<u8>, Vec<String>)> = Vec::with_capacity(data.len());
    for ((ls, ps), &k) in data.iter().zip(&keep) {
        let Some(al) = aligner.align(ls, ps) else {
            continue;
        };
        let us = &parsed[k].as_ref().unwrap().1;
        // With a stress rule, only the marks the rule cannot derive.
        let relabelled;
        let us = match latent {
            Some(l) => {
                relabelled = units(&l.latent_marks(&rows[k].1, &rows[k].0));
                if relabelled.len() == us.len() {
                    &relabelled
                } else {
                    us
                }
            }
            None => us,
        };
        // Of two equal letters the first one carries the sound.
        let mut al = al;
        for i in 1..ls.len() {
            if ls[i] == ls[i - 1] && al[i - 1] == 0 && al[i] > 0 {
                al.swap(i - 1, i);
            }
        }
        let mut outs = Vec::with_capacity(ls.len());
        let mut j = 0;
        for &n in &al {
            let mut s = String::new();
            for (st, u) in &us[j..j + n as usize] {
                if *st != ' ' {
                    s.push(*st);
                }
                s.push_str(u);
            }
            j += n as usize;
            outs.push(s);
        }
        examples.push((ls.clone(), outs));
    }
    // Segments and stress marks are learned apart: the mark of a letter's
    // vowel depends on the stress written so far and on the word's ending.
    let mut states: Vec<Vec<u32>> = Vec::with_capacity(examples.len());
    let mut keys: Vec<Vec<u32>> = Vec::with_capacity(examples.len());
    let mut segs = Vec::with_capacity(examples.len());
    let mut marks = Vec::with_capacity(examples.len());
    for (ls, outs) in &examples {
        let mut s = 0u8;
        let mut st = Vec::with_capacity(outs.len());
        let mut sg = Vec::with_capacity(outs.len());
        let mut mk = Vec::with_capacity(outs.len());
        let mut ky = Vec::with_capacity(outs.len());
        for o in outs {
            st.push(s as u32);
            let seg: String = o
                .chars()
                .filter(|c| !matches!(c, 'ˈ' | 'ˌ' | '°'))
                .collect();
            ky.push(uccharan::data::stress_key(s, &seg));
            s = uccharan::data::stress_state(s, o);
            let plain: String = o
                .chars()
                .filter(|c| !matches!(c, 'ˈ' | 'ˌ' | '°'))
                .collect();
            mk.push(if plain.chars().any(uccharan::data::is_vowel) {
                o.chars()
                    .find(|c| matches!(c, 'ˈ' | 'ˌ' | '°'))
                    .map(String::from)
                    .unwrap_or_default()
            } else {
                lts::SKIP.to_string()
            });
            sg.push(plain);
        }
        states.push(st);
        keys.push(ky);
        segs.push((ls.clone(), sg));
        marks.push((ls.clone(), mk));
    }
    let (seg_shapes, stress_shapes): (Vec<(u8, u8)>, Vec<(u8, u8)>) =
        shapes.iter().partition(|s| s.0 & 0x40 == 0);
    let mut a = lts::train_full(&segs, Some(&states), min_count, &seg_shapes);
    let b = lts::train_full(&marks, Some(&keys), min_count, &stress_shapes);
    let base = a.outputs.len() as u16;
    a.outputs.extend(b.outputs);
    for (l, r, t) in b.tables {
        a.tables
            .push((l, r, t.into_iter().map(|(h, o)| (h, o + base)).collect()));
    }
    (extra, a)
}

/// The back-off chain of the word-table models: segment tables, then the
/// stress tables (flag 0x40), which also see the stress state (0x80).
pub const SHAPES: [(u8, u8); 16] = [
    (4, 4),
    (3, 3),
    (2, 3),
    (2, 2),
    (1, 2),
    (1, 1),
    (0, 1),
    (0, 0),
    (0xc4, 4),
    (0xc3, 3),
    (0xc2, 3),
    (0xc2, 2),
    (0xc1, 2),
    (0xc1, 1),
    (0xc0, 0),
    (0x40, 0),
];
