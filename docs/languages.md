# Languages

uccharan phonemises 39 language tags. English (US and British) and Hindi have hand-built front
ends with their own lexicons; the others run on two shared engines, one for Brahmic and
Perso-Arabic scripts and one for Latin-script languages, each driven by a short language file of
letters, rewrite rules, exceptions and number words (`crates/uccharan/lang/*.lang`; see
[adding-a-language.md](adding-a-language.md)).

## Agreement and speed

Compatible mode is measured against espeak-ng 1.52.0, called through its library API, on
held-out sentences that were never used to write rules or build tables. The phoneme error rate
(PER) is the edit distance between the two outputs over the length of the reference, stress marks
included; "without digits" leaves out the sentences that contain digits. Method, breakdowns and
development-set figures are in [evaluation.md](evaluation.md); the numbers come from
`tools/eval/results/agreement_<tag>_heldout.json`.

Speed is uccharan's speed divided by espeak-ng's on the held-out sentences, one thread, both
called as shared libraries from one process on a shared 4-vCPU machine. Plain figures compare the
median (p50) time per sentence: for English and Hindi over 3 000 sentences, best of 11 rounds;
for the other South Asian languages over 500 sentences, measured once. Figures marked † compare
throughput in words per second with `tools/bench/compare.py --rounds 15 --threads 1` and are the
lower of two runs; they were taken after the word-table models got a faster run-time in 0.4.0.
Runs on that machine differ by about ±10 %.

