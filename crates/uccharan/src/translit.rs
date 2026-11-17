//! Romanised text into Indian scripts.
//!
//! People type `aap kaise ho` or `mujhe kal office jana hai`; voices and
//! speech recognisers do better with `आप कैसे हो` and
//! `मुझे कल office जाना है`. [`transliterate`] writes the romanised words of
//! a language in its script, leaves English words as they are (or respells
//! them, on request) and returns spans aligned to the source.
//!
//! The work is done with a language pack (`xlit-hi.ucd`, …; see
//! `docs/transliteration.md`), loaded with
//! [`Phonemizer::load_pack`](crate::Phonemizer::load_pack). Without a pack a
//! plain rule converter answers (for Hindi, the hand list and rules of
//! [`crate::hi::roman`]).
//!
//! With a pack, each Latin word gets candidates from a lexicon of attested
//! spellings and from an n-gram transliteration model ([`model`]); they are
//! ranked by the model, by how often the native word is used and by how
//! alike the romanised word and the candidate sound
//! ([`crate::soundalike::ipa_similarity`]). One pass over the sentence then
//! chooses, per word, between "English" and its native candidates, using
//! the neighbouring words.
//!
//! Sections of a pack: `NGRM` `SUCC` `TOKS` `UNIT` (model), `LEXR` `LEXE`
//! `LEXO` `LEXW` (spellings), `NFRQ` (native word counts), `BIGR` (word
//! pairs), `ENFQ` (English word counts), `CLMI` `CLME` (letter models),
//! `HOMO` (homographs) and `META` (`lang=xlit-<tag>`, `format`, `license`,
//! and the fitted weights as `w.<name>=<value>`).

pub mod lex;
pub mod model;
pub mod pack;
pub mod rules;

use crate::normalize_api::{normalize, LatinPolicy, NormOptions, Normalized};
use crate::{Lang, Mode, Phonemizer};
use lex::{charlm_score, Lex};
use model::Model;
use pack::{hash_bytes, Sections};
use std::collections::HashMap;
use std::ops::Range;
use std::sync::{Arc, Mutex};

/// The pack format this code reads.
pub const FORMAT: u32 = 1;

macro_rules! weights {
    ($( $(#[$doc:meta])* $name:ident = $default:expr, )*) => {
        /// The fitted numbers of a pack (`w.<name>=<value>` in its metadata).
        #[derive(Clone, Debug)]
        struct Weights { $( $(#[$doc])* $name: f32, )* }

        impl Default for Weights {
            fn default() -> Self {
                Weights { $( $name: $default, )* }
            }
        }

        impl Weights {
            fn set(&mut self, name: &str, v: f32) -> bool {
                match name {
                    $( stringify!($name) => self.$name = v, )*
                    _ => return false,
                }
                true
            }
        }
    };
}

weights! {
    // Candidate ranking.
    model = 1.0,
    attested = 1.0,
    /// Weight of ln(share of the word's spellings) for listed spellings.
    share = 0.0,
    /// Model score of a listed spelling the model did not produce.
    floor = -10.0,
    freq = 0.3,
    in_vocab = 1.0,
    phonetic = 2.0,
    beam = 16.0,
    /// Hypotheses this many nats behind the best are dropped.
    gap = 1.0e9,
    // Language decision (in ln counts per million).
    bias = 1.0,
    en_lex = 0.0,
    en_unk = -4.0,
    hi_unatt = -2.0,
    hi_oov = -2.0,
    clm = 1.0,
    caps = 4.0,
    switch = 1.5,
    en_run = 0.5,
    /// English words whose native candidate sounds like them lean English
    /// by `loan * (similarity - loan_mid)`.
    loan = 6.0,
    loan_mid = 0.6,
    /// A chosen candidate that sounds this much like the English word is
    /// that word in native script: the token is English after all …
    loan_sure = 0.8,
    /// … unless the native word is this much (ln) more frequent.
    loan_gap = 3.0,
    homograph = 1.0,
    // Sentence level.
    lm = 0.5,
    lm_unseen = -0.3,
    tau = 1.0,
    /// Hindi: है / हैं follow the verb form before them (0 = off).
    agree = 1.0,
}

/// What to do with English words.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum EnglishWords {
    /// Leave them in Latin letters.
    #[default]
    Keep,
    /// Respell them in the native script (the respeller of
    /// [`crate::normalize_api`] with `latin = transliterate`).
    Native,
    /// Make no language decision: every Latin word is a romanised word of
    /// the language. For text known to be fully romanised.
    Convert,
}

/// Request options of [`transliterate`].
#[derive(Clone, Debug, PartialEq)]
pub struct TranslitOptions {
    /// What to do with English words.
    pub english: EnglishWords,
    /// How many alternatives to list per word (0: none).
    pub alternatives: usize,
    /// Let neighbouring words decide (`false`: each word alone).
    pub context: bool,
    /// Weights replacing those of the pack, by name (for fitting them;
    /// unknown names are ignored).
    pub weights: Vec<(String, f32)>,
}

impl Default for TranslitOptions {
    fn default() -> Self {
        TranslitOptions {
            english: EnglishWords::Keep,
            alternatives: 3,
            context: true,
            weights: Vec::new(),
        }
    }
}

/// What a span is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordKind {
    /// A romanised word of the language, written in its script.
    Indic,
    /// An English word (kept, or respelled on request).
    English,
    /// Numbers, punctuation, spaces, text already in a native script.
    Other,
}

impl WordKind {
    /// `indic`, `english` or `other`.
    pub fn name(self) -> &'static str {
        match self {
            WordKind::Indic => "indic",
            WordKind::English => "english",
            WordKind::Other => "other",
        }
    }
}

/// One span of the input and what was written for it.
#[derive(Clone, Debug, PartialEq)]
pub struct TranslitSpan {
    /// Byte range in the input.
    pub src: Range<usize>,
    /// Byte range in the output text.
    pub out: Range<usize>,
    /// What the span is.
    pub kind: WordKind,
    /// For Latin words: native candidates, the chosen one first, with a
    /// share of belief (the shares of all candidates sum to 1).
    pub alternatives: Vec<(String, f32)>,
}

/// The result of [`transliterate`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Transliterated {
    /// The converted text.
    pub text: String,
    /// Spans covering the input in order, without gaps.
    pub spans: Vec<TranslitSpan>,
}

/// One candidate of a word with the numbers behind its score.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    /// The native-script word.
    pub text: String,
    /// The ranking score.
    pub score: f32,
    /// ln posterior under the model (the pack's floor when the model did
    /// not list it).
    pub model: f32,
    /// ln(1 + attestations) of this spelling in the lexicon.
    pub attested: f32,
    /// ln(share of the word's listed spellings).
    pub share: f32,
    /// ln corpus count of the native word (0 when unseen).
    pub freq: f32,
    /// Phonetic agreement with the romanised word, 0 to 1.
    pub phonetic: f32,
}

