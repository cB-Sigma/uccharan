//! A few words and phrases per Latin-script data-driven language, written
//! from the language descriptions, in compatible mode.

use uccharan::{Lang, Options, Phonemizer};

fn check(lang: Lang, pairs: &[(&str, &str)]) {
    let p = Phonemizer::new();
    let opt = Options::new(lang);
    for (text, want) in pairs {
        let got = uccharan::profile::nfd(&p.phonemize(text, &opt));
        assert_eq!(got, uccharan::profile::nfd(want), "{} {text}", lang.tag());
    }
}

#[test]
fn spanish() {
    check(
        Lang::Es,
        &[
            ("casa", "kˈasa"),
            ("cantó", "kantˈo"),
            ("hablar", "aβlˈaɾ"),
            ("ciudadano", "θjˌuðaðˈano"),
            ("gente", "xˈɛnte"),
            ("hay", "ˈaɪ"),
            ("el perro de la casa", "el pˈero ðe la kˈasa"),
            ("en Barcelona", "em bˌaɾθelˈona"),
            ("rápidamente", "rˈapiðamˈente"),
            ("1984", "mˈil nˌoβeθjˈɛntos otʃˌɛntaikwˈatɾo"),
            ("siglo XIX", "sˈiɣlo ðjˌeθinwˈeβe"),
        ],
    );
}

#[test]
fn german() {
    check(
        Lang::De,
        &[
            ("Krankenhaus", "kɾˈankənhˌaʊs"),
            ("verstehen", "fɛɾʃtˈeːən"),
            ("Entschuldigung", "ɛntʃˈʊldɪɡˌʊŋ"),
            ("Universität", "ˌuːniːvˌɛɾziːtˈɛːt"),
            ("Geschichte", "ɡəʃˈɪçtə"),
            ("Wie geht es Ihnen", "viː ɡˈeːt ɛs ˈiːnən"),
            ("21", "ˈaɪn ʊnttsvˈantsɪç"),
            ("1984", "ˈaɪn tˈaʊzənt nˈɔønhˈʊndɜt fˈiːɾ ʊntˈaxtsɪç"),
            ("3,14", "dɾˈaɪ kɔmɑː ˈaɪns fˈiːɾ"),
        ],
    );
    // A word outside the table, read by the rules alone.
    let l = Lang::De.latin().unwrap();
    assert_eq!(
        l.rule_word("Regenbogenfarben", true),
        "rˈeːɡənbˌoːɡənfˌaɾbən"
    );
}

#[test]
fn dutch() {
    check(
        Lang::Nl,
        &[
            ("gelopen", "ɣəlˈoːpən"),
            ("school", "sxˈoːl"),
            ("onmogelijk", "ɔnmˈoːɣələk"),
            ("Hoe gaat het met je", "hˈu ɣˈaːt hət mɛt jə"),
            ("met de man", "mɛ tə mˈɑn"),
            ("21", "ˈeːnɛntʋˌɪntəx"),
            ("1984", "nˈeːɣəntˌinhˌɔndərt vˌirɛntˌɑxtəx"),
        ],
    );
}

#[test]
fn improved_mode_notation() {
    let p = Phonemizer::new();
    let mut opt = Options::new(Lang::De);
    opt.mode = uccharan::Mode::Improved;
    assert_eq!(p.phonemize("Mutter", &opt), "mˈʊtɐ");
}

#[test]
fn french() {
    check(
        Lang::Fr,
        &[
            ("maison", "mɛzˈɔ̃"),
            ("les amis", "le-z amˈi"),
            ("les livres", "le- lˈivʁ"),
            ("je ne sais pas", "ʒə- nə- sˈɛ pˈa"),
            ("l'école", "lekˈɔl"),
            ("un ami", "œ̃n amˈi"),
            ("21", "vɛ̃teˈœ̃"),
            ("1984", "mil nœfsɑ̃ katʁəvɛ̃kˈatʁ"),
            ("3,14", "tʁwa viʁɡyl katˈɔʁz"),
        ],
    );
    let l = Lang::Fr.latin().unwrap();
    assert_eq!(l.rule_word("ornithologique", true), "ɔʁnitoloʒˈik");
}

