//! # uccharan
//!
//! Text-to-phoneme conversion (G2P) for neural text-to-speech, written from
//! scratch under the Apache-2.0 licence. It produces the phoneme streams that
//! espeak-trained voices expect (Piper, Kokoro, Kitten TTS, Matcha-TTS,
//! sherpa-onnx) for en-US, en-GB and Hindi, including code-mixed
//! Hindi + English.
//!
//! ```
//! use uccharan::{Lang, Options, Phonemizer};
//!
//! let p = Phonemizer::new();
//! let ipa = p.phonemize("Hello, world.", &Options::new(Lang::EnUs));
//! assert_eq!(ipa, "həlˈoʊ, wˈɜːld.");
//! ```
//!
//! The pipeline is: segment → normalise → per-token language routing →
//! lexicon / morphology / letter-to-sound → clause-level pass → render to a
//! [`profile`]. See `docs/architecture.md` in the repository.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod data;
pub mod en;
pub mod hi;
pub mod ipa;
pub mod known;
pub mod kokoro;
pub mod lex2;
pub mod names;
pub mod norm;
pub mod profile;
pub mod text;

use data::{Data, DataError};
pub use text::Terminator;

/// Languages (and accents) with a front end.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lang {
    /// General American English (espeak voice `en-us`).
    EnUs,
    /// British English, Received Pronunciation (espeak voice `en-gb-x-rp`).
    EnGb,
    /// Hindi in Devanagari (espeak voice `hi`).
    Hi,
}

impl Lang {
    /// Parse a language tag or an espeak voice name.
    pub fn parse(s: &str) -> Option<Lang> {
        match s.to_ascii_lowercase().replace('_', "-").as_str() {
            "en" | "en-us" | "a" => Some(Lang::EnUs),
            "en-gb" | "en-gb-x-rp" | "en-uk" | "b" => Some(Lang::EnGb),
            "hi" | "hi-in" | "h" | "hin" => Some(Lang::Hi),
            _ => None,
        }
    }

    /// Canonical tag.
    pub fn tag(self) -> &'static str {
        match self {
            Lang::EnUs => "en-us",
            Lang::EnGb => "en-gb",
            Lang::Hi => "hi",
        }
    }

    /// All languages.
    pub const ALL: [Lang; 3] = [Lang::EnUs, Lang::EnGb, Lang::Hi];

    fn index(self) -> usize {
        self as usize
    }
}

/// How numbers, dates and symbols are read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Normalize {
    /// Natural readings: "$5.50" → "five dollars and fifty cents".
    #[default]
    Natural,
    /// The reference phonemizer's readings: "$5.50" → "dollar five point five zero".
    EspeakLike,
    /// No expansion; digits and symbols are dropped.
    Off,
}

/// Which behaviour to reproduce where the reference phonemizer is known to
/// be idiosyncratic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    /// Match the phoneme streams existing voices were trained on, quirks included.
    #[default]
    Compatible,
    /// Linguistically cleaner output (see `docs/improved-mode.md`). Not what
    /// existing voices were trained on.
    Improved,
}

/// Request options.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    /// Language of the text. Tokens in another script are routed per token.
    pub lang: Lang,
    /// Number and symbol reading.
    pub normalize: Normalize,
    /// Compatible or improved behaviour.
    pub mode: Mode,
    /// Detect romanised Hindi ("kya haal hai") in Latin-script tokens of a
    /// Hindi request. Experimental.
    pub hinglish: bool,
    /// Keep `(en)`…`(hi)` language-switch flags in phonemizer-shaped output.
    pub keep_language_flags: bool,
    /// Accent used for Latin-script words inside Hindi text.
    pub embedded_english: Lang,
}

impl Options {
    /// Defaults for a language.
    pub fn new(lang: Lang) -> Options {
        Options {
            lang,
            normalize: Normalize::Natural,
            mode: Mode::Compatible,
            hinglish: false,
            keep_language_flags: false,
            embedded_english: Lang::EnGb,
        }
    }
}

/// One output word.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordOut {
    /// The source text of the word (after normalisation).
    pub text: String,
    /// Phonemes in espeak-style IPA with stress marks.
    pub ipa: String,
    /// Language whose rules produced the phonemes.
    pub lang: Lang,
    /// Joined to the previous word without a space.
    pub glue: bool,
}