struct WordInfo {
    cands: Vec<Candidate>,
    indic: f32,
    english: f32,
    /// ln(count per million) in the English word list, when listed.
    en_cpm: Option<f32>,
    lower: String,
}

/// A loaded transliteration pack.
pub struct TranslitPack {
    lang: Lang,
    base: u32,
    sections: Sections,
    model: Option<Model>,
    w: Weights,
    words: Mutex<HashMap<String, Arc<WordInfo>>>,
    sounds: Mutex<HashMap<String, String>>,
}

impl std::fmt::Debug for TranslitPack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TranslitPack")
            .field("lang", &self.lang)
            .field("bytes", &self.sections.byte_len())
            .finish()
    }
}

fn script_base(lang: Lang) -> Option<u32> {
    match lang.script() {
        crate::text::Script::Devanagari => Some(0x900),
        crate::text::Script::Brahmic(b) if (0x900..0xe00).contains(&b) => Some(b),
        _ => None,
    }
}

impl TranslitPack {
    /// Load a pack image (`xlit-<lang>.ucd`).
    pub fn load(bytes: Vec<u8>) -> Result<TranslitPack, String> {
        let name = pack::pack_language(&bytes).ok_or("not a transliteration pack")?;
        let lang = Lang::parse(&name).ok_or_else(|| format!("unknown pack language {name:?}"))?;
        let base = script_base(lang).ok_or_else(|| format!("no script for {name:?}"))?;
        let sections = Sections::parse(bytes)?;
        let format: u32 = sections
            .meta("format")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        if format != FORMAT {
            return Err(format!(
                "transliteration pack format {format} is not supported (this version reads format {FORMAT})"
            ));
        }
        let mut w = Weights::default();
        if let Ok(meta) = std::str::from_utf8(sections.get(b"META")) {
            for l in meta.lines() {
                if let Some((k, v)) = l.strip_prefix("w.").and_then(|r| r.split_once('=')) {
                    if let Ok(v) = v.parse() {
                        w.set(k, v);
                    }
                }
            }
        }
        let model = Model::load(&sections)?;
        Ok(TranslitPack {
            lang,
            base,
            sections,
            model,
            w,
            words: Mutex::default(),
            sounds: Mutex::default(),
        })
    }

    /// The language the pack writes.
    pub fn lang(&self) -> Lang {
        self.lang
    }

    /// The licence named in the pack (`license=` in its metadata).
    pub fn license(&self) -> &str {
        self.sections.meta("license").unwrap_or("")
    }

    /// A metadata line of the pack.
    pub fn meta(&self, key: &str) -> Option<&str> {
        self.sections.meta(key)
    }

    /// Section tags and sizes.
    pub fn sizes(&self) -> Vec<(String, usize)> {
        self.sections.sizes()
    }

    /// The best transliterations of a lower-case a–z word by the model alone
    /// (no lexicon, frequency or sound), with joint log-probabilities.
    pub fn model_only(&self, word: &str, nbest: usize) -> Vec<(String, f32)> {
        match &self.model {
            Some(m) => m.decode(
                &self.sections,
                word,
                self.w.beam as usize,
                nbest,
                self.w.gap,
            ),
            None => Vec::new(),
        }
    }
}

impl Phonemizer {
    /// Load a transliteration pack (`xlit-<lang>.ucd`); returns its name.
    /// [`Phonemizer::load_pack`] calls this for such a file. A second pack
    /// for a language is ignored.
    pub fn load_translit(&self, bytes: Vec<u8>) -> Result<String, String> {
        let pack = TranslitPack::load(bytes)?;
        let lang = pack.lang;
        let _ = self.translit[lang.index()].set(Arc::new(pack));
        Ok(format!("xlit-{}", lang.tag()))
    }

    /// True when a transliteration pack for `lang` is loaded.
    pub fn has_translit(&self, lang: Lang) -> bool {
        self.translit[lang.index()].get().is_some()
    }

