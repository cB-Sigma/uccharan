//! End-to-end tests of the public API. None of them needs a reference
//! phonemizer: the expected strings are a small hand-picked regression set.

use uccharan::profile::{self, PhonemeIdMap, Profile};
use uccharan::{Lang, Mode, Normalize, Options, Phonemizer};

fn nfd(s: &str) -> String {
    profile::nfd(s)
}

fn improved(lang: Lang) -> Options {
    let mut o = Options::new(lang);
    o.mode = Mode::Improved;
    o
}

#[test]
fn english_sentences() {
    let p = Phonemizer::new();
    let us = Options::new(Lang::EnUs);
    assert_eq!(
        p.phonemize("The quick brown fox jumps over the lazy dog.", &us),
        "ðə kwˈɪk bɹˈaʊn fˈɑːks dʒˈʌmps ˌoʊvɚ ðə lˈeɪzi dˈɑːɡ."
    );
    let gb = Options::new(Lang::EnGb);
    assert_eq!(
        p.phonemize("Better late than never, isn't it?", &gb),
        "bˈɛtɐ lˈeɪt ðɐn nˈɛvɐ, ˌɪzənt ɪt?"
    );
}

#[test]
fn hindi_sentences_and_modes() {
    let p = Phonemizer::new();
    let hi = Options::new(Lang::Hi);
    assert_eq!(
        nfd(&p.phonemize("आप कैसे हैं? मैं ठीक हूँ।", &hi)),
        nfd("ˌaːp kˈɛːseː hɛ̃? mɛ̃ ʈʰˈiːk hũ.")
    );
    // The compatible mode keeps the reference's conventions; the improved mode is plain IPA
    // with morpheme-aware schwa deletion and homorganic nasals.
    assert_eq!(
        nfd(&p.phonemize("लड़कपन हिंदी गाँव", &hi)),
        nfd("lˈʌr.əkˌəpən hˈĩdi ɡˈãnʋ")
    );
    assert_eq!(
        nfd(&p.phonemize("लड़कपन हिंदी गाँव", &improved(Lang::Hi))),
        nfd("lˈʌɽəkpən hˈɪndiː ɡˈãʋ")
    );
}

#[test]
fn code_mixing_routes_by_script() {
    let p = Phonemizer::new();
    let hi = Options::new(Lang::Hi);
    let s = p.analyze("मैं office जा रहा हूँ", &hi);
    let langs: Vec<Lang> = s[0].clauses[0].words.iter().map(|w| w.lang).collect();
    assert_eq!(langs, [Lang::Hi, Lang::EnGb, Lang::Hi, Lang::Hi, Lang::Hi]);
    assert_eq!(
        nfd(&p.phonemize("मैं office जा रहा हूँ", &hi)),
        nfd("mɛ̃ ˈɒfɪs ɟˈaː ɾˌəhaː hũ")
    );
    // Improved: the English word is mapped into the Hindi phone set.
    assert_eq!(
        nfd(&p.phonemize("मैं office जा रहा हूँ", &improved(Lang::Hi))),
        nfd("mɛ̃ ˈɔfɪs ɟˈaː ɾˌəhaː hũ")
    );
    let mut flags = hi.clone();
    flags.keep_language_flags = true;
    let flagged = p.phonemize("मैं office जा रहा हूँ", &flags);
    assert!(flagged.contains("(en)ˈɒfɪs(hi)"), "{flagged}");
    // Devanagari inside English text is read by the Hindi rules.
    let en = p.phonemize("She said नमस्ते to me.", &Options::new(Lang::EnUs));
    assert!(en.contains("nəmˈʌsteː"), "{en}");
    // Romanised Hindi (experimental).
    let mut hing = hi.clone();
    hing.hinglish = true;
    assert_eq!(
        p.phonemize("kya haal hai", &hing),
        p.phonemize("क्या हाल है", &hi)
    );
}

