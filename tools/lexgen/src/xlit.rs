//! Builds the transliteration packs (`data/packs/xlit-<lang>.ucd`): the
//! n-gram transliteration model and the word data read by
//! `uccharan::translit`.
//!
//! ```text
//! lexgen xlit <lang> [--data DIR] [--out FILE] [--set name=value]...
//! lexgen xlit-toy          # the tiny pack of the library's tests
//! lexgen xlit-check        # fail if a pack whose sources are present is stale
//! ```
//!
//! A language is built from `data/sources/translit/<lang>/`: `recipe.txt`
//! (model order, cut-offs, licence, fitted weights) and the hand-written
//! `chat.tsv` and `homographs.tsv` are in the repository; the large inputs
//! `train.tsv` (native, romanised, source, count), `freq.tsv` (word, count)
//! and `bigrams.tsv` (word, word, count) are fetched and prepared by
//! `tools/eval/translit/prep.py` and `corpus.py` and are looked for in the
//! same directory, or under `--data DIR/<lang>/`.
//!
//! The model: a word pair is aligned one romanised letter at a time (each
//! letter writes zero, one or two native units) with the same
//! expectation-maximisation aligner the word tables use; an interpolated
//! Kneser-Ney n-gram model over the (letter, units) tokens is counted,
//! pruned and stored as hashed tables.

use crate::align::Aligner;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use uccharan::translit::lex::{CLM_STEP, F_STEP, SHARE_STEP};
use uccharan::translit::model::{key, units, BITS, BOS, B_STEP, EOS, MASK, P_STEP};
use uccharan::translit::pack::{build_table, build_table3, hash2, hash_bytes, write_container};

/// Sections of a pack being written.
#[derive(Default)]
struct Writer {
    sections: Vec<([u8; 4], Vec<u8>)>,
}

impl Writer {
    fn section(&mut self, tag: &[u8; 4], body: Vec<u8>) {
        self.sections.push((*tag, body));
    }
}

struct Trained {
    order: usize,
    units: Vec<String>,
    tokens: Vec<(u8, u8, u8)>,
    /// Per order: (packed n-gram, log p, back-off weight as a context or NaN).
    grams: Vec<Vec<(u128, f32, f32)>>,
    pairs_used: usize,
}

