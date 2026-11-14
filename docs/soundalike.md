# Sound-alike matching

A speech recogniser writes what it knows. A name, product or term it has never seen comes out as
the nearest words it has: "Kothari" as *Coat Harry*, "Paytm" as *pay tm*, "kubectl" as *cube
control*. `uccharan::soundalike` puts such terms back: it compares the transcript with a
vocabulary **by sound** and offers the vocabulary's spelling wherever a span of words sounds like a
term.

```rust
use uccharan::soundalike::{ApplyOptions, IndexOptions, SoundIndex};
use uccharan::{Lang, Phonemizer};

let p = Phonemizer::new();
let index = SoundIndex::from_terms(&p, &["Kothari", "Paytm"], &IndexOptions::default());
let fixed = index.apply(&p, "send it to Coat Harry", Lang::EnUs, &ApplyOptions::default());
assert_eq!(fixed.text, "send it to Kothari");
assert_eq!(fixed.changes[0].from, "Coat Harry");   // nothing is rewritten silently
```

```python
import uccharan

index = uccharan.SoundIndex(["Kothari", "Paytm"])
fixed = index.apply("send it to Coat Harry", "en-us")
fixed.text                                   # 'send it to Kothari'
[(c.old, c.new, c.score) for c in fixed.changes]
uccharan.match_terms("कोठारी जी को फ़ोन करना है।", ["Kothari"], "hi")   # candidates only
```

```js
import { createSoundIndex } from "uccharan";

const index = await createSoundIndex(["Kothari", { word: "Paytm", say: "pay T M" }]);
const fixed = index.apply("send it to Coat Harry");     // { text, changes, candidates }
```

```c
UccharanSoundIndex *ix = uccharan_sound_index_new(p, "Kothari\nPaytm\n", NULL);
char *json = uccharan_match_terms(p, ix, "send it to Coat Harry", "en-us", NULL);
/* {"text":"send it to Kothari","changes":[...],"candidates":[...]} */
uccharan_string_free(json);
uccharan_sound_index_free(ix);
```

```sh
uccharan -l en-us --match-terms vocabulary.tsv "send it to Coat Harry"
```

## The vocabulary

An index is built from a [word list](wordlists.md), a JSON array of terms, or plain text with one
term per line. Each term is stored as one or more phoneme sequences:

| The entry has | Stored |
| --- | --- |
| `ipa` | those phonemes |
| `say` (a spoken form such as `pay T M`, `engine X`) | uccharan's readings of the spoken form |
| neither | uccharan's readings of the spelling |

"Readings" are several on purpose, because a term may be said in more than one way and the
recogniser hears the speaker, not the spelling. A Latin-script term is read as English in
compatible and improved mode, as a romanised Indic word with every written vowel kept (`Kothari`
→ *koːtʰəriː*), split at inner capitals (`PhonePe` → *phone pe*) and, for a short all-capitals
term, letter by letter. A
term in another script is read in the languages of that script. `IndexOptions::langs` (default
English and Hindi) chooses the languages. `SoundIndex::sounds(i)` shows what was stored for a term.

Give `say` or `ipa` for terms whose spelling does not show their sound (`nginx`, `kubectl`,
`Paytm`). In the evaluation such hints raise the recovery of the terms that have one from 46 % to
51 % (held-out half; 38 % to 52 % on the development half).

Regular-expression entries of a word list are not terms and are left out.

## How a transcript is matched

1. The transcript is cut into words. Spans of one to three neighbouring words (`max_words`; up to
   six when a spoken form of the vocabulary has more words) are candidates; the words of a span
   are separated by nothing more than a space, a hyphen, a comma or a full stop.
2. Every word is read as phonemes in the transcript's language (words in another script are routed
   to that script's language, as everywhere in uccharan), with a second reading where one is
   plausible and a romanised-Indic reading for Latin-script words (`Kotari` as *koːtəriː*).
   Readings are remembered per index, so the words a recogniser keeps writing are read once.
