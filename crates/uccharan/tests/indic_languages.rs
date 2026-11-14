//! A few words per data-driven Indic language, written from the language
//! specifications (specs/<tag>.md), in compatible mode.

use uccharan::{Lang, Mode, Options, Phonemizer};

fn check(lang: Lang, pairs: &[(&str, &str)]) {
    let p = Phonemizer::new();
    let opt = Options::new(lang);
    for (text, want) in pairs {
        let got = uccharan::profile::nfd(&p.phonemize(text, &opt));
        assert_eq!(got, uccharan::profile::nfd(want), "{} {text}", lang.tag());
    }
}

#[test]
fn bengali() {
    check(
        Lang::Bn,
        &[
            ("কমলা", "kˈɔmɔlˌa"),
            ("সত্য", "ʃˈɔtːɔ"),
            ("রবি", "ɾˈobi"),
            ("হাত", "hˈato"),
            ("বাংলা", "bˈaŋla"),
            ("চাঁদ", "tʃˈãd"),
            ("আমি বাংলায় কথা বলি।", "ˌami bˈaŋlaj kˈɔtʰa bˈoli."),
        ],
    );
}

#[test]
fn marathi() {
    check(
        Lang::Mr,
        &[
            ("मराठी", "məɾˈaːʈʰi"),
            ("करतो", "kˈʌɾtˌoː"),
            ("समजले", "sˈʌməzleː"),
            ("धर्म", "dʰˈʌrm"),
            ("पत्ता", "pˈʌtːaː"),
            ("ज्ञान", "dɳjˈaːn"),
            ("मुंबई", "mˈũbˌi"),
        ],
    );
}

#[test]
fn telugu() {
    check(
        Lang::Te,
        &[
            ("తెలుగు", "tˈeluɡu"),
            ("పుస్తకం", "pˈustakan"),
            ("జ్ఞానం", "ɡnˈaːnan"),
            ("హైదరాబాదు", "hˈaɪdarˌaːbaːdu"),
        ],
    );
}

#[test]
fn nepali() {
    check(
        Lang::Ne,
        &[
            ("नेपाल", "neːpˈaːl"),
            ("घर", "ɡʰˈʌɾə"),
            ("धर्म", "dʰˈʌrrmə"),
            ("खानु", "kʰˈaːnuʲˌu"),
        ],
    );
}

#[test]
fn kannada_tamil_malayalam() {
    check(
        Lang::Kn,
        &[
            ("ಕನ್ನಡ", "kˈɐnnɐɖɐ"),
            ("ಸರ್ಕಾರ", "sˈɐrkaːɹɐ"),
            ("ಸಂಗೀತ", "sˈɐŋɡiːtɐ"),
        ],
    );
    check(
        Lang::Ta,
        &[
            ("தமிழ்", "tˈʌmiɻ"),
            ("பத்து", "pˈʌttʉ"),
            ("தம்பி", "tˈʌmbi"),
            ("பச்சை", "pˈʌtʃːaɪ"),
        ],
    );
    check(
        Lang::Ml,
        &[
            ("മലയാളം", "mˈɐləjˌaːɭəm"),
            ("സന്തോഷം", "sɐndˈoːʂəm"),
            ("കണ്ണ്", "kˈɐɳɳɨ"),
            ("ഞാൻ", "ɲˈaːn"),
        ],
    );
}

#[test]
fn gujarati_assamese() {
    check(
        Lang::Gu,
        &[("ઘર", "ɡʰˈʌɾ"), ("ઝાડ", "zˈaːɖ"), ("વાત", "wˈaːt")],
    );
    check(
        Lang::As,
        &[("কমল", "kˈɔmɔl"), ("সাপ", "χˈap"), ("ভাত", "bʰˈato")],
    );
}

#[test]
fn scripts_route_to_their_language() {
    let p = Phonemizer::new();
    let mut opt = Options::new(Lang::Hi);
    opt.keep_language_flags = true;
    // Bengali inside Hindi text is read by the Bengali rules, flagged.
    assert_eq!(p.phonemize("नमस्ते কমলা", &opt), "nəmˈʌsteː (bn)kˈɔmɔlˌa(hi)");
    // Every language parses from its tag, and its rules load.
    for l in Lang::ALL {
        assert_eq!(Lang::parse(l.tag()), Some(l));
        let mut o = Options::new(l);
        o.mode = Mode::Improved;
        let _ = p.phonemize("a", &o);
    }
    // ASCII digits in Nepali are read in English.
    let ne = p.phonemize("नेपाल 5", &Options::new(Lang::Ne));
    assert_eq!(ne, "neːpˈaːl fˈaɪv");
}

