//! Word tables of [uccharan](https://crates.io/crates/uccharan)'s languages
//! other than English and Hindi.
//!
//! uccharan depends on this crate when its `bundle` feature (every table) or
//! one of its `embed-<tag>` features is on; there is nothing to call here
//! directly. Each table has a cargo feature named after its language tag.
#![no_std]

/// Tags of the tables this crate can carry.
pub const TAGS: &[&str] = &[
    "ur", "sd", "de", "nl", "fr", "it", "pt-br", "pt", "pl", "tr", "cs", "sv", "ca", "ro", "hu",
    "fi", "nb", "da",
];

/// The compiled-in word table of a language (`<tag>.ucd`), when its feature is on.
pub fn table(tag: &str) -> Option<&'static [u8]> {
    match tag {
        #[cfg(feature = "ur")]
        "ur" => Some(include_bytes!("../data/ur.ucd")),
        #[cfg(feature = "sd")]
        "sd" => Some(include_bytes!("../data/sd.ucd")),
        #[cfg(feature = "de")]
        "de" => Some(include_bytes!("../data/de.ucd")),
        #[cfg(feature = "nl")]
        "nl" => Some(include_bytes!("../data/nl.ucd")),
        #[cfg(feature = "fr")]
        "fr" => Some(include_bytes!("../data/fr.ucd")),
        #[cfg(feature = "it")]
        "it" => Some(include_bytes!("../data/it.ucd")),
        #[cfg(feature = "pt-br")]
        "pt-br" => Some(include_bytes!("../data/pt-br.ucd")),
        #[cfg(feature = "pt")]
        "pt" => Some(include_bytes!("../data/pt.ucd")),
        #[cfg(feature = "pl")]
        "pl" => Some(include_bytes!("../data/pl.ucd")),
        #[cfg(feature = "tr")]
        "tr" => Some(include_bytes!("../data/tr.ucd")),
        #[cfg(feature = "cs")]
        "cs" => Some(include_bytes!("../data/cs.ucd")),
        #[cfg(feature = "sv")]
        "sv" => Some(include_bytes!("../data/sv.ucd")),
        #[cfg(feature = "ca")]
        "ca" => Some(include_bytes!("../data/ca.ucd")),
        #[cfg(feature = "ro")]
        "ro" => Some(include_bytes!("../data/ro.ucd")),
        #[cfg(feature = "hu")]
        "hu" => Some(include_bytes!("../data/hu.ucd")),
        #[cfg(feature = "fi")]
        "fi" => Some(include_bytes!("../data/fi.ucd")),
        #[cfg(feature = "nb")]
        "nb" => Some(include_bytes!("../data/nb.ucd")),
        #[cfg(feature = "da")]
        "da" => Some(include_bytes!("../data/da.ucd")),
        _ => None,
    }
}
