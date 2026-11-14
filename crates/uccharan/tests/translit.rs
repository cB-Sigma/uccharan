//! Romanised text into native script, against the tiny pack
//! `tests/data/xlit-toy.ucd` (built by `lexgen xlit-toy` from
//! `data/sources/translit/toy`).

use uccharan::normalize_api::NormOptions;
use uccharan::translit::{
    candidates, normalize_romanised, transliterate, transliterate_json, EnglishWords,
    TranslitOptions, WordKind,
};
use uccharan::{Lang, Phonemizer};

const TOY: &[u8] = include_bytes!("data/xlit-toy.ucd");

fn with_pack() -> Phonemizer {
    let p = Phonemizer::new();
    assert_eq!(p.load_pack(TOY.to_vec()).as_deref(), Ok("xlit-hi"));
    assert!(p.has_translit(Lang::Hi));
    assert!(!p.has_translit(Lang::Bn));
    p
}

#[test]
fn spans_cover_the_input_and_offsets_agree() {
    let p = with_pack();
    let text = "mujhe kal the jana hai 😀!";
    let out = transliterate(&p, text, Lang::Hi, &TranslitOptions::default());
    assert_eq!(out.spans.first().unwrap().src.start, 0);
    assert_eq!(out.spans.last().unwrap().src.end, text.len());
    for pair in out.spans.windows(2) {
        assert_eq!(pair[0].src.end, pair[1].src.start);
        assert_eq!(pair[0].out.end, pair[1].out.start);
    }
    assert_eq!(out.spans.last().unwrap().out.end, out.text.len());
    let span = |w: &str| {
        out.spans
            .iter()
            .find(|s| &text[s.src.clone()] == w)
            .unwrap()
    };
    assert_eq!(span("kal").kind, WordKind::Indic);
    assert_eq!(&out.text[span("kal").out.clone()], "कल");
    assert!(!span("kal").alternatives.is_empty());
    assert_eq!(span("the").kind, WordKind::English);
    assert_eq!(&out.text[span("the").out.clone()], "the");
    assert!(out.text.ends_with(" 😀!"));
    // The emoji is one code point and two UTF-16 units.
    let json = out.to_json(text);
    assert!(json.contains("\"kind\":\"english\""), "{json}");
    assert!(
        json.contains("\"char\":[22,25],\"utf16\":[22,26]"),
        "{json}"
    );
}

#[test]
fn english_can_be_converted_or_respelled() {
    let p = with_pack();
    let text = "kal the jana hai";
    let all = transliterate(
        &p,
        text,
        Lang::Hi,
        &TranslitOptions {
            english: EnglishWords::Convert,
            ..Default::default()
        },
    );
    assert!(all.spans.iter().all(|s| s.kind != WordKind::English));
    assert!(!all.text.contains("the"));
    let native = transliterate(
        &p,
        text,
        Lang::Hi,
        &TranslitOptions {
            english: EnglishWords::Native,
            ..Default::default()
        },
    );
    assert!(native.spans.iter().any(|s| s.kind == WordKind::English));
    assert!(
        !native.text.chars().any(|c| c.is_ascii_alphabetic()),
        "{}",
        native.text
    );
}

#[test]
fn hai_agrees_with_the_verb_before_it() {
    let p = with_pack();
    let o = TranslitOptions::default();
    assert_eq!(
        transliterate(&p, "ham karte hai", Lang::Hi, &o).text,
        "हम करते हैं"
    );
    assert_eq!(transliterate(&p, "karta hai", Lang::Hi, &o).text, "करता है");
}

#[test]
fn hand_written_spellings_and_candidates() {
    let p = with_pack();
    let o = TranslitOptions::default();
    // "h" is only in the hand list.
    assert_eq!(
        transliterate(&p, "kal jana h", Lang::Hi, &o).text,
        "कल जाना है"
    );
    let c = candidates(&p, "naam", Lang::Hi, &o);
    assert_eq!(c[0].text, "नाम");
    assert!(c[0].attested > 0.0 && c[0].freq > 0.0);
    // An unseen word from seen pieces.
    assert!(candidates(&p, "nana", Lang::Hi, &o)
        .iter()
        .any(|c| c.text == "नाना"));
    assert!(candidates(&Phonemizer::new(), "naam", Lang::Hi, &o).is_empty());
}

#[test]
fn without_a_pack_rules_answer() {
    let p = Phonemizer::new();
    let o = TranslitOptions::default();
    let hi = transliterate(&p, "kya haal hai", Lang::Hi, &o);
    assert_eq!(hi.text, "क्या हाल है");
    assert!(hi.spans.iter().any(|s| s.kind == WordKind::Indic));
    let bn = transliterate(
        &p,
        "kamal",
        Lang::Bn,
        &TranslitOptions {
            english: EnglishWords::Convert,
            ..Default::default()
        },
    );
    assert_eq!(bn.text, "কমল");
    // A language without an Indian script: nothing to do.
    let en = transliterate(&p, "kya haal hai", Lang::EnUs, &o);
    assert_eq!(en.text, "kya haal hai");
    assert!(en.spans.iter().all(|s| s.kind == WordKind::Other));
}

#[test]
fn json_in_and_out() {
    let p = with_pack();
    let j = transliterate_json(&p, "kal jana hai", Lang::Hi, r#"{"alternatives":1}"#).unwrap();
    assert!(j.starts_with("{\"text\":\"कल जाना है\""), "{j}");
    assert!(transliterate_json(&p, "kal", Lang::Hi, r#"{"english":"x"}"#).is_err());
}

#[test]
fn normalised_spans_point_into_the_romanised_text() {
    let p = with_pack();
    let text = "kal 5 kaam hai";
    let n = normalize_romanised(
        &p,
        text,
        Lang::Hi,
        &TranslitOptions::default(),
        &NormOptions::default(),
    );
    assert!(
        n.text.starts_with("कल ") && n.text.ends_with("काम है"),
        "{}",
        n.text
    );
    assert!(!n.text.contains('5'), "{}", n.text);
    for s in &n.spans {
        assert!(s.src.start <= s.src.end && s.src.end <= text.len());
        assert!(text.is_char_boundary(s.src.start) && text.is_char_boundary(s.src.end));
    }
    assert_eq!(n.spans.first().unwrap().src.start, 0);
    assert!(n.spans.iter().any(|s| &text[s.src.clone()] == "5"));
}

#[test]
fn bad_packs_are_refused() {
    let p = Phonemizer::new();
    // A newer format.
    let mut newer = TOY.to_vec();
    let at = newer.windows(9).position(|w| w == b"format=1\n").unwrap();
    newer[at + 7] = b'9';
    let e = p.load_pack(newer).unwrap_err();
    assert!(e.contains("format 9"), "{e}");
    // Truncated: an error, not a panic.
    assert!(p.load_pack(TOY[..TOY.len() / 2].to_vec()).is_err());
    assert!(!p.has_translit(Lang::Hi));
}

#[test]
fn hinglish_phonemes_use_the_pack() {
    let p = with_pack();
    let mut o = uccharan::Options::new(Lang::Hi);
    o.hinglish = true;
    let native = uccharan::Options::new(Lang::Hi);
    // The romanised sentence reads like its Devanagari form.
    assert_eq!(
        p.phonemize("mujhe kal jana hai", &o),
        p.phonemize("मुझे कल जाना है", &native)
    );
}
