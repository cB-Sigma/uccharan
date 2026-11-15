//! Number formats, ordinals and dates: compatible mode as the reference
//! voices read them (checked against espeak-ng 1.52.0), improved mode and
//! normalise-only mode in full words.

use uccharan::normalize_api::{normalize, NormOptions, SpanKind};
use uccharan::{Lang, Mode, Options, Phonemizer};

fn p() -> &'static Phonemizer {
    static P: std::sync::OnceLock<Phonemizer> = std::sync::OnceLock::new();
    P.get_or_init(Phonemizer::new)
}

fn ipa(lang: Lang, mode: Mode, text: &str) -> String {
    let mut o = Options::new(lang);
    o.mode = mode;
    uccharan::profile::nfd(&p().phonemize(text, &o))
}

fn norm(lang: Lang, text: &str) -> String {
    normalize(p(), text, lang, &NormOptions::default()).text
}

#[test]
fn compatible_mode_reads_number_formats_as_the_reference() {
    let cases: &[(Lang, &str, &str)] = &[
        (Lang::De, "Haus -5 Haus", "hˈaʊs mˈiːnʊs fˈʏnf hˈaʊs"),
        (Lang::De, "Haus 2.5 Haus", "hˈaʊs tsvˈaɪ pʊŋkt fˈʏnf hˈaʊs"),
        (Lang::Hu, "ház 2,5 ház", "hˈaːz kˈɛtːøː ɛɡeːs øttizɛd hˌaːz"),
        (
            Lang::Hu,
            "ház 1.000 ház",
            "hˈaːz ˈɛɟ pont nˈullɑnullɑ nullɑ hˌaːz",
        ),
        (
            Lang::It,
            "casa 3,25 casa",
            "kˈaza trˈe vˈirɡola ventitʃˈinkwe kˈaza",
        ),
        (Lang::Si, "පොත 100 පොත", "pˈotə sˈijəjə pˈotə"),
        (Lang::Si, "පොත 1000 පොත", "pˈotə ˈɐhəs pˈotə"),
        (Lang::Si, "පොත 200 පොත", "pˈotə dˈekəsˈijə pˈotə"),
        (Lang::Si, "පොත 1500 පොත", "pˈotə dˈɐhəs pˈɐhəsˈijə pˈotə"),
        (Lang::Si, "පොත 50% පොත", "pˈotə sˌijəjˌəʈə pˈɐnəhə pˈotə"),
        (
            Lang::Da,
            "Han blev nummer 3. i løbet",
            "han blˈʔe nˈɒmʔʌ tʁˈɛdjə i lˈœbəð",
        ),
        (Lang::Fr, "Le 1er mai", "lə- pʁəmjˈe mˈɛ"),
        (Lang::Sv, "Han kom 1:a och 2:a", "han kˈɔm fˈœʂta ɔk ˈandra"),
        (Lang::Nl, "de 1e keer", "də ˈɪːrstə ˈeː kˈɪːr"),
    ];
    for (l, t, want) in cases {
        assert_eq!(
            ipa(*l, Mode::Compatible, t),
            uccharan::profile::nfd(want),
            "{l:?} {t}"
        );
    }
    // Compatible mode keeps the reference's piece-by-piece date.
    assert!(ipa(Lang::De, Mode::Compatible, "15.08.2024").contains("nˈʊl"));
    assert!(ipa(Lang::De, Mode::Compatible, "am 3. Mai").contains("dɾˈaɪ"));
}

#[test]
fn improved_mode_reads_ordinals_and_dates() {
    assert!(ipa(Lang::De, Mode::Improved, "am 3. Mai").contains("dʁˈɪtən"));
    assert!(ipa(Lang::De, Mode::Improved, "2.5").contains("kɔmaː"));
    let fr = ipa(Lang::Fr, Mode::Improved, "15/08/2024");
    assert!(!fr.contains("zeʁo") && fr.contains("ut"), "{fr}");
    // Konkani and Sinhala numbers as their written words.
    assert!(ipa(Lang::Kok, Mode::Improved, "100").contains("ʃəmb"));
    assert_eq!(ipa(Lang::Si, Mode::Improved, "100"), "sˈijəjə");
}