    /// The loaded transliteration pack of `lang`.
    pub fn translit_pack(&self, lang: Lang) -> Option<&TranslitPack> {
        self.translit[lang.index()].get().map(|p| &**p)
    }
}

/// One request against a pack.
struct Engine<'a> {
    pack: &'a TranslitPack,
    lex: Lex<'a>,
    ph: &'a Phonemizer,
    w: Weights,
    /// With request weights the pack's word cache does not apply.
    local: Option<std::cell::RefCell<HashMap<String, Arc<WordInfo>>>>,
}

fn locked<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl<'a> Engine<'a> {
    fn new(pack: &'a TranslitPack, ph: &'a Phonemizer, weights: &[(String, f32)]) -> Engine<'a> {
        let mut w = pack.w.clone();
        for (k, v) in weights {
            w.set(k, *v);
        }
        Engine {
            pack,
            lex: Lex::load(&pack.sections),
            ph,
            w,
            local: (!weights.is_empty()).then(Default::default),
        }
    }

    fn sound_of(&self, text: &str, lang: Lang) -> String {
        let key = format!("{}\u{1}{text}", lang.tag());
        if let Some(s) = locked(&self.pack.sounds).get(&key) {
            return s.clone();
        }
        let mut o = crate::Options::new(lang);
        o.mode = Mode::Improved;
        let s = self.ph.phonemize(text, &o);
        let mut c = locked(&self.pack.sounds);
        if c.len() > 50_000 {
            c.clear();
        }
        c.insert(key, s.clone());
        s
    }

    /// How alike a romanised word and a native-script candidate sound, 0 to
    /// 1: the romanised word is read by the romanised-Hindi rules, the
    /// candidate by the language's own reader.
    fn phonetic(&self, roman: &str, native: &str) -> f32 {
        let d = crate::hi::roman::transliterate(roman);
        if d.is_empty() {
            return 0.0;
        }
        let a = self.sound_of(&d, Lang::Hi);
        let b = self.sound_of(native, self.pack.lang);
        crate::soundalike::ipa_similarity(&a, &b)
    }

    /// Candidates of one lower-case a–z word, best first.
    fn candidates(&self, word: &str) -> Vec<Candidate> {
        let w = &self.w;
        let blank = |text: String, model: f32| Candidate {
            text,
            score: 0.0,
            model,
            attested: 0.0,
            share: 0.0,
            freq: 0.0,
            phonetic: 0.0,
        };
        let mut cands: Vec<Candidate> = Vec::new();
        for (n, att, share) in self.lex.spellings(word) {
            let mut c = blank(n.to_string(), w.floor);
            c.attested = att;
            c.share = share;
            cands.push(c);
        }
        if let Some(m) = &self.pack.model {
            let list = m.decode(&self.pack.sections, word, w.beam as usize, 8, w.gap);
            if let Some(top) = list.first().map(|x| x.1) {
                let z = top + list.iter().map(|x| (x.1 - top).exp()).sum::<f32>().ln();
                for (n, s) in list {
                    match cands.iter_mut().find(|c| c.text == n) {
                        Some(c) => c.model = s - z,
                        None => cands.push(blank(n, s - z)),
                    }
                }
            }
        }
        if cands.is_empty() {
            let r = rules::itrans(word, self.pack.base);
            if !r.is_empty() {
                cands.push(blank(r, -10.0));
            }
        }
        for c in cands.iter_mut() {
            let f = self.lex.native_ln(&c.text);
            c.freq = f.unwrap_or(0.0);
            if w.phonetic != 0.0 {
                c.phonetic = self.phonetic(word, &c.text);
            }
            c.score = w.model * c.model
                + w.attested * c.attested
                + w.share * c.share
                + w.freq * c.freq
                + if f.is_some() { w.in_vocab } else { 0.0 }
                + w.phonetic * c.phonetic;
        }
        cands.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.text.cmp(&b.text)));
        cands
    }

    fn info(&self, token: &str) -> Arc<WordInfo> {
        let cached = match &self.local {
            Some(l) => l.borrow().get(token).cloned(),
            None => locked(&self.pack.words).get(token).cloned(),
        };
        if let Some(i) = cached {
            return i;
        }
        let info = Arc::new(self.info_uncached(token));
        match &self.local {
            Some(l) => {
                l.borrow_mut().insert(token.to_string(), info.clone());
            }
            None => {
                let mut c = locked(&self.pack.words);
                if c.len() > 100_000 {
                    c.clear();
                }
                c.insert(token.to_string(), info.clone());
            }
        }
        info
    }

    fn info_uncached(&self, token: &str) -> WordInfo {
        let w = &self.w;
        let lower: String = token
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .collect::<String>()
            .to_lowercase();
        // Letters held down ("yaaar", "plzzz"): three or more of a letter count as two.
        let mut word = String::new();
        for c in lower.chars() {
            if word.chars().rev().take_while(|x| *x == c).count() < 2 {
                word.push(c);
            }
        }
        let cands = if word.is_empty() {
            Vec::new()
        } else {
            self.candidates(&word)
        };
        // How English the word is.
        let en_cpm = self.lex.english_cpm(&lower);
        let en_known = en_cpm.or_else(|| {
            let d = self.ph.data(Lang::EnUs)?;
            crate::en::English::new(d, Lang::EnUs)
                .in_lexicon(&lower)
                .then_some(w.en_lex)
        });
        let n = word.len().min(8) as f32;
        let clm = charlm_score(self.lex.clm_en, &word) - charlm_score(self.lex.clm_indic, &word);
        let mut english = match en_known {
            Some(e) => e,
            None => w.en_unk + w.clm * n * clm,
        };
        let homograph = self.lex.homograph(&lower).is_some();
        if let (Some(_), Some(c), false) = (en_known, cands.first(), homograph) {
            if w.loan != 0.0 {
                let e = self.sound_of(&lower, Lang::EnGb);
                let n = self.sound_of(&c.text, self.pack.lang);
                let loan = crate::soundalike::ipa_similarity(&e, &n);
                english += w.loan * (loan - w.loan_mid).max(0.0);
            }
        }
        let caps = token.chars().filter(|c| c.is_ascii_uppercase()).count();
        if (caps >= 2 && caps == lower.len()) || token.contains(['\'', '’']) {
            english += w.caps;
        }
        // How much a word of the language it is.
        let listed = cands
            .iter()
            .filter(|c| c.attested > 0.0)
            .map(|c| self.lex.native_cpm(&c.text).unwrap_or(-1.0) + c.share)
            .fold(f32::NEG_INFINITY, f32::max);
        let indic = if listed > f32::NEG_INFINITY {
            listed
        } else if let Some(c) = cands.first() {
            self.lex
                .native_cpm(&c.text)
                .map_or(w.hi_oov, |f| f + c.model.max(-3.0))
                + w.hi_unatt
                - w.clm * n * clm * 0.5
        } else {
            -30.0
        };
        WordInfo {
            cands,
            indic: indic + w.bias + if homograph { w.homograph } else { 0.0 },
            english,
            en_cpm,
            lower,
        }
    }

    fn convert(&self, text: &str, opt: &TranslitOptions) -> Transliterated {
        let tokens = tokenize(text);
        let mut out = Transliterated::default();
        let mut i = 0;
        while i < tokens.len() {
            // One sentence of tokens at a time.
            let mut j = i;
            while j < tokens.len() {
                let t = &tokens[j];
                j += 1;
                if !t.word && text[t.range.clone()].contains(['.', '?', '!', '।', '\n']) {
                    break;
                }
            }
            self.sentence(text, &tokens[i..j], opt, &mut out);
            i = j;
        }
        out
    }

    #[allow(clippy::needless_range_loop)] // the lattice is indexed by word and node throughout
    fn sentence(
        &self,
        text: &str,
        tokens: &[Token],
        opt: &TranslitOptions,
        out: &mut Transliterated,
    ) {
        let w = &self.w;
        let words: Vec<usize> = (0..tokens.len()).filter(|&i| tokens[i].word).collect();
        let infos: Vec<Arc<WordInfo>> = words
            .iter()
            .map(|&i| self.info(&text[tokens[i].range.clone()]))
            .collect();
        const K: usize = 5;
        // The lattice: per word, node 0 = English, nodes 1.. = candidates.
        let nodes = |i: usize| 1 + infos[i].cands.len().min(K);
        let node_score = |i: usize, k: usize| -> f32 {
            let inf = &infos[i];
            if k == 0 {
                if opt.english == EnglishWords::Convert && !inf.cands.is_empty() {
                    f32::NEG_INFINITY
                } else {
                    inf.english
                }
            } else {
                inf.indic + w.tau * (inf.cands[k - 1].score - inf.cands[0].score)
            }
        };
        // Words separated by something other than spaces do not see each other.
        let adjacent = |i: usize| {
            i > 0
                && words[i] == words[i - 1] + 2
                && text[tokens[words[i] - 1].range.clone()]
                    .chars()
                    .all(|c| c == ' ')
        };
        let mut choice: Vec<usize> = vec![0; words.len()];
        if opt.context && !words.is_empty() {
            let mut best: Vec<Vec<f32>> = Vec::with_capacity(words.len());
            let mut back: Vec<Vec<usize>> = Vec::with_capacity(words.len());
            for i in 0..words.len() {
                let mut b = vec![f32::NEG_INFINITY; nodes(i)];
                let mut bk = vec![0usize; nodes(i)];
                let adj = adjacent(i);
                for k in 0..nodes(i) {
                    let s = node_score(i, k);
                    if i == 0 {
                        b[k] = s;
                        continue;
                    }
                    for p in 0..nodes(i - 1) {
                        let prev = best[i - 1][p];
                        if prev == f32::NEG_INFINITY || s == f32::NEG_INFINITY {
                            continue;
                        }
                        let t = match (p, k) {
                            (0, 0) => w.en_run,
                            (0, _) | (_, 0) => -w.switch,
                            _ if !adj => 0.0,
                            _ => {
                                let (a, c) = (&infos[i - 1].cands[p - 1], &infos[i].cands[k - 1]);
                                match self.lex.pmi(&a.text, &c.text) {
                                    Some(x) => w.lm * x,
                                    None if a.freq > 0.0 && c.freq > 0.0 => w.lm_unseen,
                                    None => 0.0,
                                }
                            }
                        };
                        if prev + t + s > b[k] {
                            b[k] = prev + t + s;
                            bk[k] = p;
                        }
                    }
                }
                best.push(b);
                back.push(bk);
            }
            let last = words.len() - 1;
            let mut k = (0..nodes(last))
                .max_by(|&a, &b| best[last][a].total_cmp(&best[last][b]))
                .unwrap_or(0);
            for i in (0..words.len()).rev() {
                choice[i] = k;
                k = back[i][k];
            }
        } else {
            for (i, c) in choice.iter_mut().enumerate() {
                *c = usize::from(!infos[i].cands.is_empty() && node_score(i, 1) > node_score(i, 0));
            }
        }
        // A chosen candidate that is the English word itself written in the
        // native script (a loan) makes the token English.
        if opt.english != EnglishWords::Convert && w.loan_sure <= 1.0 {
            for i in 0..words.len() {
                let inf = &infos[i];
                let (k, Some(en)) = (choice[i], inf.en_cpm) else {
                    continue;
                };
                if k == 0 {
                    continue;
                }
                let c = &inf.cands[k - 1];
                if self.lex.homograph(&inf.lower) == Some(hash_bytes(c.text.as_bytes()) as u32) {
                    continue;
                }
                if self
                    .lex
                    .native_cpm(&c.text)
                    .is_some_and(|f| f > en + w.loan_gap)
                {
                    continue;
                }
                let e = self.sound_of(&inf.lower, Lang::EnGb);
                let n = self.sound_of(&c.text, self.pack.lang);
                if crate::soundalike::ipa_similarity(&e, &n) >= w.loan_sure {
                    choice[i] = 0;
                }
            }
        }
        // Hindi: है and हैं are both written "hai"; the verb form before decides.
        let mut forced: Vec<Option<&'static str>> = vec![None; words.len()];
        if self.pack.lang == Lang::Hi && w.agree != 0.0 {
            let chosen =
                |i: usize| (choice[i] > 0).then(|| infos[i].cands[choice[i] - 1].text.as_str());
            for i in 1..words.len() {
                let (Some(cur), Some(prev)) = (chosen(i), chosen(i - 1)) else {
                    continue;
                };
                if !matches!(cur, "है" | "हैं") || !adjacent(i) {
                    continue;
                }
                match hai_number(prev) {
                    Some(true) if cur == "है" => forced[i] = Some("हैं"),
                    Some(false) if cur == "हैं" => forced[i] = Some("है"),
                    _ => {}
                }
            }
        }
        let mut wi = 0;
        for t in tokens {
            let src = &text[t.range.clone()];
            let start = out.text.len();
            if !t.word {
                out.text.push_str(src);
                // Adjacent "other" spans merge.
                if let Some(last) = out.spans.last_mut() {
                    if last.kind == WordKind::Other && last.src.end == t.range.start {
                        last.src.end = t.range.end;
                        last.out.end = out.text.len();
                        continue;
                    }
                }
                out.spans.push(TranslitSpan {
                    src: t.range.clone(),
                    out: start..out.text.len(),
                    kind: WordKind::Other,
                    alternatives: Vec::new(),
                });
                continue;
            }
            let inf = &infos[wi];
            let k = choice[wi];
            let force = forced[wi];
            wi += 1;
            let mut alts: Vec<(String, f32)> = Vec::new();
            if !inf.cands.is_empty() && opt.alternatives > 0 {
                let top = inf.cands[0].score;
                let z: f32 = inf.cands.iter().map(|c| (c.score - top).exp()).sum();
                // The chosen candidate first, then the others by score.
                let mut order: Vec<usize> = (0..inf.cands.len()).collect();
                let first = match force {
                    Some(f) => inf.cands.iter().position(|c| c.text == f),
                    None => k.checked_sub(1),
                };
                if let Some(f) = first {
                    order.remove(f);
                    order.insert(0, f);
                }
                for &c in order.iter().take(opt.alternatives) {
                    alts.push((
                        inf.cands[c].text.clone(),
                        (inf.cands[c].score - top).exp() / z,
                    ));
                }
            }
            let kind = if k == 0 {
                match opt.english {
                    EnglishWords::Native => {
                        out.text
                            .push_str(&english_native(self.ph, src, self.pack.lang))
                    }
                    _ => out.text.push_str(src),
                }
                WordKind::English
            } else {
                out.text.push_str(force.unwrap_or(&inf.cands[k - 1].text));
                WordKind::Indic
            };
            out.spans.push(TranslitSpan {
                src: t.range.clone(),
                out: start..out.text.len(),
                kind,
                alternatives: alts,
            });
        }
    }
}

