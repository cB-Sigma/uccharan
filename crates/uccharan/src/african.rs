//! African names in English text (improved mode).
//!
//! Most African languages written in the Latin alphabet spell almost
//! phonemically: five plain vowels read as written, prenasalised stops
//! (`mb`, `nd`, `ng`), labial-velars (`gb`, `kp`), affricate digraphs (`ch`,
//! `ts`, `dz`), and names that end in full vowels. English readers of such
//! names get them wrong in predictable ways (silent final e, reduced vowels,
//! `ch` as k, stress on the first syllable). This module recognises names
//! written this way and reads them with those conventions, mapped onto
//! English phonemes, with stress on the penultimate syllable (the Bantu
//! default, and the usual English rendering of names from tone languages).
//!
//! French-based spellings (Wolof, Fula, Bambara, Moore, North African names:
//! `ou`, `ch` = ʃ, `dj`, `di` + vowel) and Somali spellings (`x` = h, `c` =
//! a pharyngeal English speakers drop, doubled vowels) are recognised by
//! their own cues. Nguni clicks (`c`, `q`, `x`) are read as k, as most
//! English speakers say them.

/// Spelling conventions a name is read with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Style {
    Plain,
    French,
    Somali,
    Nguni,
    Malagasy,
    /// Arabic names in French or English transliteration (Maghreb, Egypt).
    Arabic,
}

const VOWELS: &[u8] = b"aeiou";

fn is_v(b: u8) -> bool {
    VOWELS.contains(&b)
}

/// Onsets typical of African names and rare at the start of English or
/// European ones.
const ONSETS: &[&str] = &[
    "mb", "mp", "mv", "mw", "ms", "mz", "mt", "nd", "ng", "nk", "nj", "nt", "ny", "nw", "ns", "nz",
    "nx", "nq", "nc", "nn", "ss", "gb", "kp", "dz", "tsh", "tsw", "tsv", "dl", "hl", "gq", "xh",
    "ngc", "nqh", "mth", "mph", "nts", "nkh", "ndz", "ts", "hw", "kw", "mn",
];

/// Ethiopian and Eritrean (Amharic, Tigrinya) name elements: gebre "servant
/// of", wolde "son of", habte "gift of", tesfa "hope", berhan "light", haile
/// "power", tekle "plant", kidane "covenant", mehret "mercy", tsega "grace",
/// alem "world", fikre "love" (Leslau 1995).
const ETHIOPIC: &[&str] = &[
    "gebre", "ghebre", "wolde", "welde", "habt", "tesf", "berh", "haile", "hailu", "tekl", "kidan",
    "mehr", "mihr", "tseg", "fikr", "alem",
];

fn ethiopic(w: &str) -> bool {
    ETHIOPIC.iter().any(|p| w.starts_with(p)) && w.len() >= 5
}

/// Spelling cues of Arabic names in French or English transliteration.
fn arabic_cues(w: &str) -> bool {
    let b = w.as_bytes();
    let starts = |p: &str| w.starts_with(p) && b.get(p.len()).is_some_and(|&c| !is_v(c));
    (w.starts_with("abd") && w.len() > 4)
        || ((starts("ben") || starts("bou") || starts("bel")) && w.len() >= 6)
        || w.contains("dj")
        || w.starts_with("kh")
        || (w.contains("gh") && !w.contains("ght") && !w.contains("ugh"))
        || w.contains("aou")
        || w.contains("oua")
        || w.ends_with("oui")
        || w.ends_with("ine") && w.len() >= 5
        || w.ends_with("ane") && w.len() >= 5
        || w.contains("ch") && w.contains("ou")
        // Berber feminine and place names: ta-…-t, ti-…-t.
        || ((w.starts_with("ta") || w.starts_with("ti")) && w.ends_with('t') && w.len() >= 6)
        // Egyptian English-based spellings: final -y after two different
        // consonants (Magdy, Hamdy, Sabry, Shawky).
        || (b.len() >= 5
            && b[b.len() - 1] == b'y'
            && !is_v(b[b.len() - 2])
            && !is_v(b[b.len() - 3])
            && b[b.len() - 3] != b'y'
            && b[b.len() - 2] != b[b.len() - 3]
            && !matches!(&w[w.len() - 3..], "thy" | "ley" | "rly" | "sty" | "chy" | "phy" | "shy" | "rty"))
}

