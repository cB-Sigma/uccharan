//! Text normalisation: numbers, ordinals, currency, dates, times, units,
//! abbreviations and symbols are rewritten as words before G2P.
//!
//! Scope (documented in docs/normalisation.md): cardinals up to 10^15,
//! ordinals, decimals, signs, percentages, `$ £ € ₹` amounts, `hh:mm` times,
//! `dd/mm/yyyy` and ISO dates, years, a short list of units and abbreviations,
//! Indian digit grouping (`1,00,000`) and Devanagari digits. Everything else
//! falls back to reading runs of letters, digits and named symbols in order.

use crate::text::{runs, Chunk, Script};
use crate::{Lang, Normalize};

/// One word-like unit handed to a language module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// Letters of the word (Latin or Devanagari), original case.
    pub text: String,
    /// Spell the text letter by letter.
    pub spell: bool,
    /// Join to the previous word without a space in the output.
    pub glue: bool,
    /// The item came from a number or symbol expansion.
    pub generated: bool,
    /// The item follows a hyphen inside the same token (`well-known`).
    pub hyphen: bool,
    /// A word of a number reading other than its first.
    pub number_tail: bool,
}

impl Item {
    pub(crate) fn word(t: &str) -> Item {
        Item {
            text: t.to_string(),
            spell: false,
            glue: false,
            generated: true,
            hyphen: false,
            number_tail: false,
        }
    }
    fn raw(t: &str) -> Item {
        Item {
            text: t.to_string(),
            spell: false,
            glue: false,
            generated: false,
            hyphen: false,
            number_tail: false,
        }
    }
    fn spell(t: &str) -> Item {
        Item {
            text: t.to_string(),
            spell: true,
            glue: false,
            generated: false,
            hyphen: false,
            number_tail: false,
        }
    }
}

const ONES: [&str; 20] = [
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
];
const TENS: [&str; 10] = [
    "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
];
const ORD_ONES: [&str; 20] = [
    "zeroth",
    "first",
    "second",
    "third",
    "fourth",
    "fifth",
    "sixth",
    "seventh",
    "eighth",
    "ninth",
    "tenth",
    "eleventh",
    "twelfth",
    "thirteenth",
    "fourteenth",
    "fifteenth",
    "sixteenth",
    "seventeenth",
    "eighteenth",
    "nineteenth",
];
const ORD_TENS: [&str; 10] = [
    "",
    "",
    "twentieth",
    "thirtieth",
    "fortieth",
    "fiftieth",
    "sixtieth",
    "seventieth",
    "eightieth",
    "ninetieth",
];
const SCALES: [&str; 5] = ["", "thousand", "million", "billion", "trillion"];
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Hindi numerals 0–99.
pub const HI_NUM: [&str; 100] = [
    "शून्य",
    "एक",
    "दो",
    "तीन",
    "चार",
    "पांच",
    "छह",
    "सात",
    "आठ",
    "नौ",
    "दस",
    "ग्यारह",
    "बारह",
    "तेरह",
    "चौदह",
    "पंद्रह",
    "सोलह",
    "सत्रह",
    "अठारह",
    "उन्नीस",
    "बीस",
    "इक्कीस",
    "बाईस",
    "तेईस",
    "चौबीस",
    "पच्चीस",
    "छब्बीस",
    "सत्ताईस",
    "अट्ठाईस",
    "उनतीस",
    "तीस",
    "इकतीस",
    "बत्तीस",
    "तैंतीस",
    "चौंतीस",
    "पैंतीस",
    "छत्तीस",
    "सैंतीस",
    "अड़तीस",
    "उनतालीस",
    "चालीस",
    "इकतालीस",
    "बयालीस",
    "तैंतालीस",
    "चौवालीस",
    "पैंतालीस",
    "छियालीस",
    "सैंतालीस",
    "अड़तालीस",
    "उनचास",
    "पचास",
    "इक्यावन",
    "बावन",
    "तिरेपन",
    "चौवन",
    "पचपन",
    "छप्पन",
    "सत्तावन",
    "अट्ठावन",
    "उनसठ",
    "साठ",
    "इकसठ",
    "बासठ",
    "तिरेसठ",
    "चौंसठ",
    "पैंसठ",
    "छियासठ",
    "सड़सठ",
    "अड़सठ",
    "उनहत्तर",
    "सत्तर",
    "इकहत्तर",
    "बहत्तर",
    "तिहत्तर",
    "चौहत्तर",
    "पचहत्तर",
    "छिहत्तर",
    "सतहत्तर",
    "अठहत्तर",
    "उनासी",
    "अस्सी",
    "इक्यासी",
    "बयासी",
    "तिरासी",
    "चौरासी",
    "पचासी",
    "छियासी",
    "सत्तासी",
    "अट्ठासी",
    "नवासी",
    "नब्बे",
    "इक्यानवे",
    "बानवे",
    "तिरानवे",
    "चौरानवे",
    "पंचानवे",
    "छियानवे",
    "सत्तानवे",
    "अट्ठानवे",
    "निन्यानवे",
];
const HI_MONTHS: [&str; 12] = [
    "जनवरी",
    "फ़रवरी",
    "मार्च",
    "अप्रैल",
    "मई",
    "जून",
    "जुलाई",
    "अगस्त",
    "सितंबर",
    "अक्टूबर",
    "नवंबर",
    "दिसंबर",
];

struct Ctx {
    lang: Lang,
    style: Normalize,
    out: Vec<Item>,
    /// Keep known mixed-case names whole (improved mode).
    brands: bool,
}

