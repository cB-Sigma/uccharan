//! Builds uccharan's compiled language data (`crates/uccharan/data/*.ucd`)
//! from the sources in `data/sources`.
//!
//! ```text
//! cargo run --release -p uccharan-lexgen            # write the .ucd files
//! cargo run --release -p uccharan-lexgen -- check   # fail if they are stale
//! cargo run --release -p uccharan-lexgen -- dump    # TSV of every conversion (debugging)
//! cargo run --release -p uccharan-lexgen -- lts-eval  # held-out accuracy of the letter-to-sound model
//! ```

// The conversion rules are written as flat, ordered condition chains: one branch per
// observation, even where two branches give the same result.
#![allow(
    clippy::if_same_then_else,
    clippy::type_complexity,
    clippy::too_many_arguments
)]

mod align;
mod cmudict;
mod en;
mod g2p;
mod hi;
mod lts;
mod wikipron;
mod words;

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use uccharan::data::{Builder, Data};

fn lts_examples(
    entries: &[cmudict::Entry],
    accent: en::Accent,
    skip: impl Fn(&str) -> bool,
) -> Vec<(Vec<u8>, Vec<String>)> {
    let mut out = Vec::new();
    for e in entries {
        if e.variant != 1 || !e.aligned || skip(&e.word) || e.word.contains('.') {
            continue;
        }
        let letters = cmudict::letters_of(&e.word);
        if letters.len() < 2 {
            continue;
        }
        let ph = en::convert(e, accent);
        let mut outs = vec![String::new(); letters.len()];
        for p in &ph {
            let slot = &mut outs[p.li.min(letters.len() - 1)];
            match p.stress {
                1 => slot.push('ˈ'),
                2 => slot.push('ˌ'),
                _ => {}
            }
            slot.push_str(&p.s);
        }
        out.push((letters, outs));
    }
    out
}

/// A plain non-rhotic rendering of a General American form, for names that
/// have no British column.
fn gb_from_us(us: &str) -> String {
    let mut s = us.replace("oʊ", "əʊ").replace('ɚ', "ə");
    let cs: Vec<char> = s.chars().collect();
    let mut out = String::new();
    for (i, &c) in cs.iter().enumerate() {
        if c == 'ɹ' {
            let next = cs[i + 1..].iter().find(|&&n| n != 'ˈ' && n != 'ˌ');
            if next.is_none_or(|n| !"aeiouæɑɒɐɔəɛɜɪʊʌ".contains(*n)) {
                continue;
            }
        }
        out.push(c);
    }
    s = out;
    if s.ends_with('i') {
        s.pop();
        s.push('ɪ');
    }
    s
}

/// Word tables built from `data/sources/<lang>/words.tsv`: file, language, word source.
const LATIN_TABLES: [(&str, &str, &str); 3] = [
    (
        "de.ucd",
        "de",
        "the words of Tatoeba and Leipzig Corpora Collection German sentences",
    ),
    (
        "nl.ucd",
        "nl",
        "the words of Tatoeba and Leipzig Corpora Collection Dutch sentences",
    ),
    (
        "fr.ucd",
        "fr",
        "the words of Tatoeba and Leipzig Corpora Collection French sentences",
    ),
];

/// misaki's dictionaries (data/sources/misaki, Apache-2.0) for the Kokoro English front end.
fn build_kokoro(root: &Path, british: bool) -> Vec<u8> {
    let accent = if british { "gb" } else { "us" };
    let read = |name: &str| {
        std::fs::read_to_string(root.join(format!("data/sources/misaki/{accent}_{name}.json")))
            .expect("misaki dictionary")
    };
    let (entries, gold, silver) =
        uccharan::kokoro::MisakiLexicon::image_entries(&read("gold"), &read("silver"))
            .expect("valid JSON");
    let mut b = Builder::new();
    // The Kokoro front end makes several lookups per word: small blocks.
    b.block_size(8);
    for (k, v) in entries {
        b.entry(&k, &v);
    }
    b.meta(
        "lang",
        if british {
            "kokoro-en-gb"
        } else {
            "kokoro-en-us"
        },
    );
    b.meta("format", "1");
    b.meta("gold", &gold.to_string());
    b.meta("silver", &silver.to_string());
    b.meta(
        "source",
        "misaki 0.9.4 dictionaries (hexgrad/misaki, Apache-2.0), see data/sources/misaki/README.md",
    );
    b.build().expect("build")
}

