//! Improved-mode readings for English homographs.
//!
//! The compatible mode reproduces the reference's one-word look-back. Here a
//! word's sense is chosen from cues anywhere in the clause: auxiliaries and
//! time words for tense, the following word for noun/verb pairs, and topic
//! words for unrelated senses ("bass" the fish, "bow" the weapon). All tables
//! are hand-written; pronunciations follow CMUdict's variants.

use super::Hint;

/// What to do with a word.
pub enum Pick {
    /// A fixed pronunciation: `(en-US, en-GB)`.
    Ipa(&'static str, &'static str),
    /// Look the word up with this part-of-speech expectation.
    Hint(Hint),
}

const VERB_CUES: &[&str] = &[
    "to",
    "will",
    "would",
    "can",
    "could",
    "shall",
    "should",
    "may",
    "might",
    "must",
    "do",
    "does",
    "did",
    "don't",
    "doesn't",
    "didn't",
    "won't",
    "can't",
    "couldn't",
    "wouldn't",
    "shouldn't",
    "let's",
    "please",
    "cannot",
    "i'll",
    "you'll",
    "we'll",
    "they'll",
    "he'll",
    "she'll",
    "it'll",
    "never",
    "always",
    "often",
    "usually",
    "sometimes",
];
const SUBJECTS: &[&str] = &["i", "you", "we", "they", "he", "she", "it", "who"];
const DETERMINERS: &[&str] = &[
    "a", "an", "the", "my", "your", "his", "her", "its", "our", "their", "this", "that", "these",
    "those", "some", "any", "no", "each", "every", "another", "one", "two", "three", "many",
    "several", "few",
];
const OBJECT_STARTS: &[&str] = &[
    "a",
    "an",
    "the",
    "my",
    "your",
    "his",
    "her",
    "their",
    "our",
    "it",
    "them",
    "him",
    "me",
    "us",
    "this",
    "that",
    "these",
    "those",
    "everything",
    "something",
    "anything",
    "nothing",
];
const PAST_AUX: &[&str] = &[
    "have", "has", "had", "having", "was", "were", "been", "be", "is", "are", "being", "i've",
    "you've", "we've", "they've", "he's", "she's", "it's", "already", "just", "ever", "haven't",
    "hasn't", "hadn't", "wasn't", "weren't", "got", "get", "gets",
];
const PAST_TIME: &[&str] = &[
    "yesterday",
    "ago",
    "earlier",
    "previously",
    "once",
    "then",
    "was",
    "were",
    "had",
    "did",
    "said",
    "went",
    "came",
];
const BE: &[&str] = &[
    "is", "are", "was", "were", "be", "been", "am", "it's", "that's", "he's", "she's", "they're",
    "we're", "i'm", "you're",
];

fn has(list: &[&str], w: &str) -> bool {
    list.contains(&w)
}

/// A regular past-tense verb elsewhere in the clause ("... and smiled").
fn past_verb_nearby(words: &[String], i: usize) -> bool {
    words.iter().enumerate().any(|(k, w)| {
        k != i && (has(PAST_TIME, w) || (w.len() > 4 && w.ends_with("ed") && !w.ends_with("eed")))
    })
}

fn any_of(words: &[String], list: &[&str]) -> bool {
    words.iter().any(|w| list.contains(&w.as_str()))
}

