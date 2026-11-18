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
fn money_with_cents() {
    let cases: &[(Lang, &str, &str)] = &[
        (
            Lang::De,
            "Es kostet 25,50 €.",
            "Es kostet fünfundzwanzig Euro fünfzig.",
        ),
        (Lang::De, "€25,50", "fünfundzwanzig Euro fünfzig"),
        (
            Lang::De,
            "1.250,05 €",
            "eintausend zweihundertfünfzig Euro fünf",
        ),
        (Lang::De, "3,00 €", "drei Euro"),
        (Lang::Es, "25,50 €", "veinticinco euros cincuenta"),
        (Lang::Fr, "25,50 €", "vingt-cinq euros cinquante"),
        (Lang::It, "25,50 €", "venticinque euro cinquanta"),
        (Lang::Pt, "25,50 €", "vinte e cinco euros cinquenta"),
        (Lang::Nl, "25,50 €", "vijfentwintig euro vijftig"),
        (Lang::EnUs, "$25.50", "twenty five dollars and fifty cents"),
        (Lang::De, "Am 3. Mai", "Am dritten Mai"),
    ];
    for (l, t, want) in cases {
        assert_eq!(norm(*l, t), *want, "{l:?} {t}");
    }
    let n = normalize(p(), "25,50 €", Lang::De, &NormOptions::default());
    assert!(
        n.spans.iter().any(|s| s.kind == SpanKind::Money),
        "{:?}",
        n.spans
    );
    let money = ipa(Lang::De, Mode::Improved, "25,50 €");
    assert!(
        money.contains(&uccharan::profile::nfd("ˈɔʏʁo fˈʏnftsɪç")),
        "{money}"
    );
    // Compatible mode keeps the reference reading.
    let money = ipa(Lang::De, Mode::Compatible, "25,50 €");
    assert!(money.contains(&uccharan::profile::nfd("kɔmɑː")), "{money}");
}

#[test]
fn vietnamese_letters_stay_inside_their_word() {
    for l in [Lang::EnUs, Lang::Es, Lang::Fr, Lang::De] {
        let o = p().phonemize("Nguyễn Phạm", &Options::new(l));
        assert_eq!(o.split(' ').count(), 2, "{l:?}: {o}");
    }
}

#[test]
fn clock_times_are_read_as_times() {
    let cases: &[(Lang, &str, &str)] = &[
        (Lang::Fr, "Il est 10h30.", "Il est dix heures trente."),
        (Lang::Fr, "à 10h", "à dix heures"),
        (Lang::Fr, "à 10 h 30", "à dix heures trente"),
        (Lang::Fr, "à 10h 30", "à dix heures trente"),
        (Lang::Fr, "à 1h05", "à une heure cinq"),
        (Lang::Fr, "à 21h", "à vingt et une heures"),
        (Lang::Fr, "à 10:30", "à dix heures trente"),
        (Lang::Pt, "às 10h30", "às dez horas e trinta"),
        (Lang::Pt, "às 1h", "às uma hora"),
        (Lang::Pt, "às 22h15", "às vinte e duas horas e quinze"),
        (Lang::PtBr, "às 10h30", "às dez horas e trinta"),
        (Lang::Es, "a las 10h30", "a las diez horas y treinta"),
        (Lang::Es, "a las 10:30", "a las diez treinta"),
        (Lang::Ca, "a les 10h30", "a les deu hores i trenta"),
        (Lang::De, "um 10.30 Uhr", "um zehn Uhr dreißig"),
        (Lang::De, "um 10:30 Uhr", "um zehn Uhr dreißig"),
        (Lang::De, "um 10.30", "um zehn Uhr dreißig"),
        (Lang::De, "um 1 Uhr", "um ein Uhr"),
        (Lang::De, "um 10:05", "um zehn Uhr fünf"),
        (Lang::Nl, "om 10.30 uur", "om tien uur dertig"),
        (
            Lang::Fi,
            "Kello on 10.30.",
            "Kello on kymmenen kolmekymmentä.",
        ),
        (Lang::Fi, "klo 10.05", "klo kymmenen nolla viisi"),
        (Lang::Sv, "kl 10.30", "kl tio trettio"),
        (Lang::Da, "kl 10.30", "kl ti tredive"),
        (Lang::Nb, "kl 10.30", "kl ti tretti"),
        (Lang::It, "alle 10.30", "alle dieci trenta"),
        (Lang::It, "alle 10:00", "alle dieci"),
    ];
    for (l, t, want) in cases {
        assert_eq!(norm(*l, t), *want, "{l:?} {t}");
    }
    let n = normalize(p(), "à 10 h 30", Lang::Fr, &NormOptions::default());
    let s = n.spans.iter().find(|s| s.kind == SpanKind::Time).unwrap();
    assert_eq!(&"à 10 h 30"[s.src.clone()], "10 h 30");
    // Improved mode says the hour word; compatible mode reads "h" as the
    // reference does.
    let imp = ipa(Lang::Fr, Mode::Improved, "Il est 10h30.");
    assert!(imp.contains(&uccharan::profile::nfd("ˈœʁ")), "{imp}");
    let compat = ipa(Lang::Fr, Mode::Compatible, "Il est 10h30.");
    assert!(compat.contains(&uccharan::profile::nfd("ˈaʃ")), "{compat}");
    let imp = ipa(Lang::Pt, Mode::Improved, "às 10h30");
    assert!(imp.contains(&uccharan::profile::nfd("ˈɔɾɐz")), "{imp}");
}