#[test]
fn french_liaison_numbers_and_variants() {
    check(
        Lang::Fr,
        &[
            // h aspiré blocks liaison; h muet links.
            ("les haricots sont verts", "le- aʁikˈo sˈɔ̃ vˈɛʁ"),
            ("les hôtels sont pleins", "le-z otˈɛl sˈɔ̃ plˈɛ̃"),
            ("les huit enfants", "le- yˈit ɑ̃fˈɑ̃"),
            ("le onze novembre", "lə- ˈɔ̃z novˈɑ̃bʁ"),
            // Number words before a vowel, a consonant and at the end.
            ("six enfants et six livres", "sˈiz ɑ̃fˈɑ̃z e sˈi lˈivʁ"),
            ("neuf ans", "nˈœv ˈɑ̃"),
            // sont / ont after a clitic pronoun.
            ("ils ont faim", "ilz ɔ̃ fˈɛ̃"),
            ("les enfants ont faim", "le-z ɑ̃fˈɑ̃z ˈɔ̃ fˈɛ̃"),
            // A space between groups of three digits.
            ("1 000 000 euros", "œ̃ miljˈɔ̃ øʁˈo"),
            ("de 11\u{a0}000 euros", "də- ˈɔ̃z zeʁozeʁo zeʁˈo øʁˈo"),
        ],
    );
    check(
        Lang::FrBe,
        &[
            ("70 ans", "sɛptˈɑ̃t ˈɑ̃"),
            ("80", "katʁəvˈɛ̃"),
            ("90", "nonˈɑ̃t"),
        ],
    );
    check(Lang::FrCh, &[("80 ans", "yitˈɑ̃t ˈɑ̃"), ("96", "nonɑ̃tsˈis")]);
    let p = Phonemizer::new();
    let mut opt = Options::new(Lang::Fr);
    opt.mode = uccharan::Mode::Improved;
    let got = p.phonemize("de 11\u{a0}000 euros", &opt);
    assert!(got.contains("mˈil"), "{got}");
}

#[test]
fn italian() {
    check(
        Lang::It,
        &[
            ("delle", "dˈelle"),
            ("questa", "kwˈesta"),
            ("tutti", "tˈutːɪ"),
            ("prima", "prˈima"),
            ("lavoro", "lavˈɔro"),
            ("qualcosa", "kwalkˈɔza"),
            ("vanno", "vˈanno"),
            ("parte", "pˈarte"),
            ("21", "ventˈuno"),
        ],
    );
}

#[test]
fn portuguese_brazil() {
    check(
        Lang::PtBr,
        &[
            ("fazer", "fazˈer"),
            ("estava", "estˌavæ"),
            ("estão", "estˌɐ̃ʊ̃"),
            ("tenho", "tˌeɲʊ"),
            ("grande", "ɡrˈɐ̃ŋdʒy"),
            ("trabalho", "trˌabˈaljʊ"),
            ("apenas", "ˌapˈenæs"),
            ("maria", "mˌaɾˈiæ"),
            ("21", "vˈiŋtʃyiˈũŋ"),
        ],
    );
}

#[test]
fn portuguese_portugal() {
    check(
        Lang::Pt,
        &[
            ("ainda", "ˌɐˈiŋdɐ"),
            ("todos", "tˈodʊʃ"),
            ("pessoas", "pˌɨsˈoɐʃ"),
            ("minha", "mˈiɲɐ"),
            ("sempre", "sˈeɪmpɹɨ"),
            ("governo", "ɡˌuvˈeɾənʊ"),
            ("grande", "ɡɹˈɐ̃ŋdɨ"),
            ("mundo", "mˈũŋdʊ"),
            ("21", "vˈiŋtɨiˈum"),
        ],
    );
}