fn style(w: &str) -> Style {
    let b = w.as_bytes();
    let nguni = (b.first().is_some_and(|c| matches!(c, b'q' | b'x'))
        && !w.starts_with("xi")
        && b.get(1).is_some_and(|&c| is_v(c) || c == b'h'))
        || ["ngc", "nqh", "nx", "gq", "xh", "hl", "dl"]
            .iter()
            .any(|x| w.contains(x));
    if nguni {
        return Style::Nguni;
    }
    let somali = ["aa", "uu", "ii", "ee"].iter().any(|x| w.contains(x))
        || (b.first() == Some(&b'x')
            && b.get(1).is_some_and(|&c| is_v(c))
            && b.get(1) != Some(&b'i'));
    if somali {
        return Style::Somali;
    }
    // Malagasy: "oa", ra-/andri- names, -arivo, -tra/-ina/-ana endings.
    let malagasy = (w.contains("oa") && !w.contains("oua"))
        || w.starts_with("andri")
        || (w.starts_with("ra")
            && w.len() >= 7
            && (w.contains("tr") || w.contains("ts") || w.ends_with('o') || w.ends_with('y')))
        || w.ends_with("arivo")
        || w.ends_with("tra");
    if malagasy {
        return Style::Malagasy;
    }
    if arabic_cues(w) {
        return Style::Arabic;
    }
    if w.contains("ou") || w.contains('é') || w.contains('è') || w.starts_with("dj") {
        return Style::French;
    }
    Style::Plain
}

/// True when the spelling has cues that are (nearly) unique to African
/// languages: prenasalised or labial-velar onsets, clicks, `mw`, Somali
/// doubled vowels.
pub fn strongly_african(word: &str) -> bool {
    let w = word.to_lowercase();
    let b = w.as_bytes();
    if b.len() < 3 || !w.chars().all(|c| c.is_ascii_lowercase() || c == '\'') {
        return false;
    }
    ethiopic(&w)
        || ONSETS.iter().any(|o| {
            w.starts_with(o)
                && b.get(o.len())
                    .is_some_and(|&c| is_v(c) || c == b'w' || c == b'y')
        })
        || ["gb", "kp", "mw", "nyw", "aa", "uu"]
            .iter()
            .any(|x| w.contains(x))
        || matches!(
            style(&w),
            Style::Nguni | Style::Somali | Style::Malagasy | Style::Arabic
        )
}

/// True for an African-looking name without the letter groups typical of
/// romanised Indian names (`v`, `sh`, aspirates written with `h`, `ee`,
/// `oo`, common Indian endings), so that it is read as African even when
/// the Indian-name classifier would claim it.
pub fn prefer_over_indian(word: &str) -> bool {
    let w = word.to_lowercase();
    if !looks_african(&w) {
        return false;
    }
    let indian = [
        "v", "sh", "bh", "dh", "kh", "gh", "th", "ph", "jh", "ee", "oo", "aa",
    ]
    .iter()
    .any(|x| w.contains(x))
        || [
            "an", "esh", "ar", "raj", "ita", "pur", "ika", "ini", "deep", "preet", "jit", "ender",
            "endra", "ant", "ya", "ra",
        ]
        .iter()
        .any(|x| w.ends_with(x));
    !indian
}