/// Choose a reading for word `i` of the clause, or `None` to use the
/// ordinary lookup. `texts` are the words as written (for capitals).
pub fn pick(words: &[String], texts: &[&str], i: usize, hint: Hint) -> Option<Pick> {
    let w = words[i].as_str();
    let prev = if i > 0 { words[i - 1].as_str() } else { "" };
    let prev2 = if i > 1 { words[i - 2].as_str() } else { "" };
    let next = words.get(i + 1).map_or("", |s| s.as_str());
    let next2 = words.get(i + 2).map_or("", |s| s.as_str());
    let capital = texts[i].chars().next().is_some_and(|c| c.is_uppercase());
    let prev_capital = i > 0
        && texts[i - 1]
            .chars()
            .next()
            .is_some_and(|c| c.is_uppercase());
    let ipa = |us, gb| Some(Pick::Ipa(us, gb));
    match w {
        "read" => {
            let present = has(VERB_CUES, prev) || (prev == "not" && has(VERB_CUES, prev2));
            let past = !present
                && (has(PAST_AUX, prev)
                    || (has(PAST_AUX, prev2) && !has(SUBJECTS, prev))
                    || matches!(prev, "he" | "she" | "it")
                    || (prev_capital && i > 0 && !has(VERB_CUES, prev))
                    || past_verb_nearby(words, i)
                    || (words.iter().any(|x| x == "last") && !any_of(words, &["will", "going"])));
            if past {
                ipa("ɹˈɛd", "ɹˈɛd")
            } else {
                ipa("ɹˈiːd", "ɹˈiːd")
            }
        }
        "lead" => {
            const AFTER: &[&str] = &[
                "pipe",
                "pipes",
                "paint",
                "poisoning",
                "pencil",
                "pencils",
                "weight",
                "weights",
                "bullet",
                "bullets",
                "shot",
                "acid",
                "levels",
                "exposure",
                "crystal",
                "ore",
                "mine",
                "mines",
                "free",
                "based",
                "lined",
                "foil",
                "sheet",
                "roof",
            ];
            const TOPIC: &[&str] = &[
                "metal",
                "metals",
                "toxic",
                "poisoning",
                "copper",
                "zinc",
                "tin",
                "mercury",
                "contains",
                "contain",
                "containing",
                "lined",
                "heavy",
            ];
            let verbish = has(VERB_CUES, prev)
                || matches!(prev, "the" | "a" | "in" | "take" | "took" | "takes");
            if has(AFTER, next)
                || (!verbish
                    && matches!(
                        prev,
                        "of" | "with" | "contains" | "contain" | "containing" | "like" | "from"
                    ))
                || (prev != "to" && any_of(words, TOPIC) && !matches!(prev, "a" | "in"))
            {
                ipa("lˈɛd", "lˈɛd")
            } else {
                ipa("lˈiːd", "lˈiːd")
            }
        }
        "live" => {
            const AFTER: &[&str] = &[
                "wire",
                "wires",
                "music",
                "show",
                "shows",
                "broadcast",
                "broadcasts",
                "concert",
                "concerts",
                "band",
                "audience",
                "animal",
                "animals",
                "stream",
                "streams",
                "coverage",
                "performance",
                "performances",
                "tv",
                "television",
                "event",
                "events",
                "feed",
                "ammunition",
                "bait",
                "video",
                "chat",
                "action",
                "demo",
                "album",
                "recording",
                "update",
                "updates",
                "match",
                "rounds",
                "bombs",
                "lobster",
                "lobsters",
                "footage",
                "report",
                "reports",
                "telecast",
                "commentary",
            ];
            const BEFORE: &[&str] = &[
                "go",
                "goes",
                "going",
                "went",
                "gone",
                "broadcast",
                "performed",
                "performing",
                "streamed",
                "streaming",
                "recorded",
                "aired",
                "shown",
                "played",
                "sang",
                "a",
                "the",
            ];
            let adj = has(AFTER, next)
                || has(BEFORE, prev)
                || (has(BE, prev)
                    && matches!(
                        next,
                        "" | "on" | "from" | "now" | "tonight" | "today" | "at"
                    ));
            if adj && !has(VERB_CUES, prev) && !has(SUBJECTS, prev) {
                ipa("lˈaɪv", "lˈaɪv")
            } else {
                ipa("lˈɪv", "lˈɪv")
            }
        }
        "lives" => {
            const BEFORE: &[&str] = &[
                "their", "our", "your", "many", "saved", "save", "saves", "saving", "lost", "cost",
                "costs", "claimed", "human", "whose", "of", "these", "those", "two", "three",
                "nine", "innocent", "people's", "daily", "private", "long", "short", "the",
            ];
            if has(BEFORE, prev) {
                ipa("lˈaɪvz", "lˈaɪvz")
            } else {
                ipa("lˈɪvz", "lˈɪvz")
            }
        }
        "wind" | "winds" => {
            const AFTER: &[&str] = &[
                "up", "down", "around", "round", "back", "through", "along", "its", "his", "her",
                "your", "my", "their", "it",
            ];
            let noun_before = has(DETERMINERS, prev)
                || matches!(
                    prev,
                    "strong"
                        | "cold"
                        | "high"
                        | "north"
                        | "south"
                        | "east"
                        | "west"
                        | "light"
                        | "of"
                        | "in"
                        | "with"
                        | "by"
                        | "and"
                );
            let verb = !noun_before
                && (has(VERB_CUES, prev)
                    || (has(AFTER, next) && !prev.is_empty())
                    || (w == "wind" && has(SUBJECTS, prev)));
            match (w, verb) {
                ("wind", true) => ipa("wˈaɪnd", "wˈaɪnd"),
                ("wind", false) => ipa("wˈɪnd", "wˈɪnd"),
                (_, true) => ipa("wˈaɪndz", "wˈaɪndz"),
                _ => ipa("wˈɪndz", "wˈɪndz"),
            }
        }
        "tear" | "tears" => {
            const AFTER: &[&str] = &[
                "up", "apart", "down", "off", "open", "out", "into", "away", "it", "them",
                "through", "along", "the", "this", "that", "your", "my",
            ];
            const TORN: &[&str] = &[
                "cloth", "fabric", "paper", "shirt", "jeans", "page", "sleeve", "curtain",
                "muscle", "ligament", "tendon", "seam", "sail", "canvas", "jacket", "dress",
                "trousers", "pants", "bag", "net",
            ];
            let rip = has(VERB_CUES, prev)
                || (prev == "and" && prev2 == "wear")
                || (w == "tear" && has(SUBJECTS, prev))
                || (has(AFTER, next) && !matches!(prev, "a" | "single" | "one" | "the" | "in"))
                || (matches!(next, "in" | "on") && any_of(words, TORN))
                || (w == "tears" && matches!(prev, "he" | "she" | "it") && has(AFTER, next));
            match (w, rip) {
                ("tear", true) => ipa("tˈɛɹ", "tˈeə"),
                ("tear", false) => ipa("tˈɪɹ", "tˈiə"),
                (_, true) => ipa("tˈɛɹz", "tˈeəz"),
                _ => ipa("tˈɪɹz", "tˈiəz"),
            }
        }
        "bass" => {
            const FISH: &[&str] = &[
                "fish",
                "fishing",
                "fished",
                "caught",
                "catch",
                "lake",
                "river",
                "pond",
                "angler",
                "anglers",
                "sea",
                "largemouth",
                "smallmouth",
                "striped",
                "bait",
                "hook",
                "boat",
                "stream",
            ];
            if any_of(words, FISH) {
                ipa("bˈæs", "bˈæs")
            } else {
                ipa("bˈeɪs", "bˈeɪs")
            }
        }
        "bow" | "bows" => {
            const KNOT: &[&str] = &[
                "arrow", "arrows", "tie", "ties", "tied", "violin", "cello", "ribbon", "ribbons",
                "hair", "string", "strings", "archer", "archery", "hunted", "hunting", "quiver",
                "shoelace", "gift", "wrapped",
            ];
            let knot = any_of(words, KNOT) && !has(VERB_CUES, prev);
            match (w, knot) {
                ("bow", true) => ipa("bˈoʊ", "bˈəʊ"),
                ("bow", false) => ipa("bˈaʊ", "bˈaʊ"),
                (_, true) => ipa("bˈoʊz", "bˈəʊz"),
                _ => ipa("bˈaʊz", "bˈaʊz"),
            }
        }
        "wound" => {
            const AFTER: &[&str] = &[
                "up", "down", "around", "round", "back", "through", "along", "its", "his", "her",
                "the", "itself", "tightly", "a", "their", "my",
            ];
            const BEFORE_NOUN: &[&str] = &[
                "bullet", "stab", "head", "open", "deep", "gunshot", "knife", "fatal", "serious",
                "flesh", "old", "chest", "leg", "war",
            ];
            let injury = has(DETERMINERS, prev)
                || has(BEFORE_NOUN, prev)
                || (has(BE, prev) && !matches!(next, "up" | "around" | "round" | "tightly"));
            if !injury && has(AFTER, next) {
                ipa("wˈaʊnd", "wˈaʊnd")
            } else {
                ipa("wˈuːnd", "wˈuːnd")
            }
        }
        "minute" => {
            const AFTER: &[&str] = &[
                "detail",
                "details",
                "amount",
                "amounts",
                "quantity",
                "quantities",
                "particle",
                "particles",
                "difference",
                "differences",
                "trace",
                "traces",
                "fraction",
                "fractions",
                "change",
                "changes",
                "variation",
                "variations",
                "size",
                "creature",
                "creatures",
                "organism",
                "organisms",
                "crack",
                "cracks",
                "adjustment",
                "adjustments",
                "fragment",
                "fragments",
            ];
            let counted = matches!(
                prev,
                "one"
                    | "per"
                    | "each"
                    | "last"
                    | "next"
                    | "another"
                    | "five"
                    | "ten"
                    | "two"
                    | "three"
                    | "thirty"
                    | "final"
                    | "first"
            );
            if has(AFTER, next) && !counted {
                ipa("maɪnˈuːt", "maɪnjˈuːt")
            } else {
                None
            }
        }
        "close" => {
            const AFTER: &[&str] = &[
                "to",
                "friend",
                "friends",
                "relative",
                "relatives",
                "call",
                "range",
                "look",
                "attention",
                "enough",
                "together",
                "by",
                "proximity",
                "quarters",
                "second",
                "race",
                "contact",
                "ties",
                "relationship",
                "family",
                "behind",
                "match",
                "watch",
                "eye",
            ];
            const BEFORE: &[&str] = &[
                "very", "so", "too", "quite", "pretty", "really", "how", "as", "stay", "stayed",
                "get", "getting", "got", "come", "came", "coming", "feel", "felt", "remain",
                "remained", "draw", "drew", "live", "lived", "lives", "sit", "sat", "stand",
                "stood", "keep", "kept", "stick", "a", "in", "that",
            ];
            let object_next = has(OBJECT_STARTS, next) && next != "to";
            let adj = !object_next
                && !has(VERB_CUES, prev)
                && (has(AFTER, next)
                    || has(BEFORE, prev)
                    || (has(BE, prev) && !matches!(next, "down" | "up")));
            if adj {
                ipa("klˈoʊs", "klˈəʊs")
            } else {
                ipa("klˈoʊz", "klˈəʊz")
            }
        }
        "does" => {
            let plural_noun = matches!(
                prev,
                "the" | "these" | "those" | "two" | "three" | "some" | "many" | "several" | "few"
            ) && !matches!(
                next,
                "it" | "he" | "she" | "this" | "that" | "not" | "the" | "a" | ""
            );
            if plural_noun {
                ipa("dˈoʊz", "dˈəʊz")
            } else {
                None
            }
        }
        "dove" => {
            const AFTER: &[&str] = &[
                "into",
                "in",
                "off",
                "under",
                "down",
                "for",
                "headfirst",
                "deep",
                "right",
                "from",
                "through",
                "to",
                "underwater",
                "back",
                "out",
                "at",
                "onto",
                "straight",
            ];
            let noun_before = has(DETERMINERS, prev)
                || matches!(prev, "white" | "turtle" | "of" | "mourning" | "peace");
            if !noun_before && has(AFTER, next) {
                ipa("dˈoʊv", "dˈəʊv")
            } else {
                ipa("dˈʌv", "dˈʌv")
            }
        }
        "sow" => {
            if has(DETERMINERS, prev) || matches!(prev, "old" | "fat" | "big" | "mother") {
                ipa("sˈaʊ", "sˈaʊ")
            } else {
                ipa("sˈoʊ", "sˈəʊ")
            }
        }
        "resume" => {
            if (has(DETERMINERS, prev)
                || matches!(
                    prev,
                    "updated" | "impressive" | "strong" | "good" | "short" | "long"
                ))
                && !matches!(next, "the" | "their" | "its" | "normal" | "work" | "talks")
            {
                ipa("ɹˈɛzəmˌeɪ", "ɹˈɛzjuːmˌeɪ")
            } else {
                None
            }
        }
        "polish" => {
            let verb_like = i == 0 && (has(OBJECT_STARTS, next) || matches!(next, "up" | "off"));
            if capital && !verb_like {
                ipa("pˈoʊlɪʃ", "pˈəʊlɪʃ")
            } else {
                ipa("pˈɑːlɪʃ", "pˈɒlɪʃ")
            }
        }
        "refuse" if hint == Hint::Noun || (has(DETERMINERS, prev) && !has(SUBJECTS, prev)) => {
            ipa("ɹˈɛfjuːs", "ɹˈɛfjuːs")
        }
        "content"
            if has(BE, prev)
                || matches!(
                    prev,
                    "felt"
                        | "feel"
                        | "feels"
                        | "seem"
                        | "seems"
                        | "seemed"
                        | "quite"
                        | "very"
                        | "so"
                        | "perfectly"
                        | "more"
                        | "not"
                ) =>
        {
            ipa("kəntˈɛnt", "kəntˈɛnt")
        }
        "conduct" | "contract" | "contracts" | "conducts" | "present" | "presents" | "desert"
        | "produce" | "permit" | "permits" | "object" | "objects" | "record" | "records"
        | "project" | "projects" | "protest" | "rebel" | "contrast" | "convict" | "conflict"
        | "contest" | "extract" | "insult" | "increase" | "decrease" | "import" | "suspect"
        | "subject" | "progress" | "transfer" | "transport" | "survey" | "refuse" | "content"
        | "perfect" | "addict" | "escort" | "digest" => {
            let plural_subject = prev.len() > 3
                && prev.ends_with('s')
                && !prev.ends_with("ss")
                && !prev.ends_with("'s")
                && !has(DETERMINERS, prev)
                && !matches!(prev, "this" | "business" | "always" | "its" | "his");
            let verb = hint == Hint::Verb
                || has(VERB_CUES, prev)
                || (matches!(prev, "me" | "us" | "him" | "them") && has(OBJECT_STARTS, next))
                || (has(OBJECT_STARTS, next) && !has(DETERMINERS, prev) && hint != Hint::Noun)
                || (plural_subject && !matches!(next, "of" | "is" | "was" | "for"))
                || (matches!(w, "object") && next == "to" && !has(DETERMINERS, prev));
            let noun = !verb
                && (hint == Hint::Noun
                    || has(DETERMINERS, prev)
                    || next.is_empty()
                    || matches!(
                        next,
                        "of" | "is" | "was" | "were" | "are" | "has" | "had" | "for"
                    ));
            let _ = next2;
            if verb {
                match w {
                    "conduct" => return ipa("kəndˈʌkt", "kəndˈʌkt"),
                    "conducts" => return ipa("kəndˈʌkts", "kəndˈʌkts"),
                    "contract" => return ipa("kəntɹˈækt", "kəntɹˈakt"),
                    "contracts" => return ipa("kəntɹˈækts", "kəntɹˈakts"),
                    _ => {}
                }
                Some(Pick::Hint(Hint::Verb))
            } else if noun {
                match w {
                    "produce" => return ipa("pɹˈoʊduːs", "pɹˈɒdjuːs"),
                    "permit" => return ipa("pˈɜːmɪt", "pˈɜːmɪt"),
                    "permits" => return ipa("pˈɜːmɪts", "pˈɜːmɪts"),
                    _ => {}
                }
                Some(Pick::Hint(Hint::Noun))
            } else {
                None
            }
        }
        _ => None,
    }
}