/// For the Hindi word before है / हैं: `Some(true)` when it is a plural (or
/// honorific) verb form or noun that takes हैं, `Some(false)` when it is a
/// masculine singular verb form that takes है, `None` when it does not say.
fn hai_number(prev: &str) -> Option<bool> {
    const PLURAL: &[&str] = &[
        "रहे",
        "गए",
        "गये",
        "आए",
        "आये",
        "हुए",
        "किए",
        "किये",
        "दिए",
        "दिये",
        "चुके",
        "सकते",
        "होते",
        "वाले",
    ];
    const SINGULAR: &[&str] = &[
        "रहा",
        "गया",
        "आया",
        "हुआ",
        "किया",
        "दिया",
        "चुका",
        "सकता",
        "होता",
    ];
    if PLURAL.contains(&prev) {
        return Some(true);
    }
    if SINGULAR.contains(&prev) {
        return Some(false);
    }
    // Imperfective participles: करते हैं, करता है. Short words in -ता / -ते
    // are mostly nouns and postpositions (पता, बीते).
    if prev.chars().count() >= 3 {
        if prev.ends_with("ते") {
            return Some(true);
        }
        // Plural perfectives: बनाए हैं, दिखाये हैं (not the postposition लिए).
        if (prev.ends_with('ए') || prev.ends_with("ये"))
            && !matches!(prev, "लिए" | "लिये" | "इसलिए" | "इसलिये" | "किसलिए")
        {
            return Some(true);
        }
        if prev.ends_with("ता")
            && !matches!(
                prev,
                "देवता"
                    | "नेता"
                    | "जनता"
                    | "कविता"
                    | "माता"
                    | "पिता"
                    | "पता"
                    | "रास्ता"
            )
        {
            return Some(false);
        }
    }
    None
}

