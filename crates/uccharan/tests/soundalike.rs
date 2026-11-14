//! Sound-alike matching through the public API.

use uccharan::soundalike::{ApplyOptions, IndexOptions, MatchOptions, SoundIndex, Term};
use uccharan::wordlist::WordList;
use uccharan::{Lang, Phonemizer};

const NAMES: &[&str] = &[
    "Kothari",
    "Agarwal",
    "Natarajan",
    "Kalpana",
    "Mishra",
    "Nigam",
    "Vistara",
    "Kibana",
    "Paytm",
    "Zomato",
    "Flipkart",
    "Srivastava",
    "Chatterjee",
    "Kulkarni",
    "Deshpande",
    "Venkataraman",
];

#[test]
fn recogniser_mishearings_are_put_back() {
    let p = Phonemizer::new();
    let ix = SoundIndex::from_terms(&p, NAMES, &IndexOptions::default());
    assert_eq!(ix.len(), NAMES.len());
    assert!(ix.skipped().is_empty());
    let o = ApplyOptions::default();
    for (heard, want) in [
        (
            "Please send the invoice to Mr. Nutter Rajan by Friday.",
            "Please send the invoice to Mr. Natarajan by Friday.",
        ),
        (
            "Can you ask Calpana to send the file?",
            "Can you ask Kalpana to send the file?",
        ),
        (
            "Dr. Mischra will see you at 4.",
            "Dr. Mishra will see you at 4.",
        ),
        (
            "Dr. Nigham will see you at four.",
            "Dr. Nigam will see you at four.",
        ),
        (
            "I ordered from Zomatto and Flip Cart.",
            "I ordered from Zomato and Flipkart.",
        ),
        (
            "Mr. Kulkarni's order and Miss Chatterji's file",
            "Mr. Kulkarni's order and Miss Chatterjee's file",
        ),
    ] {
        let a = ix.apply(&p, heard, Lang::EnUs, &o);
        assert_eq!(a.text, want, "{heard}: {:?}", a.candidates);
        // Every change is reported with ranges into both texts.
        let mut rebuilt = String::new();
        let mut pos = 0;
        for c in &a.changes {
            assert_eq!(&heard[c.src.clone()], c.from);
            assert_eq!(&a.text[c.out.clone()], c.to);
            assert_eq!(ix.terms()[c.term].word, c.to);
            rebuilt.push_str(&heard[pos..c.src.start]);
            rebuilt.push_str(&c.to);
            pos = c.src.end;
        }
        rebuilt.push_str(&heard[pos..]);
        assert_eq!(rebuilt, a.text);
    }
}

#[test]
fn ordinary_sentences_are_not_touched() {
    let p = Phonemizer::new();
    let ix = SoundIndex::from_terms(&p, NAMES, &IndexOptions::default());
    let o = ApplyOptions::default();
    for (text, lang) in [
        ("Can you send the report by Monday?", Lang::EnUs),
        (
            "The weather was fine, so we walked to the station at the end of the day.",
            Lang::EnUs,
        ),
        (
            "I shall stand behind this crate, and do you conceal yourselves behind those.",
            Lang::EnUs,
        ),
        ("आज मौसम बहुत अच्छा है और हम लोग बाहर जा रहे हैं।", Lang::Hi),
        ("मुझे कल सुबह दफ़्तर जाना है।", Lang::Hi),
    ] {
        let a = ix.apply(&p, text, lang, &o);
        assert!(a.changes.is_empty(), "{text}: {:?}", a.changes);
        assert_eq!(a.text, text);
    }
}

#[test]
fn both_directions_across_scripts() {
    let p = Phonemizer::new();
    let latin = SoundIndex::from_terms(&p, NAMES, &IndexOptions::default());
    let a = latin.apply(
        &p,
        "चटर्जी जी और कुलकर्णी जी आ रहे हैं।",
        Lang::Hi,
        &ApplyOptions::default(),
    );
    assert_eq!(
        a.text, "Chatterjee जी और Kulkarni जी आ रहे हैं।",
        "{:?}",
        a.candidates
    );
    let deva = SoundIndex::from_terms(&p, &["कुलकर्णी", "नटराजन", "देशपांडे"], &IndexOptions::default());
    let a = deva.apply(
        &p,
        "Mr. Kulkarni and Mr. Deshpanday called.",
        Lang::EnUs,
        &ApplyOptions::default(),
    );
    assert_eq!(
        a.text, "Mr. कुलकर्णी and Mr. देशपांडे called.",
        "{:?}",
        a.candidates
    );
}

#[test]
fn word_list_sounds_and_spoken_forms() {
    let p = Phonemizer::new();
    let list = WordList::parse_tsv(
        "# uccharan wordlist 1\nword\tlang\tipa\tsay\tmatch\nkubectl\ten\t\tcube control\t\nJio\ten\tdʒˈiːoʊ\t\t\nMH\\d+\t*\t\tmaharashtra\tregex\n",
    )
    .unwrap();
    let ix = SoundIndex::from_wordlists(&p, &[&list], &IndexOptions::default());
    // The regular-expression row is not a term.
    assert_eq!(ix.len(), 2);
    assert_eq!(ix.sounds(1), vec!["dʒiːoː".to_string()]);
    // "Geo" is a word: it is taken for the term only with a lower bar.
    let a = ix.apply(
        &p,
        "run cube control and call Geo",
        Lang::EnUs,
        &ApplyOptions::default(),
    );
    assert_eq!(a.text, "run kubectl and call Geo");
    let eager = ApplyOptions {
        uncertain_threshold: 0.8,
        ..ApplyOptions::default()
    };
    let a = ix.apply(&p, "run cube control and call Geo", Lang::EnUs, &eager);
    assert_eq!(a.text, "run kubectl and call Jio");
    // Built in code, with a term per language.
    let terms = vec![
        Term {
            word: "Zerodha".into(),
            lang: Some(Lang::EnUs),
            ipa: Some("zɪɹˈoʊdɑː".into()),
            say: None,
        },
        Term::new("Swiggy"),
    ];
    let ix = SoundIndex::build(&p, terms, &IndexOptions::default());
    let a = ix.apply(
        &p,
        "I use Zeroda and Swiggie",
        Lang::EnUs,
        &ApplyOptions::default(),
    );
    assert_eq!(a.text, "I use Zerodha and Swiggy", "{:?}", a.candidates);
}

