//! Normalise-only mode and word lists: invariants and behaviour.

use std::sync::Arc;
use uccharan::normalize_api::{
    normalize, options_from_json, render_with_wordlists, spoken_number, EnScale, LatinPolicy,
    NormOptions, Normalized, SpanKind, UrlPolicy,
};
use uccharan::profile::Profile;
use uccharan::respell::{self, Style};
use uccharan::wordlist::{select, Entry, MatchRule, Regex, WordList};
use uccharan::{Lang, Mode, Options, Phonemizer};

fn p() -> &'static Phonemizer {
    static P: std::sync::OnceLock<Phonemizer> = std::sync::OnceLock::new();
    P.get_or_init(Phonemizer::new)
}

fn norm(t: &str, l: Lang) -> Normalized {
    normalize(p(), t, l, &NormOptions::default())
}

/// The invariants every result must satisfy.
fn check(input: &str, n: &Normalized) {
    // Spans tile the input in order.
    let mut pos = 0;
    for s in &n.spans {
        assert_eq!(s.src.start, pos, "gap or overlap before {s:?} in {input:?}");
        assert!(s.src.end >= s.src.start);
        pos = s.src.end;
    }
    assert_eq!(pos, input.len(), "spans do not reach the end of {input:?}");
    // Outputs, in order, make up the text.
    let joined: String = n.spans.iter().map(|s| &n.text[s.out.clone()]).collect();
    assert_eq!(
        joined, n.text,
        "outputs do not make up the text for {input:?}"
    );
    // Punctuation and unchanged words are copied.
    for s in &n.spans {
        if matches!(s.kind, SpanKind::Text | SpanKind::Latin) {
            assert_eq!(&input[s.src.clone()], &n.text[s.out.clone()]);
        }
    }
    // Sentences lie inside the text, in order, without overlap.
    let mut end = 0;
    for r in &n.sentences {
        assert!(r.start >= end && r.end <= n.text.len() && r.start < r.end);
        end = r.end;
    }
}

const SAMPLES: &[(&str, Lang)] = &[
    (
        "Pay ₹500 by 10:30. Dr. Smith & co. visit www.example.com",
        Lang::EnUs,
    ),
    ("It cost ₹1,50,00,000. Call 555-123-4567!", Lang::EnUs),
    (
        "The FBI's 21st report (1990s): -5, 3.5% and 5km?",
        Lang::EnUs,
    ),
    (
        "Mail john@example.com or see https://github.com/rust/x.",
        Lang::EnUs,
    ),
    ("Born 12/25/2023 at 9:05am — the 3rd of 4.", Lang::EnUs),
    ("“Quoted” 1/2 cup… and 60 km/h", Lang::EnGb),
    ("मेरे पास 3 किताबें हैं। हम 10:30 बजे मिलेंगे।", Lang::Hi),
    (
        "₹5 करोड़ का सौदा। 1.5 लाख लोग। कॉल +91 98765 43210 करें।",
        Lang::Hi,
    ),
    (
        "ram@gmail.com पर लिखें। अंग्रेज़ी में 25 people आए। UPI से।",
        Lang::Hi,
    ),
    ("डॉ. शर्मा 15/08/2025 को 5वां आया, ५० प्रतिशत।", Lang::Hi),
    ("माझ्याकडे 3 पुस्तके आहेत. 50% आणि ₹200", Lang::Mr),
    ("আমার 25টি বই, ১২ জন।", Lang::Bn),
    ("எனக்கு 45 வயது", Lang::Ta),
    (
        "Ich habe 23 Euro, 1 500 € und 3,5 %. Dr. Müller zahlt $5.",
        Lang::De,
    ),
    ("Il a 71 ans et 12 500,50 € — 1 280 hiboux.", Lang::FrBe),
    ("Tengo 1 500 libros y 12,50 €.", Lang::Es),
    ("Jeg har 95 kroner. Łódź!", Lang::Da),
    ("", Lang::EnUs),
    ("   ", Lang::Hi),
    ("...!?", Lang::EnUs),
    ("a\n\nb", Lang::EnUs),
];

#[test]
fn spans_tile_input_and_rebuild_output() {
    for (t, l) in SAMPLES {
        check(t, &norm(t, *l));
    }
}

#[test]
fn normalising_twice_changes_nothing() {
    for (t, l) in SAMPLES {
        let once = norm(t, *l).text;
        let twice = norm(&once, *l).text;
        assert_eq!(once, twice, "not idempotent for {t:?}");
    }
}

#[test]
fn fuzzed_input_keeps_invariants() {
    let pieces = [
        "a",
        "Hello",
        "₹",
        "$",
        "5",
        "12",
        "1,00,000",
        "3.14",
        ":",
        "/",
        "-",
        ".",
        ",",
        "!",
        " ",
        "  ",
        "\n",
        "राम",
        "१२",
        "%",
        "@",
        "x.com",
        "(",
        ")",
        "\"",
        "Dr.",
        "10:30",
        "क़",
        "UPI",
        "e.g.",
        "+91",
        "—",
        "…",
        "২৫",
        "ஐந்து",
        "\u{200d}",
        "km",
        "1st",
    ];
    let mut seed: u64 = 0x9e37_79b9_7f4a_7c15;
    for _ in 0..400 {
        let mut s = String::new();
        for _ in 0..(seed % 9 + 1) {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            s.push_str(pieces[(seed >> 33) as usize % pieces.len()]);
            if seed & 1 == 0 {
                s.push(' ');
            }
        }
        for l in [Lang::EnUs, Lang::Hi, Lang::Ta, Lang::De, Lang::FrCh] {
            let n = norm(&s, l);
            check(&s, &n);
            let again = norm(&n.text, l);
            check(&n.text, &again);
        }
    }
}