#[test]
fn normalise_only_mode_writes_dates() {
    let cases: &[(Lang, &str, &str)] = &[
        (
            Lang::De,
            "15.08.2024",
            "fünfzehnter August zweitausend vierundzwanzig",
        ),
        (
            Lang::De,
            "am 15.08.2024",
            "am fünfzehnten August zweitausend vierundzwanzig",
        ),
        (
            Lang::De,
            "01.05.1984",
            "erster Mai neunzehnhundertvierundachtzig",
        ),
        (
            Lang::Fr,
            "15/08/2024",
            "quinze août deux mille vingt-quatre",
        ),
        (
            Lang::Fr,
            "01/05/2024",
            "premier mai deux mille vingt-quatre",
        ),
        (
            Lang::Es,
            "15/08/2024",
            "quince de agosto de dos mil veinticuatro",
        ),
        (
            Lang::Pt,
            "2024-08-15",
            "quinze de agosto de dois mil e vinte e quatro",
        ),
        (Lang::It, "1-5-2024", "primo maggio duemilaventiquattro"),
        (
            Lang::Ca,
            "15/04/2024",
            "quinze d'abril de dos mil vint-i-quatre",
        ),
        (
            Lang::Nl,
            "15-08-1984",
            "vijftien augustus negentienhonderdvierentachtig",
        ),
        (
            Lang::Pl,
            "15.08.2024",
            "piętnastego sierpnia dwa tysiące dwudziestego czwartego roku",
        ),
        (
            Lang::Cs,
            "15.08.2024",
            "patnáctého srpna dva tisíce dvacet čtyři",
        ),
        (
            Lang::Hu,
            "2024.08.15",
            "kétezer huszonnégy augusztus tizenötödike",
        ),
        (Lang::Hu, "2024.03.01", "kétezer huszonnégy március elseje"),
        (
            Lang::Fi,
            "15.08.2024",
            "viidestoista elokuuta kaksi tuhatta kaksikymmentäneljä",
        ),
        (
            Lang::Sv,
            "15/08/2024",
            "femtonde augusti två tusen tjugofyra",
        ),
        (Lang::Tr, "15.08.2024", "on beş Ağustos iki bin yirmi dört"),
        (
            Lang::EnGb,
            "15.08.2024",
            "the fifteenth of August twenty twenty four",
        ),
        (
            Lang::EnUs,
            "08/15/2024",
            "August fifteenth twenty twenty four",
        ),
        (Lang::Bn, "15/08/2024", "পনেরো আগস্ট দুই হাজার চব্বিশ"),
        (
            Lang::Ta,
            "15/08/2024",
            "பதினைந்து ஆகஸ்ட் இரண்டாயிரத்து இருபத்து நான்கு",
        ),
        (Lang::Kok, "15/08/2024", "पंदरा ऑगस्ट दोन हजार चोवीस"),
        (Lang::Si, "15/08/2024", "පහළොව අගෝස්තු දෙදහස් විසිහතර"),
    ];
    for (l, t, want) in cases {
        let n = normalize(p(), t, *l, &NormOptions::default());
        assert_eq!(n.text, *want, "{l:?} {t}");
        let date = n.spans.iter().any(|s| s.kind == SpanKind::Date);
        assert!(date, "{l:?} {t}: {:?}", n.spans);
    }
}

#[test]
fn normalise_only_mode_writes_ordinals() {
    let cases: &[(Lang, &str, &str)] = &[
        (Lang::De, "am 3. Mai", "am dritten Mai"),
        (Lang::De, "der 21. Sieger", "der einundzwanzigste Sieger"),
        (Lang::Da, "den 21. maj", "den etogtyvende maj"),
        (Lang::Nb, "på 3. plass", "på tredje plass"),
        (Lang::Fi, "3. kerran", "kolmas kerran"),
        (Lang::Hu, "a 21. század", "a huszonegyedik század"),
        (Lang::Cs, "3. místo", "třetí místo"),
        (Lang::Tr, "21. yüzyıl", "yirmi birinci yüzyıl"),
        (
            Lang::Fr,
            "le 1er mai, la 1re fois, le 2e jour",
            "le premier mai, la première fois, le deuxième jour",
        ),
        (Lang::Es, "el 1º y la 2ª", "el primero y la segunda"),
        (Lang::It, "il 2º", "il secondo"),
        (Lang::Pt, "a 2ª vez", "a segunda vez"),
        (Lang::Ca, "el 1r i el 2n", "el primer i el segon"),
        (Lang::Nl, "de 1e en de 2de", "de eerste en de tweede"),
        (Lang::Sv, "1:a och 3:e", "första och tredje"),
        (Lang::EnUs, "3rd and 21st", "third and twenty first"),
    ];
    for (l, t, want) in cases {
        let n = normalize(p(), t, *l, &NormOptions::default());
        assert_eq!(n.text, *want, "{l:?} {t}");
    }
    let n = normalize(p(), "le 1er", Lang::Fr, &NormOptions::default());
    assert!(
        n.spans.iter().any(|s| s.kind == SpanKind::Ordinal),
        "{:?}",
        n.spans
    );
}

#[test]
fn written_number_fixes() {
    assert_eq!(norm(Lang::Fr, "200 000"), "deux cent mille");
    assert_eq!(norm(Lang::Fr, "80 000"), "quatre-vingt mille");
    assert_eq!(norm(Lang::Fr, "200 et 80"), "deux cents et quatre-vingts");
    assert_eq!(norm(Lang::Fr, "200 000 000"), "deux cents millions");
    assert_eq!(norm(Lang::De, "2.5"), "zwei Komma fünf");
    assert_eq!(norm(Lang::Hu, "2,5"), "kettő egész öt tized");
    assert_eq!(norm(Lang::It, "3,25"), "tre virgola venticinque");
    assert_eq!(norm(Lang::Pt, "3,25"), "três vírgula vinte e cinco");
    assert_eq!(norm(Lang::Cs, "3,25"), "tři celá dvacet pět");
}

#[test]
fn vietnamese_letters_stay_inside_their_word() {
    for l in [Lang::EnUs, Lang::Es, Lang::Fr, Lang::De] {
        let o = p().phonemize("Nguyễn Phạm", &Options::new(l));
        assert_eq!(o.split(' ').count(), 2, "{l:?}: {o}");
    }
}