/// An English word respelled in the script of `lang`.
fn english_native(ph: &Phonemizer, word: &str, lang: Lang) -> String {
    let o = NormOptions {
        latin: LatinPolicy::Transliterate,
        ..Default::default()
    };
    let n = normalize(ph, word, lang, &o);
    if n.text.is_empty() {
        word.to_string()
    } else {
        n.text
    }
}

struct Token {
    range: Range<usize>,
    /// A Latin-letter word (apostrophes inside allowed).
    word: bool,
}

/// Split into Latin words and everything else. A run of letters glued to
/// digits or to other letters ("b4", "2nd") is not a word.
fn tokenize(text: &str) -> Vec<Token> {
    fn push(out: &mut Vec<Token>, r: Range<usize>, word: bool) {
        if !word {
            if let Some(l) = out.last_mut() {
                if !l.word {
                    l.range.end = r.end;
                    return;
                }
            }
        }
        out.push(Token { range: r, word });
    }
    let mut out: Vec<Token> = Vec::new();
    let cs: Vec<(usize, char)> = text.char_indices().collect();
    let mut i = 0;
    while i < cs.len() {
        let (at, c) = cs[i];
        if c.is_ascii_alphabetic() {
            let mut j = i + 1;
            while j < cs.len()
                && (cs[j].1.is_ascii_alphabetic()
                    || (matches!(cs[j].1, '\'' | '’')
                        && j + 1 < cs.len()
                        && cs[j + 1].1.is_ascii_alphabetic()))
            {
                j += 1;
            }
            let end = if j < cs.len() { cs[j].0 } else { text.len() };
            let glued = (i > 0 && cs[i - 1].1.is_alphanumeric())
                || (j < cs.len() && cs[j].1.is_alphanumeric());
            push(&mut out, at..end, !glued);
            i = j;
        } else {
            let end = if i + 1 < cs.len() {
                cs[i + 1].0
            } else {
                text.len()
            };
            push(&mut out, at..end, false);
            i += 1;
        }
    }
    out
}