/// Mixed-case names that are one word, not camelCase humps.
const BRANDS: &[&str] = &[
    "iphone",
    "ipad",
    "ipod",
    "imac",
    "ios",
    "macos",
    "youtube",
    "latex",
    "github",
    "gitlab",
    "whatsapp",
    "linkedin",
    "paypal",
    "javascript",
    "typescript",
    "powerpoint",
    "wifi",
    "ebay",
    "macbook",
    "playstation",
    "facebook",
    "wordpress",
    "airbnb",
    "tiktok",
    "onedrive",
    "outlook",
    "mcdonald",
    "mcdonald's",
    "openstreetmap",
    "postgresql",
    "mysql",
];

impl Ctx {
    fn w(&mut self, t: &str) {
        self.out.push(Item::word(t));
    }

    fn hi(&self) -> bool {
        self.lang == Lang::Hi
    }

    fn espeak(&self) -> bool {
        self.style == Normalize::EspeakLike
    }

    /// English 0..=999.
    fn en_below_1000(&mut self, n: u64, and: bool) {
        let (h, r) = (n / 100, n % 100);
        if h > 0 {
            self.w(ONES[h as usize]);
            self.w("hundred");
            if self.espeak() {
                if let Some(l) = self.out.last_mut() {
                    l.glue = true;
                }
            }
            if r > 0 && and {
                self.w("and");
            }
        }
        if r >= 20 {
            self.w(TENS[(r / 10) as usize]);
            if r % 10 > 0 {
                self.w(ONES[(r % 10) as usize]);
            }
        } else if r > 0 || h == 0 {
            self.w(ONES[r as usize]);
        }
    }

    fn en_cardinal(&mut self, n: u64) {
        if n == 0 {
            self.w("zero");
            return;
        }
        let and = self.lang == Lang::EnGb;
        let mut groups: Vec<u64> = Vec::new();
        let mut m = n;
        while m > 0 {
            groups.push(m % 1000);
            m /= 1000;
        }
        if groups.len() > SCALES.len() {
            self.digits(&n.to_string());
            return;
        }
        for i in (0..groups.len()).rev() {
            let g = groups[i];
            if g == 0 {
                continue;
            }
            if i == 0 && g < 100 && n > 1000 && and {
                self.w("and");
            }
            self.en_below_1000(g, and);
            if i > 0 {
                self.w(SCALES[i]);
            }
        }
    }

    fn en_ordinal(&mut self, n: u64) {
        if n % 100 == 0 || n >= 1_000_000 {
            self.en_cardinal(n);
            if let Some(l) = self.out.last_mut() {
                l.text.push_str("th");
            }
            return;
        }
        let (head, r) = (n - n % 100, n % 100);
        if head > 0 {
            self.en_cardinal(head);
            if self.lang == Lang::EnGb {
                self.w("and");
            }
        }
        if r < 20 {
            self.w(ORD_ONES[r as usize]);
        } else if r % 10 == 0 {
            self.w(ORD_TENS[(r / 10) as usize]);
        } else {
            self.w(TENS[(r / 10) as usize]);
            self.w(ORD_ONES[(r % 10) as usize]);
        }
    }

    /// A year in natural reading: 1984 → nineteen eighty four, 2005 → two thousand five.
    fn en_year(&mut self, y: u64) {
        let (hi, lo) = (y / 100, y % 100);
        if self.espeak() {
            if (1900..2000).contains(&y) {
                self.w("nineteen");
                self.w("hundred");
                self.out.last_mut().unwrap().glue = true;
                if lo > 0 {
                    self.en_below_1000(lo, false);
                }
            } else {
                self.en_cardinal(y);
            }
        } else if !(1100..2100).contains(&y) || (2000..2010).contains(&y) || lo == 0 && hi % 10 == 0
        {
            self.en_cardinal(y);
        } else {
            self.en_below_1000(hi, false);
            if lo == 0 {
                self.w("hundred");
            } else if lo < 10 {
                self.w("oh");
                self.w(ONES[lo as usize]);
            } else {
                self.en_below_1000(lo, false);
            }
        }
    }

    fn hi_below_1000(&mut self, n: u64) {
        let (h, r) = (n / 100, n % 100);
        if h > 0 {
            self.w(HI_NUM[h as usize]);
            self.w("सौ");
            if self.espeak() {
                self.out.last_mut().unwrap().glue = true;
            }
        }
        if r > 0 || h == 0 {
            self.w(HI_NUM[r as usize]);
        }
    }

    fn hi_cardinal(&mut self, n: u64) {
        if n < 1000 {
            self.hi_below_1000(n);
            return;
        }
        // Indian grouping: 3 digits, then pairs.
        const SC: [(u64, &str); 5] = [
            (100_000_000_000, "खरब"),
            (1_000_000_000, "अरब"),
            (10_000_000, "करोड़"),
            (100_000, "लाख"),
            (1000, "हजार"),
        ];
        let mut rest = n;
        for (i, (unit, name)) in SC.iter().enumerate() {
            let q = rest / unit;
            rest %= unit;
            if q > 0 {
                if i == 0 && q >= 100 {
                    self.hi_cardinal(q);
                } else {
                    self.hi_below_1000(q.min(999));
                }
                self.w(name);
            }
        }
        if rest > 0 {
            self.hi_below_1000(rest);
        }
    }

    fn cardinal(&mut self, n: u64) {
        if self.hi() {
            self.hi_cardinal(n)
        } else {
            self.en_cardinal(n)
        }
    }