3. A **pre-filter** finds the terms worth comparing. Each phoneme sequence is reduced to a
   skeleton of nine consonant classes (vowels, *h* and *y* dropped; a vowel at the start kept as
   a mark). Stored sequences with the same skeleton form a group; groups are indexed by the pairs
   of neighbouring classes in their skeleton, and only groups that share enough of those pairs
   with the span (a Dice coefficient of 0.6 or more) and have a skeleton of similar length are
   compared in full, best overlap first, about 128 sequences per span. On the development half of
   the evaluation the pre-filter recovers 1 278 of the 1 281 terms that comparing every span with
   every term recovers.
4. The span and the term are compared with a **weighted edit distance** over phonemes:

   | Edit | Cost |
   | --- | --- |
   | same sound | 0 |
   | aspiration (*k / kʰ*), vowel length (*i / iː*) | 0.10 – 0.15 |
   | dental / alveolar / retroflex (*t / ʈ*, *n / ɳ*), the r sounds, *v / w / ʋ* | 0.05 – 0.12 |
   | voicing (*p / b*, *s / z*) | 0.40 |
   | neighbouring vowels (*ɪ / e*, *ɑ / ɒ*); *ə / ʌ / ɐ* count as one sound | 0.05 – 0.25 |
   | distant vowels (*i / a*) | up to 0.70 |
   | *θ / t̪ʰ*, *ð / d̪*, *f / pʰ*, *z / dʒ*, *x / kʰ* (the usual Indian-English and Hindi–Urdu pairs) | 0.05 – 0.35 |
   | unrelated consonants (*p / s*, *m / k*) | 0.7 – 1.0 |
   | dropping a schwa | 0.25 |
   | dropping *h*, a glide, an r after a vowel | 0.35 – 0.50 |
   | dropping another vowel / consonant | 0.5 – 0.85 / 1.0 |

   Stress marks and word boundaries are ignored. A stop followed by *h* counts as an aspirated
   stop, a doubled sound as one, a nasalised vowel as vowel + *n*, and the closing diphthongs
   *eɪ*, *oʊ* as the long vowels other languages write.
5. The distance is divided by the length of the longer sequence (the sum of its dropping costs),
   and the **score** is one minus that. Sequences shorter than five phonemes lose 0.06 per missing
   phoneme: three phonemes that agree are weak evidence.
6. Where spans or terms compete for the same words, a term that is already written out wins, then
   the higher score.

Because both sides are phonemes, scripts do not matter: a Latin vocabulary matches a Devanagari
transcript and a Devanagari vocabulary matches a Latin transcript.

## Candidates and corrections

`find` returns candidates and changes nothing. A candidate has the span (`src`, as byte offsets in
Rust; byte, code-point and UTF-16 offsets in the JSON, Python and JavaScript results), the term, the
score, `exact` (the transcript already has the term here, compared without case) and the `kind`
of the span.

