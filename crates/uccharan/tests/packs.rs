//! Word tables that are not compiled in: found as packs, or a clear error.
// Some tests are compiled only without the `bundle` and `download` features.
#![allow(unused_imports, dead_code)]

use uccharan::packs::{self, PackError};
use uccharan::{Lang, Options, Phonemizer};

/// The tests below change process-wide environment variables.
static ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn tables_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../uccharan-data/data")
}

#[test]
fn built_in_languages_need_no_pack() {
    let p = Phonemizer::new();
    #[cfg(all(feature = "embed-en-us", feature = "embed-hi"))]
    {
        assert!(p.missing_packs(Lang::EnUs).is_empty());
        assert!(p.missing_packs(Lang::Hi).is_empty());
        // Bhojpuri is read with the Hindi data.
        assert!(p.missing_packs(Lang::Bho).is_empty());
    }
    // Languages read by rules alone have no pack.
    assert!(p.missing_packs(Lang::Es).is_empty());
    assert!(p.missing_packs(Lang::Bn).is_empty());
    assert!(p.ensure(Lang::Bn).is_ok());
}

#[cfg(not(feature = "embed-de"))]
#[test]
fn a_table_is_loaded_from_the_data_directory_on_first_use() {
    let _g = ENV.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("UCCHARAN_DATA_DIR", tables_dir());
    let p = Phonemizer::new();
    assert_eq!(p.missing_packs(Lang::De), ["de"]);
    assert!(!p.has(Lang::De));
    // No ensure() call: the first use loads the table.
    let out = p.phonemize("Ich heiße Anna.", &Options::new(Lang::De));
    assert!(!out.is_empty());
    assert!(p.missing_packs(Lang::De).is_empty() && p.has(Lang::De));
    // The table changes the reading of words the letter rules get wrong.
    let rules = Phonemizer::empty().phonemize("Ich heiße Anna.", &Options::new(Lang::De));
    assert!(!rules.is_empty());
}

#[cfg(not(any(feature = "embed-de", feature = "download")))]
#[test]
fn a_missing_table_is_an_error_that_names_the_pack() {
    let _g = ENV.lock().unwrap_or_else(|e| e.into_inner());
    let empty = std::env::temp_dir().join(format!("uccharan-no-packs-{}", std::process::id()));
    std::fs::create_dir_all(&empty).unwrap();
    std::env::set_var("UCCHARAN_DATA_DIR", &empty);
    std::env::set_var("XDG_CACHE_HOME", &empty);
    let p = Phonemizer::new();
    let e = p.ensure(Lang::De).unwrap_err();
    assert_eq!(e.pack(), "de");
    assert!(matches!(e, PackError::Missing { .. }));
    let msg = e.to_string();
    assert!(msg.contains("\"de\"") && msg.contains("de.ucd") && msg.contains("UCCHARAN_DATA_DIR"));
    assert!(msg.contains("releases/download/v"), "{msg}");
    // The infallible call stops with the same message instead of reading
    // German with the letter rules alone.
    let r = std::panic::catch_unwind(|| p.phonemize("Guten Tag.", &Options::new(Lang::De)));
    let payload = r.unwrap_err();
    let text = payload
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_default();
    assert!(text.contains("language pack \"de\""), "{text}");
    // Opting in to the rules is explicit.
    let lenient = Phonemizer::new().rules_fallback(true);
    assert!(!lenient
        .phonemize("Guten Tag.", &Options::new(Lang::De))
        .is_empty());
    // Other languages are not affected.
    assert!(!p.phonemize("Hello.", &Options::new(Lang::EnUs)).is_empty());

    // A file with the wrong content is refused, not used.
    std::fs::write(empty.join("de.ucd"), b"not a table").unwrap();
    assert!(matches!(p.ensure(Lang::De), Err(PackError::Corrupt { .. })));
    let _ = std::fs::remove_dir_all(&empty);
    std::env::set_var("UCCHARAN_DATA_DIR", tables_dir());
    std::env::remove_var("XDG_CACHE_HOME");
}

#[test]
fn pack_list_matches_the_files() {
    for name in ["de", "fr", "ur", "sd", "da"] {
        let bytes = std::fs::read(tables_dir().join(format!("{name}.ucd"))).unwrap();
        assert!(packs::verify(name, &bytes), "{name}: run tools/packs.py");
    }
    assert!(packs::find("no-such-pack").is_err());
}