#[test]
fn polish() {
    check(
        Lang::Pl,
        &[
            ("także", "tˈaɡʒɛ"),
            ("jeszcze", "jˈɛʃtʃɛ"),
            ("polski", "pˈɔlski"),
            ("teraz", "tˈɛras"),
            ("dzieci", "dʑˈɛtɕi"),
            ("wszystko", "fʃˈɨstkɔ"),
            ("którym", "ktˈurɨm"),
            ("dzień", "dʑˈɛɲ"),
            ("21", "dvadʑˈɛɕtɕajˈɛdɛn"),
        ],
    );
}

#[test]
fn turkish() {
    check(
        Lang::Tr,
        &[
            ("mary'nin", "marjnˈɪn"),
            ("devam", "devˈam"),
            ("bugün", "buɡˈøn"),
            ("tom'a", "tomˈa"),
            ("bunun", "bunˈʊn"),
            ("şimdi", "ʃɪmdˈɪ"),
            ("önemli", "œnæmlˈɪ"),
            ("verdi", "vɛrdˈɪ"),
            ("21", "jirmˌibˈir"),
        ],
    );
}

#[test]
fn czech() {
    check(
        Lang::Cs,
        &[
            ("která", "ktˈeraː"),
            ("kvůli", "kvˈuːli"),
            ("velmi", "vˈelmi"),
            ("například", "nˈapr̝̊iːklat"),
            ("české", "tʃˈeskeː"),
            ("letech", "lˈetex"),
            ("navíc", "nˈaviːts"),
            ("tento", "tˈento"),
            ("21", "dvˈatsetjˈeden"),
        ],
    );
}

#[test]
fn swedish() {
    check(
        Lang::Sv,
        &[
            ("sedan", "sˈeːdan"),
            ("skulle", "skˌɵlə"),
            ("flera", "flˈeːra"),
            ("bland", "blˈand"),
            ("mellan", "mˈɛlan"),
            ("stora", "stˈuːra"),
            ("redan", "rˈeːdan"),
            ("huset", "hˈʉsət"),
            ("21", "ɕˌʉɡɔˈɛt"),
        ],
    );
}

#[test]
fn catalan() {
    check(
        Lang::Ca,
        &[
            ("aquesta", "əkˈɛstə"),
            ("segons", "səɣˈons"),
            ("sense", "sˈensə"),
            ("contra", "kˈontɾə"),
            ("altres", "ˈaltɾəs"),
            ("moment", "mumˈen"),
            ("milions", "miliˈons"),
            ("21", "bˌintiˈu"),
            ("el ben", "əl βˈen"),
        ],
    );
}

#[test]
fn romanian() {
    check(
        Lang::Ro,
        &[
            ("acest", "atʃˈest"),
            ("românia", "rˌomɨnˈia"),
            ("toate", "tˈɔate"),
            ("potrivit", "pˌotrivˈit"),
            ("fiind", "fiˈind"),
            ("putin", "putˈin"),
            ("persoane", "persˈɔane"),
            ("există", "eɡzˌistə"),
            ("21", "dˈouˌəzetʃʲʃiˈunu"),
        ],
    );
}

#[test]
fn hungarian() {
    check(
        Lang::Hu,
        &[
            ("három", "hˈaːrom"),
            ("egész", "ˈɛɡeːs"),
            ("saját", "ʃˈɑjaːt"),
            ("21", "hˈusonɛɟ"),
            ("1984", "ˈɛzɛr kilɛntssaːz ɲoltsvɑnneːɟ"),
            ("és százalék", "ˌeːʃ sˈaːzɑleːk"),
            // 2,000 as one word; from 10,000 one stress per group of digits.
            ("2024", "kˈeːtɛzɛr husonneːɟ"),
            ("15300", "tˈizɛnøt ɛzɛr hˈaːromsaːz"),
            (
                "1234567",
                "ˈɛɟmillioː kˈeːtsaːz hɑrmintsneːɟ ɛzɛr ˈøtsaːz hɑtvɑnheːt",
            ),
            // Secondary stress at the end of a clause, and the article before it.
            ("A város", "ˈɑ vˌaːroʃ"),
            ("Megjött a tavasz", "mˈɛɡjøtː ˌɑ tˈɑvɑs"),
        ],
    );
}

