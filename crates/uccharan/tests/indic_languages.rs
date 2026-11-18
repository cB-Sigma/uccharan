//! A few words per data-driven Indic language, written from the language
//! descriptions, in compatible mode.

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

#[test]
fn urdu_sindhi() {
    // Common words come from the word tables, others from the rules.
    check(
        Lang::Ur,
        &[
            ("آپ کیسے ہیں؟", "ˌaːp kˈeːseː hẽ?"),
            ("بوت", "bˈoːt"),
            ("ببت", "bˈʌbət"),
        ],
    );
    check(Lang::Sd, &[("سنڌي ٻولي", "sˈɪndhi bˈoːli"), ("۽", "ˈaẽ")]);
    // A listed word that has no table entry: the rules' reading, corrected
    // by the table's model.
    check(Lang::Sd, &[("آزارن", "ˌaːzaːrˈʌn")]);
    let p = Phonemizer::new();
    p.ensure(Lang::Sd).expect("the Sindhi word table");
    assert!(p.data(Lang::Sd).is_some_and(|d| d.reads_rules()));
    // Improved mode reads ص and ظ as s and z, and the Urdu digits.
    let p = Phonemizer::new();
    let mut o = Options::new(Lang::Ur);
    o.mode = Mode::Improved;
    assert_eq!(p.phonemize("صبح ۱۹", &o), "sˌəbˈah ˈʊnnis");
    assert_eq!(p.phonemize("صبح ۱۹", &Options::new(Lang::Ur)), "ʂˌəbˈah");
}

#[test]
fn magahi() {
    // Compatible mode is the Hindi reading.
    let p = Phonemizer::new();
    let t = "हम घरे जाइत ही।";
    assert_eq!(
        p.phonemize(t, &Options::new(Lang::Mag)),
        p.phonemize(t, &Options::new(Lang::Hi))
    );
}

#[test]
fn dogri() {
    // Compatible mode is the Hindi reading (the reference has no Dogri
    // voice); a word with the apostrophe of a lost h is read as one word.
    let p = Phonemizer::new();
    assert_eq!(
        p.phonemize("घर", &Options::new(Lang::Doi)),
        p.phonemize("घर", &Options::new(Lang::Hi))
    );
    assert_eq!(p.phonemize("कि’यां", &Options::new(Lang::Doi)), "kˈɪjã");
    // Improved mode: a word-initial voiced aspirate is voiceless with low
    // tone after it; an h before a vowel is lost with low tone; ह् , the
    // apostrophe and the avagraha after a stressed vowel give it high
    // tone; a voiced aspirate inside the word loses its aspiration.
    let mut o = Options::new(Lang::Doi);
    o.mode = Mode::Improved;
    let w = |s: &str| p.phonemize(s, &o);
    assert_eq!(w("घर"), "kˈʌ̀ɾ");
    assert_eq!(w("भाशा"), "pˈàːʃaː");
    assert_eq!(w("ज्हार"), "ɟˈàːɾ");
    assert_eq!(w("म्हीना"), "mˈìːnaː");
    assert_eq!(w("साह्ब"), "sˈáːb");
    assert_eq!(w("त्रीह्"), "tɾˈíː");
    assert_eq!(w("कि’यां"), "kˈɪ́jãː");
    assert_eq!(w("रपेऽ"), "ɾəpˈéː");
    assert_eq!(w("सिद्धा"), "sˈɪ́dːaː");
    assert_eq!(w("बधाई"), "bədˈàːiː");
    assert_eq!(w("ऐ"), "ɛː");
    assert_eq!(w("गल्ल"), "ɡˈʌlː");
    // ऎ is the copula too; a capital S after a letter is the avagraha.
    assert_eq!(w("ऎ"), "ɛː");
    assert_eq!(w("रुपेS"), w("रुपेऽ"));
    assert_eq!(w("एह् मेरा घर ऐ।"), "éː mˈeːɾaː kˈʌ̀ɾ ɛː.");
    // Numbers, ordinals and clock times in Dogri words.
    assert_eq!(w("25"), w("पंजी"));
    assert_eq!(w("12मीं"), w("बारांमीं"));
    assert_eq!(w("10:30"), w("दस बजियै त्रीह् मिंट"));
    // Romanised Dogri.
    let mut r = o.clone();
    r.hinglish = true;
    assert_eq!(p.phonemize("eh gall shail ae", &r), w("एह् गल्ल शैल ऐ"));
}