/// True when a capitalised word unknown to the English lexicon is spelled
/// like a name from an African language.
pub fn looks_african(word: &str) -> bool {
    let w = word.to_lowercase();
    let b = w.as_bytes();
    if b.len() < 3
        || !w
            .chars()
            .all(|c| c.is_ascii_lowercase() || c == '\'' || c == 'é' || c == 'è')
    {
        return false;
    }
    if ONSETS.iter().any(|o| {
        w.starts_with(o)
            && b.get(o.len())
                .is_some_and(|&c| is_v(c) || c == b'w' || c == b'y')
    }) {
        return true;
    }
    if ["gb", "kp", "mw", "nyw", "dh", "aa", "uu"]
        .iter()
        .any(|x| w.contains(x))
    {
        return true;
    }
    if style(&w) != Style::Plain && style(&w) != Style::French {
        return true;
    }
    // Phonotactics: (consonant)(glide) vowel syllables, prenasalised stops,
    // ending in a vowel; no English-only spellings.
    let vowel_letters = b.iter().filter(|&&c| is_v(c)).count();
    let last = *b.last().unwrap();
    if b.len() < 4 || !(is_v(last) || (vowel_letters >= 3 && b"nmlrsbdkt".contains(&last))) {
        return false;
    }
    if [
        "th", "ck", "ph", "qu", "wh", "gh", "ee", "oo", "ea", "x", "q", "v",
    ]
    .iter()
    .any(|x| {
        w.contains(x) && !(*x == "x" && w.starts_with("xi")) && !(*x == "ph" && w.contains("mph"))
    }) || w
        .match_indices('c')
        .any(|(i, _)| b.get(i + 1) != Some(&b'h'))
    {
        return false;
    }
    let mut i = 0;
    while i < b.len() {
        if is_v(b[i]) {
            i += 1;
            continue;
        }
        let mut j = i;
        while j < b.len() && !is_v(b[j]) {
            j += 1;
        }
        let cl = &w[i..j];
        let geminate = cl.len() == 2 && cl.as_bytes()[0] == cl.as_bytes()[1];
        let final_c = j == b.len() && cl.len() == 1;
        let ok = cl.len() == 1
            || geminate
            || final_c
            || matches!(
                cl,
                "ny" | "ng"
                    | "nd"
                    | "mb"
                    | "nj"
                    | "nk"
                    | "nt"
                    | "mp"
                    | "ch"
                    | "sh"
                    | "ts"
                    | "dz"
                    | "kw"
                    | "gw"
                    | "mw"
                    | "bw"
                    | "tw"
                    | "dw"
                    | "sw"
                    | "ky"
                    | "gy"
                    | "ndw"
                    | "ndz"
                    | "nts"
                    | "tsw"
                    | "dzw"
                    | "ntsw"
                    | "hw"
                    | "rh"
                    | "ngw"
                    | "mph"
                    | "nkh"
                    | "nz"
                    | "ns"
                    | "ly"
                    | "my"
                    | "ry"
                    | "by"
                    | "dy"
            );
        if !ok {
            return false;
        }
        i = j;
    }
    // Several syllables of that shape.
    b.iter().filter(|&&c| is_v(c)).count() >= 3
}

/// Arabic given names in English transliteration (Wikidata, CC0; see
/// `data/sources/names/README.md`), lower case, sorted. Names English already
/// has its own reading for are left out.
const ARABIC_NAMES: &str = include_str!("en/arabic_names.txt");

/// Words of the names of Eritrean and Ethiopian people on Wikidata (CC0;
/// see `tools/arabic_names.py`), lower case, sorted; words CMUdict has, the
/// Arabic list's names and common English words are left out.
const HORN_NAMES: &str = include_str!("en/horn_names.txt");

/// True when the word (any case) is a listed Arabic given name.
pub fn arabic_name(word: &str) -> bool {
    in_list(ARABIC_NAMES, word)
}

/// True when the word (any case) is a listed Eritrean or Ethiopian name.
pub fn horn_name(word: &str) -> bool {
    in_list(HORN_NAMES, word)
}

