//! Command-line front end: one output line per input line.
//!
//! ```text
//! uccharan [-l en-us|en-gb|hi] [-p espeak-ipa|piper|kokoro|kitten|matcha|sherpa-onnx]
//!          [--normalize natural|espeak|off] [--improved] [--hinglish]
//!          [--no-punctuation] [--ids [--voice-config FILE]] [--threads N] [TEXT...]
//! ```
//! `--ids` prints model input ids instead of symbols (`piper` needs the
//! voice's `.onnx.json` via `--voice-config`; sentences are separated by
//! ` | `). `--threads N` phonemizes standard input in parallel (0 = all cores).
//! `--misaki DIR` replaces the built-in copy of misaki's English dictionaries
//! (`us_gold.json`, `us_silver.json`, `gb_*` in DIR) for the `kokoro` profile.
//! `-p text` prints the spoken form of each line (normalise-only mode) and
//! `-p text-json` the spoken form with spans; `--norm-options JSON` sets the
//! normalise options. `--wordlist FILE` (TSV or PLS, repeatable) applies a
//! pronunciation word list in both modes.
//! `-p native` writes the romanised words of each line in the script of the
//! language (`aap kaise ho` → `आप कैसे हो`) and `-p native-json` adds spans and
//! alternatives; `--pack xlit-hi.ucd` loads the language's transliteration
//! pack and `--translit-options JSON` sets the options.
//! `--match-terms FILE` corrects each line against a vocabulary (a word list,
//! a JSON array of terms, or one term per line) by sound and prints one JSON
//! object per line with the new text, the changes and the candidates;
//! `--match-options JSON` sets the matching options, `--index-options JSON`
//! the languages the vocabulary is read in, and `--match-stats` prints
//! timings to standard error.
//! English and Hindi are built in; the word table of another language is a
//! pack, looked for in `$UCCHARAN_DATA_DIR` and the cache at first use (see
//! `uccharan::packs`). `--packs` lists the packs of this version and
//! `--download NAME...` fetches packs into the cache (cargo feature
//! `download`).
//! Without TEXT arguments, lines are read from standard input.

use std::io::{BufRead, Write};
use uccharan::{profile, Lang, Mode, Normalize, Options, Phonemizer};