#[test]
fn bodo() {
    // Compatible mode is the Hindi reading (the reference has no Bodo
    // voice); a word written with the apostrophe is read as one word.
    let p = Phonemizer::new();
    assert_eq!(
        p.phonemize("बोलो", &Options::new(Lang::Brx)),
        p.phonemize("बोलो", &Options::new(Lang::Hi))
    );
    assert_eq!(
        p.phonemize("कर’ना", &Options::new(Lang::Brx)),
        p.phonemize("करना", &Options::new(Lang::Hi))
    );
    // Improved mode (Unicode L2/08-197; Bhattacharya 1977): ो is ɯ, ै ौ
    // are ɯi ɯu, the inherent vowel is ɔ and the apostrophe says it, ं is
    // ŋ, य व after a vowel close a diphthong, ज is z, no voiced aspirates.
    let mut o = Options::new(Lang::Brx);
    o.mode = Mode::Improved;
    let w = |s: &str| uccharan::profile::nfd(&p.phonemize(s, &o));
    assert_eq!(w("बोलो"), "bˈɯlɯ");
    assert_eq!(w("बॊलॊ"), "bˈɯlɯ");
    assert_eq!(w("दै"), "dˈɯi");
    assert_eq!(w("मोसौ"), "mˈɯsɯu");
    assert_eq!(w("बर’"), "bˈɔrɔ");
    assert_eq!(w("बर'"), w("बर’"));
    assert_eq!(w("बरʼ"), w("बर’"));
    assert_eq!(w("दख'ना"), "dˈɔkʰɔna");
    assert_eq!(w("सुबुं"), "sˈubuŋ");
    assert_eq!(w("थाखाय"), "tʰˈakʰai");
    assert_eq!(w("दाव"), "dˈau");
    assert_eq!(w("मोनथाय"), "mˈɯntʰˌai");
    assert_eq!(w("जोनोम"), "zˈɯnɯm");
    assert_eq!(w("संख्रि"), "sˈɔŋkʰri");
    assert_eq!(w("भारत"), "bˈarɔt");
    assert_eq!(w("गथ"), "ɡˈɔtʰɔ");
    // Numbers, ordinals and clock times in Bodo words.
    assert_eq!(w("25"), w("नैजिबा"));
    assert_eq!(w("6"), w("द’"));
    assert_eq!(w("21थि"), w("नैजिसेथि"));
    assert_eq!(w("10:30"), w("जि रिंगा थामजि मिनिट"));
    // Romanised Bodo (w = ɯ, o = the inherent vowel).
    let mut r = o.clone();
    r.hinglish = true;
    assert_eq!(
        p.phonemize("ang boro raw bungw", &r),
        p.phonemize("आं बर’ राव बुङो", &o)
    );
}