impl WordOut {
    pub(crate) fn new(text: &str, ipa: String, lang: Lang, glue: bool) -> WordOut {
        WordOut {
            text: text.to_string(),
            ipa,
            lang,
            glue,
        }
    }
}

/// One clause: words and the punctuation that ended it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clause {
    /// Words in order.
    pub words: Vec<WordOut>,
    /// The clause terminator.
    pub terminator: Terminator,
}

/// One sentence: one or more clauses, the last of which ends the sentence.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Sentence {
    /// Clauses in order.
    pub clauses: Vec<Clause>,
}

/// The phonemizer. Cheap to share between threads (`&Phonemizer` is `Sync`).
#[derive(Clone, Debug, Default)]
pub struct Phonemizer {
    data: [std::sync::OnceLock<Data>; 3],
    /// misaki dictionaries for the Kokoro English front end (US, GB).
    misaki: [std::sync::OnceLock<std::sync::Arc<kokoro::MisakiLexicon>>; 2],
    /// Compiled Kokoro English packs loaded at run time (US, GB); dictionaries
    /// given to `load_misaki` take precedence.
    kokoro_packs: [std::sync::OnceLock<std::sync::Arc<kokoro::MisakiLexicon>>; 2],
}

impl Phonemizer {
    /// A phonemizer with the language data that was compiled in (see the
    /// `embed-*` cargo features).
    pub fn new() -> Phonemizer {
        #[allow(unused_mut)]
        let mut p = Phonemizer::default();
        #[cfg(feature = "embed-en-us")]
        {
            if let Ok(d) = Data::from_static(include_bytes!("../data/en-us.ucd")) {
                p.data[0] = d.into();
            }
        }
        #[cfg(feature = "embed-en-gb")]
        {
            if let Ok(d) = Data::from_static(include_bytes!("../data/en-gb.ucd")) {
                p.data[1] = d.into();
            }
        }
        #[cfg(feature = "embed-hi")]
        {
            if let Ok(d) = Data::from_static(include_bytes!("../data/hi.ucd")) {
                p.data[2] = d.into();
            }
        }
        p
    }

    /// A phonemizer with no data; load languages with [`Phonemizer::load`].
    pub fn empty() -> Phonemizer {
        Phonemizer::default()
    }

    /// Load (or replace) the data image of a language.
    pub fn load(&mut self, lang: Lang, bytes: Vec<u8>) -> Result<(), DataError> {
        self.data[lang.index()] = Data::from_vec(bytes)?.into();
        Ok(())
    }

    /// Load a language pack (`en-us.ucd`, `en-gb.ucd`, `hi.ucd`,
    /// `kokoro-en-us.ucd` or `kokoro-en-gb.ucd`) into a shared phonemizer.
    /// The pack names its language in its metadata. Returns that name.
    /// A language that already has data keeps it (the call still succeeds),
    /// so that a pack can be loaded on first use from several threads.
    pub fn load_pack(&self, bytes: Vec<u8>) -> Result<String, String> {
        let d = Data::from_vec(bytes).map_err(|e| e.to_string())?;
        let name = d.meta("lang").unwrap_or("").to_string();
        if let Some(accent) = name.strip_prefix("kokoro-") {
            let lang =
                Lang::parse(accent).ok_or_else(|| format!("unknown pack language {name:?}"))?;
            if self.has_misaki(lang) {
                return Ok(name);
            }
            return match self.load_kokoro_data(d) {
                Ok(_) => Ok(name),
                Err(_) if self.has_misaki(lang) => Ok(name),
                Err(e) => Err(e),
            };
        }
        let lang = Lang::parse(&name).ok_or_else(|| format!("unknown pack language {name:?}"))?;
        let _ = self.data[lang.index()].set(d);
        Ok(name)
    }

    /// True when the language's data is available.
    pub fn has(&self, lang: Lang) -> bool {
        self.data[lang.index()].get().is_some()
    }

    /// The loaded data image of a language.
    pub fn data(&self, lang: Lang) -> Option<&Data> {
        self.data[lang.index()].get()
    }