/// Train on `(native, roman, weight)` rows. `min_count[k]` is the least
/// raw count an n-gram of order k+1 needs to be stored.
fn train(
    rows: &[(String, String, u32)],
    order: usize,
    min_count: &[u32],
    min_token: u32,
    prune: f32,
    log: bool,
) -> Trained {
    // Unit ids by frequency.
    let mut ufreq: HashMap<String, usize> = HashMap::new();
    let split: Vec<Vec<String>> = rows.iter().map(|(n, _, _)| units(n)).collect();
    for us in &split {
        for u in us {
            *ufreq.entry(u.clone()).or_default() += 1;
        }
    }
    let mut by: Vec<(String, usize)> = ufreq.into_iter().filter(|(_, c)| *c >= 5).collect();
    by.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    by.truncate(254);
    let mut unit_names = vec![String::new()];
    let mut uid: HashMap<String, u8> = HashMap::new();
    for (u, _) in &by {
        uid.insert(u.clone(), unit_names.len() as u8);
        unit_names.push(u.clone());
    }
    let mut data: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    let mut weights: Vec<u32> = Vec::new();
    for ((_, r, w), us) in rows.iter().zip(&split) {
        let ls: Vec<u8> = r.bytes().map(|b| b - b'a' + 1).collect();
        let Some(ps) = us
            .iter()
            .map(|u| uid.get(u).copied())
            .collect::<Option<Vec<u8>>>()
        else {
            continue;
        };
        if ls.is_empty() || ps.is_empty() || ps.len() > 2 * ls.len() || ls.len() > 3 * ps.len() + 2
        {
            continue;
        }
        data.push((ls, ps));
        weights.push(*w);
    }
    if log {
        eprintln!(
            "aligning {} pairs, {} units",
            data.len(),
            unit_names.len() - 1
        );
    }
    // EM on a sample is enough for the unit probabilities.
    let step = (data.len() / 400_000).max(1);
    let sample: Vec<(Vec<u8>, Vec<u8>)> = data.iter().step_by(step).cloned().collect();
    let aligner = Aligner::train(&sample, 6, |_, _| true);
    drop(sample);
    // Token sequences.
    let mut tok_id: HashMap<(u8, u8, u8), u16> = HashMap::new();
    let mut tok_count: Vec<u32> = vec![0, 0, 0];
    let mut tokens: Vec<(u8, u8, u8)> = vec![(0, 0, 0); 3];
    let mut seqs: Vec<Vec<u16>> = Vec::new();
    for (ls, ps) in &data {
        let Some(al) = aligner.align(ls, ps) else {
            seqs.push(Vec::new());
            continue;
        };
        let mut j = 0;
        let mut s = Vec::with_capacity(ls.len() + 2);
        s.push(BOS);
        for (i, &l) in ls.iter().enumerate() {
            let k = al[i] as usize;
            let t = (
                l,
                if k > 0 { ps[j] } else { 0 },
                if k > 1 { ps[j + 1] } else { 0 },
            );
            j += k;
            let id = *tok_id.entry(t).or_insert_with(|| {
                tokens.push(t);
                tok_count.push(0);
                (tokens.len() - 1) as u16
            });
            tok_count[id as usize] += 1;
            s.push(id);
        }
        s.push(EOS);
        seqs.push(s);
    }
    // Rare tokens out (and the pairs that use them); renumber.
    let mut remap: Vec<u16> = vec![0; tokens.len()];
    let mut kept: Vec<(u8, u8, u8)> = vec![(0, 0, 0); 3];
    remap[1] = 1;
    remap[2] = 2;
    let mut order_by: Vec<usize> = (3..tokens.len()).collect();
    order_by.sort_by(|&a, &b| {
        tok_count[b]
            .cmp(&tok_count[a])
            .then(tokens[a].cmp(&tokens[b]))
    });
    for i in order_by {
        if tok_count[i] >= min_token && kept.len() < 4000 {
            remap[i] = kept.len() as u16;
            kept.push(tokens[i]);
        }
    }
    let mut used = 0;
    let mut train_seqs: Vec<(Vec<u16>, u32)> = Vec::new();
    for (s, w) in seqs.into_iter().zip(&weights) {
        if s.is_empty() {
            continue;
        }
        let m: Vec<u16> = s.iter().map(|&t| remap[t as usize]).collect();
        if m.contains(&0) {
            continue;
        }
        used += 1;
        train_seqs.push((m, *w));
    }
    if log {
        eprintln!("{} pairs in the model, {} tokens", used, kept.len() - 3);
    }
    // Counts per order, an order only for n-grams whose prefix was seen twice.
    let mut counts: Vec<HashMap<u128, u32>> = Vec::new();
    for k in 1..=order {
        let mut c: HashMap<u128, u32> = HashMap::new();
        for (s, w) in &train_seqs {
            if s.len() < k {
                continue;
            }
            for i in 0..=s.len() - k {
                // An n-gram may only start with BOS, never contain it later.
                let mut g: u128 = 0;
                for &t in &s[i..i + k] {
                    g = (g << BITS) | t as u128;
                }
                if k >= 3 {
                    let prefix = g >> BITS;
                    if counts[k - 2].get(&prefix).copied().unwrap_or(0) < 2 {
                        continue;
                    }
                }
                *c.entry(g).or_default() += *w;
            }
        }
        if log {
            eprintln!("order {k}: {} n-grams", c.len());
        }
        counts.push(c);
    }
    // Kneser-Ney adjusted counts: below the top order, the number of
    // distinct tokens seen before the n-gram (raw counts after BOS).
    let mut adj: Vec<HashMap<u128, u32>> = Vec::new();
    for k in 1..=order {
        if k == order {
            adj.push(counts[k - 1].clone());
            continue;
        }
        let mut a: HashMap<u128, u32> = HashMap::with_capacity(counts[k - 1].len());
        let low = (1u128 << (BITS * k as u32)) - 1;
        for &g in counts[k].keys() {
            *a.entry(g & low).or_default() += 1;
        }
        for (&g, &c) in &counts[k - 1] {
            let first = (g >> (BITS * (k as u32 - 1))) & MASK;
            if first == BOS as u128 {
                a.insert(g, c);
            } else {
                a.entry(g).or_insert(1);
            }
        }
        adj.push(a);
    }
    // Probabilities, order by order.
    let ntok = kept.len() as f64;
    let mut probs: Vec<HashMap<u128, f32>> = Vec::new(); // interpolated p
    let mut bows: Vec<HashMap<u128, f32>> = Vec::new(); // per context of order k (k tokens)
    for k in 1..=order {
        let a = &adj[k - 1];
        let (mut n1, mut n2) = (0f64, 0f64);
        for &c in a.values() {
            if c == 1 {
                n1 += 1.0;
            } else if c == 2 {
                n2 += 1.0;
            }
        }
        let d = if n1 + 2.0 * n2 > 0.0 {
            (n1 / (n1 + 2.0 * n2)).clamp(0.1, 0.95)
        } else {
            0.75
        };
        let mut tot: HashMap<u128, (f64, f64)> = HashMap::new(); // ctx -> (sum, types)
        for (&g, &c) in a {
            let e = tot.entry(g >> BITS).or_insert((0.0, 0.0));
            e.0 += c as f64;
            e.1 += 1.0;
        }
        let mut p: HashMap<u128, f32> = HashMap::with_capacity(a.len());
        let low = if k >= 2 {
            (1u128 << (BITS * (k as u32 - 1))) - 1
        } else {
            0
        };
        for (&g, &c) in a {
            let (sum, types) = tot[&(g >> BITS)];
            let lower = if k == 1 {
                1.0 / ntok
            } else {
                probs[k - 2].get(&(g & low)).copied().unwrap_or(1e-7) as f64
            };
            let v = ((c as f64 - d).max(0.0) + d * types * lower) / sum;
            p.insert(g, v as f32);
        }
        let mut b: HashMap<u128, f32> = HashMap::with_capacity(tot.len());
        for (&ctx, &(sum, types)) in &tot {
            b.insert(ctx, (d * types / sum) as f32);
        }
        probs.push(p);
        bows.push(b);
    }
    let mut grams = Vec::new();
    for k in 1..=order {
        let minc = min_count.get(k - 1).copied().unwrap_or(1);
        let mut v: Vec<(u128, f32, f32)> = Vec::new();
        for (&g, &p) in &probs[k - 1] {
            let c = counts[k - 1].get(&g).copied().unwrap_or(0);
            if c < minc && k > 1 {
                continue;
            }
            if prune > 0.0 && k >= 3 {
                // Weighted-difference pruning: what the model loses on the
                // training data when this n-gram is answered by backing off.
                let low = (1u128 << (BITS * (k as u32 - 1))) - 1;
                let lower = probs[k - 2].get(&(g & low)).copied().unwrap_or(1e-7);
                let bow = bows[k - 1].get(&(g >> BITS)).copied().unwrap_or(1.0);
                if c as f32 * (p.ln() - (bow * lower).ln()) < prune {
                    continue;
                }
            }
            // The back-off weight of this n-gram used as a context.
            let bow = if k < order {
                bows[k].get(&g).copied().unwrap_or(f32::NAN)
            } else {
                f32::NAN
            };
            v.push((g, p.ln(), if bow.is_nan() { f32::NAN } else { bow.ln() }));
        }
        v.sort_by_key(|a| a.0);
        grams.push(v);
    }
    // Drop back-off weights of contexts that kept no extension.
    for k in 1..order {
        let ext: std::collections::HashSet<u128> =
            grams[k].iter().map(|(g, _, _)| g >> BITS).collect();
        for e in grams[k - 1].iter_mut() {
            if !ext.contains(&e.0) {
                e.2 = f32::NAN;
            }
        }
    }
    Trained {
        order,
        units: unit_names,
        tokens: kept,
        grams,
        pairs_used: used,
    }
}