#[test]
fn candidates_thresholds_and_confidence() {
    let p = Phonemizer::new();
    let ix = SoundIndex::from_terms(&p, NAMES, &IndexOptions::default());
    let text = "Mr. Not Regen called about the Vistara ticket";
    let found = ix.find(&p, text, Lang::EnUs, &MatchOptions::default());
    let exact: Vec<_> = found.iter().filter(|c| c.exact).collect();
    assert_eq!(exact.len(), 1);
    assert_eq!(&text[exact[0].src.clone()], "Vistara");
    let near = found
        .iter()
        .find(|c| ix.terms()[c.term].word == "Natarajan")
        .expect("a candidate for the misheard name");
    assert!(!near.exact);
    assert_eq!(&text[near.src.clone()], "Not Regen");
    assert_eq!(near.words, 1..3);
    assert!(near.score > 0.6 && near.score < 0.9, "{}", near.score);
    assert_eq!(near.kind, uccharan::soundalike::SpanKind::Uncertain);
    // The defaults leave it; lower thresholds take it.
    assert!(ix
        .apply(&p, text, Lang::EnUs, &ApplyOptions::default())
        .changes
        .is_empty());
    let loose = ApplyOptions {
        threshold: 0.7,
        uncertain_threshold: 0.7,
        ..ApplyOptions::default()
    };
    assert_eq!(
        ix.apply(&p, text, Lang::EnUs, &loose).text,
        "Mr. Natarajan called about the Vistara ticket"
    );
    // With confidences, only spans the recogniser was unsure about change.
    let mut gated = loose.clone();
    gated.max_confidence = 0.5;
    gated.confidence = Some(vec![0.9, 0.95, 0.9, 0.9, 0.9, 0.9, 0.9, 0.9]);
    assert!(ix.apply(&p, text, Lang::EnUs, &gated).changes.is_empty());
    gated.confidence = Some(vec![0.9, 0.3, 0.4, 0.9, 0.9, 0.9, 0.9, 0.9]);
    assert_eq!(ix.apply(&p, text, Lang::EnUs, &gated).changes.len(), 1);
    // Every span and term, for callers that rank themselves.
    let all = MatchOptions {
        all: true,
        min_score: 0.5,
        ..MatchOptions::default()
    };
    assert!(ix.find(&p, text, Lang::EnUs, &all).len() > found.len());
}

#[test]
fn a_large_vocabulary_finds_the_same_terms() {
    // 6 000 terms: the pre-filter must not lose what the full search finds.
    let p = Phonemizer::new();
    let first = [
        "Aarav",
        "Deepak",
        "Gaurav",
        "Kalpana",
        "Lakshmi",
        "Meenakshi",
        "Pranav",
        "Rohit",
        "Shweta",
        "Vikram",
    ];
    let last = [
        "Kothari",
        "Agarwal",
        "Natarajan",
        "Mishra",
        "Chatterjee",
        "Kulkarni",
        "Deshpande",
        "Iyer",
        "Reddy",
        "Srivastava",
    ];
    let mut terms: Vec<String> = Vec::new();
    for a in first {
        for b in last {
            terms.push(format!("{a} {b}"));
        }
    }
    let syllables = [
        "ka", "ri", "mo", "tan", "shu", "vel", "dra", "pi", "gho", "lam", "ne", "yas",
    ];
    'fill: for a in syllables {
        for b in syllables {
            for c in syllables {
                for d in ["", "n", "sh", "r"] {
                    if terms.len() >= 6000 {
                        break 'fill;
                    }
                    terms.push(format!("{a}{b}{c}{d}"));
                }
            }
        }
    }
    let ix = SoundIndex::from_terms(&p, &terms, &IndexOptions::default());
    assert_eq!(ix.len(), 6000);
    assert!(ix.variants() >= ix.len());
    let full = MatchOptions {
        prefilter: false,
        ..MatchOptions::default()
    };
    for text in [
        "I met Calpana Kotari and Gorav Agarwal yesterday.",
        "Shveta Chatterji and Rohit Ayer will join.",
        "The karimotan order went to Vikrum Mishra.",
    ] {
        // Compared where the match is strong; among the weak ones the
        // pre-filter may prefer another of several equally poor terms.
        let strong = |o: &MatchOptions| -> Vec<_> {
            let mut v = ix.find(&p, text, Lang::EnUs, o);
            v.retain(|c| c.score >= 0.85);
            v
        };
        let (a, b) = (strong(&MatchOptions::default()), strong(&full));
        assert_eq!(a, b, "{text}");
        assert!(!a.is_empty(), "{text}: {a:?}");
    }
}
