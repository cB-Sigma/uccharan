//! Language packs: the data files (`<name>.ucd`) that are not compiled into
//! the library.
//!
//! English and Hindi are built in. The word tables of the other languages
//! (`de`, `fr`, `ur` …) are packs, attached to the GitHub release of the same
//! version. A pack is looked for, first match wins, in
//!
//! 1. `$UCCHARAN_DATA_DIR/<name>.ucd` (offline machines, containers; several
//!    directories may be given, separated like `PATH`);
//! 2. the cache, `$XDG_CACHE_HOME/uccharan/<version>/` or
//!    `~/.cache/uccharan/<version>/`;
//! 3. the release download, only with the `download` cargo feature and
//!    unless `UCCHARAN_OFFLINE=1`.
//!
//! Every file is checked against the SHA-256 sum in [`PACKS`] before it is
//! used. The `bundle` cargo feature compiles every word table in instead.
//! [`Phonemizer::ensure`](crate::Phonemizer::ensure) loads what a language
//! needs and reports what is missing.

use std::fmt;

/// A pack of this version: name, SHA-256 sum (hex) and size in bytes.
#[derive(Clone, Copy, Debug)]
pub struct Pack {
    /// File name without `.ucd`; for a word table, the language tag.
    pub name: &'static str,
    /// SHA-256 of the file, lower-case hex.
    pub sha256: &'static str,
    /// Size of the file in bytes.
    pub size: usize,
}

/// Every pack of this version.
pub static PACKS: &[Pack] = include!("packs_list.rs");

/// The pack with this name.
pub fn pack(name: &str) -> Option<&'static Pack> {
    PACKS.iter().find(|p| p.name == name)
}

/// Where a pack is published.
pub fn url(name: &str) -> String {
    format!(
        "https://github.com/cB-Sigma/uccharan/releases/download/v{}/{name}.ucd",
        env!("CARGO_PKG_VERSION")
    )
}

/// Why a pack could not be loaded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackError {
    /// No pack has this name.
    Unknown(String),
    /// The pack is in neither directory and was not downloaded.
    Missing {
        /// Pack name.
        name: String,
        /// True when `UCCHARAN_OFFLINE=1` stopped a download.
        offline: bool,
    },
    /// A file does not have the pack's SHA-256 sum.
    Corrupt {
        /// Pack name.
        name: String,
        /// The file or URL.
        source: String,
    },
    /// The download failed.
    Download {
        /// Pack name.
        name: String,
        /// What went wrong.
        reason: String,
    },
    /// The file is not a data image the library can read.
    Invalid {
        /// Pack name.
        name: String,
        /// What went wrong.
        reason: String,
    },
}

impl PackError {
    /// The pack the error is about.
    pub fn pack(&self) -> &str {
        match self {
            PackError::Unknown(n) => n,
            PackError::Missing { name, .. }
            | PackError::Corrupt { name, .. }
            | PackError::Download { name, .. }
            | PackError::Invalid { name, .. } => name,
        }
    }
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let v = env!("CARGO_PKG_VERSION");
        match self {
            PackError::Unknown(n) => write!(f, "unknown pack {n:?}"),
            PackError::Missing { name, offline } => {
                write!(
                    f,
                    "the language pack {name:?} is not installed. Put {name}.ucd of \
                     version {v} ({}) into the directory named by UCCHARAN_DATA_DIR",
                    url(name)
                )?;
                if let Some(c) = cache_dir() {
                    write!(f, " or into {}", c.display())?;
                }
                if *offline {
                    write!(f, ", or unset UCCHARAN_OFFLINE to let it be downloaded")
                } else if cfg!(feature = "download") {
                    Ok(())
                } else {
                    write!(
                        f,
                        ", or build with the cargo feature `bundle` (all word tables compiled \
                         in) or `download` (fetched on first use)"
                    )
                }
            }
            PackError::Corrupt { name, source } => write!(
                f,
                "{source} does not have the SHA-256 sum of pack {name:?} of version {v}"
            ),
            PackError::Download { name, reason } => write!(
                f,
                "could not download the language pack {name:?} from {}: {reason}",
                url(name)
            ),
            PackError::Invalid { name, reason } => {
                write!(f, "the language pack {name:?} is not usable: {reason}")
            }
        }
    }
}

impl std::error::Error for PackError {}

/// The cache directory of this version (`None` when no home directory is known).
pub fn cache_dir() -> Option<std::path::PathBuf> {
    let var = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty());
    let base = match var("XDG_CACHE_HOME") {
        Some(x) => std::path::PathBuf::from(x),
        None => {
            std::path::PathBuf::from(var("HOME").or_else(|| var("USERPROFILE"))?).join(".cache")
        }
    };
    Some(base.join("uccharan").join(env!("CARGO_PKG_VERSION")))
}