impl Trained {
    fn write(&self, w: &mut Writer) {
        w.section(b"UNIT", self.units.join("\n").into_bytes());
        w.section(
            b"TOKS",
            self.tokens.iter().flat_map(|t| [t.0, t.1, t.2]).collect(),
        );
        let mut body = vec![self.order as u8];
        for (k, g) in self.grams.iter().enumerate() {
            let items: Vec<(u64, [u8; 2])> = g
                .iter()
                .map(|&(g, p, b)| {
                    let pq = (-p / P_STEP).round().clamp(0.0, 255.0) as u8;
                    let bq = if b.is_nan() {
                        0
                    } else {
                        (-b / B_STEP).round().clamp(0.0, 255.0) as u8
                    };
                    (key(k + 1, g), [pq, bq])
                })
                .collect();
            let t = build_table3(&items);
            body.extend((t.len() as u32).to_le_bytes());
            body.extend(t);
        }
        w.section(b"NGRM", body);
        let mut succ = Vec::new();
        for &(g, _, _) in &self.grams[1] {
            let (a, b) = ((g >> BITS) as u16, (g & MASK) as u16);
            if b != EOS {
                succ.extend(a.to_le_bytes());
                succ.extend(b.to_le_bytes());
            }
        }
        w.section(b"SUCC", succ);
    }