/// Without a pack: the hand list and rules of [`crate::hi::roman`] for the
/// Hindi-like languages, a plain rule converter for the others. English is
/// whatever the English lexicon lists.
fn fallback(ph: &Phonemizer, text: &str, lang: Lang, opt: &TranslitOptions) -> Transliterated {
    let tokens = tokenize(text);
    let base = script_base(lang);
    let en = ph
        .data(Lang::EnGb)
        .map(|d| crate::en::English::new(d, Lang::EnGb))
        .or_else(|| {
            ph.data(Lang::EnUs)
                .map(|d| crate::en::English::new(d, Lang::EnUs))
        });
    let convert = opt.english == EnglishWords::Convert;
    // Per word: the native text, or None for a word that stays.
    let mut written: Vec<Option<String>> = Vec::new();
    if lang.romanised_as_hindi() && !convert {
        let hi = crate::hi::Hindi::new(ph.data(Lang::Hi));
        let mut i = 0;
        while i < tokens.len() {
            let mut j = i;
            while j < tokens.len() {
                let t = &tokens[j];
                j += 1;
                if !t.word && text[t.range.clone()].contains(['.', '?', '!', '।', '\n']) {
                    break;
                }
            }
            let mut items: Vec<crate::norm::Item> = tokens[i..j]
                .iter()
                .filter(|t| t.word)
                .map(|t| crate::norm::Item {
                    text: text[t.range.clone()].to_string(),
                    spell: false,
                    glue: false,
                    generated: false,
                    hyphen: false,
                    number_tail: false,
                })
                .collect();
            let before: Vec<String> = items.iter().map(|x| x.text.clone()).collect();
            if lang == Lang::Doi {
                crate::doi::rewrite_roman(&mut items);
            }
            crate::hi::roman::rewrite(&mut items, &hi, en.as_ref());
            for (a, b) in before.iter().zip(items) {
                written.push((*a != b.text).then_some(b.text));
            }
            i = j;
        }
    } else {
        for t in tokens.iter().filter(|t| t.word) {
            let src = &text[t.range.clone()];
            let lower = src.to_lowercase();
            let english = !convert && en.as_ref().is_some_and(|e| e.in_lexicon(&lower));
            let native = match base {
                Some(b) if !english => {
                    let mut s = String::new();
                    if lang.romanised_as_hindi()
                        && ph
                            .data(Lang::Hi)
                            .is_some_and(|d| d.lookup(&format!("~{lower}"), &mut s))
                    {
                        s
                    } else if lang.romanised_as_hindi() {
                        crate::hi::roman::transliterate(&lower)
                    } else {
                        rules::itrans(&lower, b)
                    }
                }
                _ => String::new(),
            };
            written.push((!native.is_empty()).then_some(native));
        }
    }
    let mut out = Transliterated::default();
    let mut wi = 0;
    for t in &tokens {
        let src = &text[t.range.clone()];
        let start = out.text.len();
        let mut alternatives = Vec::new();
        let kind = if !t.word {
            out.text.push_str(src);
            WordKind::Other
        } else {
            let native = written[wi].take();
            wi += 1;
            match native {
                Some(n) => {
                    if opt.alternatives > 0 {
                        alternatives.push((n.clone(), 1.0));
                    }
                    out.text.push_str(&n);
                    WordKind::Indic
                }
                None if base.is_none() => {
                    out.text.push_str(src);
                    WordKind::Other
                }
                None => {
                    match opt.english {
                        EnglishWords::Native => out.text.push_str(&english_native(ph, src, lang)),
                        _ => out.text.push_str(src),
                    }
                    WordKind::English
                }
            }
        };
        if kind == WordKind::Other {
            if let Some(last) = out.spans.last_mut() {
                if last.kind == WordKind::Other && last.src.end == t.range.start {
                    last.src.end = t.range.end;
                    last.out.end = out.text.len();
                    continue;
                }
            }
        }
        out.spans.push(TranslitSpan {
            src: t.range.clone(),
            out: start..out.text.len(),
            kind,
            alternatives,
        });
    }
    out
}