| Language | Tag | espeak-ng voice compared with | Held-out text | Phoneme error rate | Without digits | Speed against espeak-ng | Open voices trained on these phonemes |
| --- | --- | --- | --- | ---: | ---: | ---: | --- |
| English (US) | `en-us` | `en-us` | 3,000 sentences | 1.16 % | — | 5.3× | Piper `en_US`, Kokoro (via the misaki port), Kitten TTS, Matcha-TTS |
| English (British, RP) | `en-gb` | `en-gb` | 3,000 sentences | 1.39 % | — | 5.2× | Piper `en_GB`, Kokoro |
| Hindi, and Hindi + English | `hi` | `hi` | 3,000 sentences | 0.09 % | — | 2.8× | Piper `hi_IN`, Kokoro |
| Bengali | `bn` | `bn` | Tatoeba, 2,000 sentences | 0.37 % | — | 2.0× | Piper `bn_BD`, Mimic-3 `bn` |
| Marathi | `mr` | `mr` | Tatoeba, 2,000 sentences | 0.27 % | — | 3.2× | Piper `mr_IN` |
| Telugu | `te` | `te` | FLORES-200, 1,012 sentences | 0.87 % | 0.52 % | 3.0× | Piper `te_IN` (3) |
| Kannada | `kn` | `kn` | FLORES-200, 1,012 sentences | 1.58 % | 0.50 % | 2.4× | MMS / SYSPIN (character input) |
| Malayalam | `ml` | `ml` | FLORES-200, 1,012 sentences | 2.01 % | 0.97 % | 2.6× | Piper `ml_IN` (2) |
| Nepali | `ne` | `ne` | FLORES-200, 1,012 sentences | 2.67 % | 2.32 % | 15× | Piper `ne_NP` (3), Mimic-3 `ne` |
| Tamil | `ta` | `ta` | FLORES-200, 1,012 sentences | 4.09 % | 2.83 % | 37× | Piper `ta_IN` (2, community) |
| Gujarati | `gu` | `gu` | FLORES-200, 1,012 sentences | 2.41 % | 2.04 % | 3.8× | Mimic-3 `gu` |
| Assamese | `as` | `as` | Tatoeba, 2,000 sentences | 7.21 % | — | 2.2× | MMS (character input) |
| Punjabi (Gurmukhi) | `pa` | `pa` | FLORES-200, 1,012 sentences | 0.41 % | 0.22 % | 3.8× | MMS (character input) |
| Odia | `or` | `or` | FLORES-200, 1,012 sentences | 0.50 % | 0.28 % | 2.4× | MMS (character input) |
| Sinhala | `si` | `si` | FLORES-200, 1,012 sentences | 0.86 % | 0.24 % | 2.3× | MMS (character input) |
| Konkani | `kok` | `kok` | Sangraha, 1,000 sentences | 0.20 % | 0.13 % | 5.1× | none |
| Urdu | `ur` | `ur` | FLORES-200, 1,012 sentences | 1.44 % | 1.16 % | 4.0× | Piper `ur_PK` (2) |
| Sindhi | `sd` | `sd` | FLORES-200, 1,012 sentences | 1.30 % | 0.70 % | 2.2× | none |
| Sanskrit | `sa` | `hi` ‡ | FLORES-200, 1,012 sentences | 0.64 % | 0.43 % | 4.2× | none |
| Bhojpuri | `bho` | `hi` ‡ | FLORES-200, 1,012 sentences | 0.51 % | 0.33 % | 3.3× | SYSPIN (character input) |
| Maithili | `mai` | `hi` ‡ | FLORES-200, 1,012 sentences | 0.46 % | 0.29 % | 3.0× | MMS, SYSPIN (character input) |
| Magahi | `mag` | `hi` ‡ | FLORES-200, 1,012 sentences | 0.40 % | 0.23 % | 2.8× | MMS (character input) |
| Spanish | `es` | `es` | FLORES-200, 1,012 sentences | 0.54 % | 0.32 % | 2.3× † | Piper `es_ES` (5), Kokoro `ef_dora`, `em_alex`, `em_santa` |
| German | `de` | `de` | FLORES-200, 1,012 sentences | 0.95 % | 0.77 % | 5.8× † | Piper `de_DE` (11) |
| Dutch | `nl` | `nl` | FLORES-200, 1,012 sentences | 1.69 % | 1.32 % | 3.2× † | Piper `nl_NL`, `nl_BE` (11) |
| French | `fr` | `fr` | FLORES-200, 1,012 sentences | 0.63 % | 0.54 % | 5.9× † | Piper `fr_FR` (7), Kokoro `ff_siwis` |
| French (Belgium) | `fr-be` | `fr-be` | FLORES-200, 1,012 sentences ✦ | 0.63 % | 0.54 % | 5.9× † | none |
| French (Switzerland) | `fr-ch` | `fr-ch` | FLORES-200, 1,012 sentences ✦ | 0.62 % | 0.54 % | 5.9× † | none |
| Italian | `it` | `it` | FLORES-200, 1,012 sentences | 0.53 % | 0.37 % | 4.6× † | Piper `it_IT` (4), Kokoro `if_sara`, `im_nicola` |
| Portuguese (Brazil) | `pt-br` | `pt-br` | FLORES-200, 1,012 sentences | 0.61 % | 0.40 % | 5.1× † | Piper `pt_BR` (4), Kokoro `pf_dora`, `pm_alex`, `pm_santa` |
| Portuguese (Portugal) | `pt` | `pt` | FLORES-200, 1,012 sentences | 0.84 % | 0.60 % | 5.2× † | Piper `pt_PT` (1) |
| Polish | `pl` | `pl` | FLORES-200, 1,012 sentences | 1.12 % | 0.92 % | 3.1× † | Piper `pl_PL` (5) |
| Turkish | `tr` | `tr` | FLORES-200, 1,012 sentences | 0.68 % | 0.47 % | 2.7× † | Piper `tr_TR` (1) |
| Czech | `cs` | `cs` | FLORES-200, 1,012 sentences | 1.31 % | 1.07 % | 1.8× † | Piper `cs_CZ` (3) |
| Swedish | `sv` | `sv` | FLORES-200, 1,012 sentences | 1.32 % | 0.83 % | 3.7× † | Piper `sv_SE` (3) |
| Catalan | `ca` | `ca` | FLORES-200, 1,012 sentences | 1.44 % | 1.10 % | 4.9× † | Piper `ca_ES` (3) |
| Romanian | `ro` | `ro` | FLORES-200, 1,012 sentences | 0.96 % | 0.86 % | 3.3× † | Piper `ro_RO` (1) |
| Hungarian | `hu` | `hu` | FLORES-200, 1,012 sentences | 1.34 % | 0.95 % | 13.1× † | Piper `hu_HU` (3) |
| Finnish | `fi` | `fi` | FLORES-200, 1,012 sentences | 1.36 % | 1.08 % | 1.7× † | Piper `fi_FI` (2) |
| Norwegian Bokmål | `nb` | `nb` | FLORES-200, 1,012 sentences | 0.96 % | 0.46 % | 2.3× † | Piper `no_NO` (2) |
| Danish | `da` | `da` | FLORES-200, 1,012 sentences | 1.95 % | 1.49 % | 27.1× † | Piper `da_DK` (1) |

✦ The Belgian and Swiss French voices are scored on the French sentences; they differ from the
French reading only in the numbers 70–99 (septante, huitante, nonante). Their speed is the French
voice's: in one run on a busy machine French 4.5×, `fr-be` 4.5×, `fr-ch` 5.2×.

‡ espeak-ng has no voice for Sanskrit, Bhojpuri, Maithili or Magahi; text in them is read with its
Hindi voice, and compatible mode is measured against that reading.

## Notes on the languages