const LTS_MIN_COUNT: u32 = 2;

fn build_english(
    entries: &[cmudict::Entry],
    accent: en::Accent,
    name: &str,
    overrides: &str,
    names: &str,
) -> Vec<u8> {
    let mut map: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for e in entries {
        let pron = en::render(&en::convert(e, accent));
        let v = map.entry(e.word.as_str()).or_default();
        if !v.contains(&pron) {
            v.push(pron);
        }
    }
    // Hand-written corrections replace the first variant.
    let col = if accent == en::Accent::Us { 1 } else { 2 };
    for line in overrides
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
    {
        let f: Vec<&str> = line.split('\t').collect();
        if let (Some(w), Some(p)) = (f.first(), f.get(col)) {
            let v = map.entry(w).or_default();
            v.retain(|x| x != p);
            if v.is_empty() {
                v.push(p.to_string());
            } else {
                v[0] = p.to_string();
            }
            // The possessive is derived from the corrected form at run time.
            let possessive = format!("{w}'s");
            map.remove(possessive.as_str());
        }
    }
    let min_count: u32 = std::env::var("LEXGEN_MIN_COUNT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(LTS_MIN_COUNT);
    let model = lts::train(&lts_examples(entries, accent, |_| false), min_count);
    let names_model = name_model(entries);
    let lang = if accent == en::Accent::Us {
        uccharan::Lang::EnUs
    } else {
        uccharan::Lang::EnGb
    };
    let image = |dropped: &std::collections::HashSet<&str>| -> Vec<u8> {
        let mut b = Builder::new();
        for (w, v) in &map {
            if !dropped.contains(w) {
                b.entry(w, &v.join("|"));
            }
        }
        add_names(&mut b, names, accent);
        b.lts(model.outputs.clone(), model.tables.clone());
        b.section(*b"NAMC", names_model.clone());
        let left_out: Vec<&str> = dropped.iter().copied().collect();
        b.section(
            *b"KNWN",
            uccharan::known::encode(&left_out, uccharan::known::RICE_K).expect("known-word set"),
        );
        b.meta("lang", name);
        b.meta("format", "2");
        b.meta("source", "CMUdict (BSD-2-Clause) converted by tools/lexgen, plus data/sources/en/overrides.tsv and names.tsv; entries the letter-to-sound model reproduces exactly are left out and listed in KNWN");
        b.build().expect("build")
    };
    // Leave out every entry that the run-time rules (morphology, compounds,
    // letter-to-sound) already reproduce exactly, and check the result: each
    // left-out word must still come out identical to the full lexicon.
    let full = Data::from_vec(image(&Default::default())).expect("image");
    let full_en = uccharan::en::English::new(&full, lang);
    let want: BTreeMap<&str, String> = map
        .keys()
        .map(|w| (*w, full_en.word(w, uccharan::en::Hint::None)))
        .collect();
    let mut dropped: std::collections::HashSet<&str> = map
        .iter()
        // Words with an apostrophe stay: the run-time set of left-out words is
        // not consulted for them (a false hit there would hide the 's rules).
        .filter(|(w, v)| {
            !w.contains('\'')
                && v.len() == 1
                && uccharan::en::letter_to_sound(&full, w, lang) == v[0]
        })
        .map(|(w, _)| *w)
        .collect();
    for round in 0..8 {
        let reduced = Data::from_vec(image(&dropped)).expect("image");
        let en_r = uccharan::en::English::new(&reduced, lang);
        let bad: Vec<&str> = map
            .keys()
            .copied()
            .filter(|w| en_r.word(w, uccharan::en::Hint::None) != want[w])
            .collect();
        eprintln!(
            "{name}: prune round {round}: {} left out, {} differ",
            dropped.len(),
            bad.len()
        );
        if bad.is_empty() {
            break;
        }
        for w in bad {
            dropped.remove(w);
        }
    }
    let bytes = image(&dropped);
    let check = Data::from_vec(bytes.clone()).expect("image");
    let en_c = uccharan::en::English::new(&check, lang);
    assert!(
        map.keys()
            .all(|w| en_c.word(w, uccharan::en::Hint::None) == want[w]),
        "pruned lexicon is not equivalent"
    );
    bytes
}

/// The name-origin model: Indian names (data/sources/names) against CMUdict's words.
fn name_model(entries: &[cmudict::Entry]) -> Vec<u8> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let text = std::fs::read_to_string(root.join("data/sources/names/indian_names.txt"))
        .expect("indian_names.txt");
    let indian: Vec<String> = text
        .lines()
        .filter(|l| !l.starts_with('#'))
        .flat_map(|l| l.split_whitespace())
        .map(|w| w.to_lowercase())
        .collect();
    let set: std::collections::HashSet<&str> = indian.iter().map(String::as_str).collect();
    let english: Vec<String> = entries
        .iter()
        .filter(|e| {
            e.variant == 1
                && e.word.bytes().all(|b| b.is_ascii_lowercase())
                && !set.contains(e.word.as_str())
        })
        .map(|e| e.word.clone())
        .collect();
    let model = uccharan::names::train_model(&indian, &english);
    let t = uccharan::names::INDIC_THRESHOLD;
    let hit_i = indian
        .iter()
        .filter(|w| uccharan::names::indic_score(&model, w) >= t)
        .count();
    let hit_e = english
        .iter()
        .filter(|w| uccharan::names::indic_score(&model, w) >= t)
        .count();
    eprintln!(
        "name-origin model: {} trigrams; flags {}/{} Indian training names, {}/{} CMUdict words",
        model.len() / 4,
        hit_i,
        indian.len(),
        hit_e,
        english.len()
    );
    model
}

