# Improved mode

`compatible` mode exists so that voices trained on espeak-ng phonemes keep sounding the way they
do. That means reproducing the reference where it is wrong. `improved` mode is for everything else:
training new voices, and reading text out correctly. Measured results are in
[evaluation.md](evaluation.md); this page lists what changes.

## English

| What | Compatible | Improved |
| --- | --- | --- |
| Homographs | part of speech guessed from the previous word only | cues from the whole clause: tense auxiliaries and time words (*I read it yesterday* → `ɹˈɛd`), the following word for noun/verb pairs (*record a song* / *a record*), topic words for unrelated senses (*bass* the fish, *bow* and arrows, *lead* pipe, *live* music, *minute* detail, *close* friends, *tear* the paper, *wound* the rope) |
| Names and brands | lexicon, then letter-to-sound | a hand-written list is consulted first (*Siobhan*, *Xiaomi*, *Nguyen*, Indian names and cities); mixed-case names are not split at capitals (*iPhone*, *YouTube*, *LaTeX*) |
| `Dr.` `St.` | always "doctor" / by position | "drive", "street" after a capitalised name (*Maple Dr.*, *Baker St.*) |

Numbers, dates, money, units, ranges, fractions and telephone-style digit strings are read
naturally in **both** modes unless you ask for the reference's readings (`normalize = espeak`).

## Hindi

| What | Compatible | Improved |
| --- | --- | --- |
| Schwa deletion | the reference's stress-driven pattern (`लड़कपन` → `lˈʌr.əkˌəpən`) | final deletion, then medial deletion right to left in a V C _ C V frame, with morpheme seams for common suffixes and compound heads (`lˈʌɽəkpən`, `लोकसभा` → lok·sabhā), and no deletion before a final short i/u in learned words (`अनुमति`) |
| Anusvara | nasalised vowel after a matra (`हिंदी` → `hˈĩdi`) | homorganic nasal before a stop (`hˈɪndiː`), velar before ह (`सिंह`) |
| Chandrabindu | sometimes vowel + n (`गाँव` → `ɡˈãnʋ`) | nasalised vowel (`ɡˈãʋ`) |
| Flap ड़ ढ़ | `r.` — an ASCII stand-in that leaks into the output | `ɽ` `ɽʰ` |
| Final long vowels | printed short (`पानी` → `pˈaːni`) | long (`pˈaːniː`) |
| English words in Hindi text | British English phonemes, including ones a Hindi voice never saw (`ɒ`, `əʊ`, `ɹ`, `w`) | mapped into the Hindi phone set (retroflex stops, `eː` `oː`, `ʋ`, `ɾ`) |
| Romanised Hindi | read as English | with `hinglish`: detected and read as Hindi (experimental) |

## Caveats

- Existing espeak-trained Hindi voices have never seen `ɽ` or a homorganic nasal after a matra.
  Feed them compatible output.
- The homograph cues are word lists, not a tagger. On sentences written after the rules were
  frozen they were right 82.5 % of the time (the reference: 62.5 %).
- The names list is short. On unseen names the reference did slightly better (60 % against 55 %).