    fn gram_count(&self) -> usize {
        self.grams.iter().map(|g| g.len()).sum()
    }
}

/// Letter trigram model over a–z with word boundaries: 27^3 bytes.
fn charlm_train(words: &[(&str, f32)]) -> Vec<u8> {
    let mut c = vec![0.2f32; 27 * 27 * 27];
    for (w, wt) in words {
        let mut a = 0usize;
        let mut b = 0usize;
        for x in w.bytes().map(|x| (x - b'a' + 1) as usize).chain([0]) {
            c[(a * 27 + b) * 27 + x] += wt;
            a = b;
            b = x;
        }
    }
    let mut out = vec![0u8; c.len()];
    for ctx in 0..27 * 27 {
        let tot: f32 = c[ctx * 27..ctx * 27 + 27].iter().sum();
        for x in 0..27 {
            out[ctx * 27 + x] = (-(c[ctx * 27 + x] / tot).ln() / CLM_STEP)
                .round()
                .clamp(0.0, 255.0) as u8;
        }
    }
    out
}

/// What the builder is given.
#[derive(Default)]
struct Sources {
    /// (native, roman, source, count)
    pairs: Vec<(String, String, String, u32)>,
    /// native word counts and their total
    freq: Vec<(String, u32)>,
    total: u64,
    bigrams: Vec<(String, String, u32)>,
    /// extra hand-written (roman, native) spellings
    extra: Vec<(String, String)>,
    en_freq: Vec<(String, u32)>,
    en_total: u64,
    /// romanised spellings shared with English words, and the native word meant
    homographs: Vec<(String, String)>,
}

struct BuildOptions {
    /// Least corpus count of a native word in the frequency table.
    freq_min: u32,
    /// Least corpus count of a native word whose spellings enter the lexicon.
    lex_min: u32,
    /// Least count of a stored word pair.
    bigram_min: u32,
    /// Sources whose pairs may enter the lexicon (empty = all).
    lex_sources: Vec<String>,
}