/// The `hinglish` option with a pack: rewrite the Latin items of a clause
/// that are romanised words of `lang` in its script. Returns `false` when
/// `lang` has no pack loaded (nothing is changed).
pub(crate) fn rewrite_items(ph: &Phonemizer, lang: Lang, items: &mut [crate::norm::Item]) -> bool {
    let Some(pack) = ph.translit_pack(lang) else {
        return false;
    };
    // The clause as text again, remembering where each item went.
    let mut text = String::new();
    let mut at: Vec<Range<usize>> = Vec::with_capacity(items.len());
    for it in items.iter() {
        if !text.is_empty() {
            text.push(' ');
        }
        let start = text.len();
        text.push_str(&it.text);
        at.push(start..text.len());
    }
    let opt = TranslitOptions {
        alternatives: 0,
        ..Default::default()
    };
    let out = Engine::new(pack, ph, &[]).convert(&text, &opt);
    for s in out.spans.iter().filter(|s| s.kind == WordKind::Indic) {
        if let Ok(i) = at.binary_search_by(|r| r.start.cmp(&s.src.start)) {
            if at[i] == s.src && !items[i].spell && !items[i].generated {
                items[i].text = out.text[s.out.clone()].to_string();
            }
        }
    }
    true
}

/// Write the romanised words of `text` in the script of `lang`.
///
/// Uses the transliteration pack of `lang` when one is loaded
/// ([`Phonemizer::has_translit`]); otherwise a rule converter answers (see
/// the module notes). Text of a language without an Indian script comes
/// back unchanged.
pub fn transliterate(
    ph: &Phonemizer,
    text: &str,
    lang: Lang,
    opt: &TranslitOptions,
) -> Transliterated {
    match ph.translit_pack(lang) {
        Some(pack) => Engine::new(pack, ph, &opt.weights).convert(text, opt),
        None => fallback(ph, text, lang, opt),
    }
}

/// The candidates of one romanised word (lower-case a–z) with the numbers
/// behind their ranking, best first. Empty without a pack.
pub fn candidates(
    ph: &Phonemizer,
    word: &str,
    lang: Lang,
    opt: &TranslitOptions,
) -> Vec<Candidate> {
    match ph.translit_pack(lang) {
        Some(pack) => Engine::new(pack, ph, &opt.weights).candidates(word),
        None => Vec::new(),
    }
}

/// [`transliterate`], then [`normalize`]: the spoken form of romanised text
/// for a voice that reads the native script. The spans of the result point
/// into the original `text`; a span that came from a converted word covers
/// the whole romanised word.
pub fn normalize_romanised(
    ph: &Phonemizer,
    text: &str,
    lang: Lang,
    translit: &TranslitOptions,
    norm: &NormOptions,
) -> Normalized {
    let t = transliterate(ph, text, lang, translit);
    let mut n = normalize(ph, &t.text, lang, norm);
    // A position of the converted text, back in the source.
    let back = |p: usize, end: bool| -> usize {
        let i = t
            .spans
            .partition_point(|s| if end { s.out.end < p } else { s.out.end <= p })
            .min(t.spans.len().saturating_sub(1));
        let Some(s) = t.spans.get(i) else { return p };
        if s.out.len() == s.src.len() && t.text[s.out.clone()] == text[s.src.clone()] {
            s.src.start + (p.clamp(s.out.start, s.out.end) - s.out.start)
        } else if end {
            s.src.end
        } else {
            s.src.start
        }
    };
    for s in n.spans.iter_mut() {
        let (a, b) = (back(s.src.start, false), back(s.src.end, true));
        s.src = a..b.max(a);
    }
    n
}

