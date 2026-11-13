//! English function words: how a closed class of words is stressed in
//! running text, and the handful of weak forms that change segments.
//!
//! Each row gives the stress a word carries inside a clause and at the end
//! of one (`0` none, `s` secondary, `p` primary), and optionally a base form
//! that replaces the lexicon's citation form (General American, then RP; the
//! stress marks in a base form are ignored and re-applied from the class).
//! The table was written by hand from the behaviour of the target notation in
//! three frames (clause-medial, clause-final, before a vowel).

/// `(word, medial stress, final stress, en-US base, en-GB base)`, sorted by word.
pub static WORDS: &[(&str, u8, u8, &str, &str)] = &[
    ("a", b'0', b'0', "ɐ", "ɐ"),
    ("about", b's', b'p', "", ""),
    ("across", b's', b'p', "", ""),
    ("already", b's', b'p', "", ""),
    ("am", b'0', b'0', "", ""),
    ("an", b'0', b'0', "æn", "æn"),
    ("and", b'0', b'0', "ænd", "ænd"),
    ("any", b's', b's', "", ""),
    ("are", b'0', b'0', "", ""),
    ("aren't", b's', b'p', "ɑːɹnt", "ɑːnt"),
    ("as", b'0', b'0', "", ""),
    ("at", b'0', b'0', "", ""),
    ("be", b'0', b'p', "", ""),
    ("became", b's', b'p', "", ""),
    ("because", b'p', b'p', "bɪkˈʌz", "bɪkˈɒz"),
    ("become", b's', b'p', "", ""),
    ("becomes", b's', b'p', "", ""),
    ("been", b's', b's', "bɪn", "biːn"),
    ("before", b's', b'p', "", ""),
    ("behind", b's', b'p', "", ""),
    ("being", b's', b'p', "", ""),
    ("between", b's', b'p', "", ""),
    ("but", b's', b's', "", ""),
    ("by", b'0', b'p', "", ""),
    ("can", b'0', b'p', "", ""),
    ("could", b'0', b'p', "", ""),
    ("couldn't", b's', b'p', "", ""),
    ("didn't", b'p', b'p', "dˈɪdnt", "dˈɪdnt"),
    ("doing", b's', b'p', "", ""),
    ("down", b's', b'p', "", ""),
    ("far", b's', b'p', "", ""),
    ("for", b'0', b'0', "fɔːɹ", "fɔː"),
    ("from", b'0', b'0', "fɹʌm", "fɹɒm"),
    ("get", b'0', b'p', "", ""),
    ("getting", b's', b'p', "", ""),
    ("go", b's', b'p', "", ""),
    ("going", b's', b'p', "", ""),
    ("gonna", b's', b's', "ɡənə", "ɡənɐ"),
    ("got", b'0', b'p', "", ""),
    ("had", b'0', b's', "", ""),
    ("has", b'0', b'p', "hɐz", "hɐz"),
    ("have", b'0', b'p', "", ""),
    ("having", b's', b'p', "", ""),
    ("he", b'0', b'0', "", ""),
    ("he'd", b'0', b'0', "", ""),
    ("he'll", b'0', b'0', "", ""),
    ("he's", b'0', b'0', "", ""),
    ("her", b'0', b'0', "hɜː", "hɜː"),
    ("him", b's', b's', "", ""),
    ("his", b'0', b'p', "", ""),
    ("how", b's', b'p', "", ""),
    ("i", b'0', b'p', "aɪ", "aɪ"),
    ("i'd", b'0', b'0', "", ""),
    ("i'll", b'0', b'0', "", ""),
    ("i'm", b'0', b'0', "", ""),
    ("i've", b'0', b'0', "", ""),
    ("if", b'0', b'0', "", ""),
    ("in", b'0', b'p', "", ""),
    ("into", b's', b's', "ɪntʊ", "ɪntʊ"),
    ("is", b'0', b'0', "", ""),
    ("isn't", b's', b'p', "", ""),
    ("it", b'0', b'0', "", ""),
    ("it's", b'0', b'0', "", ""),
    ("its", b'0', b'0', "", ""),
    ("made", b's', b'p', "", ""),
    ("make", b's', b'p', "", ""),
    ("makes", b's', b'p', "", ""),
    ("making", b's', b'p', "", ""),
    ("me", b's', b's', "", ""),
    ("might", b's', b'p', "", ""),
    ("much", b's', b'p', "", ""),
    ("my", b'0', b'p', "", ""),
    ("near", b's', b'p', "", ""),
    ("not", b's', b'p', "", ""),
    ("of", b'0', b'0', "ʌv", "ɒv"),
    ("on", b's', b'p', "ɔn", "ɒn"),
    ("one", b'p', b's', "wʌn", "wɒn"),
    ("onto", b's', b'p', "ˈɑːntʊ", "ˈɒntʊ"),
    ("or", b'0', b'0', "ɔːɹ", "ɔː"),
    ("our", b's', b's', "aʊɚ", "aʊɐ"),
    ("over", b's', b'p', "", ""),
    ("put", b's', b'p', "", ""),
    ("shall", b's', b'p', "", ""),
    ("she", b'0', b'0', "", ""),
    ("she'd", b'0', b'0', "", ""),
    ("she'll", b'0', b'0', "", ""),
    ("she's", b'0', b'0', "", ""),
    ("should", b's', b'p', "", ""),
    ("shouldn't", b's', b'p', "", ""),
    ("so", b's', b'p', "", ""),
    ("some", b's', b's', "sʌm", "sʌm"),
    ("than", b'0', b'0', "ðɐn", "ðɐn"),
    ("that", b'0', b'p', "", ""),
    ("that's", b'0', b'p', "", ""),
    ("the", b'0', b'0', "ðə", "ðə"),
    ("their", b'0', b'0', "ðɛɹ", "ðeə"),
    ("them", b's', b's', "", ""),
    ("there", b'0', b'p', "ðɛɹ", "ðeə"),
    ("there's", b'0', b'p', "ðɛɹz", "ðeəz"),
    ("these", b'0', b'p', "", ""),
    ("they", b'0', b'0', "", ""),
    ("they'd", b'0', b'0', "", ""),
    ("they'll", b'0', b'0', "", ""),
    ("they're", b'0', b'0', "ðeɪɚ", "ðeə"),
    ("they've", b'0', b'0', "", ""),
    ("this", b'0', b'p', "", ""),
    ("those", b'0', b'p', "", ""),
    ("though", b's', b'p', "", ""),
    ("through", b'0', b'p', "", ""),
    ("to", b'0', b'0', "tə", "tə"),
    ("together", b's', b'p', "", ""),
    ("under", b's', b'p', "", ""),
    ("up", b's', b'p', "", ""),
    ("upon", b's', b's', "əpˌɑːn", "əpˌɒn"),
    ("us", b's', b's', "", ""),
    ("was", b'0', b'0', "wʌz", "wɒz"),
    ("wasn't", b'p', b'p', "wˈʌzn̩t", "wˈɒzənt"),
    ("we", b'0', b'0', "", ""),
    ("we'd", b'0', b'0', "", ""),
    ("we'll", b'0', b'0', "", ""),
    ("we're", b'0', b'0', "wɪɹ", "wiə"),
    ("we've", b'0', b'0', "", ""),
    ("went", b'0', b'p', "", ""),
    ("were", b'0', b'0', "wɜː", "wɜː"),
    ("weren't", b's', b'p', "wɜːnt", "wɜːnt"),
    ("what", b'0', b'p', "wʌt", "wɒt"),
    ("what's", b'0', b'p', "wʌts", "wɒts"),
    ("when", b'0', b'p', "", ""),
    ("where", b's', b'p', "", ""),
    ("which", b's', b'p', "", ""),
    ("while", b's', b'p', "", ""),
    ("who", b's', b'p', "", ""),
    ("who's", b's', b'p', "", ""),
    ("whose", b's', b's', "", ""),
    ("why", b's', b'p', "", ""),
    ("will", b'0', b'p', "", ""),
    ("with", b'0', b'0', "wɪð", "wɪð"),
    ("within", b's', b'p', "", ""),
    ("without", b's', b'p', "", ""),
    ("won't", b'0', b'p', "", ""),
    ("would", b'0', b'p', "", ""),
    ("you", b'0', b'0', "", ""),
    ("you'd", b'0', b'0', "", ""),
    ("you'll", b'0', b'0', "", ""),
    ("you're", b'0', b'0', "jʊɹ", "jɔː"),
    ("you've", b'0', b'0', "", ""),
    ("your", b'0', b'0', "jʊɹ", "jɔː"),
];