    fn digits(&mut self, s: &str) {
        for c in s.chars() {
            if let Some(d) = c.to_digit(10) {
                if self.hi() {
                    self.w(HI_NUM[d as usize]);
                } else {
                    self.w(ONES[d as usize]);
                }
            }
        }
    }

    /// Integer string (ASCII digits, optional group commas) and optional fraction.
    fn number(&mut self, int: &str, frac: Option<&str>) {
        let clean: String = int.chars().filter(|c| c.is_ascii_digit()).collect();
        if clean.is_empty() {
            self.w(if self.hi() { "शून्य" } else { "zero" });
        } else if (clean.starts_with('0') && clean.len() > 1) || clean.len() > 15 {
            self.digits(&clean);
        } else {
            self.cardinal(clean.parse().unwrap_or(0));
        }
        if let Some(f) = frac {
            self.w(if self.hi() {
                "दशमलव"
            } else {
                "point"
            });
            self.digits(f);
        }
    }

    fn symbol(&mut self, c: char) {
        let name: &[&str] = if self.hi() {
            match c {
                '&' => &["और"],
                '%' => &["प्रतिशत"],
                '+' => &["प्लस"],
                '=' => &["बराबर"],
                '₹' => &["रुपये"],
                '$' => &["डॉलर"],
                '@' => &["ऐट"],
                '°' => &["डिग्री"],
                _ => &[],
            }
        } else {
            match c {
                '&' => &["and"],
                '@' => &["at"],
                '%' => &["percent"],
                '+' => &["plus"],
                '=' => &["equals"],
                '#' => &["hash"],
                '/' => &["slash"],
                '\\' => &["backslash"],
                '~' => &["tilde"],
                '*' => &["asterisk"],
                '$' => &["dollar"],
                '£' => &["pound"],
                '€' => &["euro"],
                '₹' => &["rupee"],
                '°' => &["degrees"],
                '©' => &["copyright"],
                '½' => &["a", "half"],
                '×' => &["times"],
                _ => &[],
            }
        };
        for n in name {
            self.w(n);
        }
    }
}

fn split_number(s: &str) -> Option<(&str, Option<&str>)> {
    // digits with optional commas, optional single ".digits"
    let (int, frac) = match s.split_once('.') {
        Some((a, b)) => (a, Some(b)),
        None => (s, None),
    };
    if int.is_empty() && frac.is_none() {
        return None;
    }
    if !int.chars().all(|c| c.is_ascii_digit() || c == ',')
        || int.starts_with(',')
        || int.ends_with(',')
    {
        return None;
    }
    if int.contains(',') {
        // Western 1,234,567 or Indian 12,34,567 grouping: last group of three, others of two or three.
        let parts: Vec<&str> = int.split(',').collect();
        let last_ok = parts.last().is_some_and(|p| p.len() == 3);
        let mid_ok = parts[1..parts.len() - 1]
            .iter()
            .all(|p| p.len() == 2 || p.len() == 3);
        let first_ok = (1..=3).contains(&parts[0].len());
        if !(last_ok && mid_ok && first_ok) {
            return None;
        }
    }
    if let Some(f) = frac {
        if f.is_empty() || !f.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
    }
    Some((int, frac))
}

fn to_ascii_digits(s: &str) -> String {
    s.chars()
        .map(|c| match crate::text::indic_digit(c) {
            Some(d) => char::from(b'0' + d),
            None => c,
        })
        .collect()
}

const UNITS: &[(&str, &str, &str, &str)] = &[
    // symbol, singular, plural, Hindi
    ("km", "kilometer", "kilometers", "किलोमीटर"),
    ("m", "meter", "meters", "मीटर"),
    ("cm", "centimeter", "centimeters", "सेंटीमीटर"),
    ("mm", "millimeter", "millimeters", "मिलीमीटर"),
    ("kg", "kilogram", "kilograms", "किलोग्राम"),
    ("g", "gram", "grams", "ग्राम"),
    ("mg", "milligram", "milligrams", "मिलीग्राम"),
    ("l", "liter", "liters", "लीटर"),
    ("ml", "milliliter", "milliliters", "मिलीलीटर"),
    (
        "kmh",
        "kilometer per hour",
        "kilometers per hour",
        "किलोमीटर प्रति घंटा",
    ),
    (
        "km/h",
        "kilometer per hour",
        "kilometers per hour",
        "किलोमीटर प्रति घंटा",
    ),
    ("mph", "mile per hour", "miles per hour", "मील प्रति घंटा"),
    ("°c", "degree Celsius", "degrees Celsius", "डिग्री सेल्सियस"),
    (
        "°f",
        "degree Fahrenheit",
        "degrees Fahrenheit",
        "डिग्री फ़ारेनहाइट",
    ),
    ("hz", "hertz", "hertz", "हर्ट्ज़"),
    ("khz", "kilohertz", "kilohertz", "किलोहर्ट्ज़"),
    ("gb", "gigabyte", "gigabytes", "गीगाबाइट"),
    ("mb", "megabyte", "megabytes", "मेगाबाइट"),
    ("kb", "kilobyte", "kilobytes", "किलोबाइट"),
    ("ms", "millisecond", "milliseconds", "मिलीसेकंड"),
    ("min", "minute", "minutes", "मिनट"),
    ("hr", "hour", "hours", "घंटा"),
    ("ft", "foot", "feet", "फ़ुट"),
    ("lb", "pound", "pounds", "पाउंड"),
    ("lbs", "pound", "pounds", "पाउंड"),
];