#[test]
fn a_number_at_the_end_of_a_sentence_is_a_cardinal() {
    let cases: &[(Lang, &str, &str)] = &[
        (
            Lang::De,
            "Ich kaufe das iPhone 15. Am Montag kommt es.",
            "Ich kaufe das iPhone fünfzehn. Am Montag kommt es.",
        ),
        (
            Lang::De,
            "Das sind 15. Der Rest fehlt.",
            "Das sind fünfzehn. Der Rest fehlt.",
        ),
        (Lang::De, "der 21. Sieger", "der einundzwanzigste Sieger"),
        (Lang::De, "Am 3. Mai.", "Am dritten Mai."),
        (
            Lang::De,
            "Er kam als 3. ins Ziel.",
            "Er kam als dritter ins Ziel.",
        ),
        (
            Lang::Da,
            "Jeg har en iPhone 15. Den var dyr.",
            "Jeg har en iPhone femten. Den var dyr.",
        ),
        (
            Lang::Nb,
            "Jeg har en iPhone 15. Den var dyr.",
            "Jeg har en iPhone femten. Den var dyr.",
        ),
        (
            Lang::Fi,
            "Ostin iPhone 15. Se oli kallis.",
            "Ostin iPhone viisitoista. Se oli kallis.",
        ),
        (
            Lang::Hu,
            "Vettem egy iPhone 15. A telefon jó.",
            "Vettem egy iPhone tizenöt. A telefon jó.",
        ),
        (
            Lang::Hu,
            "2024. május 15. A nap szép.",
            "kétezerhuszonnegyedik május tizenötödik. A nap szép.",
        ),
        (
            Lang::Cs,
            "Koupil jsem iPhone 15. To je dobré.",
            "Koupil jsem iPhone patnáct. To je dobré.",
        ),
        (
            Lang::Pl,
            "Kupiłem iPhone 15. To był błąd.",
            "Kupiłem iPhone piętnaście. To był błąd.",
        ),
        (
            Lang::Tr,
            "iPhone 15. Bu pahalı.",
            "iPhone on beş. Bu pahalı.",
        ),
    ];
    for (l, t, want) in cases {
        assert_eq!(norm(*l, t), *want, "{l:?} {t}");
    }
    let imp = ipa(Lang::De, Mode::Improved, "iPhone 15. Am Montag.");
    assert!(imp.contains(&uccharan::profile::nfd("fˈʏnftsen.")), "{imp}");
}

#[test]
fn a_full_stop_before_two_digits_does_not_group_thousands() {
    // Compatible mode: as the reference, the full stop is read.
    let fi = ipa(Lang::Fi, Mode::Compatible, "talo 10.30 talo");
    assert!(fi.contains("iste"), "{fi}");
    assert!(!fi.contains("tˈuhat"), "{fi}");
    let de = ipa(Lang::De, Mode::Compatible, "Haus 10.30 Haus");
    assert_eq!(
        de,
        uccharan::profile::nfd("hˈaʊs tsˈeːn pʊŋkt dɾˈaɪsɪç hˈaʊs")
    );
    // Three digits still make a thousand.
    assert_eq!(norm(Lang::Fi, "1.000"), "tuhat");
    assert_eq!(norm(Lang::De, "10.300"), "zehntausend dreihundert");
    // Improved mode and normalise-only mode: a time or a decimal.
    assert_eq!(norm(Lang::Fi, "10.30"), "kymmenen kolmekymmentä");
    assert_eq!(
        norm(Lang::Fi, "2.75"),
        "kaksi pilkku seitsemänkymmentäviisi"
    );
    assert_eq!(norm(Lang::De, "2.45"), "zwei Komma vier fünf");
}

