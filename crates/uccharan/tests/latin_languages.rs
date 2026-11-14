//! A few words and phrases per Latin-script data-driven language, written
//! from the language specifications (specs/<tag>.md), in compatible mode.

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