    fn english(&self, lang: Lang) -> Option<en::English<'_>> {
        // Fall back to the other accent's data rather than produce nothing.
        let d = self.data[lang.index()]
            .get()
            .or(self.data[0].get())
            .or(self.data[1].get())?;
        Some(en::English::new(d, lang))
    }

    /// Analyse text into sentences, clauses and words.
    pub fn analyze(&self, text: &str, opt: &Options) -> Vec<Sentence> {
        let compat = opt.mode == Mode::Compatible;
        let mut sentences: Vec<Sentence> = Vec::new();
        let mut cur = Sentence::default();
        for raw in text::clauses(text) {
            let mut items: Vec<norm::Item> = Vec::new();
            for (ci, chunk) in raw.chunks.iter().enumerate() {
                // "Maple Dr." / "Baker St.": after a capitalised name and not before
                // another one, these are "drive" and "street", not titles.
                if opt.normalize == Normalize::Natural
                    && chunk.dot
                    && ci > 0
                    && opt.lang != Lang::Hi
                {
                    let capital =
                        |c: &text::Chunk| c.text.chars().next().is_some_and(|x| x.is_uppercase());
                    let road = match chunk.text.as_str() {
                        "Dr" => Some("drive"),
                        "St" => Some("street"),
                        "Rd" => Some("road"),
                        "Ave" => Some("avenue"),
                        "Blvd" => Some("boulevard"),
                        _ => None,
                    };
                    let prev = &raw.chunks[ci - 1];
                    let next_is_name = raw.chunks.get(ci + 1).is_some_and(capital);
                    if let Some(word) = road {
                        if capital(prev) && !prev.dot && !next_is_name {
                            items.push(norm::Item::word(word));
                            continue;
                        }
                    }
                }
                items.extend(norm::expand_with(chunk, opt.lang, opt.normalize, !compat));
            }
            if compat {
                // The reference writes hyphenated compounds as one word.
                for it in &mut items {
                    it.glue |= it.hyphen;
                }
            }
            let words = self.clause_words(&items, opt, compat);
            if words.is_empty() {
                continue;
            }
            let ends = raw.terminator.ends_sentence();
            cur.clauses.push(Clause {
                words,
                terminator: raw.terminator,
            });
            if ends {
                sentences.push(std::mem::take(&mut cur));
            }
        }
        if !cur.clauses.is_empty() {
            sentences.push(cur);
        }
        sentences
    }

    /// Route each item to a language front end, span by span.
    fn clause_words(&self, items: &[norm::Item], opt: &Options, compat: bool) -> Vec<WordOut> {
        let route = |it: &norm::Item| -> Lang {
            let deva = it
                .text
                .chars()
                .any(|c| text::script_of(c) == text::Script::Devanagari);
            if deva {
                Lang::Hi
            } else if opt.lang == Lang::Hi {
                opt.embedded_english
            } else {
                opt.lang
            }
        };
        let hi = hi::Hindi::new(self.data[Lang::Hi.index()].get());
        // Romanised Hindi: rewrite Latin items as Devanagari where the classifier says so.
        let mut owned: Vec<norm::Item>;
        let mut items = items;
        if opt.hinglish && opt.lang == Lang::Hi {
            owned = items.to_vec();
            let english = self.english(opt.embedded_english);
            hi::roman::rewrite(&mut owned, &hi, english.as_ref());
            items = &owned;
        }
        let mut out: Vec<WordOut> = Vec::with_capacity(items.len());
        let n = items.len();
        let mut i = 0;
        while i < n {
            let lang = route(&items[i]);
            let mut j = i + 1;
            while j < n && route(&items[j]) == lang {
                j += 1;
            }
            let last = j == n;
            let span = &items[i..j];
            match lang {
                Lang::Hi => out.extend(hi.span(span, last, compat)),
                l => {
                    if let Some(e) = self.english(l) {
                        let mut ws = e.span(span, last, compat);
                        if !compat && opt.lang != Lang::Hi {
                            // Indian names in English text (improved mode).
                            for w in ws.iter_mut() {
                                let t = w.text.as_str();
                                let capital = t.chars().next().is_some_and(char::is_uppercase)
                                    && t.chars().skip(1).all(char::is_lowercase);
                                if capital && !t.contains(' ') && !e.knows(&t.to_lowercase()) {
                                    if let Some(ipa) = names::indian_name(e.name_model(), t, &hi) {
                                        w.ipa = ipa;
                                    }
                                }
                            }
                        }
                        if opt.lang == Lang::Hi {
                            hi::adapt_english(&mut ws, compat);
                        }
                        out.extend(ws);
                    }
                }
            }
            i = j;
        }
        out.retain(|w| !w.ipa.is_empty());
        out
    }

    /// Load misaki's dictionaries from their JSON text (`us_gold.json` and
    /// `us_silver.json`, or the `gb_` files for [`Lang::EnGb`]) in place of the
    /// built-in copy, for example a newer misaki release. Takes `&self` so that
    /// a shared phonemizer can be given them once; a second call for the same
    /// accent is an error.
    pub fn load_misaki(
        &self,
        lang: Lang,
        gold_json: &str,
        silver_json: &str,
    ) -> Result<(), String> {
        let slot = match lang {
            Lang::EnUs => 0,
            Lang::EnGb => 1,
            Lang::Hi => return Err("misaki dictionaries exist for en-us and en-gb only".into()),
        };
        let lex = kokoro::MisakiLexicon::from_json(gold_json, silver_json, slot == 1)?;
        self.misaki[slot]
            .set(std::sync::Arc::new(lex))
            .map_err(|_| "misaki dictionaries are already loaded for this accent".to_string())
    }

    /// Load a compiled Kokoro English image (`kokoro-en-us.ucd` or
    /// `kokoro-en-gb.ucd`) at run time, for builds without the
    /// `embed-kokoro` feature (the WebAssembly package fetches it on demand).
    pub fn load_kokoro(&self, bytes: Vec<u8>) -> Result<Lang, String> {
        self.load_kokoro_data(Data::from_vec(bytes).map_err(|e| e.to_string())?)
    }

    fn load_kokoro_data(&self, data: Data) -> Result<Lang, String> {
        let lex = kokoro::MisakiLexicon::from_image(data)?;
        let (slot, lang) = if lex.british() {
            (1, Lang::EnGb)
        } else {
            (0, Lang::EnUs)
        };
        self.kokoro_packs[slot]
            .set(std::sync::Arc::new(lex))
            .map_err(|_| "Kokoro English data is already loaded for this accent".to_string())?;
        Ok(lang)
    }

    /// True when the Kokoro English front end has its dictionaries for
    /// `lang` (loaded, or compiled in).
    pub fn has_misaki(&self, lang: Lang) -> bool {
        self.misaki_lexicon(lang).is_some()
    }

    /// The dictionaries in use: ones loaded explicitly win over the built-in ones.
    fn misaki_lexicon(&self, lang: Lang) -> Option<&kokoro::MisakiLexicon> {
        let slot = match lang {
            Lang::EnUs => 0,
            Lang::EnGb => 1,
            Lang::Hi => return None,
        };
        if let Some(l) = self.misaki[slot].get().or(self.kokoro_packs[slot].get()) {
            return Some(l);
        }
        #[cfg(feature = "embed-kokoro")]
        {
            static BUILT_IN: [std::sync::OnceLock<Option<kokoro::MisakiLexicon>>; 2] =
                [std::sync::OnceLock::new(), std::sync::OnceLock::new()];
            let bytes: &'static [u8] = if slot == 0 {
                include_bytes!("../data/kokoro-en-us.ucd")
            } else {
                include_bytes!("../data/kokoro-en-gb.ucd")
            };
            return BUILT_IN[slot]
                .get_or_init(|| {
                    Data::from_static(bytes)
                        .ok()
                        .and_then(|d| kokoro::MisakiLexicon::from_image(d).ok())
                })
                .as_ref();
        }
        #[allow(unreachable_code)]
        None
    }

    /// Kokoro tokens for English text with misaki's conventions (the front end
    /// Kokoro's English voices were trained on), or `None` when no misaki
    /// dictionaries are available for `lang` (built without `embed-kokoro`
    /// and nothing loaded). Words outside those
    /// dictionaries are read by uccharan's own rules and mapped to misaki's
    /// phone set; Devanagari words by the Hindi rules.
    pub fn kokoro_english(&self, text: &str, lang: Lang) -> Option<String> {
        let lex = self.misaki_lexicon(lang)?;
        // The role misaki gives to espeak-ng: read the token with uccharan's own
        // (reference-compatible) rules and map the result to misaki's phones.
        let mut opt = Options::new(lang);
        opt.normalize = Normalize::EspeakLike;
        let fallback = |word: &str| -> Option<String> {
            let mut parts: Vec<String> = Vec::new();
            for s in self.analyze(word, &opt) {
                for c in s.clauses {
                    for w in c.words {
                        parts.push(if w.lang == Lang::Hi {
                            w.ipa.replace('-', "")
                        } else {
                            kokoro::espeak_style_to_misaki(&w.ipa, lang == Lang::EnGb)
                        });
                    }
                }
            }
            (!parts.is_empty()).then(|| parts.join(" "))
        };
        let out = kokoro::phonemize(lex, text, &fallback);
        Some(
            profile::nfd(&out)
                .chars()
                .filter(|c| profile::KOKORO_VOCAB.iter().any(|v| v.0 == *c))
                .collect(),
        )
    }

    /// One line of output in a profile, as [`profile::render_text`] but with
    /// the Kokoro English front end used for [`profile::Profile::Kokoro`]
    /// when its dictionaries are loaded.
    pub fn render(
        &self,
        text: &str,
        opt: &Options,
        prof: profile::Profile,
        punctuation: bool,
    ) -> String {
        if prof == profile::Profile::Kokoro && opt.lang != Lang::Hi {
            if let Some(k) = self.kokoro_english(text, opt.lang) {
                return k;
            }
        }
        let sentences = self.analyze(text, opt);
        match prof {
            profile::Profile::EspeakIpa => profile::espeak_ipa(
                &sentences,
                punctuation,
                opt.keep_language_flags.then_some(opt.lang),
            ),
            other => profile::render_text(&sentences, other, opt.lang, punctuation),
        }
    }

    /// Phonemize to a single espeak-style IPA string with punctuation kept
    /// (`həlˈoʊ, wˈɜːld.`). Use [`profile`] for the other output shapes.
    pub fn phonemize(&self, text: &str, opt: &Options) -> String {
        profile::espeak_ipa(
            &self.analyze(text, opt),
            true,
            opt.keep_language_flags.then_some(opt.lang),
        )
    }

    /// Phonemize several texts with all available cores on native targets
    /// and sequentially elsewhere.
    pub fn phonemize_batch(&self, texts: &[&str], opt: &Options) -> Vec<String> {
        self.render_batch(texts, opt, profile::Profile::EspeakIpa, true, 0)
    }

    /// Render several texts to one line each (see [`profile::render_text`])
    /// on `threads` worker threads; `0` means one per available core. The
    /// phonemizer is shared by reference: no locks, no per-thread state.
    pub fn render_batch(
        &self,
        texts: &[&str],
        opt: &Options,
        profile: profile::Profile,
        punctuation: bool,
        threads: usize,
    ) -> Vec<String> {
        let one = |t: &&str| -> String { self.render(t, opt, profile, punctuation) };
        #[cfg(not(target_arch = "wasm32"))]
        {
            let wanted = if threads == 0 {
                std::thread::available_parallelism().map_or(1, |n| n.get())
            } else {
                threads
            };
            let workers = wanted.min(texts.len() / 4).max(1);
            if workers > 1 {
                let chunk = texts.len().div_ceil(workers);
                let mut out: Vec<String> = Vec::with_capacity(texts.len());
                std::thread::scope(|s| {
                    let handles: Vec<_> = texts
                        .chunks(chunk)
                        .map(|part| s.spawn(move || part.iter().map(one).collect::<Vec<_>>()))
                        .collect();
                    for h in handles {
                        out.extend(h.join().unwrap_or_default());
                    }
                });
                return out;
            }
        }
        let _ = threads;
        texts.iter().map(one).collect()
    }
}