#[test]
fn ordinals_before_a_hyphen() {
    let cases: &[(Lang, &str, &str)] = &[
        (Lang::EnUs, "the 3rd-floor flat", "the third floor flat"),
        (
            Lang::EnUs,
            "a 21st-century idea",
            "a twenty first century idea",
        ),
        (Lang::EnGb, "a 2nd-hand car", "a second hand car"),
        (Lang::Fr, "le 1er-mai", "le premier mai"),
        (Lang::Fr, "le 2e-étage", "le deuxième étage"),
        (Lang::Es, "el 2º-piso", "el segundo piso"),
        (Lang::Nl, "de 3e-klas", "de derde klas"),
    ];
    for (l, t, want) in cases {
        assert_eq!(norm(*l, t), *want, "{l:?} {t}");
    }
    // Compatible mode: the reference reads the ordinal and joins the word.
    let en = ipa(Lang::EnUs, Mode::Compatible, "the 3rd-floor flat");
    assert!(en.contains(&uccharan::profile::nfd("θˈɜːdflˈɔːɹ")), "{en}");
    let imp = ipa(Lang::EnUs, Mode::Improved, "the 3rd-floor flat");
    assert!(imp.contains(&uccharan::profile::nfd("θˈɜːd")), "{imp}");
    assert!(!imp.contains("ɑːɹdˈiː"), "{imp}");
}

#[test]
fn currency_words_agree_with_the_amount() {
    let cases: &[(Lang, &str, &str)] = &[
        (Lang::Fr, "25 €", "vingt-cinq euros"),
        (Lang::Fr, "1 €", "un euro"),
        (Lang::Fr, "1 £", "une livre"),
        (Lang::Fr, "25 $", "vingt-cinq dollars"),
        (Lang::Fr, "0,50 €", "zéro euro cinquante"),
        (Lang::Es, "25 €", "veinticinco euros"),
        (Lang::Es, "1 €", "un euro"),
        (Lang::Es, "21 €", "veintiún euros"),
        (Lang::Es, "3 $", "tres dólares"),
        (Lang::Es, "1 £", "una libra"),
        (Lang::It, "25 €", "venticinque euro"),
        (Lang::It, "1 $", "un dollaro"),
        (Lang::It, "3 $", "tre dollari"),
        (Lang::Pt, "25 €", "vinte e cinco euros"),
        (Lang::Pt, "2 £", "duas libras"),
        (Lang::Pt, "1 ¢", "um centavo"),
        (Lang::PtBr, "25 €", "vinte e cinco euros"),
        (Lang::Ca, "25 €", "vint-i-cinc euros"),
        (Lang::Ca, "1 £", "una lliura"),
        (Lang::Ro, "25 $", "douăzeci și cinci de dolari"),
        (Lang::Ro, "5 $", "cinci dolari"),
        (Lang::Ro, "1 €", "un euro"),
        (Lang::De, "25 €", "fünfundzwanzig Euro"),
        (Lang::De, "1 €", "ein Euro"),
        (Lang::De, "3 ₹", "drei Rupien"),
        (Lang::Nl, "5 €", "vijf euro"),
        (Lang::Sv, "1 €", "en euro"),
        (Lang::Fi, "25 €", "kaksikymmentäviisi euroa"),
        (Lang::Fi, "1 €", "yksi euro"),
        (Lang::Cs, "3 $", "tři dolary"),
        (Lang::Cs, "25 €", "dvacet pět eur"),
        (Lang::Cs, "1 €", "jedno euro"),
        (Lang::Pl, "5 $", "pięć dolarów"),
        (Lang::Pl, "22 $", "dwadzieścia dwa dolary"),
        (Lang::Pl, "12 $", "dwanaście dolarów"),
        (Lang::EnUs, "25 €", "twenty five euros"),
        (Lang::EnUs, "1 €", "one euro"),
        (Lang::EnUs, "5 ¢", "five cents"),
        (Lang::Es, "12,50 €", "doce euros cincuenta"),
    ];
    for (l, t, want) in cases {
        assert_eq!(norm(*l, t), *want, "{l:?} {t}");
    }
    let imp = ipa(Lang::Fr, Mode::Improved, "Ça coûte 25 €.");
    assert!(imp.contains(&uccharan::profile::nfd("øʁˈo")), "{imp}");
    // Compatible mode keeps the reference reading.
    let compat = ipa(Lang::Fr, Mode::Compatible, "Ça coûte 25 €.");
    assert!(compat.contains(&uccharan::profile::nfd("əʁˌo")), "{compat}");
    let compat = ipa(Lang::EnUs, Mode::Compatible, "5 ¢");
    assert!(compat.contains("s"), "{compat}");
}