#[test]
fn improved_mode_homographs_and_names() {
    let p = Phonemizer::new();
    let imp = improved(Lang::EnUs);
    let compat = Options::new(Lang::EnUs);
    assert_eq!(
        p.phonemize("They will record a new record.", &imp),
        "ðeɪ wɪl ɹᵻkˈɔːɹd ɐ nˈuː ɹˈɛkɚd."
    );
    assert!(p.phonemize("I read it yesterday.", &imp).contains("ɹˈɛd"));
    assert!(p
        .phonemize("I read it yesterday.", &compat)
        .contains("ɹˈiːd"));
    assert!(p
        .phonemize("He caught a bass in the lake.", &imp)
        .contains("bˈæs"));
    assert!(p.phonemize("She plays bass.", &imp).contains("bˈeɪs"));
    assert!(p.phonemize("Her iPhone fell.", &imp).contains("ˈaɪfoʊn"));
    assert!(p
        .phonemize("Turn left on Maple Dr. after the bridge.", &imp)
        .contains("dɹˈaɪv"));
    assert!(p.phonemize("Dr. Smith is here.", &imp).contains("dˈɑːktɚ"));
}

#[test]
fn number_reading_styles() {
    let p = Phonemizer::new();
    let mut o = Options::new(Lang::EnUs);
    let natural = p.phonemize("It costs $5.50.", &o);
    assert_eq!(
        natural,
        p.phonemize("It costs five dollars and fifty cents.", &o)
    );
    assert_eq!(
        p.phonemize("Read pages 10-15.", &o),
        p.phonemize("Read pages ten to fifteen.", &o)
    );
    assert_eq!(
        p.phonemize("Call 555-1234 now.", &o),
        p.phonemize("Call five five five one two three four now.", &o)
    );
    o.normalize = Normalize::EspeakLike;
    assert!(p.phonemize("It costs $5.50.", &o).contains("pɔɪnt"));
    o.normalize = Normalize::Off;
    assert_eq!(p.phonemize("It costs 5 dollars.", &o), "ɪt kˈɔsts dˈɑːlɚz.");
    let hi = Options::new(Lang::Hi);
    assert_eq!(
        p.phonemize("यह ₹200 का है।", &hi),
        p.phonemize("यह दो सौ रुपये का है।", &hi)
    );
}

#[test]
fn profiles_have_the_documented_shapes() {
    let p = Phonemizer::new();
    let us = Options::new(Lang::EnUs);
    let s = p.analyze("Wait: one, two; three! Four? Five.", &us);
    let piper = profile::piper_sentences(&s);
    assert_eq!(piper, ["wˈeɪt: wˌʌn, tˈuː; θɹˈiː!", "fˈɔːɹ?", "fˈaɪv."]);
    let cps = profile::sherpa_codepoints(&s);
    assert_eq!(cps.len(), 3);
    assert_eq!(
        cps[1],
        "fˈɔːɹ?".chars().map(|c| c as u32).collect::<Vec<_>>()
    );
    // Piper framing with a voice's id map.
    let map = PhonemeIdMap::from_json(
        r#"{"phoneme_id_map": {"_": [0], "^": [1], "$": [2], " ": [3], "?": [13], "f": [19], "ɔ": [54], "ɹ": [88], "ˈ": [120], "ː": [122]}}"#,
    )
    .unwrap();
    let (ids, missing) = map.ids(&piper[1]);
    assert_eq!(ids, [1, 0, 19, 0, 120, 0, 54, 0, 122, 0, 88, 0, 13, 0, 2]);
    assert!(missing.is_empty());
    // Kokoro tokens stay inside the vocabulary; ids are padded with 0.
    let k = profile::kokoro_tokens(&p.analyze("The butter is better.", &us), Lang::EnUs);
    assert_eq!(k, "ðə bˈʌTəɹ ɪz bˈɛTəɹ.");
    assert!(profile::unmapped(&k, Profile::Kokoro).is_empty());
    let ids = profile::kokoro_ids(&k);
    assert_eq!((ids[0], *ids.last().unwrap()), (0, 0));
    // Kitten and Matcha use the same phoneme string with different framing.
    let ipa = profile::render_text(&s, Profile::Kitten, Lang::EnUs, true);
    assert!(profile::unmapped(&ipa, Profile::Kitten).is_empty());
    assert_eq!(profile::kitten_ids(&ipa).len(), ipa.chars().count() + 3);
    assert_eq!(profile::matcha_ids(&ipa).len(), 2 * ipa.chars().count() + 1);
    for name in [
        "espeak-ipa",
        "piper",
        "kokoro",
        "kitten",
        "matcha",
        "sherpa-onnx",
    ] {
        assert_eq!(Profile::parse(name).unwrap().name(), name);
    }
}

