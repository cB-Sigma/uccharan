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
