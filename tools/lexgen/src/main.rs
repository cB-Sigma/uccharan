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
mod hi;
mod lts;

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
    let mut b = Builder::new();
    for (w, v) in &map {
        b.entry(w, &v.join("|"));
    }
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
    let model = lts::train(&lts_examples(entries, accent, |_| false), 2);
    b.lts(model.outputs, model.tables);
    b.meta("lang", name);
    b.meta("format", "1");
    b.meta("source", "CMUdict (BSD-2-Clause) converted by tools/lexgen, plus data/sources/en/overrides.tsv and names.tsv");
    b.build().expect("build")
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
            let files = [
                (
                    "en-us.ucd",
                    build_english(&entries, en::Accent::Us, "en-us", &overrides, &names),
                ),
                (
                    "en-gb.ucd",
                    build_english(&entries, en::Accent::Gb, "en-gb", &overrides, &names),
                ),
                ("hi.ucd", hi::build(&root)),
                ("kokoro-en-us.ucd", build_kokoro(&root, false)),
                ("kokoro-en-gb.ucd", build_kokoro(&root, true)),
            ];
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