const ABBREVIATIONS: &[(&str, &str)] = &[
    ("mr", "mister"),
    ("mrs", "missus"),
    ("ms", "miz"),
    ("dr", "doctor"),
    ("prof", "professor"),
    ("st", "saint"),
    ("jr", "junior"),
    ("sr", "senior"),
    ("vs", "versus"),
    ("etc", "etcetera"),
    ("no", "number"),
    ("ltd", "limited"),
    ("inc", "incorporated"),
    ("mt", "mount"),
    ("approx", "approximately"),
    ("dept", "department"),
    ("jan", "January"),
    ("feb", "February"),
    ("mar", "March"),
    ("apr", "April"),
    ("jun", "June"),
    ("jul", "July"),
    ("aug", "August"),
    ("sep", "September"),
    ("sept", "September"),
    ("oct", "October"),
    ("nov", "November"),
    ("dec", "December"),
    ("डॉ", "डॉक्टर"),
    ("रु", "रुपये"),
    ("नं", "नंबर"),
    ("प्रो", "प्रोफ़ेसर"),
];

/// Abbreviations that are expanded even without a dot.
const BARE_ABBREVIATIONS: &[&str] = &["mr", "mrs", "dr", "vs"];

/// Expand one chunk into word items.
pub fn expand(chunk: &Chunk, lang: Lang, style: Normalize) -> Vec<Item> {
    expand_with(chunk, lang, style, false)
}

/// [`expand`], optionally keeping known mixed-case names ("iPhone",
/// "YouTube") as one word instead of splitting them at the capitals.
pub fn expand_with(chunk: &Chunk, lang: Lang, style: Normalize, brands: bool) -> Vec<Item> {
    let mut cx = Ctx {
        lang,
        style,
        out: Vec::new(),
        brands,
    };
    let raw = chunk.text.as_str();
    if style == Normalize::Off {
        for (sc, r) in runs(raw) {
            if matches!(sc, Script::Latin | Script::Devanagari) {
                latin_or_deva(&mut cx, r, sc);
            }
        }
        return cx.out;
    }
    let ascii = to_ascii_digits(raw);
    let s = ascii.as_str();
    let lower = s.to_lowercase();

    // Abbreviations (with a dot, or the few that never need one).
    if let Some(&(_, full)) = ABBREVIATIONS.iter().find(|a| a.0 == lower) {
        let is_title_case = raw.chars().next().is_some_and(|c| c.is_uppercase()) || !raw.is_ascii();
        if (chunk.dot
            && (is_title_case
                || !matches!(lower.as_str(), "no" | "mar" | "jun" | "dec" | "sep" | "st")))
            || (BARE_ABBREVIATIONS.contains(&lower.as_str()) && is_title_case)
        {
            for w in full.split(' ') {
                cx.w(w);
            }
            return cx.out;
        }
    }
    // Dotted initialisms: U.S.A, a.m, Ph.D-like chunks of single letters.
    if s.contains('.')
        && s.split('.')
            .all(|p| p.chars().count() == 1 && p.chars().all(|c| c.is_ascii_alphabetic()))
        && !matches!(lower.as_str(), "e.g" | "i.e")
    {
        cx.out.push(Item::spell(&s.replace('.', "")));
        return cx.out;
    }
    match lower.as_str() {
        "e.g" => {
            cx.w("for");
            cx.w("example");
            return cx.out;
        }
        "i.e" => {
            cx.w("that");
            cx.w("is");
            return cx.out;
        }
        _ => {}
    }

    if !cx.espeak() {
        let per = match lower.as_str() {
            "km/h" | "kmph" | "kph" => Some(("kilometers per hour", "किलोमीटर प्रति घंटा")),
            "m/s" => Some(("meters per second", "मीटर प्रति सेकंड")),
            "mph" => Some(("miles per hour", "मील प्रति घंटा")),
            _ => None,
        };
        if let Some((en, hi)) = per {
            for w in (if cx.hi() { hi } else { en }).split(' ') {
                cx.w(w);
            }
            return cx.out;
        }
    }
    if !cx.espeak() && special(&mut cx, s, &lower) {
        return cx.out;
    }
    if cx.espeak() && special_espeak(&mut cx, s, &lower) {
        return cx.out;
    }

    // Generic fallback: read runs in order.
    let rs = runs(s);
    let n = rs.len();
    for (i, (sc, r)) in rs.iter().enumerate() {
        match sc {
            Script::Latin | Script::Devanagari => {
                let before = cx.out.len();
                latin_or_deva(&mut cx, r, *sc);
                if i >= 2
                    && rs[i - 1].1 == "-"
                    && matches!(rs[i - 2].0, Script::Latin | Script::Devanagari)
                {
                    if let Some(it) = cx.out.get_mut(before) {
                        it.hyphen = true;
                    }
                }
            }
            Script::Digit => match split_number(r) {
                Some((int, frac)) => cx.number(int, frac),
                None => {
                    for part in r.split([',', '.', ':']) {
                        if !part.is_empty() {
                            cx.number(part, None);
                        }
                    }
                }
            },
            // Other Indic scripts are read by their own languages (see `gnorm`).
            Script::Brahmic(_) => cx.out.push(Item::raw(r)),
            Script::Other => {
                let c = r.chars().next().unwrap_or(' ');
                match c {
                    '-' if i == 0 && n > 1 && rs[1].0 == Script::Digit => {
                        cx.w(if cx.hi() { "ऋण" } else { "minus" })
                    }
                    '.' if i > 0 && i + 1 < n && !cx.hi() => cx.w("dot"),
                    '-' | '\'' | '’' | '.' | ',' | ':' | '_' | '|' | '^' | '<' | '>' | '('
                    | ')' | '"' => {}
                    _ => cx.symbol(c),
                }
            }
        }
    }
    cx.out
}