#[test]
fn santali() {
    // Ol Chiki (Everson, Unicode L2/05-243R; Ghosh 2008): every vowel is a
    // letter, ᱹ makes ə ɛ, ᱸ ᱺ nasalise, ᱻ lengthens; ᱜ ᱡ ᱫ ᱵ are checked
    // stops at the end of a word and before a voiceless consonant, ᱼ keeps
    // them checked, ᱽ voices them; ᱷ aspirates; stress on the second syllable
    // (Anderson & Horo 2021). espeak-ng cannot read the script, so both modes
    // give this reading.
    let p = Phonemizer::new();
    let o = Options::new(Lang::Sat);
    let mut i = o.clone();
    i.mode = Mode::Improved;
    let w = |s: &str| uccharan::profile::nfd(&p.phonemize(s, &o));
    let n = uccharan::profile::nfd;
    assert_eq!(w("ᱥᱟᱱᱛᱟᱲᱤ ᱯᱟᱹᱨᱥᱤ"), p.phonemize("ᱥᱟᱱᱛᱟᱲᱤ ᱯᱟᱹᱨᱥᱤ", &i));
    assert_eq!(w("ᱥᱟᱱᱛᱟᱲᱤ"), "santˈaɽi");
    assert_eq!(w("ᱯᱟᱹᱨᱥᱤ"), "pərsˈi");
    assert_eq!(w("ᱨᱮᱹᱞ"), "rˈɛl");
    assert_eq!(w("ᱢᱤᱫ"), "mˈit\u{31a}ʔ");
    assert_eq!(w("ᱟᱡᱟᱜ"), "aɟˈak\u{31a}ʔ");
    assert_eq!(w("ᱢᱮᱱᱟᱜᱼᱟ"), "menˈak\u{31a}ʔa");
    assert_eq!(w("ᱜᱚ"), "ɡˈɔ");
    assert_eq!(w("ᱜᱼᱚ"), "k\u{31a}ʔˈɔ");
    assert_eq!(w("ᱨᱟᱡᱽ"), "rˈaɟ");
    assert_eq!(w("ᱢᱤᱫᱴᱟᱹᱝ"), "mit\u{31a}ʔʈˈəŋ");
    assert_eq!(w("ᱜᱤᱫᱨᱟᱹ"), "ɡidrˈə");
    assert_eq!(w("ᱜᱷᱟᱨᱚᱸᱡᱽ"), n("ɡʱarˈɔ̃ɟ"));
    assert_eq!(w("ᱞᱮᱠᱷᱟ"), "lekʰˈa");
    assert_eq!(w("ᱥᱟᱺᱜᱤᱧ"), n("sə̃ɡˈiɲ"));
    assert_eq!(w("ᱨᱮᱻᱞ"), "rˈeːl");
    assert_eq!(w("ᱥᱟᱶ"), "sˈau\u{303}");
    assert_eq!(w("ᱱᱚᱶᱟ"), "nɔw\u{303}ˈa");
    assert_eq!(w("ᱠᱟᱱᱟᱭ"), "kanˈai");
    assert_eq!(w("ᱫᱚ ᱦᱚᱲ"), "dɔ hˈɔɽ");
    // A full stop typed for ᱹ, a hyphen for ᱼ.
    assert_eq!(w("ᱟ.ᱲᱟ"), w("ᱟᱹᱲᱟ"));
    assert_eq!(w("ᱢᱮᱱᱟᱜ-ᱟ"), w("ᱢᱮᱱᱟᱜᱼᱟ"));
    // Numbers in Santali words, from Ol Chiki or ASCII digits.
    assert_eq!(w("᱒᱕"), w("ᱵᱟᱨ ᱜᱮᱞ ᱢᱚᱬᱮ"));
    assert_eq!(w("25"), w("ᱵᱟᱨ ᱜᱮᱞ ᱢᱚᱬᱮ"));
    assert_eq!(w("᱑᱐᱐᱐"), w("ᱢᱤᱫ ᱦᱟᱡᱟᱨ"));
    assert_eq!(w("₹᱕᱐᱐"), w("ᱢᱚᱬᱮ ᱥᱟᱭ ᱴᱟᱠᱟ"));
    // ᱾ ends a sentence.
    assert_eq!(p.analyze("ᱫᱚ᱾ ᱟᱨ", &o).len(), 2);
    // Romanised Santali.
    let mut r = o.clone();
    r.hinglish = true;
    assert_eq!(
        p.phonemize("johar, inj disom do santali kana", &r),
        p.phonemize("ᱡᱚᱦᱟᱨ, ᱤᱧ ᱫᱤᱥᱚᱢ ᱫᱚ ᱥᱟᱱᱛᱟᱲᱤ ᱠᱟᱱᱟ", &o)
    );
}
