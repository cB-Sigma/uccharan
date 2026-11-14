//! Print the rule reading of each word on standard input (one per line)
//! for a Latin-script language file, without its word table:
//! `cargo run --release --example rule_words -- de < words.txt`.

use std::io::{BufRead, Write};

fn main() {
    let tag = std::env::args().nth(1).unwrap_or_default();
    let Some(lang) = uccharan::Lang::parse(&tag) else {
        eprintln!("unknown language {tag}");
        std::process::exit(2);
    };
    let Some(l) = lang.latin() else {
        eprintln!("{tag} is not a Latin-script language file");
        std::process::exit(2);
    };
    let out = std::io::stdout();
    let mut out = out.lock();
    for line in std::io::stdin().lock().lines().map_while(Result::ok) {
        let _ = writeln!(out, "{}", l.rule_word(line.trim(), true));
    }
}