fn latin_or_deva(cx: &mut Ctx, r: &str, sc: Script) {
    if sc == Script::Devanagari {
        let t = r.replace(['\u{200c}', '\u{200d}'], "");
        if !t.is_empty() {
            cx.out.push(Item::raw(&t));
        }
        return;
    }
    let _ = sc;
    // Fast path: a plain word in lower case or capitalised (most words).
    let b = r.as_bytes();
    // (`push_latin` would keep these unchanged: no hyphen, hump or capitals run.)
    if !b.is_empty() && b[0].is_ascii_alphabetic() && b[1..].iter().all(u8::is_ascii_lowercase) {
        cx.out.push(Item::raw(r));
        return;
    }
    // Hyphenated parts and camelCase humps are separate words.
    for (pi, part) in r.split('-').enumerate() {
        if part.is_empty() {
            continue;
        }
        let before = cx.out.len();
        let cs: Vec<char> = part.chars().collect();
        if cx.brands && BRANDS.contains(&part.to_lowercase().as_str()) {
            cx.out.push(Item::raw(part));
            continue;
        }
        let mut start = 0;
        for i in 1..cs.len() {
            let hump = cs[i].is_uppercase() && cs[i - 1].is_lowercase()
                || (cs[i].is_uppercase()
                    && cs.get(i + 1).is_some_and(|n| n.is_lowercase())
                    && cs[i - 1].is_uppercase()
                    && i - start >= 2);
            if hump {
                push_latin(cx, &cs[start..i]);
                start = i;
            }
        }
        push_latin(cx, &cs[start..]);
        if pi > 0 {
            if let Some(it) = cx.out.get_mut(before) {
                it.hyphen = true;
            }
        }
    }
}

fn push_latin(cx: &mut Ctx, cs: &[char]) {
    let t: String = cs.iter().collect();
    let t = t.trim_matches(|c| c == '\'' || c == '’').replace('’', "'");
    if t.is_empty() {
        return;
    }
    let letters = t.chars().filter(|c| c.is_alphabetic()).count();
    let all_caps = letters >= 2 && t.chars().all(|c| !c.is_lowercase());
    if all_caps
        && letters <= 3
        && !matches!(t.as_str(), "OK" | "AM" | "PM" | "US" | "UK" | "TV" | "ID")
        && is_plain_word_in_caps(&t)
    {
        cx.out.push(Item::raw(&t));
    } else if all_caps && (letters <= 3 || !t.chars().any(|c| "AEIOUY".contains(c))) {
        cx.out.push(Item::spell(&t));
    } else {
        cx.out.push(Item::raw(&t));
    }
}

/// Short capitalised words that are words, not initialisms, even in capitals.
fn is_plain_word_in_caps(t: &str) -> bool {
    matches!(
        t,
        "I" | "A"
            | "AN"
            | "THE"
            | "TO"
            | "OF"
            | "IN"
            | "ON"
            | "AT"
            | "IS"
            | "IT"
            | "HE"
            | "WE"
            | "ME"
            | "MY"
            | "NO"
            | "SO"
            | "DO"
            | "GO"
            | "UP"
            | "BY"
            | "OR"
            | "IF"
            | "AS"
            | "BE"
            | "AND"
            | "FOR"
            | "NOT"
            | "YOU"
            | "ARE"
            | "BUT"
            | "ALL"
            | "CAN"
            | "HAS"
            | "HAD"
            | "HER"
            | "HIS"
            | "HIM"
            | "WAS"
            | "ONE"
            | "OUR"
            | "OUT"
            | "NOW"
            | "NEW"
            | "YES"
            | "WHO"
            | "WHY"
            | "HOW"
            | "SHE"
            | "ITS"
            | "ANY"
            | "MAY"
            | "DID"
            | "GET"
            | "GOT"
            | "LET"
            | "SEE"
            | "TWO"
            | "TOO"
            | "USE"
            | "WAY"
            | "DAY"
            | "MAN"
            | "OLD"
            | "BIG"
            | "END"
            | "TOP"
            | "RED"
            | "BAD"
            | "HOT"
            | "WOW"
            | "OH"
            | "AH"
            | "HI"
    )
}

fn unit(cx: &mut Ctx, sym: &str, plural: bool) -> bool {
    let Some(u) = UNITS.iter().find(|u| u.0 == sym) else {
        return false;
    };
    let text = if cx.hi() {
        u.3
    } else if plural {
        u.2
    } else {
        u.1
    };
    for w in text.split(' ') {
        cx.w(w);
    }
    true
}