FLORES-200 sentences are longer news-style text with many numbers, so their rates are not
comparable with the Tatoeba rows. Most of the remaining disagreement in Telugu, Kannada and
Malayalam is in numbers, which espeak-ng reads from its own word lists. Assamese is a first version: espeak-ng spells out
letter by letter every word written with a separate nukta code point (which is how normalised text
writes য়), and the letter names are only partly reproduced. Tamil (stop voicing
depends on the word's origin: uccharan has a hand-written list of Sanskrit and foreign loan stems
that keep a voiced initial, and marks the compound element படு and prefix முன்; the rest of the gap
is numbers and secondary stress) and Nepali (stress on
many word-final long vowels) are the furthest from espeak-ng among the Indic languages. End-to-end tests with voices of these languages have not been run yet.

Punjabi, Odia, Sinhala and Konkani follow the espeak-ng voices closely, including their quirks:
Punjabi's `+` tone mark after a word-initial voiced aspirate, Odia's spelled-out words with ୱ, the
1.52 library's word break after a Sinhala ZWJ conjunct, and the Konkani voice's reading of every
vowel sign by its name. Improved mode drops those quirks: Punjabi gets vowel length, a low tone mark
and ɽ; Odia keeps ୱ as w, ଳ as ɭ and the dental/retroflex distinction; Sinhala joins ZWJ conjuncts;
Konkani is read as an ordinary Indic language. **Sanskrit** has no espeak-ng voice: compatible mode
is the Hindi voice's reading (as for Bhojpuri), and improved mode reads Sanskrit as Sanskrit: every
short a pronounced, the classical stress rule (heavy penult, else heavy antepenult, else the fourth
syllable from the end), visarga echoing the vowel before it at the end of a word, and Sanskrit
number words.

**Magahi** (`mag`) is read like Bhojpuri and Maithili: compatible mode is the Hindi voice's
reading; improved mode adds Magahi's own rules to the shared Bihari ones (no ʃ ʂ ɳ, verb endings
in -əi such as हलई hələi and गेलय ɡeːləi, a said final short ि ु, मे for में). On a held-out
Magahi gold set (270 words, 73 sentences) improved mode gets 98.5 % of words right against
64.4 % for the Hindi voice, and sentence PER 0.71 % against 7.67 %.