fn build_words(src: &Sources, opt: &BuildOptions, w: &mut Writer) -> String {
    let fmap: HashMap<&str, u32> = src.freq.iter().map(|(w, c)| (w.as_str(), *c)).collect();
    // Native frequencies.
    let items: Vec<(u64, [u8; 1])> = src
        .freq
        .iter()
        .filter(|(_, c)| *c >= opt.freq_min)
        .map(|(w, c)| {
            (
                hash_bytes(w.as_bytes()),
                [((*c as f32).ln() / F_STEP).round().min(255.0) as u8],
            )
        })
        .collect();
    let n_freq = items.len();
    w.section(b"NFRQ", build_table::<1>(&items));
    // Lexicon: spellings of frequent words.
    let mut weight: BTreeMap<(&str, &str), f32> = BTreeMap::new();
    for (n, r, s, c) in &src.pairs {
        let f = fmap.get(n.as_str()).copied().unwrap_or(0);
        if f < opt.lex_min {
            continue;
        }
        if !opt.lex_sources.is_empty() && !opt.lex_sources.iter().any(|x| x == s) {
            continue;
        }
        *weight.entry((r.as_str(), n.as_str())).or_default() += *c as f32;
    }
    for (r, n) in &src.extra {
        // A hand-written spelling counts like a very well attested one.
        let e = weight.entry((r.as_str(), n.as_str())).or_default();
        *e = e.max(0.0) + 50.0;
    }
    let mut per_native: HashMap<&str, f32> = HashMap::new();
    for ((_, n), wt) in &weight {
        *per_native.entry(n).or_default() += wt;
    }
    let mut by_roman: HashMap<&str, Vec<(&str, f32, f32)>> = HashMap::new();
    for ((r, n), wt) in &weight {
        by_roman
            .entry(r)
            .or_default()
            .push((n, *wt, *wt / per_native[n]));
    }
    let mut natives: Vec<&str> = Vec::new();
    let mut native_id: HashMap<&str, u32> = HashMap::new();
    let mut entries: Vec<u8> = Vec::new();
    let mut table: Vec<(u64, [u8; 3])> = Vec::new();
    let mut romans: Vec<&&str> = by_roman.keys().collect();
    romans.sort();
    let mut n_entries = 0;
    for r in romans {
        let mut v = by_roman[*r].clone();
        // Best first: share of the word's spellings times the word's frequency.
        let rank = |x: &(&str, f32, f32)| x.2 * (fmap.get(x.0).copied().unwrap_or(1).max(1) as f32);
        v.sort_by(|a, b| rank(b).total_cmp(&rank(a)).then(a.0.cmp(b.0)));
        v.truncate(4);
        let at = entries.len() as u32;
        if at >= 1 << 24 {
            break;
        }
        table.push((
            hash_bytes(r.as_bytes()),
            [at as u8, (at >> 8) as u8, (at >> 16) as u8],
        ));
        entries.push(v.len() as u8);
        for (n, wt, share) in v {
            let id = *native_id.entry(n).or_insert_with(|| {
                natives.push(n);
                natives.len() as u32 - 1
            });
            entries.extend(&id.to_le_bytes()[..3]);
            entries.push(((1.0 + wt).ln() / F_STEP).round().min(255.0) as u8);
            entries.push((-share.ln() / SHARE_STEP).round().clamp(0.0, 255.0) as u8);
            n_entries += 1;
        }
    }
    // Native words of the lexicon: offsets + text.
    let mut text = Vec::new();
    let mut offs: Vec<u8> = Vec::new();
    for n in &natives {
        offs.extend((text.len() as u32).to_le_bytes());
        text.extend(n.as_bytes());
    }
    offs.extend((text.len() as u32).to_le_bytes());
    w.section(b"LEXO", offs);
    w.section(b"LEXW", text);
    w.section(b"LEXE", entries);
    w.section(b"LEXR", build_table::<3>(&table));
    // Word-pair association (pointwise mutual information).
    let total = src.total.max(1) as f64;
    let mut big: Vec<(u64, [u8; 1])> = Vec::new();
    for (a, b, c) in &src.bigrams {
        if *c < opt.bigram_min {
            continue;
        }
        let (Some(&ca), Some(&cb)) = (fmap.get(a.as_str()), fmap.get(b.as_str())) else {
            continue;
        };
        if ca < opt.freq_min || cb < opt.freq_min {
            continue;
        }
        let pmi = ((*c as f64) * total / (ca as f64 * cb as f64)).ln();
        let q = (pmi * 10.0).round().clamp(-127.0, 127.0) as i8;
        big.push((
            hash2(hash_bytes(a.as_bytes()), hash_bytes(b.as_bytes())),
            [q as u8],
        ));
    }
    let n_big = big.len();
    w.section(b"BIGR", build_table::<1>(&big));
    // English word frequencies.
    let en: Vec<(u64, [u8; 1])> = src
        .en_freq
        .iter()
        .map(|(w, c)| {
            (
                hash_bytes(w.as_bytes()),
                [((*c as f32).ln() / F_STEP).round().min(255.0) as u8],
            )
        })
        .collect();
    w.section(b"ENFQ", build_table::<1>(&en));
    let homo: Vec<(u64, [u8; 4])> = src
        .homographs
        .iter()
        .map(|(r, n)| {
            (
                hash_bytes(r.as_bytes()),
                (hash_bytes(n.as_bytes()) as u32).to_le_bytes(),
            )
        })
        .collect();
    w.section(b"HOMO", build_table::<4>(&homo));
    // Letter models: romanised words weighted by their word's frequency, English words.
    let rom: Vec<(&str, f32)> = src
        .pairs
        .iter()
        .filter_map(|(n, r, _, _)| {
            fmap.get(n.as_str())
                .map(|f| (r.as_str(), (1.0 + *f as f32).ln()))
        })
        .collect();
    w.section(b"CLMI", charlm_train(&rom));
    let enw: Vec<(&str, f32)> = src
        .en_freq
        .iter()
        .filter(|(w, _)| w.bytes().all(|b| b.is_ascii_lowercase()))
        .map(|(w, c)| (w.as_str(), (1.0 + *c as f32).ln()))
        .collect();
    w.section(b"CLME", charlm_train(&enw));
    format!(
        "total={}\nen_total={}\nfreq_words={n_freq}\nlex_spellings={n_entries}\nlex_words={}\nbigrams={n_big}\n",
        src.total,
        src.en_total,
        natives.len()
    )
}