#[test]
fn finnish() {
    check(
        Lang::Fi,
        &[
            ("olisi", "ˌolɪsɪ"),
            ("venäjän", "vˈenæjæn"),
            ("aikana", "ˈaikana"),
            ("sillä", "sˈillæ"),
            ("kaksi", "kˈaksɪ"),
            ("prosenttia", "prˈosentːˌia"),
            ("suomessa", "sˈuomessa"),
            ("siihen", "sˈiːhen"),
            ("21", "kˌaksikymmentæˈyksi"),
        ],
    );
}

#[test]
fn norwegian() {
    check(
        Lang::Nb,
        &[
            ("første", "fˈœrʃta"),
            ("blant", "blˈɑnt"),
            ("mellom", "mˈɛlluːm"),
            ("mange", "mˈɑŋa"),
            ("drept", "drˈɛpt"),
            ("samme", "sˈɑmma"),
            ("største", "stˈœrʃta"),
            ("senere", "sˈeːneːra"),
            ("21", "ʂˈʉːaˈeːn"),
        ],
    );
}

#[test]
fn danish() {
    check(
        Lang::Da,
        &[
            ("bliver", "blˈʔiɐ̯"),
            ("havde", "hˈaðə"),
            ("noget", "nˈɒwð"),
            ("danske", "dˈanskə"),
            ("blandt", "blˈant"),
            ("tidligere", "tˈʔiðliˌʌʔʌ"),
            ("tilbage", "telbˈajə"),
            ("denne", "dˈεnə"),
            ("21", "ˈeːnʌtˈyːʔu"),
        ],
    );
}

/// Roman numerals and short capitals follow each voice in compatible mode:
/// cardinals (Spanish, Portuguese, Czech), "römisch" and "romain" (German,
/// French), ordinals (Italian, Danish), or letters (Swedish, Dutch, Finnish
/// with the main stress first, Turkish with its dotless I).
#[test]
fn roman_numerals_and_capitals() {
    check(
        Lang::Es,
        &[
            ("Carlos IV fue rey", "kˈaɾlos kwˈatɾo fwˈe rˈeɪ"),
            ("el plan B es mejor", "el plˈam bˈe ˈes mexˈoɾ"),
        ],
    );
    check(
        Lang::De,
        &[
            ("Ludwig XIV war da", "lˈʊdvɪç rˌøːmɪʃ fˈɪɾtseːn vɑːɾ dˈɑː"),
            ("die USA sind groß", "diː ˌuːˌɛsˈɑː zɪnt ɡɾˈoːs"),
        ],
    );
    check(
        Lang::Fr,
        &[
            ("Louis XIV est roi", "lwˈi katɔʁz ʁomˈɛ̃ ɛ ʁwˈa"),
            ("le roi VIII dort", "lə- ʁwˈa yi ʁomˈɛ̃ dˈɔʁ"),
        ],
    );
    check(
        Lang::It,
        &[
            (
                "papa Giovanni XXIII parla",
                "pˈapa dʒovˈannɪ vˈentɪtreˈɛzimo pˈarla",
            ),
            ("nel XIX secolo", "nel ditʃannovˈɛzimo sˈɛkolo"),
        ],
    );
    check(
        Lang::Da,
        &[("Christian IV var konge", "kʁˈʔistʔiˌan fjˈeʌ ʋɑ kˈʔʌŋə")],
    );
    check(
        Lang::Pt,
        &[("o século XIX foi longo", "ʊ sˈɛkulʊ dˌɨzɐnˈɔvɨ foɪ lˈoŋɡʊ")],
    );
    check(
        Lang::Cs,
        &[("Karel IV byl král", "kˈarel tʃtˈir̝i bil krˈaːl")],
    );
    check(
        Lang::Sv,
        &[("Karl XII var kung", "kˈarl ˌɛksˌiːˈiː vɑːr kˈɵŋ")],
    );
    check(
        Lang::Nl,
        &[("Willem II was koning", "ʋˈɪləm ˌiːˈiː ʋˈɑs kˈoːnɪŋ")],
    );
    check(Lang::Fi, &[("karl XI tak", "kˈarl ˈæksˌiː tˈak")]);
    check(Lang::Tr, &[("karl II tak", "kˈarl ˌɯˈɯ tˈak")]);
    // Improved mode reads the numeral as a number everywhere.
    let p = Phonemizer::new();
    let improved = |lang, text| {
        let mut o = Options::new(lang);
        o.mode = uccharan::Mode::Improved;
        p.phonemize(text, &o)
    };
    assert_eq!(
        improved(Lang::Sv, "Karl XII var kung"),
        "kˈarl tˈɔlv vɑːr kˈɵŋ"
    );
    assert_eq!(
        improved(Lang::Fr, "Louis XIV est roi"),
        "lwˈi katˈɔʁz ɛ ʁwˈa"
    );
    // IV is four, not five; XIIII and IL are not numerals.
    assert_eq!(improved(Lang::Es, "IV"), improved(Lang::Es, "4"));
    assert_ne!(improved(Lang::Es, "IL"), improved(Lang::Es, "49"));
}

