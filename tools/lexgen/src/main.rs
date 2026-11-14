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
mod xlit;

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
const LATIN_TABLES: [(&str, &str, &str); 16] = [
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
    (
        "it.ucd",
        "it",
        "the words of Tatoeba and Leipzig Corpora Collection Italian sentences",
    ),
    (
        "pt-br.ucd",
        "pt-br",
        "the words of Tatoeba and Leipzig Corpora Collection Portuguese (Brazil) sentences",
    ),
    (
        "pt.ucd",
        "pt",
        "the words of Tatoeba and Leipzig Corpora Collection Portuguese (Portugal) sentences",
    ),
    (
        "pl.ucd",
        "pl",
        "the words of Tatoeba and Leipzig Corpora Collection Polish sentences",
    ),
    (
        "tr.ucd",
        "tr",
        "the words of Tatoeba and Leipzig Corpora Collection Turkish sentences",
    ),
    (
        "cs.ucd",
        "cs",
        "the words of Tatoeba and Leipzig Corpora Collection Czech sentences",
    ),
    (
        "sv.ucd",
        "sv",
        "the words of Tatoeba and Leipzig Corpora Collection Swedish sentences",
    ),
    (
        "ca.ucd",
        "ca",
        "the words of Tatoeba and Leipzig Corpora Collection Catalan sentences",
    ),
    (
        "ro.ucd",
        "ro",
        "the words of Tatoeba and Leipzig Corpora Collection Romanian sentences",
    ),
    (
        "hu.ucd",
        "hu",
        "the words of Tatoeba and Leipzig Corpora Collection Hungarian sentences",
    ),
    (
        "fi.ucd",
        "fi",
        "the words of Tatoeba and Leipzig Corpora Collection Finnish sentences",
    ),
    (
        "nb.ucd",
        "nb",
        "the words of Tatoeba and Leipzig Corpora Collection Norwegian Bokmål sentences",
    ),
    (
        "da.ucd",
        "da",
        "the words of Tatoeba and Leipzig Corpora Collection Danish sentences",
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
            // LEXGEN_ONLY=<tag> builds one word table.
            let only = std::env::var("LEXGEN_ONLY").ok();
            type Job<'a> = (
                &'a str,
                Option<&'a str>,
                Box<dyn Fn() -> Vec<u8> + Send + Sync + 'a>,
            );
            let mut jobs: Vec<Job> = Vec::new();
            if only.is_none() {
                jobs.push((
                    "en-us.ucd",
                    None,
                    Box::new(|| {
                        build_english(&entries, en::Accent::Us, "en-us", &overrides, &names)
                    }),
                ));
                jobs.push((
                    "en-gb.ucd",
                    None,
                    Box::new(|| {
                        build_english(&entries, en::Accent::Gb, "en-gb", &overrides, &names)
                    }),
                ));
                jobs.push(("hi.ucd", None, Box::new(|| hi::build(&root))));
                jobs.push((
                    "kokoro-en-us.ucd",
                    None,
                    Box::new(|| build_kokoro(&root, false)),
                ));
                jobs.push((
                    "kokoro-en-gb.ucd",
                    None,
                    Box::new(|| build_kokoro(&root, true)),
                ));
            }
            // Word tables.
            let tables = [
                ("ur.ucd", "ur", "Sangraha Urdu word lists"),
                ("sd.ucd", "sd", "Sangraha Sindhi word lists"),
            ]
            .into_iter()
            .chain(LATIN_TABLES);
            for (name, lang, what) in tables {
                if only.as_deref().is_some_and(|o| o != lang) {
                    continue;
                }
                let root = &root;
                jobs.push((
                    name,
                    Some(lang),
                    Box::new(move || {
                        words::build(
                            root,
                            lang,
                            &format!("derived from running espeak-ng 1.52.0 on {what}"),
                        )
                    }),
                ));
            }
            // `check` remembers, under target/, the inputs a word table was
            // last found up to date with (LEXGEN_NO_CACHE=1 checks afresh).
            let cache_dir = root.join("target/lexgen-check");
            let use_cache = cmd == "check" && std::env::var("LEXGEN_NO_CACHE").is_err();
            let code = digest_dirs(&[
                root.join("crates/uccharan/src"),
                root.join("tools/lexgen/src"),
            ]);
            let stamp = |name: &str, lang: &str| -> String {
                let mut h = code;
                for p in [
                    root.join("data/sources").join(lang).join("words.tsv"),
                    root.join("crates/uccharan/lang")
                        .join(format!("{lang}.lang")),
                    out_dir.join(name),
                ] {
                    h = digest_bytes(h, &std::fs::read(p).unwrap_or_default());
                }
                format!("{h:016x}")
            };
            // The tables are built side by side (LEXGEN_JOBS, default: all cores).
            let workers = std::env::var("LEXGEN_JOBS")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .or_else(|| std::thread::available_parallelism().ok().map(|n| n.get()))
                .unwrap_or(1)
                .clamp(1, jobs.len().max(1));
            let next = std::sync::atomic::AtomicUsize::new(0);
            let stale = std::sync::atomic::AtomicBool::new(false);
            std::thread::scope(|sc| {
                for _ in 0..workers {
                    sc.spawn(|| loop {
                        let k = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some((name, lang, job)) = jobs.get(k) else {
                            break;
                        };
                        let path = out_dir.join(name);
                        let mark = cache_dir.join(format!("{name}.ok"));
                        if let Some(lang) = lang.filter(|_| use_cache) {
                            if std::fs::read_to_string(&mark).is_ok_and(|m| m == stamp(name, lang))
                            {
                                println!(
                                    "{name}: up to date (inputs unchanged since the last check)"
                                );
                                continue;
                            }
                        }
                        let bytes = job();
                        let d = Data::from_vec(bytes.clone()).expect("valid image");
                        if cmd == "check" {
                            let same = std::fs::read(&path)
                                .map(|old| old == bytes)
                                .unwrap_or(false);
                            println!("{name}: {}", if same { "up to date" } else { "STALE" });
                            if !same {
                                stale.store(true, std::sync::atomic::Ordering::Relaxed);
                            } else if let Some(lang) = lang {
                                let _ = std::fs::create_dir_all(&cache_dir);
                                let _ = std::fs::write(&mark, stamp(name, lang));
                            }
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
                    });
                }
            });
            let stale = stale.into_inner();
            if stale {
                eprintln!("compiled data is stale: run `cargo run --release -p uccharan-lexgen`");
                std::process::exit(1);
            }
        }
        "xlit" | "xlit-toy" | "xlit-check" => xlit::run(&root, cmd, &args[2..]),
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
            let latin = lang.latin();
            let indic = lang.indic();
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
            // VIA_RULES=1: the model reads the rules' reading, not the spelling.
            let via = std::env::var("VIA_RULES").is_ok();
            let input = |w: &str| match indic {
                Some(i) if via => i.word(w, true),
                _ => w.to_string(),
            };
            let tr: Vec<(String, String)> = rows
                .iter()
                .filter(|(w, _)| !held(w))
                .map(|(w, r)| (input(w), r.clone()))
                .collect();
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
            let (extra, model) = g2p::train(&tr, min, &shapes, latin.filter(|_| lat));
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
                let got = match (latin, indic) {
                    (Some(l), _) => l.model_word(w, &d),
                    (_, Some(_)) if via => d.model_word(&input(w)),
                    (_, Some(i)) => i.model_word(w, &d),
                    _ => None,
                }
                .unwrap_or_default();
                if !held(w) {
                    if k % 10 == 1 {
                        tn += 1;
                        t_ok += (got == *r) as usize;
                    }
                    continue;
                }
                n += 1;
                m_ok += (got == *r) as usize;
                let rule = match (latin, indic) {
                    (Some(l), _) => l.rule_word(w, true),
                    (_, Some(i)) => i.word(w, true),
                    _ => String::new(),
                };
                r_ok += (rule == *r) as usize;
                let st = uccharan::ipa::strip_stress;
                seg[0] += (st(&got) == st(r)) as usize;
                seg[1] += (st(&rule) == st(r)) as usize;
                let combo = latin
                    .and_then(|l| l.with_stress_of(&got, &rule))
                    .unwrap_or(rule.clone());
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

/// A running digest (FNV-1a, 64 bit) of `bytes` after `h`.
fn digest_bytes(mut h: u64, bytes: &[u8]) -> u64 {
    for b in bytes.iter().chain(&[0xff]) {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// A digest of the names and contents of the files below `dirs`.
fn digest_dirs(dirs: &[std::path::PathBuf]) -> u64 {
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    let mut todo: Vec<std::path::PathBuf> = dirs.to_vec();
    while let Some(d) = todo.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                todo.push(p);
            } else {
                files.push(p);
            }
        }
    }
    files.sort();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for f in files {
        h = digest_bytes(h, f.file_name().unwrap_or_default().as_encoded_bytes());
        h = digest_bytes(h, &std::fs::read(&f).unwrap_or_default());
    }
    h
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