#[test]
fn bhojpuri_and_maithili() {
    let p = Phonemizer::new();
    // Compatible mode is the reference's Hindi-voice reading.
    for lang in [Lang::Bho, Lang::Mai] {
        let opt = Options::new(lang);
        let hi = Options::new(Lang::Hi);
        for t in ["रउआ कइसन बानी?", "ऊ गाँव गइल।", "अलग-अलग"]
        {
            assert_eq!(p.phonemize(t, &opt), p.phonemize(t, &hi), "{t}");
        }
    }
    let mut opt = Options::new(Lang::Bho);
    opt.mode = Mode::Improved;
    let strip = |s: String| s.replace(['ˈ', 'ˌ'], "");
    assert_eq!(strip(p.phonemize("चलऽ", &opt)), "cʌlə");
    assert_eq!(strip(p.phonemize("चलs", &opt)), "cʌlə");
    assert_eq!(strip(p.phonemize("ऐसन", &opt)), "əisən");
    assert!(p.phonemize("बाड़े", &opt).contains('ɽ'));
    assert_eq!(strip(p.phonemize("रहलन", &opt)), "ɾʌhələn");
    assert_eq!(strip(p.phonemize("2", &opt)), "duː");
    // Kaithi is read as Devanagari.
    let kaithi = "\u{110A6}\u{110B7}\u{11094}\u{110A3}\u{110B3}\u{110A9}\u{110B2}";
    assert_eq!(p.phonemize(kaithi, &opt), p.phonemize("भोजपुरी", &opt));
    assert_eq!(Lang::parse("bhojpuri"), Some(Lang::Bho));
    assert_eq!(Lang::parse("maithili"), Some(Lang::Mai));
}

#[test]
fn punjabi_odia() {
    check(
        Lang::Pa,
        &[
            ("ਘਰ", "kˈʌ+ɾ"),
            ("ਕਰਨਾ", "kˈʌɾənˌa"),
            ("ਪੰਜਾਬੀ", "pənɟˈabi"),
            ("ਕੁੱਤਾ", "kˈʊtːa"),
            ("ਅੱਖ", "ˈʌkʰ"),
            ("ਮੈਂ ਪੰਜਾਬੀ ਬੋਲਦਾ ਹਾਂ।", "mɛ̃ pənɟˈabi bˈolədˌa hã."),
        ],
    );
    check(
        Lang::Or,
        &[
            ("ଘର", "ɡˈɔɾɔ"),
            ("କମଳ", "kˈɔmɔl̩ˌɔː"),
            ("ସତ୍ୟ", "sˈɔtj"),
            ("ଓଡ଼ିଶା", "ˈor.ɔˌisa"),
            ("ମୁଁ ଓଡ଼ିଆ କହେ।", "mˈũ ˈor.ɔˌia kˈɔhe."),
        ],
    );
}

#[test]
fn konkani_sinhala_sanskrit() {
    // The reference's Konkani voice reads vowel signs by their names.
    check(
        Lang::Kok,
        &[
            ("का", "kˌəaːkˈaːɾ"),
            ("कक", "kˈʌk"),
            ("करप", "kəɾˈʌp"),
            (
                "हांव कोंकणी उलयता.",
                "hˌəaːkˈaːrshɪɾˈʌwˈindˈuwə kˌəoːkˈaːrŋkəɳikˈaːɾ ˌʊləjˌətaːkˈaːɾ.",
            ),
        ],
    );
    check(
        Lang::Si,
        &[
            ("අම්මා", "ˈɐmmaː"),
            ("ලංකාව", "lˈɐŋkaːw"),
            // A ZWJ conjunct splits the word in the reference.
            (
                "\u{0d9a}\u{0dca}\u{200d}\u{0dbb}\u{0db8}\u{0dba}",
                "k ɹˈɐməjə",
            ),
            ("මම සිංහල කතා කරනවා.", "mˈɐmə sˈiŋhələ kˈɐtaː kˈɐɹənˌəwaː."),
        ],
    );
    // Sanskrit: compatible mode is the Hindi reading; improved keeps every a.
    let p = Phonemizer::new();
    assert_eq!(
        p.phonemize("गच्छति", &Options::new(Lang::Sa)),
        p.phonemize("गच्छति", &Options::new(Lang::Hi))
    );
    let mut o = Options::new(Lang::Sa);
    o.mode = Mode::Improved;
    assert_eq!(
        p.phonemize("रामः वनं गच्छति।", &o),
        "ɾˈaːməɦə ʋˈənəm ɡˈətʃtʃʰəti."
    );
}