`apply` turns candidates into corrections and returns the new text **with every change it made**
(the replaced range, the new range, both texts, the term and the score) and all the candidates it
considered. A candidate is applied when its score reaches the threshold for its kind and, if
`confidence` is given (the recogniser's confidence for each whitespace-separated word, 0 to 1),
the mean confidence of the span's words is at most `max_confidence` (default 0.9).

An exact occurrence in another case (`paytm`) is rewritten to the term's capitals unless `fix_case`
is off; that change is reported too, marked `case_only`.

### Kinds of span

A phonetic match alone cannot tell *send* from "Shinde" or *at the* from "Ather". The words of
the span say how much to trust it:

| Kind | The span | Needs a score of |
| --- | --- | --- |
| `unknown` | an English transcript has a word here that is not in the English lexicon (*Calpana*, *Niggum*): the recogniser wrote down a sound | `threshold` (0.80) |
| `uncertain` | known words with a capital inside a sentence (*Coat Harry*, *Mr. Not Regen*: the recogniser took them for a name), or words of a language without a full lexicon here that are not on the list of frequent words | `uncertain_threshold` (0.90) |
| `ordinary` | every word is ordinary vocabulary (*send*, *at the*, *कल सुबह*) | `ordinary_threshold` (0.92) |

Ordinary vocabulary is the English lexicon (personal names apart), hand-written lists of frequent
Hindi words (about 950 in Devanagari, about 400 in Arabic script, and the romanised Hindi word
list), numbers, and any `ordinary_words` given with the index. Weekdays, months and
titles do not count as capitalised names; a transcript in Title Case, or in a language other than
English, gives no capital signal. Raising `threshold` above the other two raises all three.

So *Coat Harry* is corrected by default (score 0.92, `uncertain`), and *Calpana* for "Kalpana"
at any score from 0.80. *Zero the* for "Zerodha" scores 0.87 and stays a candidate: give such a
term a spoken form or phonemes, lower the two upper thresholds, or use confidences.

### Choosing thresholds

The defaults favour leaving text alone. They were chosen on the development half of the
evaluation; on the held-out half they put back 44 % of the terms the recognisers got wrong (64 % in
English sentences) with 98 % of the corrections right, and change 2 of 775 sentences that have no
term in them. A lower `threshold` recovers more and makes more wrong corrections (50 % and 94 % at
0.70). With recogniser confidences, lower thresholds and a `max_confidence` around 0.6 are the
better trade, because confident words are never touched. To review instead of rewrite, call
`find` and show the candidates.

For a language other than English and Hindi, or a domain with its own everyday words, pass the
frequent words as `ordinary_words` (`IndexOptions`, `SoundIndex(..., ordinary_words=[...])`,
`createSoundIndex(..., { ordinaryWords })`, `--index-options '{"ordinary_words": [...]}'`).

## JSON (C, WebAssembly, command line)

`uccharan_match_terms`, `SoundIndex::apply_json` and `uccharan --match-terms FILE` return one
object per transcript; every range has `byte`, `char` (code point) and `utf16` offsets:

```json
{"text": "ask Kothari now",
 "changes": [{"src": {"byte": [4, 10], "char": [4, 10], "utf16": [4, 10]},
              "out": {"byte": [4, 11], "char": [4, 11], "utf16": [4, 11]},
              "from": "Kotari", "to": "Kothari", "index": 0, "score": 0.9534, "case_only": false}],
 "candidates": [{"src": {"byte": [4, 10], "char": [4, 10], "utf16": [4, 10]}, "text": "Kotari",
                 "term": "Kothari", "index": 0, "score": 0.9534, "exact": false,
                 "kind": "unknown", "words": [1, 2]}]}
```

Options are a JSON object: `threshold`, `uncertain_threshold`, `ordinary_threshold`, `confidence`
(one number per whitespace-separated word), `max_confidence`, `fix_case`, `max_words`, `min_score`
(the lowest score among `candidates`: 0.8 for `apply`, 0.6 for `find`), `all`, `prefilter`,
`romanised`. The index takes `langs` and `ordinary_words`. On the command line they are
`--match-options JSON` and `--index-options JSON`; `--match-stats` prints timings.

## Speed

Building an index takes about 0.04 s per 1 000 terms. Matching runs at about 7 800 transcripts a
second against 1 000 terms, 1 500 against 10 000 and 300 against 50 000 (one thread, transcripts
of eight words; [measured](evaluation.md#7-sound-alike-matching) on a busy machine). An index is
immutable and can be shared between threads.

## Evaluation

651 hand-written terms (Indian names, brands, technical and domain terms) in English, Hindi and
romanised Hindi sentences, spoken by synthetic voices and transcribed by small recognisers, plus
hand-written mishearings and sentences without any term; compared with character edit distance and
Metaphone codes. The numbers are in [evaluation.md](evaluation.md#7-sound-alike-matching); the tools
are in `tools/eval/soundalike/`.

## Limits

- The match is by sound only. Without confidences, an ordinary phrase that happens to sound like a
  term is corrected when it is not made of listed words: *मैरी को* ("to Mary") is "Marico" to the
  ear. A vocabulary full of short names raises that risk, a vocabulary of long distinctive terms
  hardly does.
- A term that the recogniser glued to the next word (`Radhikane`) or replaced by something
  unrelated is not found: spans are whole words.
- Two terms that sound alike (`Symphony`, `Symfony`; `Subramaniam`, `Subramanian`) cannot be told
  apart; the higher score wins, and equal scores go to the earlier term.
- Readings come from uccharan's own phonemiser. A term it reads badly needs `say` or `ipa`.
- The built-in lists of ordinary words cover English and Hindi. In other languages every span is
  `uncertain` unless `ordinary_words` are given; use confidences there.
- The evaluation's speech is synthetic (see [evaluation.md](evaluation.md#7-sound-alike-matching)):
  real speakers and larger recognisers make other errors.