fn main() {
    let mut lang = Lang::EnUs;
    let mut prof = "espeak-ipa".to_string();
    let mut normalize = Normalize::Natural;
    let mut mode = Mode::Compatible;
    let mut hinglish = false;
    let mut punctuation = true;
    let mut ids = false;
    let mut voice_config: Option<String> = None;
    let mut threads: Option<usize> = None;
    let mut misaki: Option<String> = None;
    let mut packs: Vec<String> = Vec::new();
    let mut texts: Vec<String> = Vec::new();
    let mut norm_options = String::new();
    let mut translit_options = String::new();
    let mut wordlists: Vec<std::sync::Arc<uccharan::wordlist::WordList>> = Vec::new();
    let mut match_terms: Option<String> = None;
    let mut match_options = String::new();
    let mut index_options = String::new();
    let mut match_stats = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "-l" | "--lang" => {
                let v = args.next().unwrap_or_default();
                lang = Lang::parse(&v).unwrap_or_else(|| die(&format!("unknown language '{v}'")));
            }
            "-p" | "--profile" => prof = args.next().unwrap_or_default(),
            "--normalize" => {
                normalize = match args.next().unwrap_or_default().as_str() {
                    "natural" => Normalize::Natural,
                    "espeak" | "espeak-like" => Normalize::EspeakLike,
                    "off" => Normalize::Off,
                    v => die(&format!("unknown normalisation '{v}'")),
                }
            }
            "--improved" => mode = Mode::Improved,
            "--hinglish" => hinglish = true,
            "--no-punctuation" => punctuation = false,
            "--ids" => ids = true,
            "--voice-config" => voice_config = args.next(),
            "--threads" => threads = args.next().and_then(|v| v.parse().ok()),
            "--misaki" => misaki = args.next(),
            "--pack" => packs.extend(args.next()),
            "--packs" => {
                // The packs of this version and where each one stands.
                let ph = Phonemizer::new();
                for k in uccharan::packs::PACKS {
                    let built_in = Lang::parse(k.name).is_some_and(|l| ph.data(l).is_some());
                    let state = if built_in {
                        "built in"
                    } else if on_disk(k.name) {
                        "installed"
                    } else {
                        "not installed"
                    };
                    println!("{:16} {:>9} bytes  {state}", k.name, k.size);
                }
                return;
            }
            "--download" => {
                // Fetch packs into the cache (cargo feature `download`).
                for name in args.by_ref() {
                    fetch(&name);
                }
                return;
            }
            "--norm-options" => norm_options = args.next().unwrap_or_default(),
            "--translit-options" => translit_options = args.next().unwrap_or_default(),
            "--wordlist" => {
                let f = args.next().unwrap_or_default();
                let t = std::fs::read_to_string(&f).unwrap_or_else(|e| die(&format!("{f}: {e}")));
                let wl = uccharan::wordlist::WordList::parse_any(&t)
                    .unwrap_or_else(|e| die(&format!("{f}: {e}")));
                wordlists.push(std::sync::Arc::new(wl));
            }
            "--match-terms" => match_terms = args.next(),
            "--match-options" => match_options = args.next().unwrap_or_default(),
            "--index-options" => index_options = args.next().unwrap_or_default(),
            "--match-stats" => match_stats = true,
            "-V" | "--version" => {
                println!("uccharan {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "-h" | "--help" => {
                println!(
                    "usage: uccharan [-l en-us|en-gb|hi] [-p espeak-ipa|piper|kokoro|kitten|matcha|sherpa-onnx]\n                [--normalize natural|espeak|off] [--improved] [--hinglish] [--no-punctuation]\n                [--ids [--voice-config FILE]] [--threads N] [--misaki DIR] [--pack FILE.ucd]... [TEXT...]\n                [--packs] [--download NAME...]\n                [--match-terms FILE [--match-options JSON] [--index-options JSON] [--match-stats]]\n                [-p native|native-json --pack xlit-LANG.ucd [--translit-options JSON]]\n\nReads lines from standard input when no TEXT is given."
                );
                return;
            }
            _ => texts.push(a),
        }
    }
    if let Some(f) = &match_terms {
        let p = Phonemizer::new();
        p.ensure(lang).unwrap_or_else(|e| die(&e.to_string()));
        let vocabulary = std::fs::read_to_string(f).unwrap_or_else(|e| die(&format!("{f}: {e}")));
        let t0 = std::time::Instant::now();
        let index = uccharan::soundalike::SoundIndex::from_text(&p, &vocabulary, &index_options)
            .unwrap_or_else(|e| die(&format!("{f}: {e}")));
        let built = t0.elapsed();
        let lines: Vec<String> = if texts.is_empty() {
            std::io::stdin()
                .lock()
                .lines()
                .map_while(Result::ok)
                .collect()
        } else {
            texts.clone()
        };
        let mut out = std::io::BufWriter::new(std::io::stdout().lock());
        let t1 = std::time::Instant::now();
        for line in &lines {
            let s = index
                .apply_json(&p, line, lang, &match_options)
                .unwrap_or_else(|e| die(&format!("--match-options: {e}")));
            let _ = writeln!(out, "{s}");
        }
        if match_stats {
            let secs = t1.elapsed().as_secs_f64();
            let words: usize = lines.iter().map(|l| l.split_whitespace().count()).sum();
            eprintln!(
                "terms {} sequences {} build_ms {:.1} lines {} words {} seconds {:.4} lines_per_s {:.0} words_per_s {:.0}",
                index.len(),
                index.variants(),
                built.as_secs_f64() * 1e3,
                lines.len(),
                words,
                secs,
                lines.len() as f64 / secs.max(1e-9),
                words as f64 / secs.max(1e-9)
            );
        }
        return;
    }
    if prof == "native" || prof == "native-json" {
        // Romanised text into the script of the language (see docs/transliteration.md).
        let p = Phonemizer::new();
        for f in &packs {
            let bytes = std::fs::read(f).unwrap_or_else(|e| die(&format!("{f}: {e}")));
            p.load_pack(bytes)
                .unwrap_or_else(|e| die(&format!("{f}: {e}")));
        }
        let o = uccharan::translit::options_from_json(&translit_options)
            .unwrap_or_else(|e| die(&format!("--translit-options: {e}")));
        let mut out = std::io::BufWriter::new(std::io::stdout().lock());
        let mut write = |line: &str| {
            let t = uccharan::translit::transliterate(&p, line, lang, &o);
            let s = if prof == "native" {
                t.text.clone()
            } else {
                t.to_json(line)
            };
            let _ = writeln!(out, "{s}");
        };
        if texts.is_empty() {
            for line in std::io::stdin().lock().lines().map_while(Result::ok) {
                write(&line);
            }
        } else {
            texts.iter().for_each(|t| write(t));
        }
        return;
    }
    if prof == "text" || prof == "text-json" {
        // The spoken form needs no word table.
        let p = Phonemizer::new().rules_fallback(true);
        let mut o = uccharan::normalize_api::options_from_json(&norm_options)
            .unwrap_or_else(|e| die(&format!("--norm-options: {e}")));
        if mode == Mode::Improved {
            o.mode = mode;
        }
        o.wordlists.extend(wordlists.iter().cloned());
        let lines: Vec<String> = if texts.is_empty() {
            std::io::stdin()
                .lock()
                .lines()
                .map_while(Result::ok)
                .collect()
        } else {
            texts.clone()
        };
        let mut out = std::io::BufWriter::new(std::io::stdout().lock());
        for line in &lines {
            let n = uccharan::normalize_api::normalize(&p, line, lang, &o);
            let s = if prof == "text" {
                n.text.clone()
            } else {
                n.to_json(line)
            };
            let _ = writeln!(out, "{s}");
        }
        return;
    }
    let Some(kind) = profile::Profile::parse(&prof) else {
        die(&format!("unknown profile '{prof}'"))
    };
    let p = Phonemizer::new();
    for f in &packs {
        let bytes = std::fs::read(f).unwrap_or_else(|e| die(&format!("{f}: {e}")));
        p.load_pack(bytes)
            .unwrap_or_else(|e| die(&format!("{f}: {e}")));
    }
    p.ensure(lang).unwrap_or_else(|e| die(&e.to_string()));
    if let Some(dir) = &misaki {
        let accent = if lang == Lang::EnGb { "gb" } else { "us" };
        let read = |name: &str| {
            std::fs::read_to_string(std::path::Path::new(dir).join(format!("{accent}_{name}.json")))
                .unwrap_or_else(|e| die(&format!("{dir}: {e}")))
        };
        if lang != Lang::Hi {
            p.load_misaki(lang, &read("gold"), &read("silver"))
                .unwrap_or_else(|e| die(&e));
        }
    }
    let mut opt = Options::new(lang);
    opt.normalize = normalize;
    opt.mode = mode;
    opt.hinglish = hinglish;
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    let id_map = match (&voice_config, ids, kind) {
        (Some(path), true, profile::Profile::Piper) => {
            let json =
                std::fs::read_to_string(path).unwrap_or_else(|e| die(&format!("{path}: {e}")));
            Some(
                profile::PhonemeIdMap::from_json(&json)
                    .unwrap_or_else(|| die("no phoneme_id_map in the voice config")),
            )
        }
        (None, true, profile::Profile::Piper) => {
            die("--ids with the piper profile needs --voice-config FILE")
        }
        _ => None,
    };
    let join = |v: &[i64]| {
        v.iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    };
    let render = |line: &str| -> String {
        if !ids && !wordlists.is_empty() {
            return uccharan::normalize_api::render_with_wordlists(
                &p,
                line,
                &opt,
                kind,
                punctuation,
                &wordlists,
            );
        }
        if !ids {
            return p.render(line, &opt, kind, punctuation);
        }
        let sentences = p.analyze(line, &opt);
        match kind {
            profile::Profile::Piper => {
                let map = id_map.as_ref().expect("checked above");
                profile::piper_sentences(&sentences)
                    .iter()
                    .map(|s| join(&map.ids(s).0))
                    .collect::<Vec<_>>()
                    .join(" | ")
            }
            profile::Profile::Kokoro => join(&profile::kokoro_ids(&profile::kokoro_tokens(
                &sentences, lang,
            ))),
            profile::Profile::Kitten => join(&profile::kitten_ids(&profile::espeak_ipa(
                &sentences, true, None,
            ))),
            profile::Profile::Matcha => join(&profile::matcha_ids(&profile::espeak_ipa(
                &sentences, true, None,
            ))),
            other => profile::render_text(&sentences, other, lang, punctuation),
        }
    };
    if let (Some(n), true, false) = (threads, texts.is_empty(), ids) {
        let lines: Vec<String> = std::io::stdin()
            .lock()
            .lines()
            .map_while(Result::ok)
            .collect();
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        for s in p.render_batch(&refs, &opt, kind, punctuation, n) {
            let _ = writeln!(out, "{s}");
        }
        return;
    }
    let mut emit = |line: &str| {
        let s = render(line);
        let _ = writeln!(out, "{s}");
    };
    if texts.is_empty() {
        for line in std::io::stdin().lock().lines().map_while(Result::ok) {
            emit(&line);
        }
    } else {
        for t in &texts {
            emit(t);
        }
    }
}

/// True when a verified copy of the pack is in the data directory or the cache.
fn on_disk(name: &str) -> bool {
    let file = format!("{name}.ucd");
    let mut dirs: Vec<std::path::PathBuf> = std::env::var_os("UCCHARAN_DATA_DIR")
        .map(|v| std::env::split_paths(&v).collect())
        .unwrap_or_default();
    dirs.extend(uccharan::packs::cache_dir());
    dirs.into_iter()
        .any(|d| std::fs::read(d.join(&file)).is_ok_and(|b| uccharan::packs::verify(name, &b)))
}

#[cfg(feature = "download")]
fn fetch(name: &str) {
    match uccharan::packs::download(name) {
        Ok(b) => eprintln!("uccharan: {name}.ucd, {} bytes", b.len()),
        Err(e) => die(&e.to_string()),
    }
}

#[cfg(not(feature = "download"))]
fn fetch(name: &str) {
    die(&format!(
        "this build cannot download (cargo feature `download`): get {} and put it into the \
         directory named by UCCHARAN_DATA_DIR",
        uccharan::packs::url(name)
    ))
}

fn die(msg: &str) -> ! {
    eprintln!("uccharan: {msg}");
    std::process::exit(2)
}
