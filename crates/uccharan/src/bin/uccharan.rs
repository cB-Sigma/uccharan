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
            "-V" | "--version" => {
                println!("uccharan {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "-h" | "--help" => {
                println!(
                    "usage: uccharan [-l en-us|en-gb|hi] [-p espeak-ipa|piper|kokoro|kitten|matcha|sherpa-onnx]\n                [--normalize natural|espeak|off] [--improved] [--hinglish] [--no-punctuation]\n                [--ids [--voice-config FILE]] [--threads N] [--misaki DIR] [--pack FILE.ucd]... [TEXT...]\n\nReads lines from standard input when no TEXT is given."
                );
                return;
            }
            _ => texts.push(a),
        }
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

fn die(msg: &str) -> ! {
    eprintln!("uccharan: {msg}");
    std::process::exit(2)
}