**German, Dutch and French** (`de`, `nl`, `fr`) run on the same Latin-script engine as Spanish,
each with letter rules and a **word table** (`de.ucd` 16 k entries, 373 KB; `nl.ucd` 12 k, 242 KB;
`fr.ucd` 12 k, 181 KB; compiled in by the `embed-de`, `embed-nl` and `embed-fr` features, fetched
on demand in the browser). The tables were made by running espeak-ng 1.52.0 over the words of
Tatoeba and Leipzig Corpora Collection sentences (139 k German, 80 k Dutch and 100 k French
words), alone and in running text, so they also hold what a word does inside a clause: German
and Dutch words that lose their stress there, French liaison consonants (`les amis` → `le-z amˈi`,
`les livres` → `le- lˈivʁ`). Each table also carries a small **letter-to-sound model** trained on
the whole word list (back-off tables over letter contexts, with the stress placed by the
language's own stress rule); it reads the words the table does not list, and the table keeps only
the words that the model and the rules get wrong. Held-out sentences are FLORES-200, not part of
those word lists; the rows above count every word, and words outside the word lists are wrong far
more often (German 47 % of 811 such words, Dutch 55 % of 803, French 32 % of 330, mostly the
stress of long compounds and names) than listed words (German 0.1 %, Dutch 1.8 %, French 1.6 %).
Numbers follow each voice: German units before tens (`ˈaɪn ʊnttsvˈantsɪç`), Dutch one-word tens
and years counted in hundreds, French one-word readings stressed on the last word.

**Italian, Portuguese (Brazil and Portugal), Polish, Turkish, Czech, Swedish, Catalan, Romanian,
Hungarian, Finnish, Norwegian Bokmål and Danish** are built the same way: a word table with a
letter-to-sound model and its exceptions (feature `embed-<tag>`, 46 KB to 660 KB each), number words
and letter names with the voice's readings, a list of words without stress inside a clause, and
rules for what happens between words: Portuguese final s, r and unstressed vowels before the next
word (`os amigos` → `ʊz ˌæmˈiɡʊs`; not before digits or h), Polish and Czech voicing of a final
consonant before the next word's first, Czech prepositions said as one word with the next
(`na stole` → `nˈastole`), Catalan spirants, voiced finals and nasal place, Finnish final n. Their
letter rules are short and only read words when the table is not loaded. Words outside the word
lists: Italian 31.6 % wrong, Brazilian Portuguese 41.0 %, European Portuguese 49.0 %, Polish 29.8 %,
Turkish 22.0 %, Czech 9.2 %, Swedish 32.3 %, Catalan 40.3 %, Romanian 48.2 %, Hungarian 2.0 %,
Finnish 24.2 %, Norwegian 15.3 %, Danish 68.1 %. Hungarian has, besides, a list of 2.4 k
words that carry only secondary stress at the end of a clause (`város`, `volt`, `is`), as the voice
reads them, and number words with one stress per group of three digits. Roman numerals and
capitals follow each voice in compatible mode: cardinals in Spanish, Portuguese, Catalan, Czech
and Romanian, "römisch vierzehn" in German, "quatorze romain" in French, ordinals in Italian and
Danish, and letters in Dutch, Polish, Turkish, Swedish, Hungarian, Finnish and Norwegian; improved
mode reads the numeral as a number in every language (an ordinal in Italian and Danish). Capitals
of two or three letters are spelled, longer ones with a vowel are read as a word. Known gaps:
Danish stress of unlisted words, Roman numerals above 49 outside Portuguese.

**Spanish** (`es`) is the first language on the Latin-script engine (`crates/uccharan/src/latin.rs`):
letter rules, the Spanish stress rule with written accents, secondary stress on alternate syllables,
two stresses on -mente adverbs, spirant b d g and nasal place across words inside a clause, Roman
numerals, acronyms spelled with Spanish letter names, and Spanish number words. Improved mode drops
the voice's open ɛ before n + consonant and its aɪ eɪ diphthong notation, and keeps d a stop after
l and n.

**Urdu and Sindhi** are written without most short vowels. uccharan reads them in two layers:
letter rules (each consonant carries a short vowel unless a vowel letter or mark follows, the
Hindi-type deletion, weight stress for Urdu and penultimate stress for Sindhi, and the consonant
pairs both voices say without a vowel), and a word table made from the 120,000 most frequent
Urdu and 61,000 most frequent Sindhi words of the AI4Bharat Sangraha corpus as espeak-ng's voices
read them in running text. The Urdu table holds the words the rules read differently (`ur.ucd`,
47 k entries, 543 KB). The Sindhi table (`sd.ucd`, 6.9 k entries, 120 KB) holds a model that
corrects the rules' reading of a word, sound by sound in its context, and the words that rules and
model together still get wrong. On the held-out sentences, words in the tables differ from
espeak-ng in 0.4 % (Urdu) and 0.9 % (Sindhi) of cases, other listed words in 0.9 % and 0.5 %, and
words outside the lists (about one word in thirty) in 48 % (Urdu) and 22 % (Sindhi; 30 % with the
rules alone). The same kind of model did not help Urdu (on the development sentences, words
outside the list went from 45.7 % wrong to 49.6 %: they are mostly foreign names, which the
rules already read as the voice's own rules do), so Urdu does not use one. Improved mode uses the same table,
reads ص and ظ as s and z, and reads the Urdu digits ۰–۹, which the espeak-ng voices skip. In the
browser the tables load with the language (`loadLanguage("ur")`).

**Bhojpuri and Maithili** have no espeak-ng voice; text in them given to espeak-ng is read with the
Hindi voice, so compatible mode reproduces that Hindi reading (the rows above measure it against
`espeak-ng -v hi`). Improved mode reads them as Bhojpuri and Maithili: ऐ औ as əi əu, vowel letters
इ उ after a vowel as vowels of their own, the avagraha (also typed as a Latin *s*: हs) as a
pronounced final ə, ड़ ढ़ as flaps, chandrabindu as a nasal vowel, the vowel before the past
suffix -ल- kept (रहलन ɾəhələn), unstressed copula and pronouns, Bhojpuri number words (दू, छव,
एगारह, पहिला), Kaithi script (U+11080–U+110CF) read as the Devanagari it transliterates to, and an
experimental romanised-Bhojpuri path (`--hinglish`); Maithili also keeps word-final short i and u
(अछि əcʰɪ). Hand-written gold sets, split before any work ([tools/eval/gold/bho](../tools/eval/gold/bho):
437 words, 153 sentences; [tools/eval/gold/mai](../tools/eval/gold/mai): 410 words, 134 of them with
Wiktionary's pronunciations via WikiPron, and 151 sentences), held-out halves:

| Held-out | Hindi-voice fallback (espeak-ng `hi`) | uccharan compatible | **uccharan improved** |
| --- | ---: | ---: | ---: |
| Bhojpuri words (224), exact | 62.9 % | 63.8 % | **96.4 %** |
| Bhojpuri sentences (82): PER / identical | 5.54 % / 28.0 % | 5.54 % / 28.0 % | **0.43 % / 91.5 %** |
| Maithili words (226), exact | 68.6 % | 66.4 % | **93.4 %** |
| — of which Wiktionary pronunciations | 62 % | 56 % | **82 %** |
| Maithili sentences (71): PER / identical | 5.66 % / 32.4 % | 5.60 % / 32.4 % | **0.29 % / 94.4 %** |

Apart from the Wiktionary items, the gold sets encode the grammars' descriptions as written down by
us, so they show that improved mode implements them, not that a native speaker would accept every
form; items to check with a native speaker are listed in
[docs/bhojpuri-maithili-review.md](bhojpuri-maithili-review.md).