fn in_list(list: &str, word: &str) -> bool {
    let w = word.to_lowercase();
    let list = list.as_bytes();
    // Binary search over the newline-separated sorted list.
    let (mut lo, mut hi) = (0usize, list.len());
    while lo < hi {
        let mid = (lo + hi) / 2;
        let start = list[..mid]
            .iter()
            .rposition(|&b| b == b'\n')
            .map_or(0, |p| p + 1);
        let end = list[start..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(list.len(), |p| start + p);
        match list[start..end].cmp(w.as_bytes()) {
            std::cmp::Ordering::Equal => return true,
            std::cmp::Ordering::Less => lo = end + 1,
            std::cmp::Ordering::Greater => hi = start,
        }
    }
    false
}

/// True when an English dictionary reading of an Arabic name already stresses
/// the same syllable with a vowel that renders the Arabic one (Nasser,
/// Hussein): then that reading is kept.
pub fn english_reading_fits(english: &str, arabic: &str) -> bool {
    fn shape(s: &str) -> Option<(usize, char)> {
        let v = |c: char| "aeiouæɑɐɒɔəɚɛɜɪʊʌᵻ".contains(c);
        let (mut runs, mut prev, mut stressed) = (0usize, false, false);
        for c in s.chars() {
            if c == 'ˈ' {
                stressed = true;
                continue;
            }
            if c == 'ː' || c == 'ˌ' {
                continue;
            }
            if v(c) {
                if stressed {
                    return Some((runs, c));
                }
                if !prev {
                    runs += 1;
                }
                prev = true;
            } else {
                prev = false;
            }
        }
        None
    }
    let (Some((ei, ev)), Some((ai, av))) = (shape(english), shape(arabic)) else {
        return false;
    };
    let ok: &[char] = match av {
        'ɑ' | 'æ' | 'a' => &['ɑ', 'æ', 'a'],
        'i' | 'ɪ' => &['i', 'ɪ'],
        'u' | 'ʊ' => &['u', 'ʊ'],
        'ɛ' | 'e' => &['ɛ', 'e'],
        'o' | 'ɔ' => &['o', 'ɔ'],
        _ => &[],
    };
    // Same consonants (an r after a vowel may be silent or written ɚ).
    let cons = |s: &str| -> String {
        s.chars()
            .filter(|&c| !"aeiouæɑɐɒɔəɚɛɜɪʊʌᵻːˈˌɹ".contains(c))
            .collect()
    };
    ei == ai && ok.contains(&ev) && cons(english) == cons(arabic)
}

/// Reading of a listed Arabic name (see [`arabic_name`]).
pub fn read_arabic(word: &str, british: bool) -> String {
    let w = word.to_lowercase().replace('\'', "");
    read_style(&w, british, Style::Arabic)
}

/// English-phoneme reading of an African name (espeak-style IPA with stress).
pub fn read(word: &str, british: bool) -> String {
    let w = word.to_lowercase().replace('\'', "");
    let st = style(&w);
    read_style(&w, british, st)
}

fn read_style(w: &str, british: bool, st: Style) -> String {
    let cs: Vec<char> = w.chars().collect();
    let n = cs.len();
    // (phones, is vowel)
    let mut units: Vec<(String, bool)> = Vec::new();
    let at = |i: usize| cs.get(i).copied().unwrap_or(' ');
    let vowel_at = |i: usize| matches!(at(i), 'a' | 'e' | 'i' | 'o' | 'u' | 'é' | 'è');
    let mut i = 0;
    let mut geminate_next = false;
    // Consonant units that close the syllable before them by themselves (a
    // written double consonant); used for Arabic syllable weight.
    let mut gem: Vec<usize> = Vec::new();
    while i < n {
        let c = cs[i];
        if geminate_next && !vowel_at(i) {
            gem.push(units.len());
            geminate_next = false;
        }
        let rest: String = cs[i..].iter().collect();
        let take = |s: &str| rest.starts_with(s);
        // Vowels.
        if vowel_at(i) {
            let (ph, len): (&str, usize) =
                if matches!(st, Style::French | Style::Arabic) && take("oua") {
                    ("wɑː", 3)
                } else if matches!(st, Style::French | Style::Arabic) && take("oue") {
                    ("wɛ", 3)
                } else if take("ou") {
                    ("uː", 2)
                } else if take("aa") {
                    ("ɑː", 2)
                } else if take("ee") && st == Style::Arabic {
                    ("iː", 2)
                } else if take("oo") && st == Style::Arabic {
                    ("uː", 2)
                } else if take("ee") {
                    ("eɪ", 2)
                } else if take("ii") {
                    ("iː", 2)
                } else if take("oo") {
                    ("oʊ", 2)
                } else if take("uu") {
                    ("uː", 2)
                } else if st == Style::Plain
                    && rest == "ai"
                    && units.iter().filter(|u| u.1).count() >= 1
                {
                    // Bantu final -ai is two syllables (Tonderai: …ra.i).
                    units.push(("ɑː".to_string(), true));
                    ("i", 2)
                } else if take("ai") && !vowel_at(i + 2) {
                    ("aɪ", 2)
                } else if st == Style::Arabic && rest == "ay" {
                    ("eɪ", 2)
                } else if (take("au") || (st == Style::Arabic && take("aw"))) && !vowel_at(i + 2) {
                    ("aʊ", 2)
                } else if take("ei") {
                    ("eɪ", 2)
                } else if take("oi") && !vowel_at(i + 2) {
                    ("ɔɪ", 2)
                } else {
                    (
                        match c {
                            'a' => "ɑː",
                            'e' | 'é' | 'è' => {
                                if i + 1 == n {
                                    "eɪ"
                                } else {
                                    "ɛ"
                                }
                            }
                            'i' => "iː",
                            'o' if st == Style::Malagasy => "uː",
                            'o' => "oʊ",
                            _ => "uː",
                        },
                        1,
                    )
                };
            // French spellings: a final e after a consonant is silent.
            if matches!(st, Style::French | Style::Arabic)
                && c == 'e'
                && i + 1 == n
                && i > 0
                && !vowel_at(i - 1)
            {
                i += 1;
                continue;
            }
            units.push((ph.to_string(), true));
            i += len;
            continue;
        }
        // A doubled consonant is said once.
        if i + 1 < n && cs[i + 1] == c {
            geminate_next = true;
            i += 1;
            continue;
        }
        let (ph, len): (&str, usize) = if take("tsh") || take("tch") {
            ("tʃ", 3)
        } else if take("ch") {
            (
                if matches!(st, Style::French | Style::Arabic) {
                    "ʃ"
                } else {
                    "tʃ"
                },
                2,
            )
        } else if take("sh") {
            ("ʃ", 2)
        } else if take("zh") {
            ("ʒ", 2)
        } else if take("ny") && (vowel_at(i + 2) || i + 2 == n) {
            ("nj", 2)
        } else if take("ng") && vowel_at(i + 2) {
            ("ŋɡ", 2)
        } else if take("ng") {
            ("ŋ", 2)
        } else if take("nkh") {
            ("ŋk", 3)
        } else if take("nk") {
            ("ŋk", 2)
        } else if take("gb") {
            ("ɡb", 2)
        } else if take("kp") {
            ("kp", 2)
        } else if take("gh") || take("kh") && st == Style::Plain {
            (if c == 'g' { "ɡ" } else { "k" }, 2)
        } else if take("kh") {
            ("k", 2)
        } else if st == Style::Arabic && i > 0 && (take("dh") || take("bh")) {
            // Egyptian spellings: Medhat, Sobhy (d or b, then h).
            (if c == 'd' { "d" } else { "b" }, 1)
        } else if take("dh") {
            ("d", 2)
        } else if take("th") {
            ("t", 2)
        } else if take("ph") {
            ("p", 2)
        } else if take("rh") {
            ("ɹ", 2)
        } else if take("bh") {
            ("b", 2)
        } else if st == Style::Nguni
            && (take("qh") || take("ch") || take("xh") || take("gq") || take("gc") || take("gx"))
        {
            ("k", 2)
        } else if take("hl") {
            ("ʃl", 2)
        } else if take("dj") {
            ("dʒ", 2)
        } else if take("qu") {
            ("k", 2)
        } else if take("gu") && matches!(st, Style::French | Style::Arabic) && vowel_at(i + 2) {
            ("ɡ", 2)
        } else if (matches!(st, Style::French | Style::Arabic)
            || i == 0
            || (i == 1 && cs[0] == 'n'))
            && take("di")
            && matches!(at(i + 2), 'a' | 'o')
        {
            ("dʒ", 2)
        } else if take("ts") {
            ("ts", 2)
        } else if take("dz") {
            ("dz", 2)
        } else {
            (
                match c {
                    'j' => {
                        if matches!(st, Style::French | Style::Arabic) {
                            "ʒ"
                        } else {
                            "dʒ"
                        }
                    }
                    // Final -y after a consonant in English-based Arabic
                    // spellings (Magdy, Fahmy) is the vowel i.
                    'y' if st == Style::Arabic && i + 1 == n && i > 0 && !vowel_at(i - 1) => {
                        units.push(("i".to_string(), true));
                        i += 1;
                        continue;
                    }
                    'y' => "j",
                    'r' => "ɹ",
                    'g' => "ɡ",
                    'c' => match st {
                        Style::Somali => "",
                        Style::French | Style::Plain if matches!(at(i + 1), 'e' | 'i') => "s",
                        _ => "k",
                    },
                    'q' => "k",
                    'x' => match st {
                        Style::Somali => "h",
                        Style::Nguni => "k",
                        // Tsonga: x is ʃ (Xikombiso, Xinavane).
                        Style::Plain if vowel_at(i + 1) => "ʃ",
                        _ => "ks",
                    },
                    'h' if st == Style::Somali && i + 1 < n && !vowel_at(i + 1) => "",
                    'h' if i + 1 == n => "",
                    _ => {
                        let mut b = [0u8; 4];
                        let s: &str = c.encode_utf8(&mut b);
                        units.push((s.to_string(), false));
                        i += 1;
                        continue;
                    }
                },
                1,
            )
        };
        if !ph.is_empty() {
            units.push((ph.to_string(), false));
        }
        i += len;
    }
    let vowels: Vec<usize> = (0..units.len()).filter(|&k| units[k].1).collect();
    let arabic = st == Style::Arabic;
    let ends_weak = ["ka", "tra", "na"].iter().any(|e| w.ends_with(e));
    // Arabic: weight-based stress (a final superheavy syllable, else a heavy
    // penult, else the antepenult). Long vowels are guessed from the
    // spelling: ou, i outside the first syllable, a before a final consonant
    // (or consonant + silent e), doubled letters, diphthongs.
    let arabic_stress = || -> Option<usize> {
        let nv = vowels.len();
        if nv == 0 {
            return None;
        }
        let long = |k: usize| -> bool {
            let v = vowels[k];
            let ph = units[v].0.trim_start_matches('w');
            let rest: Vec<&(String, bool)> = units[v + 1..].iter().collect();
            let next_v = rest.iter().position(|u| u.1);
            let cons_after = next_v.unwrap_or(rest.len());
            match ph {
                "uː" | "aɪ" | "eɪ" | "aʊ" => true,
                "iː" => k > 0 || cons_after == 0,
                "ɑː" => (k + 1 == nv && cons_after == 1) || (k + 2 == nv && cons_after == 1),
                _ => false,
            }
        };
        let closed = |k: usize| -> (bool, usize) {
            let v = vowels[k];
            let end = vowels.get(k + 1).copied().unwrap_or(units.len());
            let cons = end - v - 1;
            let g = gem.iter().any(|&x| x > v && x < end);
            let last = k + 1 == nv;
            (if last { cons >= 1 } else { cons >= 2 || g }, cons)
        };
        let last = nv - 1;
        let (lc, lcons) = closed(last);
        let superheavy = (long(last) && lc) || lcons >= 2;
        if superheavy || nv == 1 {
            return Some(vowels[last]);
        }
        let pen = last - 1;
        if long(pen) || closed(pen).0 || pen == 0 {
            return Some(vowels[pen]);
        }
        Some(vowels[pen - 1])
    };
    let last_vowel_closed = units.last().is_some_and(|u| !u.1)
        && vowels
            .last()
            .is_some_and(|&k| matches!(units[k].0.as_str(), "iː" | "uː"));
    let stressed = match vowels.len() {
        0 => None,
        1 => Some(vowels[0]),
        // Malagasy: the antepenult before the weak endings -ka, -tra, -na.
        _ if arabic => arabic_stress(),
        k if st == Style::Malagasy && ends_weak && k >= 3 => Some(vowels[k - 3]),
        // A final closed syllable with i or u (Agadir, Rachid) takes the stress.
        k if last_vowel_closed && matches!(st, Style::French | Style::Plain) => Some(vowels[k - 1]),
        k => Some(vowels[k - 2]),
    };
    let mut out = String::with_capacity(w.len() * 2);
    for (k, (ph, v)) in units.iter().enumerate() {
        if Some(k) == stressed {
            out.push('ˈ');
            // Arabic names: a stressed a before a double consonant or a
            // cluster is the English short a (Nasser, Ashraf).
            if arabic && ph == "ɑː" {
                let end = (k + 1..units.len())
                    .find(|&x| units[x].1)
                    .unwrap_or(units.len());
                let cons = end - k - 1;
                if end < units.len() && (cons >= 2 || gem.iter().any(|&g| g > k && g < end)) {
                    out.push('æ');
                    continue;
                }
            }
        }
        // Unstressed final -a is reduced in British English readings.
        if *v && Some(k) != stressed && british && ph == "ɑː" && k + 1 == units.len() {
            out.push('ə');
            continue;
        }
        out.push_str(ph);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_and_reads() {
        assert!(looks_african("Mbeki"));
        assert!(looks_african("Chukwuemeka"));
        assert!(looks_african("Ngozi"));
        assert!(!looks_african("Smith"));
        assert!(!looks_african("Cartwright"));
        assert_eq!(read("Chinwe", false), "tʃˈiːnweɪ");
        assert_eq!(read("Adebayo", false), "ɑːdɛbˈɑːjoʊ");
        assert_eq!(read("Faarax", false), "fˈɑːɹɑːh");
    }

    #[test]
    fn arabic_list_and_new_groups() {
        assert!(arabic_name("Mahmoud"));
        assert!(arabic_name("khaled"));
        assert!(!arabic_name("Smith"));
        assert!(!arabic_name("Said"));
        assert_eq!(read_arabic("Mahmoud", false), "mɑːhmˈuːd");
        assert_eq!(read("Magdy", false), "mˈæɡdi");
        assert!(strongly_african("Gebremedhin"));
        assert!(horn_name("Selam") || horn_name("Yonas"));
        assert!(english_reading_fits("nˈæsɚ", "nˈæsɛɹ"));
        assert!(!english_reading_fits("mˈæmuːd", "mɑːhmˈuːd"));
        assert!(read("Xikundu", false).starts_with('ʃ'));
        assert!(read("Tonderai", false).ends_with("ˈɑːi"));
    }
}