/// Word pairs written as one phonological word: `(first, second, en-US, en-GB)`.
/// An empty form means "join the two weak forms without a space".
pub static PAIRS: &[(&str, &str, &str, &str)] = &[
    ("as", "is", "ˌæzˌɪz", "ˌæzˌɪz"),
    ("at", "a", "æɾə", "ætɐ"),
    ("could", "have", "kˌʊdɐv", "kˌʊdɐv"),
    ("couldn't", "have", "kˌʊdəntəv", "kˌʊdəntəv"),
    ("did", "not", "dɪdnˌɑːt", "dɪdnˌɒt"),
    ("do", "not", "duːnˌɑːt", "duːnˌɒt"),
    ("does", "not", "dʌznˌɑːt", "dʌznˌɒt"),
    ("for", "a", "fɚɹə", "fəɹɐ"),
    ("for", "an", "fɚɹən", "fəɹən"),
    ("for", "the", "fɚðə", "fəðə"),
    ("from", "the", "", ""),
    ("had", "been", "hɐdbɪn", "hɐdbɪn"),
    ("had", "to", "hædtə", "hædtə"),
    ("has", "been", "hˈæzbiːn", "hˈæzbiːn"),
    ("have", "been", "hɐvbɪn", "hɐvbɪn"),
    ("have", "not", "hɐvnˌɑːt", "hɐvnˌɒt"),
    ("i", "am", "aɪɐm", "aɪɐm"),
    ("in", "the", "", ""),
    ("might", "have", "mˌaɪthɐv", "mˌaɪthɐv"),
    ("must", "have", "mˈʌstɐv", "mˈʌstɐv"),
    ("mustn't", "have", "mˌʌsənthɐv", "mˌʌsənthɐv"),
    ("no", "one", "nˈoʊwˈʌn", "nˈəʊwˈɒn"),
    ("not", "a", "nˌɑːɾə", "nˌɒtɐ"),
    ("of", "a", "əvə", "əvɐ"),
    ("of", "an", "əvən", "əvən"),
    ("of", "the", "", ""),
    ("on", "the", "ɔnðə", "ɒnðə"),
    ("out", "of", "ˌaʊɾəv", "ˌaʊtəv"),
    ("should", "have", "ʃˌʊdəv", "ʃˌʊdəv"),
    ("shouldn't", "have", "ʃˌʊdənthəv", "ʃˌʊdənthəv"),
    ("than", "a", "ðˌænə", "ðˌænɐ"),
    ("that", "a", "ðˌæɾə", "ðˌætɐ"),
    ("that", "the", "", ""),
    ("there", "are", "ðɛɹˌɑːɹ", "ðeəɹˌɑː"),
    ("there", "was", "ðɛɹwˌʌz", "ðeəwˌɒz"),
    ("there", "were", "ðɛɹwˌɜː", "ðeəwˌɜː"),
    ("to", "be", "təbi", "təbɪ"),
    ("was", "a", "wʌzɐ", "wʌzɐ"),
    ("was", "the", "", ""),
    ("with", "the", "", ""),
    ("would", "have", "wʊdhɐv", "wʊdhɐv"),
    ("wouldn't", "have", "wˌʊdəntəv", "wˌʊdəntəv"),
];

/// Find a function word.
pub fn word(w: &str) -> Option<&'static (&'static str, u8, u8, &'static str, &'static str)> {
    WORDS
        .binary_search_by(|e| e.0.cmp(w))
        .ok()
        .map(|i| &WORDS[i])
}

/// Find a fused pair.
pub fn pair(
    a: &str,
    b: &str,
) -> Option<&'static (&'static str, &'static str, &'static str, &'static str)> {
    PAIRS
        .binary_search_by(|e| (e.0, e.1).cmp(&(a, b)))
        .ok()
        .map(|i| &PAIRS[i])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_are_sorted() {
        assert!(WORDS.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(PAIRS
            .windows(2)
            .all(|w| (w[0].0, w[0].1) < (w[1].0, w[1].1)));
        assert!(word("the").is_some() && word("zebra").is_none());
        assert!(pair("of", "the").is_some() && pair("the", "of").is_none());
    }
}