/// `recipe.txt` of a language: how its pack is built.
struct Recipe {
    lang: String,
    order: usize,
    min: Vec<u32>,
    min_token: u32,
    prune: f32,
    max_pairs: usize,
    /// Least count of a word of the English list.
    en_min: u32,
    /// English word counts, relative to the language's directory.
    en_freq: String,
    exclude: Vec<String>,
    license: String,
    extra: Vec<String>,
    weights: Vec<(String, String)>,
    opt: BuildOptions,
}

fn lines(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .map(|t| t.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

fn recipe(dir: &Path, name: &str) -> Result<Recipe, String> {
    let mut r = Recipe {
        lang: name.to_string(),
        order: 6,
        min: vec![1, 1, 2, 3, 4, 5],
        min_token: 20,
        prune: 2.0,
        max_pairs: 0,
        en_min: 2,
        en_freq: "../en_freq.tsv".into(),
        exclude: Vec::new(),
        license: "unspecified".into(),
        extra: Vec::new(),
        weights: Vec::new(),
        opt: BuildOptions {
            freq_min: 3,
            lex_min: 20,
            bigram_min: 10,
            lex_sources: Vec::new(),
        },
    };
    let path = dir.join("recipe.txt");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    for l in text.lines() {
        let l = l.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        let (k, v) = l
            .split_once('=')
            .ok_or_else(|| format!("recipe line without '=': {l}"))?;
        let num = |v: &str| v.parse::<f64>().map_err(|_| format!("{k}: not a number"));
        let list = |v: &str| v.split(',').map(str::to_string).collect::<Vec<_>>();
        match k {
            "lang" => r.lang = v.to_string(),
            "order" => r.order = num(v)? as usize,
            "min" => r.min = v.split(',').filter_map(|x| x.parse().ok()).collect(),
            "min_token" => r.min_token = num(v)? as u32,
            "prune" => r.prune = num(v)? as f32,
            "max_pairs" => r.max_pairs = num(v)? as usize,
            "en_min" => r.en_min = num(v)? as u32,
            "en_freq" => r.en_freq = v.to_string(),
            "freq_min" => r.opt.freq_min = num(v)? as u32,
            "lex_min" => r.opt.lex_min = num(v)? as u32,
            "bigram_min" => r.opt.bigram_min = num(v)? as u32,
            "lex_sources" => r.opt.lex_sources = list(v),
            "exclude" => r.exclude = list(v),
            "license" => r.license = v.to_string(),
            "extra" => r.extra.push(v.to_string()),
            _ if k.starts_with("w.") => r.weights.push((k[2..].to_string(), v.to_string())),
            _ => return Err(format!("unknown recipe key {k:?}")),
        }
    }
    Ok(r)
}

/// Build the pack of `name` (a directory of `data/sources/translit`).
/// `Ok(None)` when the large inputs are not on this machine.
fn build_pack(
    root: &Path,
    name: &str,
    data: Option<&Path>,
    sets: &[(String, String)],
    log: bool,
) -> Result<Option<Vec<u8>>, String> {
    let base = root.join("data/sources/translit");
    let dir = base.join(name);
    let mut r = recipe(&dir, name)?;
    for (k, v) in sets {
        r.weights.retain(|w| w.0 != *k);
        r.weights.push((k.clone(), v.clone()));
    }
    let big = |f: &str| -> PathBuf {
        let here = dir.join(f);
        match data {
            Some(d) if !here.exists() => d.join(name).join(f),
            _ => here,
        }
    };
    if !big("train.tsv").exists() {
        return Ok(None);
    }
    let mut src = Sources::default();
    for l in lines(&big("train.tsv")) {
        let p: Vec<&str> = l.split('\t').collect();
        if p.len() < 4 || r.exclude.iter().any(|x| x == p[2]) {
            continue;
        }
        src.pairs.push((
            p[0].to_string(),
            p[1].to_string(),
            p[2].to_string(),
            p[3].parse().unwrap_or(1),
        ));
    }
    for l in lines(&big("freq.tsv")) {
        let Some((w, c)) = l.split_once('\t') else {
            continue;
        };
        if w == "#total" {
            src.total = c.parse().unwrap_or(0);
        } else {
            src.freq.push((w.to_string(), c.parse().unwrap_or(0)));
        }
    }
    for l in lines(&big("bigrams.tsv")) {
        let p: Vec<&str> = l.split('\t').collect();
        if p.len() == 3 {
            src.bigrams.push((
                p[0].to_string(),
                p[1].to_string(),
                p[2].parse().unwrap_or(0),
            ));
        }
    }
    let hand = |file: &Path| -> Vec<(String, String)> {
        lines(file)
            .iter()
            .filter(|l| !l.starts_with('#'))
            .filter_map(|l| l.split_once('\t'))
            .map(|(a, b)| (a.trim().to_lowercase(), b.trim().to_string()))
            .filter(|(a, b)| {
                !a.is_empty() && !b.is_empty() && a.bytes().all(|c| c.is_ascii_lowercase())
            })
            .collect()
    };
    src.extra = hand(&dir.join("chat.tsv"));
    for e in &r.extra {
        src.extra.extend(hand(&dir.join(e)));
    }
    src.homographs = hand(&dir.join("homographs.tsv"));
    for l in lines(&dir.join(&r.en_freq)) {
        if let Some((w, c)) = l.split_once('\t') {
            let c: u32 = c.parse().unwrap_or(0);
            src.en_total += c as u64;
            if c >= r.en_min {
                src.en_freq.push((w.to_lowercase(), c));
            }
        }
    }
    // The machine-mined rows are thinned by a fixed stride when the recipe
    // caps the pairs the model trains on; the lexicon still sees all rows.
    let mined = src.pairs.iter().filter(|p| p.2 == "IndicCorp").count();
    let others = src.pairs.len() - mined;
    let keep_mined = if r.max_pairs > 0 && src.pairs.len() > r.max_pairs {
        r.max_pairs.saturating_sub(others)
    } else {
        mined
    };
    let mut seen = 0usize;
    let rows: Vec<(String, String, u32)> = src
        .pairs
        .iter()
        .filter(|p| {
            if p.2 != "IndicCorp" {
                return true;
            }
            seen += 1;
            mined == keep_mined || (seen * keep_mined) / mined != ((seen - 1) * keep_mined) / mined
        })
        .map(|(n, r, _, _)| (n.clone(), r.clone(), 1))
        .collect();
    let t = train(&rows, r.order, &r.min, r.min_token, r.prune, log);
    let mut w = Writer::default();
    t.write(&mut w);
    let mut meta = format!(
        "lang=xlit-{}\nkind=translit\nformat={}\nlicense={}\norder={}\nngrams={}\npairs={}\n",
        r.lang,
        uccharan::translit::FORMAT,
        r.license,
        r.order,
        t.gram_count(),
        t.pairs_used
    );
    meta.push_str(&build_words(&src, &r.opt, &mut w));
    for (k, v) in &r.weights {
        meta.push_str(&format!("w.{k}={v}\n"));
    }
    w.section(b"META", meta.into_bytes());
    Ok(Some(write_container(&w.sections)))
}

fn out_path(root: &Path, name: &str) -> PathBuf {
    if name == "toy" {
        root.join("crates/uccharan/tests/data/xlit-toy.ucd")
    } else {
        root.join(format!("data/packs/xlit-{name}.ucd"))
    }
}

/// The `xlit`, `xlit-toy` and `xlit-check` commands.
pub fn run(root: &Path, cmd: &str, args: &[String]) {
    let mut names: Vec<String> = Vec::new();
    let mut data: Option<PathBuf> = std::env::var_os("UCCHARAN_XLIT_DATA").map(PathBuf::from);
    let mut out: Option<PathBuf> = None;
    let mut sets: Vec<(String, String)> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let v = args.get(i + 1).cloned().unwrap_or_default();
        match args[i].as_str() {
            "--data" => data = Some(PathBuf::from(v)),
            "--out" => out = Some(PathBuf::from(v)),
            "--set" => match v.split_once('=') {
                Some((k, x)) => sets.push((k.to_string(), x.to_string())),
                None => {
                    eprintln!("--set takes name=value");
                    std::process::exit(2);
                }
            },
            name => {
                names.push(name.to_string());
                i += 1;
                continue;
            }
        }
        i += 2;
    }
    if cmd == "xlit-toy" {
        names = vec!["toy".into()];
    }
    if cmd == "xlit-check" && names.is_empty() {
        if let Ok(rd) = std::fs::read_dir(root.join("data/sources/translit")) {
            names = rd
                .flatten()
                .filter(|e| e.path().join("recipe.txt").exists())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
        }
    }
    if names.is_empty() {
        eprintln!("usage: lexgen xlit <lang>... [--data DIR] [--out FILE] [--set name=value]");
        std::process::exit(2);
    }
    let mut stale = false;
    for name in &names {
        let path = out.clone().unwrap_or_else(|| out_path(root, name));
        let file = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        match build_pack(root, name, data.as_deref(), &sets, cmd != "xlit-check") {
            Err(e) => {
                eprintln!("{name}: {e}");
                std::process::exit(1);
            }
            Ok(None) if cmd == "xlit-check" => {
                println!("{file}: sources not on this machine, not checked");
            }
            Ok(None) => {
                eprintln!(
                    "{name}: train.tsv not found (see data/sources/translit/README.md; --data DIR)"
                );
                std::process::exit(1);
            }
            Ok(Some(bytes)) if cmd == "xlit-check" => {
                let same = std::fs::read(&path).map(|o| o == bytes).unwrap_or(false);
                println!("{file}: {}", if same { "up to date" } else { "STALE" });
                stale |= !same;
            }
            Ok(Some(bytes)) => {
                if let Some(d) = path.parent() {
                    let _ = std::fs::create_dir_all(d);
                }
                std::fs::write(&path, &bytes).unwrap();
                println!("{file}: {} bytes", bytes.len());
            }
        }
    }
    if stale {
        std::process::exit(1);
    }
}