/// Natural readings of whole-chunk patterns. Returns false when none applies.
fn special(cx: &mut Ctx, s: &str, lower: &str) -> bool {
    let b = s.as_bytes();
    let first = s.chars().next().unwrap_or(' ');
    // Currency: $5, $5.50, ₹1,00,000, £3.2m
    if matches!(first, '$' | '£' | '€' | '₹') {
        let rest = &s[first.len_utf8()..];
        let (num, mult) = match rest.to_lowercase().as_str() {
            r if r.ends_with("bn") => (&rest[..rest.len() - 2], "billion"),
            r if r.ends_with('k') => (&rest[..rest.len() - 1], "thousand"),
            r if r.ends_with('m') => (&rest[..rest.len() - 1], "million"),
            r if r.ends_with('b') => (&rest[..rest.len() - 1], "billion"),
            _ => (rest, ""),
        };
        if let Some((int, frac)) = split_number(num) {
            let (one, many, sub_one, sub_many, hi_name, hi_sub) = match first {
                '$' => ("dollar", "dollars", "cent", "cents", "डॉलर", "सेंट"),
                '£' => ("pound", "pounds", "penny", "pence", "पाउंड", "पेंस"),
                '€' => ("euro", "euros", "cent", "cents", "यूरो", "सेंट"),
                _ => ("rupee", "rupees", "paisa", "paise", "रुपये", "पैसे"),
            };
            let whole: u64 = int.replace(',', "").parse().unwrap_or(0);
            let cents = frac
                .filter(|f| f.len() == 2 && mult.is_empty())
                .and_then(|f| f.parse::<u64>().ok());
            if !mult.is_empty() || (frac.is_some() && cents.is_none()) {
                cx.number(int, frac);
                if !mult.is_empty() {
                    cx.w(if cx.hi() {
                        match mult {
                            "thousand" => "हजार",
                            "million" => "मिलियन",
                            _ => "बिलियन",
                        }
                    } else {
                        mult
                    });
                }
                cx.w(if cx.hi() { hi_name } else { many });
            } else {
                cx.number(int, None);
                cx.w(if cx.hi() {
                    if first == '₹' && whole == 1 {
                        "रुपया"
                    } else {
                        hi_name
                    }
                } else if whole == 1 {
                    one
                } else {
                    many
                });
                if let Some(c) = cents.filter(|&c| c > 0) {
                    if !cx.hi() {
                        cx.w("and");
                    }
                    cx.cardinal(c);
                    cx.w(if cx.hi() {
                        hi_sub
                    } else if c == 1 {
                        sub_one
                    } else {
                        sub_many
                    });
                }
            }
            return true;
        }
        return false;
    }
    if !b
        .first()
        .is_some_and(|c| c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.'))
    {
        return false;
    }
    // Time: 3:30, 03:05, 12:00, 3:30pm
    if let Some((h, rest)) = s.split_once(':') {
        let (m, ampm) = match lower.rsplit_once(':').map(|x| x.1).unwrap_or("") {
            r if r.ends_with("am") || r.ends_with("pm") => {
                (&rest[..rest.len() - 2], &lower[lower.len() - 2..])
            }
            _ => (rest, ""),
        };
        if let (Ok(hh), Ok(mm)) = (h.parse::<u64>(), m.parse::<u64>()) {
            if hh < 24 && mm < 60 && m.len() == 2 && h.len() <= 2 {
                if cx.hi() {
                    cx.hi_cardinal(hh);
                    if mm == 0 {
                        cx.w("बजे");
                    } else {
                        cx.w("बजकर");
                        cx.hi_cardinal(mm);
                        cx.w("मिनट");
                    }
                } else {
                    cx.en_cardinal(hh);
                    if mm == 0 {
                        if ampm.is_empty() && hh <= 12 {
                            cx.w("o'clock");
                        } else if ampm.is_empty() {
                            cx.w("hundred");
                        }
                    } else if mm < 10 {
                        cx.w("oh");
                        cx.en_cardinal(mm);
                    } else {
                        cx.en_cardinal(mm);
                    }
                    if !ampm.is_empty() {
                        cx.out.push(Item {
                            text: ampm.to_uppercase(),
                            spell: true,
                            glue: false,
                            generated: true,
                            hyphen: false,
                            number_tail: false,
                        });
                    }
                }
                return true;
            }
        }
    }
    // Dates: 2024-01-05, 5/1/2024 (day first outside en-US), 05.01.2024
    let sep = if s.matches('/').count() == 2 {
        Some('/')
    } else if s.matches('-').count() == 2 && s.len() == 10 {
        Some('-')
    } else {
        None
    };
    if let Some(sep) = sep {
        let p: Vec<u64> = s.split(sep).filter_map(|x| x.parse().ok()).collect();
        if p.len() == 3 {
            let (y, m, d) = if sep == '-' && p[0] > 31 {
                (p[0], p[1], p[2])
            } else if cx.lang == Lang::EnUs {
                (p[2], p[0], p[1])
            } else {
                (p[2], p[1], p[0])
            };
            let y = if y < 100 { 2000 + y } else { y };
            if (1..=12).contains(&m) && (1..=31).contains(&d) {
                match cx.lang {
                    Lang::Hi => {
                        cx.hi_cardinal(d);
                        cx.w(HI_MONTHS[m as usize - 1]);
                        cx.hi_cardinal(y);
                    }
                    Lang::EnUs => {
                        cx.w(MONTHS[m as usize - 1]);
                        cx.en_ordinal(d);
                        cx.en_year(y);
                    }
                    _ => {
                        cx.w("the");
                        cx.en_ordinal(d);
                        cx.w("of");
                        cx.w(MONTHS[m as usize - 1]);
                        cx.en_year(y);
                    }
                }
                return true;
            }
        }
    }
    // Telephone-style digit strings, ranges and simple fractions.
    if b[0].is_ascii_digit() || (b[0] == b'+' && b.len() > 10) {
        let digits_only = s.trim_start_matches('+');
        let groups: Vec<&str> = digits_only.split('-').collect();
        let all_digits = groups
            .iter()
            .all(|g| !g.is_empty() && g.bytes().all(|c| c.is_ascii_digit()));
        let total: usize = groups.iter().map(|g| g.len()).sum();
        let mobile = groups.len() == 1
            && ((total == 10 && matches!(b[0], b'6'..=b'9') && cx.hi()) || b[0] == b'+');
        let year_range = groups.len() == 2
            && groups
                .iter()
                .all(|g| g.len() == 4 && g.parse::<u64>().is_ok_and(|v| (1100..2100).contains(&v)))
            && groups[0] < groups[1];
        if all_digits && !year_range && ((groups.len() >= 2 && total >= 7) || mobile) {
            if b[0] == b'+' {
                cx.w(if cx.hi() { "प्लस" } else { "plus" });
            }
            for g in &groups {
                for d in g.bytes() {
                    cx.cardinal((d - b'0') as u64);
                }
            }
            return true;
        }
        if all_digits && groups.len() == 2 && (total < 7 || year_range) {
            if let (Ok(a), Ok(z)) = (groups[0].parse::<u64>(), groups[1].parse::<u64>()) {
                if a < z {
                    let year = |v: u64, g: &str| g.len() == 4 && (1100..2100).contains(&v);
                    if year(a, groups[0]) && !cx.hi() {
                        cx.en_year(a);
                    } else {
                        cx.cardinal(a);
                    }
                    cx.w(if cx.hi() { "से" } else { "to" });
                    if year(z, groups[1]) && !cx.hi() {
                        cx.en_year(z);
                    } else {
                        cx.cardinal(z);
                    }
                    return true;
                }
            }
        }
        let fraction = match s {
            "1/2" => Some(("one half", "आधा")),
            "1/3" => Some(("one third", "एक तिहाई")),
            "2/3" => Some(("two thirds", "दो तिहाई")),
            "1/4" => Some(("one quarter", "एक चौथाई")),
            "3/4" => Some(("three quarters", "तीन चौथाई")),
            _ => None,
        };
        if let Some((en, hi)) = fraction {
            for w in (if cx.hi() { hi } else { en }).split(' ') {
                cx.w(w);
            }
            return true;
        }
    }
    // Signed numbers, percentages, ordinals, units, decades, plain numbers.
    let (sign, body) = match b[0] {
        b'-' => ("minus", &s[1..]),
        b'+' => ("plus", &s[1..]),
        _ => ("", s),
    };
    let split = body
        .find(|c: char| !(c.is_ascii_digit() || c == ',' || c == '.'))
        .unwrap_or(body.len());
    let (num, tail) = body.split_at(split);
    let num_trim = num.trim_end_matches(['.', ',']);
    let Some((int, frac)) = split_number(num_trim) else {
        return false;
    };
    if num_trim.len() != num.len() && !tail.is_empty() {
        return false;
    }
    let tail_l = tail.to_lowercase();
    let value: u64 = int.replace(',', "").parse().unwrap_or(u64::MAX);
    let emit_sign = |cx: &mut Ctx| {
        if !sign.is_empty() {
            cx.w(if cx.hi() {
                if sign == "minus" {
                    "ऋण"
                } else {
                    "धन"
                }
            } else {
                sign
            });
        }
    };
    match tail_l.as_str() {
        "" => {
            emit_sign(cx);
            let plain_year = sign.is_empty()
                && frac.is_none()
                && !int.contains(',')
                && int.len() == 4
                && !cx.hi()
                && !int.starts_with('0');
            if plain_year && (1100..2100).contains(&value) {
                cx.en_year(value);
            } else {
                cx.number(int, frac);
            }
            true
        }
        "%" => {
            emit_sign(cx);
            cx.number(int, frac);
            cx.w(if cx.hi() {
                "प्रतिशत"
            } else {
                "percent"
            });
            true
        }
        "st" | "nd" | "rd" | "th" if frac.is_none() && !cx.hi() => {
            cx.en_ordinal(value);
            true
        }
        "s" | "'s" if frac.is_none() && !cx.hi() && value >= 10 => {
            // Decades: 1990s → nineteen nineties, 90s → nineties.
            let decade = value % 100;
            if value >= 1000 && decade % 10 == 0 && decade > 0 {
                cx.en_below_1000(value / 100, false);
                cx.w(&format!(
                    "{}ies",
                    TENS[(decade / 10) as usize].trim_end_matches('y')
                ));
            } else if value < 100 && value % 10 == 0 && value >= 20 {
                cx.w(&format!(
                    "{}ies",
                    TENS[(value / 10) as usize].trim_end_matches('y')
                ));
            } else {
                cx.number(int, None);
                if let Some(l) = cx.out.last_mut() {
                    l.text.push('s');
                }
            }
            true
        }
        "वां" | "वाँ" | "वीं" | "वें" if cx.hi() => {
            cx.number(int, None);
            if let Some(l) = cx.out.last_mut() {
                l.text.push_str(&tail_l);
            }
            true
        }
        t => {
            emit_sign(cx);
            let before = cx.out.len();
            cx.number(int, frac);
            if unit(cx, t, value != 1 || frac.is_some()) {
                true
            } else {
                cx.out.truncate(before);
                if !sign.is_empty() {
                    cx.out.pop();
                }
                false
            }
        }
    }
}

/// The reference's own readings, for agreement measurements: symbols are read
/// where they stand, times drop the colon, only 19xx is a "hundred" year.
fn special_espeak(cx: &mut Ctx, s: &str, _lower: &str) -> bool {
    let b = s.as_bytes();
    if !b.first().is_some_and(|c| c.is_ascii_digit()) {
        return false;
    }
    if let Some((h, m)) = s.split_once(':') {
        if let (Ok(hh), Ok(mm)) = (h.parse::<u64>(), m.parse::<u64>()) {
            if m.len() == 2 && !cx.hi() {
                cx.en_cardinal(hh);
                if mm < 10 {
                    cx.w("zero");
                    cx.w(ONES[mm as usize]);
                } else {
                    cx.en_cardinal(mm);
                }
                return true;
            }
        }
    }
    let split = s
        .find(|c: char| !(c.is_ascii_digit() || c == ',' || c == '.'))
        .unwrap_or(s.len());
    let (num, tail) = s.split_at(split);
    let Some((int, frac)) = split_number(num) else {
        return false;
    };
    let value: u64 = int.replace(',', "").parse().unwrap_or(u64::MAX);
    match tail {
        "" if frac.is_none() && !int.contains(',') && (1900..2000).contains(&value) && !cx.hi() => {
            cx.en_year(value);
            true
        }
        "st" | "nd" | "rd" | "th" if frac.is_none() && !cx.hi() && value % 100 != 0 => {
            cx.en_ordinal(value);
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ex(s: &str, lang: Lang, style: Normalize) -> String {
        let c = Chunk {
            text: s.trim_end_matches('.').to_string(),
            dot: s.ends_with('.'),
        };
        expand(&c, lang, style)
            .iter()
            .map(|i| i.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
    fn us(s: &str) -> String {
        ex(s, Lang::EnUs, Normalize::Natural)
    }
    fn hi(s: &str) -> String {
        ex(s, Lang::Hi, Normalize::Natural)
    }

    #[test]
    fn cardinals() {
        assert_eq!(us("0"), "zero");
        assert_eq!(us("21"), "twenty one");
        assert_eq!(us("101"), "one hundred one");
        assert_eq!(
            ex("101", Lang::EnGb, Normalize::Natural),
            "one hundred and one"
        );
        assert_eq!(
            us("1,234,567"),
            "one million two hundred thirty four thousand five hundred sixty seven"
        );
        assert_eq!(us("3.14"), "three point one four");
        assert_eq!(us("-5"), "minus five");
        assert_eq!(us("007"), "zero zero seven");
        assert_eq!(us("10000"), "ten thousand");
    }

    #[test]
    fn years_ordinals_decades() {
        assert_eq!(us("1984"), "nineteen eighty four");
        assert_eq!(us("2005"), "two thousand five");
        assert_eq!(us("2024"), "twenty twenty four");
        assert_eq!(us("1900"), "nineteen hundred");
        assert_eq!(us("1st"), "first");
        assert_eq!(us("22nd"), "twenty second");
        assert_eq!(us("100th"), "one hundredth");
        assert_eq!(us("1990s"), "nineteen nineties");
        assert_eq!(
            ex("1984", Lang::EnUs, Normalize::EspeakLike),
            "nineteen hundred eighty four"
        );
        assert_eq!(
            ex("2024", Lang::EnUs, Normalize::EspeakLike),
            "two thousand twenty four"
        );
    }

    #[test]
    fn money_time_dates_units() {
        assert_eq!(us("$5.50"), "five dollars and fifty cents");
        assert_eq!(us("$1"), "one dollar");
        assert_eq!(us("₹500"), "five hundred rupees");
        assert_eq!(us("$1.5bn"), "one point five billion dollars");
        assert_eq!(us("50%"), "fifty percent");
        assert_eq!(us("3:30"), "three thirty");
        assert_eq!(us("12:00"), "twelve o'clock");
        assert_eq!(us("9:05am"), "nine oh five AM");
        assert_eq!(us("2024-01-05"), "January fifth twenty twenty four");
        assert_eq!(
            ex("5/1/2024", Lang::EnGb, Normalize::Natural),
            "the fifth of January twenty twenty four"
        );
        assert_eq!(us("5km"), "five kilometers");
        assert_eq!(us("1kg"), "one kilogram");
        assert_eq!(
            ex("$5.50", Lang::EnUs, Normalize::EspeakLike),
            "dollar five point five zero"
        );
    }

    #[test]
    fn abbreviations_symbols_case() {
        assert_eq!(us("Dr."), "doctor");
        assert_eq!(us("Mr"), "mister");
        assert_eq!(us("AT&T"), "AT and T");
        assert_eq!(us("iPhone"), "i Phone");
        assert_eq!(us("well-known"), "well known");
        assert_eq!(us("user@example.com"), "user at example dot com");
        let c = Chunk {
            text: "FBI".into(),
            dot: false,
        };
        assert!(expand(&c, Lang::EnUs, Normalize::Natural)[0].spell);
        let c = Chunk {
            text: "NASA".into(),
            dot: false,
        };
        assert!(!expand(&c, Lang::EnUs, Normalize::Natural)[0].spell);
    }

    #[test]
    fn hindi() {
        assert_eq!(hi("१२३"), "एक सौ तेईस");
        assert_eq!(hi("150000"), "एक लाख पचास हजार");
        assert_eq!(hi("1,00,000"), "एक लाख");
        assert_eq!(hi("12345678"), "एक करोड़ तेईस लाख पैंतालीस हजार छह सौ अठहत्तर");
        assert_eq!(hi("₹500"), "पांच सौ रुपये");
        assert_eq!(hi("50%"), "पचास प्रतिशत");
        assert_eq!(hi("3.14"), "तीन दशमलव एक चार");
        assert_eq!(hi("10:30"), "दस बजकर तीस मिनट");
        assert_eq!(hi("डॉ."), "डॉक्टर");
    }

    #[test]
    fn off_mode_keeps_letters_only() {
        assert_eq!(ex("abc123", Lang::EnUs, Normalize::Off), "abc");
    }
}