/// Options from JSON: `{"english": "keep" | "native" | "convert",
/// "alternatives": 3, "context": true, "weights": {"beam": 8}}`. An empty
/// string gives the defaults.
pub fn options_from_json(json: &str) -> Result<TranslitOptions, String> {
    use crate::minijson::Value;
    let mut o = TranslitOptions::default();
    if json.trim().is_empty() {
        return Ok(o);
    }
    let v = crate::minijson::parse(json)?;
    let Value::Obj(map) = &v else {
        return Err("options must be a JSON object".into());
    };
    for (k, x) in map {
        match (k.as_str(), x) {
            ("english", Value::Str(s)) => {
                o.english = match s.as_str() {
                    "keep" | "" => EnglishWords::Keep,
                    "native" | "respell" => EnglishWords::Native,
                    "convert" => EnglishWords::Convert,
                    other => return Err(format!("unknown english policy {other:?}")),
                }
            }
            ("alternatives", Value::Num(n)) if *n >= 0.0 => o.alternatives = *n as usize,
            ("context", Value::Bool(b)) => o.context = *b,
            ("weights", Value::Obj(ws)) => {
                for (name, val) in ws {
                    match val {
                        Value::Num(n) => o.weights.push((name.clone(), *n as f32)),
                        _ => return Err(format!("weights.{name}: expected a number")),
                    }
                }
            }
            (other, _) => return Err(format!("unknown or malformed option {other:?}")),
        }
    }
    Ok(o)
}

/// [`transliterate`] with JSON options in and a JSON result out (see
/// [`options_from_json`] and [`Transliterated::to_json`]).
pub fn transliterate_json(
    ph: &Phonemizer,
    text: &str,
    lang: Lang,
    options_json: &str,
) -> Result<String, String> {
    let o = options_from_json(options_json)?;
    Ok(transliterate(ph, text, lang, &o).to_json(text))
}

/// Character and UTF-16 offsets of every char boundary of `s`, by byte.
fn offset_tables(s: &str) -> (Vec<usize>, Vec<usize>) {
    let mut cp = vec![0usize; s.len() + 1];
    let mut u16s = vec![0usize; s.len() + 1];
    let (mut c, mut u) = (0, 0);
    for (i, ch) in s.char_indices() {
        cp[i] = c;
        u16s[i] = u;
        c += 1;
        u += ch.len_utf16();
    }
    cp[s.len()] = c;
    u16s[s.len()] = u;
    (cp, u16s)
}

impl Transliterated {
    /// The result as JSON: `{"text", "spans": [{"kind", "src": {...},
    /// "out": {...}, "text", "written", "alternatives"?: [{"text",
    /// "score"}]}]}` where each range gives `byte`, `char` (code point) and
    /// `utf16` start and end offsets, as [`Normalized::to_json`] does.
    pub fn to_json(&self, input: &str) -> String {
        use crate::minijson::push_str;
        let (icp, iu) = offset_tables(input);
        let (ocp, ou) = offset_tables(&self.text);
        let range = |o: &mut String, r: &Range<usize>, cp: &[usize], u: &[usize]| {
            o.push_str(&format!(
                "{{\"byte\":[{},{}],\"char\":[{},{}],\"utf16\":[{},{}]}}",
                r.start, r.end, cp[r.start], cp[r.end], u[r.start], u[r.end]
            ));
        };
        let mut o = String::from("{\"text\":");
        push_str(&mut o, &self.text);
        o.push_str(",\"spans\":[");
        for (i, s) in self.spans.iter().enumerate() {
            if i > 0 {
                o.push(',');
            }
            o.push_str("{\"kind\":");
            push_str(&mut o, s.kind.name());
            o.push_str(",\"src\":");
            range(&mut o, &s.src, &icp, &iu);
            o.push_str(",\"out\":");
            range(&mut o, &s.out, &ocp, &ou);
            o.push_str(",\"text\":");
            push_str(&mut o, &input[s.src.clone()]);
            o.push_str(",\"written\":");
            push_str(&mut o, &self.text[s.out.clone()]);
            if !s.alternatives.is_empty() {
                o.push_str(",\"alternatives\":[");
                for (k, (t, sc)) in s.alternatives.iter().enumerate() {
                    if k > 0 {
                        o.push(',');
                    }
                    o.push_str("{\"text\":");
                    push_str(&mut o, t);
                    o.push_str(&format!(",\"score\":{sc:.3}}}"));
                }
                o.push(']');
            }
            o.push('}');
        }
        o.push_str("]}");
        o
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokeniser_leaves_mixed_tokens_alone() {
        let text = "b4 kal don't 2nd";
        let words: Vec<&str> = tokenize(text)
            .iter()
            .filter(|x| x.word)
            .map(|x| &text[x.range.clone()])
            .collect();
        assert_eq!(words, vec!["kal", "don't"]);
    }

    #[test]
    fn hai_follows_the_verb_form() {
        assert_eq!(hai_number("करते"), Some(true));
        assert_eq!(hai_number("रहे"), Some(true));
        assert_eq!(hai_number("करता"), Some(false));
        assert_eq!(hai_number("गया"), Some(false));
        assert_eq!(hai_number("रही"), None);
        assert_eq!(hai_number("पता"), None);
        assert_eq!(hai_number("नेता"), None);
    }

    #[test]
    fn options_json() {
        let o = options_from_json(
            r#"{"english":"native","alternatives":1,"context":false,"weights":{"beam":8}}"#,
        )
        .unwrap();
        assert_eq!(o.english, EnglishWords::Native);
        assert_eq!(o.alternatives, 1);
        assert!(!o.context);
        assert_eq!(o.weights, vec![("beam".to_string(), 8.0)]);
        assert_eq!(options_from_json("").unwrap(), TranslitOptions::default());
        assert!(options_from_json(r#"{"english":"maybe"}"#).is_err());
        assert!(options_from_json(r#"{"nope":1}"#).is_err());
    }
}