#[test]
fn every_profile_stays_inside_its_table_on_mixed_text() {
    let p = Phonemizer::new();
    let text = "Dr. Smith paid $12.50 on 3/14/2024; the naïve café-owner said \"no\" (twice) – really? ठीक है, चलो! 100% sure… #1";
    for lang in Lang::ALL {
        for mode in [Mode::Compatible, Mode::Improved] {
            let mut o = Options::new(lang);
            o.mode = mode;
            let s = p.analyze(text, &o);
            let k = profile::kokoro_tokens(&s, lang);
            assert!(profile::unmapped(&k, Profile::Kokoro).is_empty());
            for line in profile::piper_sentences(&s) {
                assert!(
                    !line.contains('(') && !line.contains(')'),
                    "language flags must not reach Piper: {line}"
                );
            }
        }
    }
}

#[test]
fn odd_input_is_harmless() {
    let p = Phonemizer::new();
    for lang in Lang::ALL {
        let o = Options::new(lang);
        assert_eq!(p.phonemize("", &o), "");
        assert_eq!(p.phonemize("!!! ...", &o), "");
        assert_eq!(p.phonemize("   \n\t ", &o), "");
        let long = "word ".repeat(5000);
        assert_eq!(p.phonemize(&long, &o).split(' ').count(), 5000);
        // No panic on arbitrary scripts, emoji, controls, lone combining marks.
        for s in [
            "😀👍🏽",
            "Привет мир",
            "中文测试",
            "\u{0}\u{1}\u{7f}",
            "\u{0303}\u{094d}",
            "a\u{200d}b",
            "ॐ ऽ ॥",
            "𝔘𝔫𝔦𝔠𝔬𝔡𝔢",
            "ﬁnal ﬂow",
        ] {
            let _ = p.phonemize(s, &o);
            let _ = profile::sherpa_codepoints(&p.analyze(s, &o));
        }
    }
    assert_eq!(
        p.phonemize("naïve café", &Options::new(Lang::EnUs)),
        "naɪˈiːv kəfˈeɪ"
    );
}

#[test]
fn batch_equals_single_and_is_thread_safe() {
    let p = Phonemizer::new();
    let us = Options::new(Lang::EnUs);
    let texts: Vec<String> = (0..200)
        .map(|i| format!("Sentence number {i} is here, isn't it?"))
        .collect();
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    let single: Vec<String> = refs.iter().map(|t| p.phonemize(t, &us)).collect();
    assert_eq!(p.phonemize_batch(&refs, &us), single);
    for threads in [1, 2, 3, 8] {
        assert_eq!(
            p.render_batch(&refs, &us, Profile::EspeakIpa, true, threads),
            single
        );
    }
    // Shared by reference across threads, different languages at once.
    std::thread::scope(|s| {
        for lang in Lang::ALL {
            let p = &p;
            s.spawn(move || {
                let o = Options::new(lang);
                let first = p.phonemize("नमस्ते and hello 42", &o);
                for _ in 0..200 {
                    assert_eq!(p.phonemize("नमस्ते and hello 42", &o), first);
                }
            });
        }
    });
}

#[test]
fn data_can_be_loaded_at_run_time() {
    let mut p = Phonemizer::empty();
    assert!(!p.has(Lang::EnUs));
    assert_eq!(p.phonemize("hello", &Options::new(Lang::EnUs)), "");
    let full = Phonemizer::new();
    let bytes = include_bytes!("../data/en-us.ucd").to_vec();
    p.load(Lang::EnUs, bytes).unwrap();
    assert_eq!(
        p.phonemize("Hello, world.", &Options::new(Lang::EnUs)),
        full.phonemize("Hello, world.", &Options::new(Lang::EnUs))
    );
    assert!(p.load(Lang::EnGb, vec![1, 2, 3]).is_err());
    assert!(Lang::parse("en-gb-x-rp") == Some(Lang::EnGb) && Lang::parse("xx").is_none());
}
