//! The optional WikiPron packs (built by `uccharan-lexgen wikipron` into
//! `data/packs`, not shipped with the crate). Skipped when they are absent.

use uccharan::{Lang, Mode, Options, Phonemizer};

fn pack(name: &str) -> Option<Vec<u8>> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/packs")
        .join(format!("{name}.ucd"));
    std::fs::read(p).ok()
}

#[test]
fn improved_mode_uses_pack_and_compatible_mode_does_not() {
    let Some(bytes) = pack("wikipron-en-us") else {
        return;
    };
    let p = Phonemizer::new();
    let mut opt = Options::new(Lang::EnUs);
    let compat_before = p.phonemize("Kwame lives in Abuja.", &opt);
    assert_eq!(p.load_pack(bytes).unwrap(), "wikipron-en-us");
    assert!(p.has_wikipron(Lang::EnUs));
    assert_eq!(p.phonemize("Kwame lives in Abuja.", &opt), compat_before);
    opt.mode = Mode::Improved;
    let out = p.phonemize("Kwame", &opt);
    assert!(out.ends_with("meɪ"), "{out}");
}

#[test]
fn hindi_pack_keeps_conventions() {
    let Some(bytes) = pack("wikipron-hi") else {
        return;
    };
    let p = Phonemizer::new();
    p.load_pack(bytes).unwrap();
    let mut opt = Options::new(Lang::Hi);
    opt.mode = Mode::Improved;
    // Nukta letters and a final short i keep uccharan's improved-mode symbols.
    assert_eq!(p.phonemize("बाज़ार", &opt), "baːzˈaːɾ");
    assert!(p.phonemize("अनुमति", &opt).ends_with('ɪ'));
}