fn add_names(b: &mut Builder, names: &str, accent: en::Accent) {
    // Names and brands for the improved mode, stored under a "^" prefix.
    for line in names
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
    {
        let f: Vec<&str> = line.split('\t').collect();
        let us = f.get(1).copied().unwrap_or("");
        let pron = match (accent, f.get(2).copied().filter(|g| !g.is_empty())) {
            (en::Accent::Us, _) => us.to_string(),
            (_, Some(gb)) => gb.to_string(),
            _ => gb_from_us(us),
        };
        if let Some(w) = f.first() {
            b.entry(&format!("^{w}"), &pron);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("build");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let text = std::fs::read_to_string(root.join("data/sources/cmudict/cmudict.dict"))
        .expect("cmudict.dict");
    let entries = cmudict::load(&text);
    let out_dir = root.join("crates/uccharan/data");
    match cmd {
        "dump" => {
            let out = std::io::stdout();
            let mut out = std::io::BufWriter::new(out.lock());
            for e in &entries {
                let us = en::render(&en::convert(e, en::Accent::Us));
                let gb = en::render(&en::convert(e, en::Accent::Gb));
                let al: Vec<String> = e.segs.iter().map(|s| format!("{}:{}", s.g, s.p)).collect();
                writeln!(
                    out,
                    "{}\t{}\t{}\t{}\t{}",
                    e.word,
                    e.variant,
                    us,
                    gb,
                    al.join(" ")
                )
                .unwrap();
            }
        }
        "build" | "check" => {
            let overrides = std::fs::read_to_string(root.join("data/sources/en/overrides.tsv"))
                .unwrap_or_default();
            let names =
                std::fs::read_to_string(root.join("data/sources/en/names.tsv")).unwrap_or_default();
            // LEXGEN_ONLY=<tag> builds one Latin-script word table.
            let only = std::env::var("LEXGEN_ONLY").ok();
            let files = if only.is_some() {
                Vec::new()
            } else {
                vec![
                    (
                        "en-us.ucd",
                        build_english(&entries, en::Accent::Us, "en-us", &overrides, &names),
                    ),
                    (
                        "en-gb.ucd",
                        build_english(&entries, en::Accent::Gb, "en-gb", &overrides, &names),
                    ),
                    ("hi.ucd", hi::build(&root)),
                    (
                        "ur.ucd",
                        words::build(
                            &root,
                            "ur",
                            "derived from running espeak-ng 1.52.0 on Sangraha Urdu word lists",
                        ),
                    ),
                    (
                        "sd.ucd",
                        words::build(
                            &root,
                            "sd",
                            "derived from running espeak-ng 1.52.0 on Sangraha Sindhi word lists",
                        ),
                    ),
                    ("kokoro-en-us.ucd", build_kokoro(&root, false)),
                    ("kokoro-en-gb.ucd", build_kokoro(&root, true)),
                ]
            };
            // Word tables of the Latin-script languages.
            let mut files: Vec<(&str, Vec<u8>)> = files;
            for (name, lang, what) in LATIN_TABLES {
                if only.as_deref().is_some_and(|o| o != lang) {
                    continue;
                }
                files.push((
                    name,
                    words::build(
                        &root,
                        lang,
                        &format!("derived from running espeak-ng 1.52.0 on {what}"),
                    ),
                ));
            }
            let mut stale = false;
            for (name, bytes) in files {
                let path = out_dir.join(name);
                let d = Data::from_vec(bytes.clone()).expect("valid image");
                if cmd == "check" {
                    let same = std::fs::read(&path)
                        .map(|old| old == bytes)
                        .unwrap_or(false);
                    println!("{name}: {}", if same { "up to date" } else { "STALE" });
                    stale |= !same;
                } else {
                    std::fs::create_dir_all(&out_dir).unwrap();
                    std::fs::write(&path, &bytes).unwrap();
                    println!(
                        "{name}: {} entries, {} bytes, lts: {}",
                        d.len(),
                        bytes.len(),
                        d.has_lts()
                    );
                }
            }
            if stale {
                eprintln!("compiled data is stale: run `cargo run --release -p uccharan-lexgen`");
                std::process::exit(1);
            }
        }
        "wikipron" | "wikipron-check" => {
            // Optional CC BY-SA packs, kept out of the core data directory.
            let dir = root.join("data/packs");
            let mut stale = false;
            for (kind, name) in [
                (wikipron::Kind::Us, "wikipron-en-us.ucd"),
                (wikipron::Kind::Gb, "wikipron-en-gb.ucd"),
                (wikipron::Kind::Hi, "wikipron-hi.ucd"),
            ] {
                let (bytes, n, skipped) = wikipron::build(&root, kind);
                let path = dir.join(name);
                if cmd == "wikipron-check" {
                    let same = std::fs::read(&path).map(|o| o == bytes).unwrap_or(false);
                    println!("{name}: {}", if same { "up to date" } else { "STALE" });
                    stale |= !same;
                } else {
                    std::fs::create_dir_all(&dir).unwrap();
                    std::fs::write(&path, &bytes).unwrap();
                    println!(
                        "{name}: {n} entries ({skipped} skipped), {} bytes",
                        bytes.len()
                    );
                }
            }
            if stale {
                std::process::exit(1);
            }
        }
        "prune-stats" => {
            // How many lexicon entries the letter-to-sound tables reproduce exactly.
            for name in ["en-us.ucd", "en-gb.ucd"] {
                let bytes = std::fs::read(out_dir.join(name)).unwrap();
                let d = Data::from_vec(bytes).unwrap();
                let lang = if name.starts_with("en-us") {
                    uccharan::Lang::EnUs
                } else {
                    uccharan::Lang::EnGb
                };
                let (mut n, mut same, mut multi) = (0usize, 0usize, 0usize);
                let mut pron = String::new();
                for e in &entries {
                    if e.variant != 1 {
                        continue;
                    }
                    if !d.lookup(&e.word, &mut pron) {
                        continue;
                    }
                    n += 1;
                    if pron.contains('|') {
                        multi += 1;
                        continue;
                    }
                    if uccharan::en::letter_to_sound(&d, &e.word, lang) == pron {
                        same += 1;
                    }
                }
                println!(
                    "{name}: entries {n}, multi-variant {multi}, LTS reproduces {same} ({:.1}%)",
                    100.0 * same as f64 / n as f64
                );
            }
        }
        "lts-eval" => {
            // Hold out every tenth word (by hash), train on the rest, and score the
            // model against the lexicon's own conversion of the held-out words.
            let held = |w: &str| {
                w.bytes()
                    .fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32))
                    % 10
                    == 0
            };
            for (accent, lang) in [
                (en::Accent::Us, uccharan::Lang::EnUs),
                (en::Accent::Gb, uccharan::Lang::EnGb),
            ] {
                let model = lts::train(&lts_examples(&entries, accent, held), 2);
                let mut b = Builder::new();
                b.entry("a", "ə");
                b.lts(model.outputs, model.tables);
                let d = Data::from_vec(b.build().unwrap()).unwrap();
                let (mut n, mut exact, mut edits, mut len) = (0usize, 0usize, 0usize, 0usize);
                for e in &entries {
                    if e.variant != 1
                        || !e.aligned
                        || !held(&e.word)
                        || e.word.contains(['.', '\'', '-'])
                        || e.word.len() < 2
                    {
                        continue;
                    }
                    let want = en::render(&en::convert(e, accent));
                    let got = uccharan::en::letter_to_sound(&d, &e.word, lang);
                    n += 1;
                    exact += (want == got) as usize;
                    edits += edit_distance(&want, &got);
                    len += want.chars().count();
                }
                println!(
                    "{lang:?}: held-out words {n}, exact {:.1}%, phoneme error rate {:.1}%",
                    100.0 * exact as f64 / n as f64,
                    100.0 * edits as f64 / len as f64
                );
            }
        }
        "g2p-eval" => {
            // g2p-eval LANG FILE [MIN_COUNT]: hold out every tenth word of a
            // `word<TAB>reading` list, train on the rest, score model and rules.
            let a: Vec<String> = std::env::args().skip(2).collect();
            let lang = uccharan::Lang::parse(&a[0]).expect("language");
            let latin = lang.latin().expect("Latin-script language");
            let min: u32 = a.get(2).and_then(|v| v.parse().ok()).unwrap_or(2);
            let rows: Vec<(String, String)> = std::fs::read_to_string(&a[1])
                .unwrap()
                .lines()
                .filter(|l| !l.starts_with('#'))
                .filter_map(|l| l.split_once('\t'))
                .filter(|(w, r)| !r.contains(' ') && *w == w.to_lowercase())
                .map(|(w, r)| (w.to_string(), words::plain(r)))
                .collect();
            let held = |w: &str| {
                w.bytes()
                    .fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32))
                    % 10
                    == 0
            };
            let tr: Vec<(String, String)> =
                rows.iter().filter(|(w, _)| !held(w)).cloned().collect();
            // SHAPES="84,4 83,3 ..." (hex left,right bytes) tries another chain.
            let shapes: Vec<(u8, u8)> = match std::env::var("SHAPES") {
                Ok(v) => v
                    .split_whitespace()
                    .filter_map(|x| x.split_once(','))
                    .map(|(l, r)| {
                        (
                            u8::from_str_radix(l, 16).unwrap(),
                            u8::from_str_radix(r, 16).unwrap(),
                        )
                    })
                    .collect(),
                Err(_) => g2p::SHAPES.to_vec(),
            };
            let lat = std::env::var("LATENT").is_ok();
            let (extra, model) = g2p::train(&tr, min, &shapes, lat.then_some(latin));
            let mut b = Builder::new();
            b.lts(model.outputs, model.tables);
            b.meta("letters", &extra);
            if lat {
                b.meta("stress", "latent");
            }
            let bytes = b.build().unwrap();
            let size = bytes.len();
            let d = Data::from_vec(bytes).unwrap();
            let (mut n, mut m_ok, mut r_ok, mut t_ok, mut tn) = (0, 0, 0, 0, 0);
            let show = std::env::var("SHOW").is_ok();
            let mut seg = [0usize; 3];
            for (k, (w, r)) in rows.iter().enumerate() {
                let got = latin.model_word(w, &d).unwrap_or_default();
                if !held(w) {
                    if k % 10 == 1 {
                        tn += 1;
                        t_ok += (got == *r) as usize;
                    }
                    continue;
                }
                n += 1;
                m_ok += (got == *r) as usize;
                let rule = latin.rule_word(w, true);
                r_ok += (rule == *r) as usize;
                let st = uccharan::ipa::strip_stress;
                seg[0] += (st(&got) == st(r)) as usize;
                seg[1] += (st(&rule) == st(r)) as usize;
                let combo = latin.with_stress_of(&got, &rule).unwrap_or(rule.clone());
                seg[2] += (combo == *r) as usize;
                if show && got != *r {
                    println!("{w}\t{r}\t{got}");
                }
            }
            let pc = |a: usize, b: usize| 100.0 * a as f64 / b.max(1) as f64;
            println!(
                "segments only: model {:.1}% rules {:.1}%; model segments + rule stress {:.1}%",
                pc(seg[0], n),
                pc(seg[1], n),
                pc(seg[2], n)
            );
            println!(
                "{}: train {} held {n}; model exact {:.1}% rules exact {:.1}%; on training words {:.1}%; model {} bytes",
                a[0],
                tr.len(),
                pc(m_ok, n),
                pc(r_ok, n),
                pc(t_ok, tn),
                size
            );
        }
        _ => eprintln!("unknown command"),
    }
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            cur[j] = (prev[j] + 1)
                .min(cur[j - 1] + 1)
                .min(prev[j - 1] + (a[i - 1] != b[j - 1]) as usize);
        }
        prev = cur;
    }
    prev[b.len()]
}