/// Letters of other alphabets in names are read as their base letters, as
/// the reference voices read them, instead of passing into the phonemes.
#[test]
fn letters_of_other_alphabets() {
    check(Lang::Es, &[("Łódź", "lˈoðθ"), ("Erdoğan", "eɾðˈoɣan")]);
    check(Lang::Fi, &[("Łódź", "lˈodts"), ("Dvořák", "dvˈorak")]);
    check(Lang::Ro, &[("Erdoğan", "ˌerdoɡˈan"), ("Łódź", "lˈodz")]);
    let p = Phonemizer::new();
    let names = "Erdoğan Łódź Dvořák Gödel Ærø Øresund Şahin Ångström Ñandú Mõisaküla Straße Þór";
    // (æ and ø are also IPA symbols: not checked.)
    let foreign = "ğłźřöşåñúõüßþ";
    for l in [
        Lang::Es,
        Lang::Fr,
        Lang::Ro,
        Lang::Fi,
        Lang::Pt,
        Lang::PtBr,
        Lang::It,
        Lang::Ca,
        Lang::Nl,
        Lang::Cs,
        Lang::Sv,
    ] {
        let got = p.phonemize(names, &Options::new(l));
        assert!(
            !got.chars().any(|c| foreign.contains(c)),
            "{}: {got}",
            l.tag()
        );
    }
    // A letter the language has keeps its own reading.
    check(Lang::Pl, &[("Łódź", "wˈutɕ")]);
    check(Lang::Tr, &[("Erdoğan", "ɛrdoːˈan")]);
}

/// Currency signs are read where they stand, as the reference voices read
/// them; Spanish reads the comma as the decimal sign.
#[test]
fn currency_signs_and_decimal_comma() {
    check(Lang::De, &[("x 5 € x", "ˈɪks fˈʏnf ˈɔøroː ˈɪks")]);
    check(Lang::Fr, &[("x $5 x", "ˈiks dɔlˈaʁ sˈɛ̃k ˈiks")]);
    check(
        Lang::Es,
        &[
            ("Tengo 3,5 kilos.", "tˈɛŋɡo tɾˈes koma θˈinko kˈilos."),
            ("Tengo 1.000 kilos.", "tˈɛŋɡo mˈil kˈilos."),
        ],
    );
    // Improved mode: the amount first, and "1 500" as one number.
    let p = Phonemizer::new();
    let mut o = Options::new(Lang::De);
    o.mode = uccharan::Mode::Improved;
    assert_eq!(p.phonemize("€5", &o), p.phonemize("5 €", &o));
    o.lang = Lang::Es;
    assert_eq!(p.phonemize("1 500", &o), p.phonemize("1500", &o));
}