fn offline() -> bool {
    std::env::var_os("UCCHARAN_OFFLINE").is_some_and(|v| v == "1")
}

/// Check bytes against a pack's size and SHA-256 sum.
pub fn verify(name: &str, bytes: &[u8]) -> bool {
    pack(name).is_some_and(|p| p.size == bytes.len() && sha256_hex(bytes) == p.sha256)
}

/// The verified bytes of a pack from `$UCCHARAN_DATA_DIR`, the cache or
/// (cargo feature `download`) the release. A downloaded pack is written to
/// the cache.
pub fn find(name: &str) -> Result<Vec<u8>, PackError> {
    if pack(name).is_none() {
        return Err(PackError::Unknown(name.to_string()));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let file = format!("{name}.ucd");
        if let Some(dirs) = std::env::var_os("UCCHARAN_DATA_DIR") {
            for dir in std::env::split_paths(&dirs).filter(|d| !d.as_os_str().is_empty()) {
                let p = dir.join(&file);
                if let Ok(bytes) = std::fs::read(&p) {
                    if verify(name, &bytes) {
                        return Ok(bytes);
                    }
                    return Err(PackError::Corrupt {
                        name: name.to_string(),
                        source: p.display().to_string(),
                    });
                }
            }
        }
        if let Some(p) = cache_dir().map(|d| d.join(&file)) {
            if let Ok(bytes) = std::fs::read(&p) {
                if verify(name, &bytes) {
                    return Ok(bytes);
                }
                // A damaged cache file is replaced by the next download.
                let _ = std::fs::remove_file(&p);
            }
        }
        #[cfg(feature = "download")]
        if !offline() {
            return download(name);
        }
    }
    Err(PackError::Missing {
        name: name.to_string(),
        offline: cfg!(feature = "download") && offline(),
    })
}

/// Download a pack from the release, verify it and write it to the cache
/// (cargo feature `download`). Ignores `UCCHARAN_OFFLINE`: this is the
/// explicit call.
#[cfg(all(feature = "download", not(target_arch = "wasm32")))]
pub fn download(name: &str) -> Result<Vec<u8>, PackError> {
    let p = pack(name).ok_or_else(|| PackError::Unknown(name.to_string()))?;
    let fail = |reason: String| PackError::Download {
        name: name.to_string(),
        reason,
    };
    let link = url(name);
    // The system's certificate store, so that a company proxy's certificate works.
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into();
    let mut resp = agent.get(&link).call().map_err(|e| fail(e.to_string()))?;
    let bytes = resp
        .body_mut()
        .with_config()
        .limit(p.size as u64 + 1)
        .read_to_vec()
        .map_err(|e| fail(e.to_string()))?;
    if !verify(name, &bytes) {
        return Err(PackError::Corrupt {
            name: name.to_string(),
            source: link,
        });
    }
    // A read-only home still works, just without a cache.
    if let Some(dir) = cache_dir() {
        if std::fs::create_dir_all(&dir).is_ok() {
            let tmp = dir.join(format!("{name}.ucd.{}.part", std::process::id()));
            if std::fs::write(&tmp, &bytes).is_ok()
                && std::fs::rename(&tmp, dir.join(format!("{name}.ucd"))).is_err()
            {
                let _ = std::fs::remove_file(&tmp);
            }
        }
    }
    Ok(bytes)
}

/// SHA-256 of `data` as lower-case hex.
pub fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut block = |b: &[u8]| {
        let mut w = [0u32; 64];
        for (i, c) in b.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([c[0], c[1], c[2], c[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [
                t1.wrapping_add(t2),
                v[0],
                v[1],
                v[2],
                v[3].wrapping_add(t1),
                v[4],
                v[5],
                v[6],
            ];
        }
        for (a, b) in h.iter_mut().zip(v) {
            *a = a.wrapping_add(b);
        }
    };
    let mut chunks = data.chunks_exact(64);
    for c in &mut chunks {
        block(c);
    }
    let rest = chunks.remainder();
    let mut tail = [0u8; 128];
    tail[..rest.len()].copy_from_slice(rest);
    tail[rest.len()] = 0x80;
    let n = if rest.len() < 56 { 64 } else { 128 };
    tail[n - 8..n].copy_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for c in tail[..n].chunks_exact(64) {
        block(c);
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known_answers() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        // 55, 56 and 64 bytes: the padding boundaries.
        for (n, want) in [
            (
                55,
                "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318",
            ),
            (
                56,
                "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a",
            ),
            (
                64,
                "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb",
            ),
        ] {
            assert_eq!(sha256_hex(&vec![b'a'; n]), want, "{n}");
        }
    }

    #[test]
    fn every_pack_has_a_sum() {
        assert!(pack("en-us").is_some() && pack("de").is_some());
        assert!(PACKS.iter().all(|p| p.sha256.len() == 64 && p.size > 0));
        assert!(url("de").ends_with("/de.ucd"));
    }
}