#[test]
fn numbers_read_back_to_their_value() {
    let mut checked = 0;
    let en = [
        "7",
        "21",
        "101",
        "999",
        "1000",
        "12000",
        "45000",
        "123456",
        "1,234,567",
        "2,500,000",
        "1,00,000",
        "5,00,000",
        "1,50,00,000",
        "987654321",
    ];
    for (texts, lang) in [(&en[..], Lang::EnUs), (&en[..], Lang::Hi)] {
        for t in texts {
            let input = format!("count {t} items");
            let n = norm(&input, lang);
            for s in n.spans.iter().filter(|s| s.kind == SpanKind::Number) {
                let want: u64 = s.value.as_deref().unwrap().parse().unwrap();
                let got = spoken_number(&n.text[s.out.clone()], s.lang);
                assert_eq!(
                    got,
                    Some(want),
                    "{lang:?} {t}: {:?}",
                    &n.text[s.out.clone()]
                );
                checked += 1;
            }
        }
    }
    // Every number up to 10^7 read in Hindi and English, sampled.
    for v in (0..10_000_000u64)
        .step_by(9_973)
        .chain([0, 1, 100, 1000, 100_000, 10_000_000])
    {
        for lang in [Lang::EnUs, Lang::Hi] {
            let n = norm(&v.to_string(), lang);
            if let Some(s) = n.spans.iter().find(|s| s.kind == SpanKind::Number) {
                assert_eq!(
                    spoken_number(&n.text[s.out.clone()], lang),
                    Some(v),
                    "{lang:?} {v}"
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 1500);
}

#[test]
fn span_kinds() {
    let kinds = |t: &str, l: Lang| -> Vec<(String, SpanKind)> {
        let n = norm(t, l);
        n.spans
            .iter()
            .filter(|s| s.kind != SpanKind::Punct)
            .map(|s| (t[s.src.clone()].to_string(), s.kind))
            .collect()
    };
    let k = kinds(
        "Pay ₹500 on 2024-01-05 at 3:30, call 555-123-4567, 21st, 3.5, 50%, Dr. FBI john@x.com www.a.org & 5km 10-20 3/4 1984",
        Lang::EnUs,
    );
    let find = |w: &str| k.iter().find(|x| x.0 == w).map(|x| x.1);
    assert_eq!(find("₹500"), Some(SpanKind::Money));
    assert_eq!(find("2024-01-05"), Some(SpanKind::Date));
    assert_eq!(find("3:30"), Some(SpanKind::Time));
    assert_eq!(find("555-123-4567"), Some(SpanKind::Phone));
    assert_eq!(find("21st"), Some(SpanKind::Ordinal));
    assert_eq!(find("3.5"), Some(SpanKind::Decimal));
    assert_eq!(find("50%"), Some(SpanKind::Percent));
    assert_eq!(find("Dr."), Some(SpanKind::Abbreviation));
    assert_eq!(find("FBI"), Some(SpanKind::Acronym));
    assert_eq!(find("john@x.com"), Some(SpanKind::Email));
    assert_eq!(find("www.a.org"), Some(SpanKind::Url));
    assert_eq!(find("&"), Some(SpanKind::Symbol));
    assert_eq!(find("5km"), Some(SpanKind::Measure));
    assert_eq!(find("10-20"), Some(SpanKind::Range));
    assert_eq!(find("3/4"), Some(SpanKind::Fraction));
    assert_eq!(find("1984"), Some(SpanKind::Year));
    assert_eq!(find("Pay"), Some(SpanKind::Text));
    let k = kinds("मेरा office है", Lang::Hi);
    assert_eq!(k[1], ("office".into(), SpanKind::Latin));
}

#[test]
fn options() {
    let o = |j: &str| options_from_json(j).unwrap();
    let t = |s: &str, l: Lang, j: &str| normalize(p(), s, l, &o(j)).text;
    assert_eq!(t("5,00,000 people", Lang::EnUs, ""), "five lakh people");
    assert_eq!(
        t("500,000 people", Lang::EnUs, ""),
        "five hundred thousand people"
    );
    assert_eq!(
        t("500000 people", Lang::EnUs, r#"{"en_scale":"indian"}"#),
        "five lakh people"
    );
    assert_eq!(
        t(
            "5,00,000 people",
            Lang::EnUs,
            r#"{"en_scale":"international"}"#
        ),
        "five hundred thousand people"
    );
    assert_eq!(t("कुल 12 students", Lang::Hi, ""), "कुल twelve students");
    assert_eq!(
        t("कुल 12 students", Lang::Hi, r#"{"digits":"request"}"#),
        "कुल बारह students"
    );
    assert_eq!(t("कुल 12", Lang::Hi, r#"{"digits":"en-us"}"#), "कुल twelve");
    assert_eq!(
        t("UPI से", Lang::Hi, r#"{"latin":"transliterate"}"#),
        "यू पी आई से"
    );
    assert_eq!(t("UPI से", Lang::Hi, r#"{"latin":"spell"}"#), "यू पी आई से");
    assert_eq!(
        t("see www.a.com now", Lang::EnUs, r#"{"urls":"keep"}"#),
        "see www.a.com now"
    );
    assert_eq!(
        t("see www.a.com now", Lang::EnUs, r#"{"urls":"drop"}"#),
        "see now"
    );
    assert!(options_from_json(r#"{"latin":"nope"}"#).is_err());
    let d = NormOptions {
        mode: Mode::Improved,
        en_scale: EnScale::Indian,
        latin: LatinPolicy::Keep,
        urls: UrlPolicy::Read,
        ..NormOptions::default()
    };
    assert_eq!(normalize(p(), "iPhone", Lang::EnUs, &d).text, "iPhone");
}

#[test]
fn offsets_in_json() {
    let input = "₹5 😀 x";
    let n = norm(input, Lang::EnUs);
    let j = n.to_json(input);
    let v = uccharan::minijson::parse(&j).unwrap();
    let uccharan::minijson::Value::Arr(spans) = v.get("spans").unwrap() else {
        panic!()
    };
    let last = spans.last().unwrap();
    let src = last.get("src").unwrap();
    let num = |k: &str| match src.get(k) {
        Some(uccharan::minijson::Value::Arr(a)) => match a[0] {
            uccharan::minijson::Value::Num(x) => x as usize,
            _ => panic!(),
        },
        _ => panic!(),
    };
    // "x" starts at byte 10, code point 5, UTF-16 unit 6.
    assert_eq!((num("byte"), num("char"), num("utf16")), (10, 5, 6));
}

#[test]
fn sentences() {
    let n = norm("One. Two? Dr. Who is 3.5 tall! Four", Lang::EnUs);
    let s: Vec<&str> = n.sentences.iter().map(|r| &n.text[r.clone()]).collect();
    assert_eq!(
        s,
        [
            "One.",
            "Two?",
            "Doctor Who is three point five tall!",
            "Four"
        ]
    );
    let n = norm("राम आया। श्याम 5 बजे गया॥", Lang::Hi);
    assert_eq!(n.sentences.len(), 2);
}

// ---------------------------------------------------------------------------
// Word lists

const TSV: &str = "# uccharan wordlist 1\n# lang: hi\n# alphabet: ipa\nword\tlang\tipa\tsay\tmatch\tnote\nKokoro\ten\tkəkˈɔːɹoʊ\t\t\tproduct name\nSBI\thi\t\tएस बी आई\tcase\t\nIshaan\thi\tiːʃaːn\tईशान\t\t\nUPI\t*\t\tयू पी आई\tcase\tall Indic voices\n2G\ten\t\ttwo G\texact\t\nNew York\ten\tnuː jˈɔːk\t\t\t\nMH\\d+\t*\t\tmaharashtra\tregex\t\n";

#[test]
fn tsv_and_pls_round_trip() {
    let w = WordList::parse_tsv(TSV).unwrap();
    assert_eq!(w.len(), 7);
    assert_eq!(w.entries[1].rule, MatchRule::Case);
    assert_eq!(w.entries[2].lang, Some(Lang::Hi));
    assert_eq!(w.entries[3].lang, None);
    // TSV -> TSV is lossless.
    let again = WordList::parse_tsv(&w.to_tsv()).unwrap();
    assert_eq!(again.to_tsv(), w.to_tsv());
    // TSV -> PLS (one file per language) -> TSV keeps every supported field.
    let mut back: Vec<Entry> = Vec::new();
    for l in w.languages() {
        let pls = w.to_pls(l);
        assert!(pls.contains("<lexicon"));
        for e in WordList::parse_pls(&pls).unwrap().entries {
            if !back.iter().any(|b| b.word == e.word && b.lang == e.lang) {
                back.push(e);
            }
        }
    }
    for e in w.entries.iter().filter(|e| e.rule != MatchRule::Regex) {
        let b = back
            .iter()
            .find(|b| b.word == e.word && b.lang == e.lang)
            .unwrap_or_else(|| panic!("{} lost", e.word));
        assert_eq!(
            (&b.ipa, &b.say, b.rule, &b.note),
            (&e.ipa, &e.say, e.rule, &e.note)
        );
    }
    // JSON entries parse back too.
    let j = WordList::parse_any(&w.to_json()).unwrap();
    assert_eq!(j.len(), w.len());
}

#[test]
fn pls_import() {
    let pls = r#"<?xml version="1.0" encoding="UTF-8"?>
<lexicon version="1.0" xmlns="http://www.w3.org/2005/01/pronunciation-lexicon"
      alphabet="ipa" xml:lang="en-US">
  <lexeme><grapheme>W3C</grapheme><alias>World Wide Web Consortium</alias></lexeme>
  <lexeme>
    <grapheme>tomato</grapheme><grapheme>Tomato</grapheme>
    <phoneme>təmeɪtoʊ</phoneme><phoneme prefer="true">təmɑːtoʊ</phoneme>
  </lexeme>
  <!-- comment -->
  <lexeme><grapheme>A&amp;B</grapheme><alias>A and B</alias></lexeme>
</lexicon>"#;
    let w = WordList::parse_pls(pls).unwrap();
    assert_eq!(w.len(), 4);
    assert_eq!(
        w.entries[0].say.as_deref(),
        Some("World Wide Web Consortium")
    );
    assert_eq!(w.entries[1].ipa.as_deref(), Some("təmɑːtoʊ"));
    assert_eq!(w.entries[3].word, "A&B");
    assert_eq!(w.entries[1].lang, Some(Lang::EnUs));
    assert!(WordList::parse_pls("<lexicon alphabet=\"x-sampa\"></lexicon>").is_err());
}

#[test]
fn parse_errors_carry_line_numbers() {
    let e = WordList::parse_tsv("word\tlang\nfoo\txx-yy\n").unwrap_err();
    assert_eq!(e.line, 2);
    let e = WordList::parse_tsv("ok\nbad(\t\t\t\tregex\n").unwrap_err();
    assert_eq!(e.line, 2);
}

#[test]
fn matching_rules() {
    let w = WordList::parse_tsv(TSV).unwrap();
    let hits = |t: &str, l: Lang| -> Vec<String> {
        select(&[&w], t, l)
            .iter()
            .map(|h| t[h.start..h.end].to_string())
            .collect()
    };
    // case-insensitive whole words; case-sensitive "case"; substring "exact".
    assert_eq!(
        hits("kokoro KOKORO kokoros", Lang::EnUs),
        ["kokoro", "KOKORO"]
    );
    assert_eq!(hits("sbi SBI", Lang::Hi), ["SBI"]);
    assert_eq!(hits("4G/2G2", Lang::EnUs), ["2G"]);
    // Phrases match across any whitespace, longest first.
    assert_eq!(hits("in New   York now", Lang::EnUs), ["New   York"]);
    // Regex rows.
    assert_eq!(hits("MH12 AB", Lang::Hi), ["MH12"]);
    // Danda and Latin inside Devanagari are word boundaries.
    assert_eq!(hits("मैंSBI।", Lang::Hi), ["SBI"]);
    // An English entry applies to Latin words in Hindi text.
    assert_eq!(hits("मेरा Kokoro", Lang::Hi), ["Kokoro"]);
    // Nukta: precomposed and decomposed letters match each other.
    let n = WordList::parse_tsv("ज़रा\thi\t\tजरा\n").unwrap();
    assert_eq!(select(&[&n], "\u{095b}रा", Lang::Hi).len(), 1);
}

#[test]
fn priority_between_lists() {
    let inline = WordList::from_entries(vec![Entry::new(
        "York",
        None,
        Some("jɔːk"),
        None,
        MatchRule::Word,
    )
    .unwrap()]);
    let list = WordList::parse_tsv(TSV).unwrap();
    // The first list wins over a later list's longer phrase.
    let h = select(&[&inline, &list], "New York", Lang::EnUs);
    assert_eq!(h.len(), 1);
    assert_eq!((h[0].list, h[0].start), (0, 4));
    // Within one list the longest match wins.
    let h = select(&[&list], "New York", Lang::EnUs);
    assert_eq!((h[0].start, h[0].end), (0, 8));
    // An entry for the request language beats one for any language.
    let both = WordList::parse_tsv("UPI\t*\t\tany\nUPI\thi\t\thindi\n").unwrap();
    let h = select(&[&both], "UPI", Lang::Hi);
    assert_eq!(both.entries[h[0].entry].say.as_deref(), Some("hindi"));
    // An entry wins over number reading.
    let o = NormOptions {
        wordlists: vec![Arc::new(list.clone())],
        ..NormOptions::default()
    };
    let n = normalize(p(), "on 2G now", Lang::EnUs, &o);
    assert_eq!(n.text, "on two G now");
    assert!(n.spans.iter().any(|s| s.kind == SpanKind::WordList));
}

#[test]
fn regex_engine() {
    let r = Regex::new(r"(?i)\bmh\s?\d{1,2}\b").unwrap();
    assert!(r.is_match("car MH 12 here"));
    assert!(r.is_match("mh4"));
    assert!(!r.is_match("xmh4"));
    let r = Regex::new("a(b|cd)*e").unwrap();
    assert_eq!(r.find_iter("xxacdbcde ae"), [(2, 9), (10, 12)]);
    let r = Regex::new("[^0-9]+").unwrap();
    assert_eq!(r.find_iter("ab12c"), [(0, 2), (4, 5)]);
    // Pathological patterns give up instead of hanging.
    let r = Regex::new("(a*)*b").unwrap();
    let _ = r.find_iter(&"a".repeat(64));
    assert!(Regex::new("(ab").is_err());
}

#[test]
fn phoneme_voices_get_the_ipa() {
    let list = Arc::new(WordList::parse_tsv(TSV).unwrap());
    let opt = Options::new(Lang::EnUs);
    let out = render_with_wordlists(
        p(),
        "Kokoro in New York.",
        &opt,
        Profile::EspeakIpa,
        true,
        std::slice::from_ref(&list),
    );
    assert!(out.starts_with("kəkˈɔːɹoʊ"), "{out}");
    assert!(out.contains("nuːjˈɔːk"), "{out}");
    // In the Kokoro phone set.
    let k = render_with_wordlists(
        p(),
        "Kokoro",
        &opt,
        Profile::Kokoro,
        true,
        std::slice::from_ref(&list),
    );
    assert!(k.starts_with("kəkˈɔɹO"), "{k}");
    // An entry with only a spoken form is phonemised from it.
    let hi = Options::new(Lang::Hi);
    let out = render_with_wordlists(p(), "SBI", &hi, Profile::EspeakIpa, true, &[list]);
    assert_eq!(out, p().phonemize("एस बी आई", &hi));
}

#[test]
fn text_voices_get_the_spoken_form() {
    let list = Arc::new(WordList::parse_tsv(TSV).unwrap());
    let mut o = NormOptions {
        wordlists: vec![list],
        ..NormOptions::default()
    };
    assert_eq!(
        normalize(p(), "SBI में Ishaan", Lang::Hi, &o).text,
        "एस बी आई में ईशान"
    );
    // An English entry with IPA only: respelled.
    o.latin = LatinPolicy::Transliterate;
    let t = normalize(p(), "Kokoro", Lang::Hi, &o).text;
    assert_eq!(t, "ककॉरो");
}

/// Respellings read back (by uccharan) within 10 % phoneme error of the
/// entry's IPA for sounds the target script can write.
#[test]
fn respelling_round_trip() {
    let cases: &[(&str, Lang, Lang)] = &[
        ("kəkˈɔːɹoʊ", Lang::EnUs, Lang::Hi),
        ("ɪʃˈɑːn", Lang::Hi, Lang::Hi),
        ("pɾijəŋkaː", Lang::Hi, Lang::Hi),
        ("mˈaːɾkeːʈ", Lang::Hi, Lang::Hi),
        ("ɖeːʋɪd", Lang::Hi, Lang::Hi),
        ("kəmpjˈuːtəɹ", Lang::EnUs, Lang::Hi),
        ("ɑːfɪs", Lang::EnUs, Lang::Mr),
        ("kəkˈɔːɹoʊ", Lang::EnUs, Lang::Gu),
        ("kəkˈɔːɹoʊ", Lang::EnUs, Lang::Te),
        ("kəkˈɔːɹoʊ", Lang::EnUs, Lang::Kn),
        ("kəkˈɔːɹoʊ", Lang::EnUs, Lang::EnUs),
        ("ɪʃˈɑːn", Lang::Hi, Lang::EnUs),
    ];
    for (ipa, src, to) in cases {
        let r = respell::respell(p(), ipa, Some(*src), *to, Style::Native)
            .unwrap_or_else(|| panic!("no respelling for {to:?}"));
        assert!(
            r.distance <= 0.1,
            "{ipa} -> {to:?}: {:?} reads {:?} ({})",
            r.text,
            r.readback,
            r.distance
        );
    }
    // Bengali has no schwa: the inherent vowel is the nearest it can write.
    let r = respell::respell(p(), "kəkˈɔːɹoʊ", Some(Lang::EnUs), Lang::Bn, Style::Native).unwrap();
    assert!(r.distance <= 0.34, "{r:?}");
    let r = respell::respell(
        p(),
        "kəkˈɔːɹoʊ",
        Some(Lang::EnUs),
        Lang::EnUs,
        Style::LatinHyphen,
    )
    .unwrap();
    assert!(
        r.text.contains('-') && r.text.chars().any(char::is_uppercase),
        "{}",
        r.text
    );
    assert!(respell::respell(p(), "", None, Lang::Hi, Style::Native).is_none());
}

/// The request language of a Latin-script text reads its numbers, with
/// the number words of its language file in their written form.
#[test]
fn latin_languages_read_their_own_numbers() {
    let cases: &[(Lang, &str, &str)] = &[
        (
            Lang::De,
            "Ich habe 23 Euro.",
            "Ich habe dreiundzwanzig Euro.",
        ),
        (
            Lang::Es,
            "Tengo 1 500 libros.",
            "Tengo uno quinientos libros.",
        ),
        (Lang::Fr, "Il a 71 ans.", "Il a soixante et onze ans."),
        (Lang::FrBe, "Il a 71 ans.", "Il a septante et un ans."),
        (Lang::FrCh, "Il a 71 ans.", "Il a septante et un ans."),
        (
            Lang::Da,
            "Jeg har 95 kroner.",
            "Jeg har femoghalvfems kroner.",
        ),
        (
            Lang::Nl,
            "Ik heb 23 boeken.",
            "Ik heb drieëntwintig boeken.",
        ),
        (Lang::It, "Ho 21 anni.", "Ho ventuno anni."),
        (Lang::PtBr, "Tenho 23 anos.", "Tenho vinte e três anos."),
        (Lang::Pt, "Custa 101 euros.", "Custa cento e um euros."),
        (Lang::Pl, "Mam 25 lat.", "Mam dwadzieścia pięć lat."),
        (Lang::Tr, "Ben 23 yaşındayım.", "Ben yirmi üç yaşındayım."),
        (Lang::Cs, "Je mi 32 let.", "Je mi třicet dva let."),
        (Lang::Sv, "Jag har 47 böcker.", "Jag har fyrtiosju böcker."),
        (Lang::Ca, "Tinc 22 anys.", "Tinc vint-i-dos anys."),
        (Lang::Ro, "Am 22 de ani.", "Am douăzeci și doi de ani."),
        (Lang::Hu, "Most 23 éves.", "Most huszonhárom éves."),
        (
            Lang::Fi,
            "Hinta on 250 000 euroa.",
            "Hinta on kaksisataa viisikymmentä tuhatta euroa.",
        ),
        (
            Lang::Nb,
            "Jeg har 1 200 kroner.",
            "Jeg har ett tusen to hundre kroner.",
        ),
    ];
    for (l, t, want) in cases {
        let n = norm(t, *l);
        check(t, &n);
        assert_eq!(n.text, *want, "{l:?}");
        let s = n.spans.iter().find(|s| s.kind == SpanKind::Number).unwrap();
        assert_eq!(s.lang, *l);
    }
    // `digits: request` and a fixed language give the same reading.
    let o = options_from_json(r#"{"digits":"request"}"#).unwrap();
    assert_eq!(
        normalize(p(), "Ich habe 23 Euro.", Lang::De, &o).text,
        "Ich habe dreiundzwanzig Euro."
    );
    // Abbreviations of the language stay as written (not English "Doctor").
    assert_eq!(norm("Dr. Müller", Lang::De).text, "Dr. Müller");
    // Hyphens and apostrophes inside words are kept.
    let t = "L'homme arrive aujourd'hui, c'est-à-dire à 3 h.";
    assert_eq!(
        norm(t, Lang::Fr).text,
        "L'homme arrive aujourd'hui, c'est-à-dire à trois h."
    );
}

/// Numbers written in groups of three digits, decimals, percentages and
/// currency signs, as the phonemiser reads them in each mode.
#[test]
fn latin_number_forms() {
    let improved = NormOptions {
        mode: Mode::Improved,
        ..NormOptions::default()
    };
    let both = |t: &str, l: Lang| (norm(t, l).text, normalize(p(), t, l, &improved).text);
    // Compatible mode joins groups where the reference voice does (German,
    // French: across a plain space); improved mode also in Spanish.
    assert_eq!(both("1 500", Lang::Es).0, "uno quinientos");
    assert_eq!(both("1 500", Lang::Es).1, "mil quinientos");
    assert_eq!(both("1 500", Lang::De).0, "eintausend fünfhundert");
    assert_eq!(both("1 280", Lang::Fr).1, "mille deux cent quatre-vingts");
    let n = norm("Il y a 12 500 personnes.", Lang::Fr);
    let s = n.spans.iter().find(|s| s.kind == SpanKind::Number).unwrap();
    assert_eq!(s.value.as_deref(), Some("12500"));
    assert_eq!(&"Il y a 12 500 personnes."[s.src.clone()], "12 500");
    // Decimal comma, percent, currency.
    assert_eq!(norm("3,5 %", Lang::De).text, "drei Komma fünf Prozent");
    assert_eq!(norm("12,50 €", Lang::Es).text, "doce euro cincuenta");
    assert_eq!(
        both("€5", Lang::De),
        ("Euro fünf".into(), "fünf Euro".into())
    );
    let kinds = |t: &str, l: Lang| -> Vec<SpanKind> {
        norm(t, l)
            .spans
            .iter()
            .filter(|s| s.kind != SpanKind::Punct)
            .map(|s| s.kind)
            .collect()
    };
    assert_eq!(
        kinds("3,5 50% 1.000 €5", Lang::De),
        [
            SpanKind::Decimal,
            SpanKind::Percent,
            SpanKind::Number,
            SpanKind::Money
        ]
    );
    // Written number words of each language.
    let w = |t: &str, l: Lang| norm(t, l).text;
    assert_eq!(w("2023", Lang::De), "zweitausend dreiundzwanzig");
    assert_eq!(w("345", Lang::De), "dreihundertfünfundvierzig");
    assert_eq!(w("1000000", Lang::De), "eine Million");
    assert_eq!(w("5000000", Lang::Fr), "cinq millions");
    assert_eq!(w("222", Lang::Fr), "deux cent vingt-deux");
    assert_eq!(w("99", Lang::FrCh), "nonante-neuf");
    assert_eq!(w("80", Lang::FrCh), "huitante");
    assert_eq!(w("2003", Lang::It), "duemilatré");
    assert_eq!(
        w("123456", Lang::It),
        "centoventitremilaquattrocentocinquantasei"
    );
    assert_eq!(w("35", Lang::Es), "treinta y cinco");
    assert_eq!(w("2001", Lang::Pt), "dois mil e um");
    assert_eq!(w("1100", Lang::Ro), "o mie o sută");
    assert_eq!(w("66", Lang::Ro), "șaizeci și șase");
    assert_eq!(w("5000000", Lang::Pl), "pięć milionów");
    assert_eq!(w("1000", Lang::Sv), "ett tusen");
    assert_eq!(w("101", Lang::Da), "et hundrede og et");
    assert_eq!(w("123", Lang::Nl), "honderddrieëntwintig");
}

/// Phonemising the normalised text says what phonemising the digits says.
/// The voices read some number words differently from the same word
/// written in letters (Danish "6" sεɡs, "seks" sʔeks; Turkish "12" onici,
/// "on iki" ɔn ikɪ; German "14" fɪɾtseːn, "vierzehn" fiːɾtseːn), so the
/// readings are compared with stress, length and the glottal stop left out,
/// and each language has its own bound on the share of phonemes that
/// differ (measured, with a margin).
#[test]
fn latin_numbers_round_trip() {
    fn strip(s: &str) -> Vec<char> {
        s.chars()
            .filter(|c| !matches!(c, 'ˈ' | 'ˌ' | 'ː' | 'ʔ' | ' ' | '-' | '.' | ','))
            .collect()
    }
    fn dist(a: &[char], b: &[char]) -> usize {
        let mut prev: Vec<usize> = (0..=b.len()).collect();
        for i in 1..=a.len() {
            let mut cur = vec![i; b.len() + 1];
            for j in 1..=b.len() {
                cur[j] = (prev[j] + 1)
                    .min(cur[j - 1] + 1)
                    .min(prev[j - 1] + usize::from(a[i - 1] != b[j - 1]));
            }
            prev = cur;
        }
        prev[b.len()]
    }
    let mut nums: Vec<String> = (0..=130).map(|n| n.to_string()).collect();
    for n in [
        150, 200, 222, 345, 999, 1000, 1001, 1100, 1234, 1999, 2001, 2023, 12345, 99999, 123456,
        1000000, 2500000, 7654321,
    ] {
        nums.push(n.to_string());
    }
    for s in ["3,5", "0,25", "50%", "1 500", "12 500"] {
        nums.push(s.to_string());
    }
    // Largest share of differing phonemes allowed, in per cent.
    let bounds: &[(Lang, f64)] = &[
        (Lang::Es, 1.5),
        (Lang::De, 6.5),
        (Lang::Nl, 7.0),
        (Lang::Fr, 3.5),
        (Lang::FrBe, 2.5),
        (Lang::FrCh, 1.5),
        (Lang::It, 8.0),
        (Lang::PtBr, 4.5),
        (Lang::Pt, 11.5),
        (Lang::Pl, 3.5),
        (Lang::Tr, 28.0),
        (Lang::Cs, 4.0),
        (Lang::Sv, 21.0),
        (Lang::Ca, 4.0),
        (Lang::Ro, 7.0),
        (Lang::Hu, 2.0),
        (Lang::Fi, 5.5),
        (Lang::Nb, 12.0),
        (Lang::Da, 32.0),
    ];
    for &(l, bound) in bounds {
        for mode in [Mode::Compatible, Mode::Improved] {
            let mut o = Options::new(l);
            o.mode = mode;
            let no = NormOptions {
                mode,
                ..NormOptions::default()
            };
            let (mut wrong, mut total) = (0usize, 0usize);
            for t in &nums {
                let spoken = normalize(p(), t, l, &no).text;
                assert!(
                    !spoken.chars().any(|c| c.is_ascii_digit()),
                    "{l:?} {t}: {spoken}"
                );
                let a = strip(&p().phonemize(t, &o));
                let b = strip(&p().phonemize(&spoken, &o));
                wrong += dist(&a, &b);
                total += a.len();
            }
            let per = 100.0 * wrong as f64 / total as f64;
            assert!(per <= bound, "{l:?} {mode:?}: {per:.1} % > {bound} %");
        }
    }
}

/// Every non-Latin language writes its numbers in its own script: no
/// digits, no letters of another block, and words that read back to
/// (nearly) the phonemes of the digits. Sinhala and Sindhi number tables
/// hold keys of the word table, not words: these must not leak out.
#[test]
fn indic_numbers_in_their_own_script() {
    use uccharan::text::script_of;
    fn strip(s: &str) -> Vec<char> {
        s.chars()
            .filter(|c| !matches!(c, 'ˈ' | 'ˌ' | 'ː' | 'ʔ' | ' ' | '-' | '.' | ','))
            .collect()
    }
    fn dist(a: &[char], b: &[char]) -> usize {
        let mut prev: Vec<usize> = (0..=b.len()).collect();
        for i in 1..=a.len() {
            let mut cur = vec![i; b.len() + 1];
            for j in 1..=b.len() {
                cur[j] = (prev[j] + 1)
                    .min(cur[j - 1] + 1)
                    .min(prev[j - 1] + usize::from(a[i - 1] != b[j - 1]));
            }
            prev = cur;
        }
        prev[b.len()]
    }
    let twenty_five: &[(Lang, &str)] = &[
        (Lang::Hi, "पच्चीस"),
        (Lang::Bn, "পঁচিশ"),
        (Lang::Mr, "पंचवीस"),
        (Lang::Te, "ఇరవై ఐదు"),
        (Lang::Ne, "पच्चीस"),
        (Lang::Kn, "ಇಪ್ಪತ್ತೈದು"),
        (Lang::Ta, "இருபத்து ஐந்து"),
        (Lang::Ml, "ഇരുപത്തിയഞ്ച്"),
        (Lang::As, "পঁচিশ"),
        (Lang::Gu, "પચ્ચીસ"),
        (Lang::Pa, "ਪੱਚੀ"),
        (Lang::Or, "ପଚିଶ"),
        (Lang::Kok, "पंचवीस"),
        (Lang::Si, "විසිපහ"),
        (Lang::Sa, "पच्चीस"),
        (Lang::Ur, "پچیس"),
        (Lang::Sd, "پنجويهه"),
        (Lang::Bho, "पच्चीस"),
        (Lang::Mai, "पच्चीस"),
        (Lang::Mag, "पच्चीस"),
    ];
    for (l, want) in twenty_five {
        assert_eq!(norm("25", *l).text, *want, "{l:?}");
    }
    let cases: &[(Lang, &str, &str)] = &[
        (Lang::Si, "1,234", "එක්දහස් දෙසිය තිස්හතර"),
        (
            Lang::Si,
            "100 1000 200 2000 300",
            "සියය දහස දෙසියය දෙදහස තුන්සියය",
        ),
        (
            Lang::Si,
            "120 1100 2500",
            "එකසිය විස්ස එක්දහස් එකසියය දෙදහස් පන්සියය",
        ),
        (Lang::Si, "50%", "පනහ ප්\u{200d}රතිශතය"),
        (Lang::Si, "3.5", "තුන දශම පහ"),
        (Lang::Si, "₹500", "පන්සියය රුපියල්"),
        (Lang::Si, "6 7 16 70 99", "හය හත දහසය හැත්තෑව අනූනවය"),
        (Lang::Sd, "1,234", "هڪ هزار ٻه سؤ چوٽيهه"),
        (Lang::Sd, "3.5", "ٽي ڏهائي پنج"),
        (Lang::Sd, "₹500", "پنج سؤ رپيا"),
        (Lang::Sd, "50%", "پنجاهه فيصد"),
        (Lang::Sd, "0 9 20 60 99", "ٻڙي نو ويهه سٺ نوانوي"),
        (Lang::Ur, "₹500", "پانچ سو روپے"),
        (Lang::Kok, "3.5", "तीन दशमलव पांच"),
        (
            Lang::Kok,
            "6 9 10 11 19 90 99",
            "स णव धा इकरा एकुणीस णव्वद णव्याण्णव",
        ),
        (
            Lang::Kok,
            "100 101 300 1000 125000",
            "शंबर एकशे एक तिनशे एक हजार एक लाख पंचवीस हजार",
        ),
    ];
    for (l, t, want) in cases {
        let n = norm(t, *l);
        check(t, &n);
        assert_eq!(n.text, *want, "{l:?} {t}");
    }
    // Sanskrit: Hindi number words in compatible mode (as the reference
    // reads digits), Sanskrit ones in improved mode.
    let improved = NormOptions {
        mode: Mode::Improved,
        ..NormOptions::default()
    };
    assert_eq!(
        normalize(p(), "25 3.5", Lang::Sa, &improved).text,
        "पञ्चविंशतिः त्रीणि दशमलव पञ्च"
    );
    let mut nums: Vec<String> = (0..=110).map(|n| n.to_string()).collect();
    for n in [
        150, 200, 222, 345, 999, 1000, 1234, 2024, 12345, 99999, 123456,
    ] {
        nums.push(n.to_string());
    }
    for s in ["1,234", "3.5", "₹500", "15/08/2024", "50%"] {
        nums.push(s.to_string());
    }
    // Largest share of differing phonemes allowed, in per cent, in
    // compatible and improved mode. The words are read by the letter
    // rules, the digits by the reference's number words: Perso-Arabic
    // script without vowel marks (ur, sd) and the Odia reference differ
    // most. Compatible Nepali reads digits in English and compatible
    // Konkani names its vowel signs, as the reference does: not checked.
    let bounds: &[(Lang, Option<f64>, f64)] = &[
        (Lang::Hi, Some(11.5), 1.0),
        (Lang::Bn, Some(8.5), 8.5),
        (Lang::Mr, Some(9.5), 9.5),
        (Lang::Te, Some(12.0), 11.0),
        (Lang::Ne, None, 6.0),
        (Lang::Kn, Some(11.5), 11.5),
        (Lang::Ta, Some(12.5), 12.5),
        (Lang::Ml, Some(10.0), 10.0),
        (Lang::As, Some(1.5), 1.5),
        (Lang::Gu, Some(3.0), 3.0),
        (Lang::Pa, Some(18.5), 18.5),
        (Lang::Or, Some(43.0), 43.0),
        (Lang::Kok, None, 1.0),
        (Lang::Si, Some(18.0), 1.0),
        (Lang::Sa, Some(11.5), 1.0),
        (Lang::Ur, Some(33.0), 33.0),
        (Lang::Sd, Some(45.5), 45.5),
        (Lang::Bho, Some(13.0), 1.0),
        (Lang::Mai, Some(12.5), 1.0),
        (Lang::Mag, Some(11.5), 1.0),
    ];
    for &(l, compat_bound, improved_bound) in bounds {
        for (mode, bound) in [
            (Mode::Compatible, compat_bound),
            (Mode::Improved, Some(improved_bound)),
        ] {
            let mut o = Options::new(l);
            o.mode = mode;
            let no = NormOptions {
                mode,
                ..NormOptions::default()
            };
            let (mut wrong, mut total) = (0usize, 0usize);
            for t in &nums {
                let n = normalize(p(), t, l, &no);
                check(t, &n);
                let spoken = n.text;
                for c in spoken.chars() {
                    let s = script_of(c);
                    assert!(
                        s == l.script() || matches!(c, ' ' | '\u{200c}' | '\u{200d}'),
                        "{l:?} {mode:?} {t}: {spoken:?} has {c:?} ({s:?})"
                    );
                }
                let a = strip(&p().phonemize(t, &o));
                let b = strip(&p().phonemize(&spoken, &o));
                wrong += dist(&a, &b);
                total += a.len();
            }
            let per = 100.0 * wrong as f64 / total as f64;
            if let Some(bound) = bound {
                assert!(per <= bound, "{l:?} {mode:?}: {per:.1} % > {bound} %");
            }
        }
    }
}