#[test]
fn improved_mode_reads_south_indian_numbers_in_written_words() {
    // Tamil, Kannada, Telugu and Malayalam read amounts with the written
    // number words (as normalize() does), not the table word by word
    // ("ஒன்று ஆயிரம் இரண்டு நூறு"), and say the rupee sign and percent.
    let cases: &[(Lang, &str, &str)] = &[
        (
            Lang::Ta,
            "₹1,250",
            "ˈaːjiɹˌʌttʉ ˌiɹʉnˌʉʈrʉ ˌaɪmbʌdʉ ɹˈuːbaːj",
        ),
        (Lang::Ta, "10%", "pˈʌttʉ sˈʌdʌvˌiːdʌm"),
        (
            Lang::Kn,
            "₹1,250",
            "sˈaːviɹˌɐdɐ ˈinnuːɹɐ ˈaɪvɐttu ɹˈuːpaːji",
        ),
        (
            Lang::Te,
            "₹1,250",
            "ʋˈejji rˌeɳɖu ʋˌandala jˌaːbʰaɪ rˈuːpaːjˌalu",
        ),
        (Lang::Ml, "₹1,250", "ˈaːjiɾˌɐtːi ˈiɾunˌuːʈʈi ˈɐnbəd ɾˈuːbɐ"),
    ];
    for &(lang, text, want) in cases {
        let want = uccharan::profile::nfd(want);
        assert_eq!(ipa(lang, Mode::Improved, text), want, "{lang:?} {text}");
    }
    // The other Indian languages keep their number table and gain the rupee.
    let bn = ipa(Lang::Bn, Mode::Improved, "₹500");
    assert!(bn.ends_with(&uccharan::profile::nfd("ʈˈaka")), "{bn}");
    // Compatible mode keeps the reference voices' reading.
    let ta = ipa(Lang::Ta, Mode::Compatible, "₹1,250");
    assert_eq!(
        ta,
        uccharan::profile::nfd("ˈonrʉ ˌaːjiɹʌm ˌiɹʌɳɖʉ nˌuːrʉ ˌaɪmbʌdʉ")
    );
}

#[test]
fn hindi_clock_times_before_baje_say_baje_once() {
    for mode in [Mode::Compatible, Mode::Improved] {
        for text in ["3:30 बजे", "3:20 बजे", "4:00 बजे"] {
            let out = ipa(Lang::Hi, mode, text);
            assert!(
                out.matches("bˈʌɟeː").count() <= 1 && !out.contains("mˈɪnəʈ bˈʌɟeː"),
                "{mode:?} {text}: {out}"
            );
        }
    }
    assert!(ipa(Lang::Hi, Mode::Compatible, "3:30 बजे").ends_with("tˈiːn bˈʌɟeː"));
    assert_eq!(norm(Lang::Hi, "3:30 बजे"), "साढ़े तीन बजे");
    assert_eq!(norm(Lang::Hi, "3:20 बजे"), "तीन बजकर बीस मिनट");
    assert_eq!(norm(Lang::Hi, "4:00 बजे"), "चार बजे");
}

#[test]
fn english_month_abbreviations_before_a_day() {
    assert_eq!(
        norm(Lang::EnUs, "Jan 5, 2024"),
        "January five, twenty twenty four"
    );
    assert_eq!(norm(Lang::EnUs, "Mar 3rd"), "March third");
    assert_eq!(norm(Lang::EnUs, "Sept 10"), "September ten");
    // Not before a day, or not capitalised: left alone.
    assert_eq!(norm(Lang::EnUs, "Jan is here"), "Jan is here");
    assert_eq!(norm(Lang::EnUs, "mar 3"), "mar three");
    assert_eq!(norm(Lang::EnUs, "Dec 32"), "Dec thirty two");
    for mode in [Mode::Compatible, Mode::Improved] {
        let out = ipa(Lang::EnUs, mode, "Jan 5");
        assert!(out.starts_with("dʒˈænjuː"), "{mode:?}: {out}");
    }
}

#[test]
fn unknown_language_lists_every_tag() {
    let m = Lang::unknown_message("xx");
    assert!(m.contains("\"xx\""), "{m}");
    for l in Lang::ALL {
        assert!(m.contains(l.tag()), "{m} lacks {}", l.tag());
    }
}
