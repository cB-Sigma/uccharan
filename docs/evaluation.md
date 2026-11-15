# Evaluation

Every number on this page comes from a run on the development machine (4 shared vCPUs, no GPU;
the machine was busy with other work, see "Speed") on 2026-11-13. The raw results are in
[`tools/eval/results`](../tools/eval/results) and [`tools/bench/results`](../tools/bench/results);
the commands to repeat each run are given in each section. The reference is espeak-ng **1.52.0**,
called through its library API (the path Piper and `phonemizer` use).

uccharan makes four separate claims, and each is measured separately:

1. **Compatible mode matches the reference** closely enough that existing voices work → agreement
   and end-to-end tests.
2. **Improved mode is more correct than the reference** → hand-curated gold sets.
3. **Both are faster than the reference** → benchmarks.
4. **The Kokoro English front end matches misaki**, the G2P Kokoro's English voices were trained
   on → section 6.

## 1. Agreement with the reference (compatible mode)

```sh
python tools/eval/run_eval.py --lang en-us --set heldout --words --out tools/eval/results
```

### Data

| Set | Source | Sentences | Used for tuning? |
| --- | --- | --- | --- |
| `en_dev`, `hi_dev` | Tatoeba (CC BY 2.0 FR) | 2 000 each | yes |
| `en_heldout`, `hi_heldout` | Tatoeba, disjoint ids | 3 000 each | **no** |
| `en_prose` | *The Adventures of Sherlock Holmes* (public domain) | 1 000 | **no** |

Words: `en_words_frequent` (5 000 frequent Tatoeba words in CMUdict), `en_words_rare` (3 000 CMUdict
words never seen in Tatoeba: mostly surnames and place names), `en_words_oov` (1 997 words not in
CMUdict), `hi_words_heldout` (3 096 distinct words of the Hindi held-out sentences).

### Metrics

Phoneme error rate (PER) = edit distance between the two NFD codepoint sequences divided by the
reference length; stress marks and spaces count as symbols. "Exact" = identical sentence. "Piper
shape" compares the per-sentence strings with clause punctuation that Piper feeds its voices.
Numbers are read in the reference's style for this comparison (`--normalize espeak`).

### Held-out results

| Language (voice) | Set | PER | PER without stress | Exact sentences | Piper-shape PER | Piper-shape exact |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| en-US (`en-us`) | Tatoeba held-out | **1.16 %** | 0.94 % | 76.8 % | 1.13 % | 76.7 % |
| en-US (`en-us`) | Sherlock Holmes | 1.40 % | 1.15 % | 49.3 % | 1.40 % | 48.0 % |
| en-GB (`en-gb-x-rp`) | Tatoeba held-out | **1.39 %** | 1.15 % | 72.5 % | 1.36 % | 72.4 % |
| en-GB (`en-gb-x-rp`) | Sherlock Holmes | 1.51 % | 1.25 % | 45.3 % | 1.51 % | 44.2 % |
| Hindi (`hi`) | Tatoeba held-out | **0.09 %** | 0.08 % | 99.1 % | 0.09 % | 99.0 % |

For comparison, the development sets (which the rules were tuned on) give en-US 1.16 %, en-GB 1.41 %,
Hindi 0.15 %: held-out and development agree, so the rules are not over-fitted to the development
sample.

**German, Dutch and French** (`de`, `nl`, `fr`; FLORES-200 devtest, 1,012 sentences each; development sets are FLORES dev). The first scoring, each done once with letter rules and larger word tables, gave PER 1.29 % (German; 1.30 % before two small fixes found on other data), 1.91 % (Dutch) and 0.98 % (French). A letter-to-sound model was then added to the word tables: back-off tables over letter contexts, trained on the full word lists (139 k German, 80 k Dutch, 100 k French words), with stress marks chosen so that the language's own stress rule gives the reference pattern. It reads words the table does not list, and the tables now keep only the words the model and the rules get wrong (`de.ucd` 54 k → 16 k entries, 748 → 357 KB; `nl.ucd` 41 k → 12 k, 512 → 231 KB; `fr.ucd` 24 k → 12 k, 225 → 170 KB). The model's settings were chosen on words held out of its own training list (every tenth word: German 62.1 → 69.0 % of those words right, Dutch 48.6 → 59.4 %, French 83.7 → 86.4 %), not on FLORES. The FLORES-200 held-out sentences were then scored again (one earlier variant of the model was also scored on them: German 1.10 %, Dutch 1.71 %, French 0.85 %), so these are not first scorings:

| | PER | Without stress | Identical sentences | Sentences without digits | Listed words wrong | Words outside the list wrong |
|---|---|---|---|---|---|---|
| German | **1.01 %** (was 1.29 %) | 0.57 % | 59.0 % | 0.82 % | 0.1 % of 16,360 | 46.7 % of 811 (was 53.8 %) |
| Dutch | **1.77 %** (was 1.91 %) | 1.11 % | 37.3 % | 1.39 % | 1.8 % of 18,370 | 54.7 % of 803 (was 60.4 %) |
| French | **0.85 %** (was 0.98 %) | 0.69 % | 57.2 % | 0.65 % | 1.6 % of 19,592 | 32.4 % of 330 (was 34.6 %) |

Speed against the same espeak-ng voices with the model in place (throughput on the held-out sentences, one thread, a busy machine): German 2.4×, Dutch 1.8×, French 2.8× at first; after the model's run-time was rewritten (same output on every development and held-out sentence) German 5.8×, Dutch 3.2×, French 5.9×. The tables also got a key filter of one byte per entry, so the files are 4 to 10 % larger than the sizes above (`de.ucd` 373 KB, `nl.ucd` 242 KB, `fr.ucd` 181 KB).

Known gaps: German and Dutch stress of long compounds and names outside the lists; Dutch final d before brackets and dashes, and "een" read as the numeral in some contexts; French liaison before h aspiré and before numbers, the stress of *sont* and *ont* before a participle, spaces as thousands separators (all four were worked on after 0.5.0: see "French gaps, Danish words, Catalan sandhi and spaced numbers" below).

**Thirteen more Latin-script languages** (FLORES-200 devtest, 1,012 sentences each, each scored once; the development half is FLORES dev; European Portuguese uses the same sentences as Brazilian Portuguese). Word groups are counted on the sentences whose words align with the reference's. Speed is throughput on one thread against the espeak-ng voice, measured after the run-time rewrite (before it: Turkish 0.9×, Czech 0.7×, Finnish 0.7×, Norwegian 0.9×, Polish 1.0×).

| language | voice | PER | without stress | without digits | identical sentences | in table | other listed | outside list (words) | speed |
|---|---|---|---|---|---|---|---|---|---|
| Italian | `it` | 0.53 % | 0.38 % | 0.37 % | 76.0 % | 0.5 % | 0.1 % | 31.6 % (503) | 4.6× |
| Portuguese (Brazil) | `pt-br` | 0.61 % | 0.38 % | 0.40 % | 61.8 % | 0.7 % | 0.8 % | 41.0 % (490) | 5.1× |
| Portuguese (Portugal) | `pt` | 0.84 % | 0.66 % | 0.60 % | 49.8 % | 1.5 % | 1.8 % | 49.0 % (484) | 5.2× |
| Polish | `pl` | 1.12 % | 0.83 % | 0.92 % | 36.7 % | 3.4 % | 2.6 % | 29.8 % (1058) | 3.1× |
| Turkish | `tr` | 0.68 % | 0.37 % | 0.47 % | 66.8 % | 0.1 % | 0.0 % | 22.0 % (1087) | 2.7× |
| Czech | `cs` | 1.31 % | 0.79 % | 1.07 % | 45.8 % | 1.1 % | 2.9 % | 9.2 % (501) | 1.8× |
| Swedish | `sv` | 1.32 % | 0.95 % | 0.83 % | 59.8 % | 0.1 % | 0.0 % | 32.3 % (1161) | 3.7× |
| Catalan | `ca` | 1.44 % | 1.30 % | 1.10 % | 32.2 % | 4.4 % | 2.3 % | 39.8 % (553) | 4.9× |
| Romanian | `ro` | 0.96 % | 0.40 % | 0.86 % | 51.7 % | 0.1 % | 0.0 % | 48.2 % (629) | 3.3× |
| Hungarian | `hu` | 1.34 % | 0.90 % | 0.95 % | 48.9 % | 1.9 % | 0.2 % | 2.0 % (1121) | 13.1× |
| Finnish | `fi` | 1.36 % | 1.06 % | 1.08 % | 30.3 % | 5.7 % | 4.5 % | 24.2 % (1938) | 1.7× |
| Norwegian Bokmål | `nb` | 0.96 % | 0.83 % | 0.46 % | 69.2 % | 1.3 % | 0.0 % | 15.3 % (1119) | 2.3× |
| Danish | `da` | 1.95 % | 1.23 % | 1.49 % | 43.2 % | 0.7 % | 0.4 % | 62.3 % (930) | 27.1× |

**Capitals, Roman numerals and single letters: all seventeen Latin-script languages scored again.**
After the scores above, four things were changed on the development sentences and on probes of the
reference voices, and every held-out half was then scored once more (a documented re-score: the
second for most languages, the third for German, Dutch, French and Hungarian). (1) A token in
capitals is spelled only when it has two or three letters or no vowel; longer capitals with a vowel
are read as a word (`NATO`, `COVID`), and a short one is spelled even when its letters make a word,
apart from a few small words per language. Finnish, Hungarian and Czech put the main stress of a
spelled abbreviation on its first letter; Turkish capital I is the dotless letter. (2) Roman
numerals follow each voice: cardinals up to 49 (Spanish, Catalan, Czech, Romanian; any value in
Portuguese), "römisch" + cardinal (German), cardinal + "romain" (French), ordinals (Italian,
Danish), letters like any abbreviation (Dutch, Polish, Turkish, Swedish, Hungarian, Finnish,
Norwegian). A fault that read `IV` as five and `IX` as ten in every language was fixed with it.
(3) Catalan: final s and ʃ are voiced before a word written with h and a vowel, ʃ and z before
voiced sounds, the conjunction *i* is a glide before a vowel; Spanish and Catalan read a single
consonant letter by its name. (4) Finnish tens after a hundred carry secondary stress
(`kˈolmesˌataː vˌiːsikymmentæ`).

| language | PER before | PER now | without digits | identical sentences |
|---|---|---|---|---|
| Spanish | 0.74 % | **0.55 %** | 0.32 % | 68.1 % |
| German | 1.01 % | **0.95 %** | 0.77 % | 59.3 % |
| Dutch | 1.77 % | **1.69 %** | 1.32 % | 37.8 % |
| French | 0.85 % | **0.79 %** | 0.59 % | 57.7 % |
| Italian | 0.66 % | **0.53 %** | 0.37 % | 76.0 % |
| Brazilian Portuguese | 0.67 % | **0.61 %** | 0.40 % | 61.8 % |
| European Portuguese | 0.89 % | **0.84 %** | 0.60 % | 49.8 % |
| Polish | 1.33 % | **1.19 %** | 0.92 % | 36.4 % |
| Turkish | 0.72 % | **0.68 %** | 0.47 % | 66.8 % |
| Czech | 1.40 % | **1.32 %** | 1.07 % | 45.2 % |
| Swedish | 1.51 % | **1.44 %** | 0.83 % | 59.1 % |
| Catalan | 1.86 % | **1.51 %** | 1.18 % | 29.2 % |
| Romanian | 1.00 % | **0.97 %** | 0.86 % | 51.7 % |
| Hungarian | 1.44 % | **1.37 %** | 0.95 % | 48.7 % |
| Finnish | 1.64 % | **1.57 %** | 1.08 % | 30.1 % |
| Norwegian | 1.11 % | **1.03 %** | 0.46 % | 68.7 % |
| Danish | 2.26 % | **2.22 %** | 1.78 % | 37.5 % |

The table of the thirteen languages above shows the numbers of the latest scoring (below) in every column. Words outside the
word lists are now wrong in: German 46.6 %, Dutch 53.7 %, French 31.6 %.

Hungarian was scored a second time. The first scoring gave PER 2.08 % (without digits 1.61 %, identical sentences 19.3 %; listed words outside the table 6.6 % wrong): the voice gives about 2,400 frequent words only secondary stress at the end of a clause, and reads numbers from 10,000 with one stress per group of three digits, 2,000 and 2,000,000 as one word. Both were added from the word list's clause-final readings and from number probes (development half 1.92 → 1.38 %), not from the held-out half.

**French gaps, Danish words, Catalan sandhi and spaced numbers: the Latin-script languages scored
again.** After the release of 0.5.0 the following was changed on the development sentences and on
probes of the reference voices (espeak-ng 1.52.0), and every held-out half was then scored once more
(a documented re-score; the first scoring of the Belgian and Swiss French voices). (1) French: in
compatible mode a word written with h first blocks liaison unless it is on a list of h-muet words
(the reference treats h as aspirated by default), and improved mode uses the h-aspiré words of the
usual dictionaries; number words keep or drop their final consonant by what follows (*six
enfants*, *six livres*, *neuf ans*); *ont* and *sont* lose their stress after a clitic pronoun
(*ils ont*, *n'en ont*, *y sont*); the voice reads *son* after *de* and *le* as the stressed noun;
`fr-be` and `fr-ch` voices (septante, huitante, nonante). (2) Numbers: compatible mode joins groups
of three digits only across an ordinary space, as the reference voices do; after a no-break or
thin space (U+00A0, U+2009, U+202F, common in news text) the groups are read as numbers of their
own, and a group with a leading zero as the voices read it (`000` "zero-zero zero"). Improved mode
joins groups across all of these spaces. (3) Danish: 13,600 more words (Leipzig Corpora Collection
`dan_news_2020_1M`, seen at least five times) were added to the word list and 41,000 rarer ones
only train the letter-to-sound model (`data/sources/da/train.tsv`); `da.ucd` grew from 490 to
658 KB. (4) Catalan: the conjunction *i* is a glide after a word written with a vowel last and
before a word written with a vowel or h first except *u*; *hi*, *ho*, *lo* and final unstressed i
and u become glides before a vowel; *o* is w between vowels; a final r is a tap before a vowel and
a trill after a stressed vowel before a consonant; p stays voiceless before l. Development PER:
French 0.80 → 0.63 %, Danish 2.21 → 1.94 %, Catalan 1.56 → 1.48 %, Spanish 0.63 → 0.62 %, Czech
1.30 → 1.29 %, the others unchanged.

| language | PER before | PER now | without stress | without digits | identical sentences |
|---|---|---|---|---|---|
| Spanish | 0.55 % | **0.54 %** | 0.43 % | 0.32 % | 68.3 % |
| French | 0.79 % | **0.63 %** | 0.54 % | 0.54 % | 64.7 % |
| French (Belgium), `fr-be` | — | **0.63 %** | 0.54 % | 0.54 % | 64.6 % |
| French (Switzerland), `fr-ch` | — | **0.62 %** | 0.53 % | 0.54 % | 64.6 % |
| Polish | 1.19 % | **1.12 %** | 0.83 % | 0.92 % | 36.7 % |
| Czech | 1.32 % | **1.31 %** | 0.79 % | 1.07 % | 45.8 % |
| Swedish | 1.44 % | **1.32 %** | 0.95 % | 0.83 % | 59.8 % |
| Catalan | 1.51 % | **1.44 %** | 1.30 % | 1.10 % | 32.2 % |
| Romanian | 0.97 % | **0.96 %** | 0.40 % | 0.86 % | 51.7 % |
| Hungarian | 1.37 % | **1.34 %** | 0.90 % | 0.95 % | 48.9 % |
| Finnish | 1.57 % | **1.36 %** | 1.06 % | 1.08 % | 30.3 % |
| Norwegian | 1.03 % | **0.96 %** | 0.83 % | 0.46 % | 69.2 % |
| Danish | 2.22 % | **1.95 %** | 1.23 % | 1.49 % | 43.2 % |

German, Dutch, Italian, both Portuguese voices and Turkish did not change. The `fr-be` and `fr-ch`
voices are scored on the French sentences (the Belgian and Swiss readings differ from the French
one only in 70–99). Words outside the word lists are now wrong in: French 31.5 % (333 words),
Catalan 39.8 % (553), Danish 62.3 % (930; was 68.1 % of 1,025).


**Spanish** (`es`, FLORES-200 devtest, 1,012 sentences, scored once): PER **0.74 %**, without
stress 0.57 %, identical sentences 66.3 %; sentences without digits 0.45 %. Development set (FLORES
dev, used for the rules): 0.73 %.

**Urdu and Sindhi** (FLORES-200 devtest, 1,012 sentences each; `python tools/eval/run_eval.py --lang
ur --set heldout`): PER **1.44 %** (Urdu) and **1.52 %** (Sindhi), without stress 1.08 % and 1.15 %,
identical sentences 56.1 % and 51.1 %. The word tables are built from corpus word lists (120 k Urdu,
61 k Sindhi words), not from FLORES, but they contain many held-out words, so word errors are
reported by group (aligned sentences only):

| Held-out words | Urdu: words / differ | Sindhi: words / differ |
| --- | ---: | ---: |
| in the word table | 13,517 / 0.4 % | 7,950 / 0.4 % |
| in the word lists, read by the letter rules | 8,296 / 0.9 % | 11,335 / 0.8 % |
| outside the word lists | 719 / 47.6 % | 895 / 29.8 % |

This is the second scoring of these held-out halves. The first (with 40 k / 20 k-word tables and the
earlier letter rules) gave PER 2.18 % and 2.79 %; its "outside the table" figure of 27 % counted
words with attached Arabic punctuation (گیا۔) as outside the table, so it is not comparable with the
last row. Development halves: Urdu 2.17 → 1.49 %, Sindhi 2.79 → 1.45 %.

**A correcting model for words outside the lists (Sindhi).** The word-table model of the
Latin-script languages reads spellings; Urdu and Sindhi spellings leave out most short vowels, and a
model on the spelling alone was worse than the letter rules (every tenth word of the lists held out
of training, whole word with stress: Urdu 41.6 % against 55.3 % for the rules, Sindhi 38.7 % against
70.4 %). A model that reads the rules' reading and corrects it was better on those words (Urdu
61.5 %, Sindhi 74.8 %). In running text (development sentences, words outside the lists) it helped
Sindhi (28.4 % wrong → 21.6 %; 19.4 % when a change of stress alone is not taken) and hurt Urdu
(45.7 % → 49.6 %; 47.0 % with that guard; with contexts seen at least 3 or 6 times 47.9 % and
48.2 %): Urdu's unlisted words are mostly foreign names, which the rules already read as the
voice's own rules do. So Sindhi uses the model and Urdu keeps its table of exceptions. The Sindhi
held-out half was scored a third time with it: PER **1.30 %** (was 1.52 %), without stress 0.90 %,
identical sentences 54.9 %, sentences without digits 0.70 %; words in the table 0.9 % wrong
(4,257), other listed words 0.5 % (15,028), words outside the list 21.9 % (895; was 29.8 %: 100
words fixed, 29 broken). Development half 1.45 → 1.23 %. `sd.ucd` 209 → 120 KB.

### Breakdown

**By kind of edit** (share of all edit operations, held-out):

| | Vowel quality | Stress | Mixed / other | Consonant | Length | Word boundary | Schwa (hi) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| en-US | 40.9 % | 29.7 % | 14.9 % | 10.5 % | 2.2 % | 1.7 % | — |
| en-GB | 43.0 % | 28.1 % | 17.3 % | 9.3 % | 0.9 % | 1.4 % | — |
| Hindi | 16.1 % | 35.0 % | 10.5 % | 21.7 % | 3.5 % | 3.5 % | 9.1 % |

**By word class** (word-level disagreement in sentences whose word counts line up):

| | Function words (weak forms) | Content words in CMUdict | Words not in CMUdict |
| --- | ---: | ---: | ---: |
| en-US | 0.60 % of 9 063 | 5.0 % of 10 533 | 31.1 % of 196 |
| en-GB | 0.60 % of 9 063 | 6.2 % of 10 533 | 30.6 % of 196 |
| Hindi (all words) | — | 0.13 % of 18 139 | — |

**Numbers.** Sentences containing digits: en-US PER 3.6 % (31 sentences), en-GB 4.3 %, Hindi 3.1 %
(22 sentences), against 1.1 %, 1.3 % and 0.08 % for the rest. The reference's number readings have
many idiosyncrasies (digit grouping, "hundred" years, Hindi stress on number words) that are only
partly reproduced.

**Number formats, ordinals and dates (0.6.3).** Compatible mode was brought closer to the
reference voices on number formats: the word for a full stop between digits, Czech and Hungarian
"1.000", the digits after a decimal comma read as one number (Italian, Portuguese, Polish, Czech,
Turkish, Finnish, Catalan, Romanian), Hungarian *tized/század/ezred*, the German minus, the
Sinhala hundred, thousand and `%`, and the ordinals the voices read ("3." in Danish, Norwegian and
Hungarian; *1er*, *1º*, *1r*, *1e*, *1:a*). On 17 Latin probe phrases per language with 14 number
formats, 116 of 238 differed from the reference before and 54 after (most of the rest are stress
marks and the joining of words after the decimal sign). Development PER before → after (all
other languages unchanged): si 0.76 → 0.36 %, hu 1.22 → 1.06 %, nb 0.94 → 0.87 %, da 1.91 →
1.84 %, pt 0.88 → 0.83 %, pt-br 0.65 → 0.61 %, fi 1.12 → 1.09 %, fr / fr-be / fr-ch 0.62 →
0.60 %, ca 1.47 → 1.45 %, nl 1.62 → 1.60 %, cs 1.28 → 1.27 %, de 1.06 → 1.05 %, es 0.55 →
0.54 %, it 0.63 → 0.62 %, pl 1.12 → 1.11 %, ro 0.95 → 0.94 %, sv 1.27 → 1.26 %, tr 0.55 →
0.54 %; no language got worse. The changes touch numbers only, so the held-out halves were not
scored again. Dates stay as the voices read them in compatible mode (piece by piece); improved
mode and normalise-only mode read them with month names, and ordinals in full (tests in
`crates/uccharan/tests/numbers_dates.rs`). Konkani and Sinhala improved mode now read numbers as
their written words: the round trip of normalise-only mode through the phonemiser (improved)
differs by 0.6 % of phonemes in both (was 38.5 % and 13.5 %).

**Word lists** (isolated words):

| List | PER | Exact | Exact ignoring stress | Stress pattern wrong |
| --- | ---: | ---: | ---: | ---: |
| en-US frequent, in CMUdict | 2.3 % | 89.8 % | 91.2 % | 3.2 % |
| en-US rare, in CMUdict | 18.0 % | 46.7 % | 49.0 % | 21.6 % |
| en-US not in CMUdict (1 997) | 19.1 % | 43.6 % | 47.8 % | 29.0 % |
| en-GB frequent | 2.6 % | 89.2 % | 90.5 % | 3.5 % |
| en-GB rare / not in CMUdict | 17.8 % / 18.7 % | 46.7 % / 44.5 % | 49.0 % / 49.1 % | 21.7 % / 29.5 % |
| Hindi held-out words | 0.23 % | 99.6 % | 99.6 % | 0.2 % |

Rare CMUdict words agree badly because the reference reads most of them (surnames, place names)
with its spelling rules while uccharan uses CMUdict's entry; which of the two is right varies.

### Top remaining error classes

1. **Unstressed vowel quality** (en): `ɪ` versus `ə` versus `ᵻ` in unstressed syllables
   (*mathematics*, *necessary*, *medicine*), `i`/`ɪ` in final *-y* and *-ies*. These follow the
   reference's spelling rules, which CMUdict-derived entries do not encode.
2. **Phrase-level stress**: which word in a clause keeps its primary stress, and secondary stress
   inside compounds (`ˌ` versus `ˈ` versus none). About 30 % of all English disagreements.
3. **Names and other words outside CMUdict**: the letter-to-sound tables get 44 % of such words
   exactly right in the reference's notation.
4. **Numbers** (all languages): readings of years, decimals and digit groups.
5. **Hindi**: stress on long compounds and a handful of loanwords; English words inside Hindi
   sentences (`en` voice quirks).

## 2. End-to-end: real voices, real speech recognition

```sh
python tools/eval/e2e_tts_asr.py --lang en-us --voice en_US-amy-low.onnx \
    --asr whisper-onnx:/models/whisper-base/base --n 500 --out tools/eval/results
```

500 held-out sentences per voice (without digits, so that number reading does not dominate) are
synthesised twice through **sherpa-onnx 2.0.1** with `GenerationConfig.phoneme_codepoints`: once
from the reference's phonemes (rebuilt in Piper's shape), once from uccharan's. Both recordings are
transcribed by the same recogniser and scored against the text. When both phoneme streams are
identical the audio is synthesised once and used for both. The interval is a 95 % paired bootstrap
over sentences of WER(uccharan) − WER(reference).

| Voice | ASR | Identical phonemes | WER reference | WER uccharan | Difference [95 % CI] | Sentences better / worse |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `en_US-amy-low` (0.2.0) | Whisper base (int8, sherpa-onnx) | 383 / 500 | **2.94 %** | 2.97 % | +0.02 [−0.21, +0.25] | 10 / 11 |
| `en_US-lessac-medium` (0.1.0) | Whisper base | 377 / 500 | 4.30 % | **4.21 %** | −0.09 [−0.50, +0.30] | 16 / 12 |
| `en_GB-alba-medium` (0.1.0) | Whisper base | 347 / 500 | **7.36 %** | 7.47 % | +0.11 [−0.55, +0.77] | 29 / 25 |
| `hi_IN-rohan-medium` (0.1.0) | Whisper tiny, Hindi fine-tune (faster-whisper) | 495 / 500 | 8.95 % | **8.82 %** | −0.12 [−0.32, 0.00] | 2 / 0 |
| `es_ES-davefx-medium` (after 0.5.0) | Whisper base | 159 / 200 | 13.23 % | **12.90 %** | −0.33 [−0.81, +0.12] | 17 / 9 |
| `de_DE-thorsten-medium` (after 0.5.0) | Whisper base | 136 / 200 | 21.76 % | **20.99 %** | −0.77 [−1.77, +0.24] | 29 / 20 |
| `fr_FR-siwis-medium` (after 0.5.0) | Whisper base | 136 / 200 | **19.28 %** | 19.39 % | +0.10 [−0.82, +1.04] | 24 / 28 |
| `it_IT-paola-medium` (after 0.5.0) | Whisper base | 159 / 200 | 30.02 % | **29.91 %** | −0.11 [−0.98, +0.79] | 18 / 13 |

Reading the table honestly:

- In every row the confidence interval contains zero, which is the claim:
  voices driven by uccharan are understood as well as when driven by the reference. "Better" is
  not claimed; the small negative differences are not significant.
- The recognisers are small (base and tiny models) because the machine has no GPU. Larger
  recognisers would have lower absolute WER; the comparison between the two columns is what
  matters.
- **The amy-low runs, all of them.** Synthesis in sherpa-onnx's VITS is random (the same
  phonemes give different audio from run to run; checked), so repeated runs of the same build
  differ. For 0.2.0 the run was repeated three times while compatible mode was still being fixed:
  2.97 % → 3.17 % (+0.21, CI [−0.09, +0.52]) and 3.06 % → 3.36 % (+0.30, CI [+0.05, +0.57] — a
  significant loss) on builds before the last fixes, then 2.94 % → 2.97 % (+0.02, CI [−0.21,
  +0.25]) on the released build (the row above). The fixes in between came from the sentences
  that came out worse (*beyond*, *realize*, *strength*, *deodorant*, *Mary's*, *n't have*); names
  such as *Gabor*, *Tatoeba* and *Skura*, which the reference reads by its spelling rules and
  uccharan by CMUdict or its letter-to-sound tables, still differ. For 0.1.0 the run gave
  3.17 % → 2.97 % after a first run of 3.13 % → 3.56 % that led to three name corrections. The
  lessac, alba and rohan rows are from 0.1.0 (those voices were not re-run for 0.2.0; their
  phoneme streams changed only slightly — agreement in section 1).
- **Spanish, German, French and Italian** (200 held-out FLORES-200 sentences without digits each,
  one run, Whisper base int8 transcribing in the voice's language): no difference is significant.
  The WER levels are high because Whisper base is weak on synthetic speech in these languages
  (Italian most of all); the comparison between the columns is what matters. Telugu, Marathi,
  Bengali and Urdu could not be checked offline: Piper has no Marathi, Bengali or Urdu voice, and
  the installed recognisers (Whisper base and tiny) do not transcribe Telugu (they answer in other
  scripts).
- A human listening test was **not** done.

## 3. Profile conformance

```sh
python tools/eval/conformance.py --piper VOICE.onnx.json … --tokens en-us=tokens.txt … \
    --kokoro-tokens kokoro/tokens.txt --kitten-tokens kitten/tokens.txt --misaki-gold us_gold.json
```

| Check | Result |
| --- | --- |
| Piper id framing (`[^, _, p, _, …, $]`) against an independent implementation, 3 000 held-out texts per voice | identical for `en_US-amy-low`, `hi_IN-rohan-medium`, `en_US-lessac-medium` and `en_GB-alba-medium` |
| Symbols outside the voice's `phoneme_id_map` | `hi_IN-rohan-medium` (161 symbols): none. `en_US-amy-low` (130 symbols): U+0329 (syllabic mark) 47 times in 3 000 texts; the reference emits it 50 times on the same texts, so the gap is the voice's, not uccharan's. `en_US-lessac-medium` and `en_GB-alba-medium` (166-symbol maps): none. |
| Piper id sequence vs ids built from the reference (compatible mode, reference-style numbers) | amy-low: 76.7 % of texts identical, id error rate 0.76 %; rohan: 98.8 %, 0.09 % (0.2.0). lessac: 76.1 %, 0.78 %; alba: 72.0 %, 0.92 % (0.1.0 run; those two voices were not on the machine for 0.2.0) |
| sherpa-onnx `tokens.txt` coverage | same as the Piper maps above |
| Kokoro: all tokens inside the 114-symbol vocabulary, `[0, …, 0]` framing | yes for en-US, en-GB and Hindi |
| Kokoro Hindi vs reference through the published espeak → Kokoro mapping | 98.8 % of sentences identical, token error rate 0.13 % |
| Kokoro English vs the misaki dictionary (`us_gold.json` of misaki 0.9.4, 3 629 frequent words) | 99.5 % identical (99.5 % ignoring stress); full sentences against misaki itself in section 6 |
| Kitten 0.8: built-in table equals the model's `tokens.txt`, ids `[0, …, 10, 0]` | yes; no symbol outside the table |
| Kitten ids vs the `phonemizer`-package front end Kitten uses | 75.8 % of texts identical, id error rate 1.32 % |
| Matcha-TTS | table and framing unit-tested; no Matcha model was available, **untested** against a model |
| `phonemizer` compatibility shim vs `phonemizer` 3.6 `EspeakBackend`, 1 000 held-out utterances | identical output: en-US 76.9 % (with stress), 83.9 % (plain), 70.3 % (with tie); en-GB 73.0 %; Hindi 98.3 % |

## 4. Correctness: improved mode against hand-curated gold sets

```sh
python tools/eval/gold_eval.py --out tools/eval/results
```

The gold sets in [`tools/eval/gold`](../tools/eval/gold) were written by hand. Expected
pronunciations come from CMUdict variants, general dictionaries' accounts of how names are said, and
standard descriptions of Hindi phonology — never from a phonemizer. Number and abbreviation items
state only the words that should be *said*; each system is scored by reading both the text and the
spoken form and comparing its own phonemes. Each file documents its check.

Two groups:

- **Development gold sets** (348 items): written first and **used to build improved mode**. A high
  score here only shows the rules do what they were written to do.
- **Unseen gold sets** (128 items, `gold/unseen/`): written after improved mode was frozen and run
  once. These are the honest numbers.

### Unseen sets (the numbers to quote)

| Set | Items | Reference (espeak-ng 1.52) | uccharan compatible | uccharan default | **uccharan improved** |
| --- | ---: | ---: | ---: | ---: | ---: |
| English homographs in context | 40 | 62.5 % | 60.0 % | 60.0 % | **82.5 %** |
| English proper nouns | 20 | 60.0 % | 50.0 % | 50.0 % | **65.0 %** |
| English numbers, dates, units | 20 | 35.0 % | 30.0 % | **85.0 %** | **85.0 %** |
| Hindi schwa / nasal / nukta words | 36 | 61.1 % | 61.1 % | 61.1 % | **97.2 %** |
| Romanised Hindi (experimental) | 12 | 0 % | 0 % | 0 % | **58.3 %** |
| **All unseen** | 128 | 51.6 % | 48.4 % | 57.0 % | **82.0 %** |

"Compatible" uses reference-style number reading; "default" is the library default (compatible
phonology, natural numbers). These sets were run once for 0.1.0 (improved 80.5 %, proper nouns
55.0 %) and again for 0.2.0, whose names path (below) was built on a separate names set; the
proper-noun set was not used to build it.

Where the reference still does as well or better: some foreign names outside every list
(*Sinead*, *Ciaran*, *Wojciech*). Romanised Hindi is experimental and fails on spellings outside
its word list (*rehte*, *kripya*).

### Names, by origin

```sh
python tools/eval/names_eval.py --split heldout --out tools/eval/results
```

[`gold/names.tsv`](../tools/eval/gold/names.tsv) holds 380 personal names of many origins, each in
a short sentence. References are CMUdict's variants where the name is in CMUdict and hand-written
IPA from standard pronunciation otherwise. The check compares the consonant skeleton and the
primary-stressed vowel, with notational variants merged for every system alike. The set was written
before any names work and split in two: `dev` (208 names) was used to build the Indian-name path,
`heldout` (172 names) was not. The Indian names in it are kept out of the classifier's training
list.

Held-out half:

| Origin | Names | Reference (espeak-ng 1.52) | uccharan compatible | **uccharan improved** | improved + WikiPron pack |
| --- | ---: | ---: | ---: | ---: | ---: |
| Indian | 48 | 43.8 % | 50.0 % | **77.1 %** | 75.0 % |
| English | 52 | 84.6 % | 100 % | **100 %** | 100 % |
| European | 17 | 70.6 % | 88.2 % | **100 %** | 100 % |
| Hispanic | 19 | 52.6 % | 89.5 % | **89.5 %** | 89.5 % |
| Chinese | 12 | 58.3 % | 91.7 % | **91.7 %** | 91.7 % |
| Other East Asian | 10 | 50.0 % | 100 % | **100 %** | 100 % |
| Arabic, Persian, Turkish | 6 | 50.0 % | 100 % | **100 %** | 100 % |
| African | 8 | **50.0 %** | 37.5 % | 37.5 % | 37.5 % |
| **Names not in CMUdict** | 41 | 34.1 % | 19.5 % | **56.1 %** | 53.7 % |
| Names in CMUdict | 131 | 70.2 % | 99.2 % | 99.2 % | 99.2 % |
| **All held-out** | 172 | 61.6 % | 80.2 % | **89.0 %** | 88.4 % |

The last column loads the optional CC BY-SA WikiPron pack (`--pack data/packs/wikipron-en-us.ucd`;
see [DATA_LICENSES.md](../DATA_LICENSES.md)). It was run once on the held-out half after the pack
was built and checked on the development half (where it changed nothing: 92.8 % with and without).
Wiktionary has only 8 of the 41 held-out names that CMUdict lacks. On Indian names it fixed one
(*Pooja*) and broke two (*Nandini*, *Patil*: Wiktionary's English entries are anglicised, and the
stress uccharan transfers onto them landed elsewhere), so the pack costs one name overall and does
not help the African names (Wiktionary lists one of the eight). Its value is elsewhere: ordinary words outside CMUdict.

Read the rows with care: for names that are in CMUdict the reference pronunciation *is* CMUdict's,
and so is uccharan's lexicon, so 99 % there is expected rather than impressive. The fair comparison
is **names not in CMUdict: 56.1 % against 34.1 %**, and Indian names (77.1 % against 43.8 %).
African names are the one origin where the reference does better (4 against 3 of 8 — too few to
say much). The development half gives 92.8 % against 74.0 % overall and 90.3 % against 73.6 % on
Indian names.

### African names

```sh
python tools/eval/build_african_names.py        # expand the hand-written source into the gold set
python tools/eval/names_eval.py --gold tools/eval/gold/african_names.tsv --split heldout
```

[`gold/african_names.src`](../tools/eval/gold/african_names.src) lists 388 African first names,
surnames and place names from 37 languages with English-context pronunciations written by hand from
each language's published grammar (sources in the header of the generated `.tsv`). Names from
tone languages have no fixed English stress, so every syllable an English speaker may stress is
accepted, and each plain vowel letter may take either of its two usual English values. The set was
written before any African-names rules and split by a hash of the id (193 dev, 195 held-out).
Held-out: espeak-ng 45.6 %, uccharan compatible 41.5 %, uccharan improved 75.4 % (41.0 % before
the African-names rules, 70.8 % after the first round, 73.3 % after the North African work, 75.9 %
now). Per-language results are in
`tools/eval/results/african_names_heldout.json`.

Several groups of that set were too small to say anything (one to four names), and the Egyptian
names of the North African set trailed the reference, so three more sets were written by hand
before any work on them, split by hash and used for tuning on the development half only. The
held-out halves were first scored after one round of work; Tigrinya still trailed by one name
(59.3 % against 63.0 %), so its development half was grown by 40 names (marked `!` in the source:
always development), the reader was improved on development data only, and all held-out halves were
scored a second and final time. Second scoring:

```sh
python tools/eval/build_african_names.py --north-african   # 181 Maghrebi, Egyptian, Berber names
python tools/eval/build_african_names.py --egyptian        # 167 Egyptian names (second set)
python tools/eval/build_african_names.py --se-african      # 184 Chichewa, Shona, Tsonga, Tigrinya names
python tools/eval/names_eval.py --gold tools/eval/gold/egyptian_names.tsv --split heldout
```

| Held-out | n | espeak-ng 1.52 | compatible | improved | improved + WikiPron |
| --- | ---: | ---: | ---: | ---: | ---: |
| North African set | 100 | 40.0 % | 33.0 % | **66.0 %** | 64.0 % |
| Egyptian set | 85 | 56.5 % | 48.2 % | **69.4 %** | 69.4 % |
| Chichewa | 24 | 41.7 % | 20.8 % | **87.5 %** | 87.5 % |
| Shona | 26 | 42.3 % | 30.8 % | **73.1 %** | 73.1 % |
| Tsonga | 19 | 42.1 % | 10.5 % | **73.7 %** | 73.7 % |
| Tigrinya | 27 | 63.0 % | 55.6 % | **70.4 %** | 70.4 % |

Development halves (used for tuning): Egyptian 75.6 % against 59.8 %; Tigrinya (59 names after
growing) 69.5 % against 45.8 %; the whole Chichewa/Shona/Tsonga/Tigrinya development half 78.1 %
against 38.3 %. First scoring of the held-out halves, for the record: North African 67.0 %,
Egyptian 69.4 %, Chichewa/Shona/Tsonga/Tigrinya 72.9 % (Tigrinya 59.3 %). The gold notation of the two new sets writes a
double consonant once, as English speakers say it (fixed before tuning; it also changed the
reference's development score from 56.1 % to 59.8 %).

What changed in improved mode: a list of 1 508 Arabic given names in English transliteration
(Wikidata, CC0; names that misaki's dictionary of common English words lists are left out, as are
Western given names) is read with the Arabic reader even when CMUdict has the name with English
stress; Egyptian spellings (final *-y* after two consonants is i, *aw* is aʊ, *ee*/*oo* long,
*dh*/*bh* split in *Medhat*/*Sobhy*, a stressed *a* before two consonants is English short a);
words of the names of Eritrean and Ethiopian people on Wikidata (3 828 words, CC0; read with the
African reader when the word is outside the English lexicon and not more Indian-looking); Ethiopian
and Eritrean name elements (*gebre*, *wolde*, *habte*, *tesfa*, *berhan*, *haile*,
*tekle*, *kidane*, *mehret*, *tsega*, *alem*, *fikre*); Bantu onsets *nts*, *nkh*, *ndz*, *ts*,
*hw*, *kw*, *mn* and clusters *ndw*, *ngw*, *mph*, *tsw*; Tsonga *x* = ʃ and *rh*; final *-ai* as two
syllables. A doubled *k* in words outside the lexicon is now read like *ck* in both modes (the
letter-to-sound model had dropped it: *Mekki*); held-out agreement is unchanged. The general names
set's held-out half moved from 91.9 % to 91.3 %: one of the six Arabic/Persian/Turkish names is now
read differently from CMUdict (5 of 6 against espeak-ng's 3 of 6). The list keeps a CMUdict reading
when it already stresses the same syllable with the same consonants and a fitting vowel; leaving
every CMUdict name alone instead would cost 10 points on the Egyptian development half.

### Development sets (used for tuning)

| Set | Items | Reference | Compatible | Default | Improved |
| --- | ---: | ---: | ---: | ---: | ---: |
| English homographs | 71 | 63.4 % | 54.9 % | 54.9 % | 100 % |
| English proper nouns and brands | 50 | 62.0 % | 74.0 % | 74.0 % | 100 % |
| English numbers, money, units | 40 | 35.0 % | 35.0 % | 97.5 % | 97.5 % |
| English abbreviations and initialisms | 29 | 51.7 % | 93.1 % | 100 % | 100 % |
| Hindi schwa deletion | 36 | 52.8 % | 52.8 % | 52.8 % | 100 % |
| Hindi anusvara / chandrabindu | 30 | 63.3 % | 63.3 % | 63.3 % | 100 % |
| Hindi nukta consonants | 24 | 54.2 % | 54.2 % | 54.2 % | 100 % |
| Hindi numbers, money, abbreviations | 18 | 16.7 % | 66.7 % | 100 % | 100 % |
| English words in Hindi text stay in the Hindi phone set | 20 | 30.0 % | 20.0 % | 20.0 % | 100 % |
| Hindi words in English text | 10 | 100 % | 100 % | 100 % | 100 % |
| Romanised Hindi | 20 | 0 % | 0 % | 0 % | 100 % |
| **All development** | 348 | 50.3 % | 55.7 % | 65.2 % | 99.7 % |

### Spanish, German, French, Italian and Portuguese words (improved mode)

`tools/eval/gold/improved/<lang>_{dev,test}.tsv`: 25 words per file and language, written by hand
from the JIPA *Illustrations of the IPA* (Castilian Spanish: Martínez-Celdrán, Fernández-Planas &
Carrera-Sabaté 2003; German: Kohler 1990; French: Fougeron & Smith 1993; Italian: Rogers &
d'Arcangeli 2004; European Portuguese: Cruz-Ferreira 1995) and pronouncing dictionaries (Duden
Aussprachewörterbuch, Warnant, DiPI and DOP, Porto Editora); each file names its sources and
notation. The split was fixed when the files were written, before any improved-mode rule was
changed on them; the test files were scored once. Check `ipa`: the segments must equal an expected
form after notation-only normalisation (stress, tie bars and non-syllabic marks removed, a long
consonant counted as a double one, vowel length only in German), and the main stress must be on the
same vowel. `python tools/eval/gold_eval.py --improved dev|test` (results
`tools/eval/results/gold_improved_{dev,test}.json`).

Changes made on the development words, all following the sources above: Spanish final *d* as ð,
ɱ before f, s voiced before a voiced consonant, unstressed u before a vowel as w; German r as ʁ and
as ɐ after a long vowel, ɔʏ, *-dchen* with t, vowel length only under stress; French gn as ɲ (a new
`[spelling.improved]` section: the phonemes alone cannot tell *gagner* from *panier*), *jouent*
ʒu; Italian ɪ ʊ as j w before a vowel and i u elsewhere; European Portuguese final u, ɐj for *ei*,
ɾ in clusters, nasal vowels at the end of a word.

| language | reference (dev) | improved before (dev) | improved now (dev) | reference (test) | improved (test) |
|---|---|---|---|---|---|
| Spanish | 72 % | 84 % | **100 %** | 84 % | **96 %** |
| German | 36 % | 60 % | **96 %** | 44 % | **96 %** |
| French | 64 % | 80 % | **92 %** | 68 % | **88 %** |
| Italian | 84 % | 84 % | **92 %** | 88 % | **92 %** |
| Portuguese (Portugal) | 52 % | 52 % | **92 %** | 36 % | **72 %** |
| all | 61.6 % | 72.0 % | **94.4 %** | 64.0 % | **88.8 %** |

What is still wrong on the test words: lexical vowel quality (French *soleil* sɔlɛj, Portuguese
*irmão*, *barco* read with an extra ə), French ɥ before a nasal vowel and before e (*juin*, *tuer*),
Italian *occhio* with i for j, Portuguese *quente* and *cinco* (nasal vowel before a stop), *azul*
with a final w, German *Garage* with a short a.

### Bhojpuri and Maithili (improved mode against the Hindi-voice fallback)

```sh
python tools/eval/build_bho_gold.py bho        # also: mai
python tools/eval/bho_eval.py --lang bho --split heldout
```

espeak-ng has no Bhojpuri or Maithili voice; the fallback is its Hindi voice, which uccharan's
compatible mode reproduces. Each gold set was written before any work on the language and split by
a hash of the item. Bhojpuri: 437 words in 12 categories and 153 short sentences, written by hand
from Shukla (1981) and Tiwari (1960); each word records where it is attested (FLORES-200 bho_Deva,
Grierson's *Linguistic Survey of India*, or the language description's examples). Maithili: 410 words (134 with
Wiktionary's own pronunciations via WikiPron, every variant accepted; the rest hand-written from
Yadav 1996 and Jha 1958) and 151 sentences. The comparison is broad: stress removed, ʌ ɐ = ə,
ɪ = i, ʊ = u, the reference's r. = ɽ, a geminate written with ː = a double consonant, aspirated
sonorants = consonant + h.

| Held-out | Hindi voice (espeak-ng `hi`) | compatible | improved |
| --- | ---: | ---: | ---: |
| Bhojpuri words (224) | 62.9 % | 63.8 % | **96.4 %** |
| — ऐ औ words / avagraha / flaps / copula | 0 / 0 / 50 / 78 % | 0 / 17 / 50 / 78 % | **86 / 100 / 95 / 100 %** |
| Bhojpuri sentences (82): PER / identical | 5.54 % / 28.0 % | 5.54 % / 28.0 % | **0.43 % / 91.5 %** |
| Maithili words (226) | 68.6 % | 66.4 % | **93.4 %** |
| — Wiktionary words / final i u / ऐ औ | 62 / 72 / 0 % | 56 / 72 / 0 % | **82 / 100 / 100 %** |
| Maithili sentences (71): PER / identical | 5.66 % / 32.4 % | 5.60 % / 32.4 % | **0.29 % / 94.4 %** |

The Bhojpuri held-out half was scored twice: first after the Bhojpuri work (words 96.0 %, sentence
PER 0.65 %, identical 89.0 %), then after the Maithili work, which added Maithili past endings to
the shared -ल- rule and the geminate notation to the comparison; the table gives the second
scoring. Development halves: Bhojpuri words 98.6 % (fallback 64.3 %), sentence PER 0.56 %
(4.50 %); Maithili words 92.9 % (67.4 %), sentence PER 0.39 % (6.15 %). Hiatus versus diphthong
(गइल) is not visible in this broad comparison, only in stress. Speech recognisers checked (model
cards, 2026-11-14): MMS-1B-all has Maithili and Magahi adapters but no Bhojpuri (CC BY-NC 4.0);
AI4Bharat IndicConformer has a Maithili model (MIT) but no Bhojpuri; for Bhojpuri there are
community models (RESPIN ESPnet Bhojpuri, CC BY 4.0; a Whisper-small Bhojpuri ONNX model, CC BY
4.0; Vakyansh wav2vec2, no licence stated). No Piper voice takes Bhojpuri or Maithili phonemes, so
no end-to-end test was run.

**Magahi** (`python tools/eval/bho_eval.py --lang mag --split heldout`). Gold set written before
any Magahi-specific rule and split by a hash of the item: 512 words (300 Wiktionary entries via
kaikki.org, with the pronunciation converted from Wiktionary's romanisation and checked by hand, 45
of them kept in Kaithi script; 212 words attested in FLORES-200 mag_Deva or given in the grammars of
Verma 2003, Aryani 1965 and Grierson's survey, hand-written) and 152 sentences (128 written for the
set, 24 from FLORES-200). Each item records its source, licence and date. Same broad comparison,
with ʋ = w.

| Held-out | Hindi voice (espeak-ng `hi`) | compatible | improved |
| --- | ---: | ---: | ---: |
| Magahi words (270) | 64.4 % | 70.0 % | **98.5 %** |
| — copula / ऐ औ and vowel + इ उ / nasals / loans / Kaithi | 25 / 20 / 42 / 53 / 0 % | 25 / 20 / 42 / 53 / 68 % | **100 / 100 / 92 / 100 / 100 %** |
| Magahi sentences (73): PER / identical | 7.67 % / 16.4 % | 7.58 % / 17.8 % | **0.71 % / 87.7 %** |

No category is below the Hindi voice. Development half: words 98.3 % (fallback 62.8 %), sentence
PER 0.54 % (7.89 %). Before the held-out scoring, ten gold items were corrected where the written
pronunciation contradicted the spelling (थ in हथ, करथ written t; ड in कुंड written d). MMS-1B-all
has a Magahi adapter (CC BY-NC 4.0), but no Piper voice takes Magahi phonemes, so no end-to-end test
was run.

## 5. Speed, start-up, memory and size

```sh
cargo build --release -p uccharan-capi
python tools/bench/compare.py --rounds 11 --langs en-us,en-gb,hi --out tools/bench/results/compare.json
node tools/bench/wasm_bench.mjs --limit 3000 --out tools/bench/results/wasm.json
```

**Method.** Both libraries are loaded as shared libraries through ctypes and called once per
sentence on the 3 000 held-out sentences, producing the same thing (an IPA string per sentence,
reference-style numbers, no punctuation). The ctypes call overhead (a few µs) is included on both
sides. Each round runs in a **fresh process**: load, initialise, warm up on 1 000 development
sentences (other text), then time every held-out sentence once. This matters for uccharan, which
keeps a small per-thread cache of recent words: timing the same sentences again and again in one
process would flatter it (0.1.0 used best-of-7 in one process; numbers are not comparable). The
two libraries take turns round by round, and the best of 11 rounds is kept for each. Throughput
is input words per second. Start-up is measured in a fresh process: loading the library,
initialising it and phonemizing one sentence. Memory is resident-set growth in a fresh process.

**The machine.** 4 vCPUs shared with other people's jobs. A control measurement (N busy Python
processes, total work relative to one) is taken at the start of each run: 1.0 / 1.6 / 1.39 for
1 / 2 / 4 processes in the run below (busy), 1.0 / 1.91 / 4.17 in the run used for the worker
rows (idle at the start). The alternation keeps load changes from favouring either side; absolute
times move by about ±10 % between runs (en-US p50 ratios of 4.65×, 5.22× and 5.28× were seen on
0.2.0 builds).

### Library, native (x86-64)

[`compare.json`](../tools/bench/results/compare.json); worker rows from
[`compare_workers.json`](../tools/bench/results/compare_workers.json), same build:

| | en-US uccharan | en-US reference | en-GB uccharan | en-GB reference | Hindi uccharan | Hindi reference |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Latency per sentence, p50 | **11.8 µs** | 62.1 µs | **11.0 µs** | 57.2 µs | **5.5 µs** | 15.7 µs |
| Latency per sentence, p99 | **48 µs** | 238 µs | **47 µs** | 216 µs | **21 µs** | 49 µs |
| Words/s, one thread | **580 k** | 114 k | **610 k** | 125 k | **965 k** | 347 k |
| Words/s, 2 workers | **1 037 k** | 203 k | **984 k** | 223 k | **1 818 k** | 595 k |
| Words/s, 4 workers | **1 639 k** | 419 k | **1 783 k** | 406 k | **2 816 k** | 768 k |
| Init + first sentence | **0.65 ms** | 4.0 ms | **0.69 ms** | 3.9 ms | **0.65 ms** | 3.8 ms |
| RSS after init | **1.9 MB** | 2.9 MB | **1.9 MB** | 2.9 MB | **1.7 MB** | 2.8 MB |
| RSS after 3 000 sentences | **3.0 MB** | 3.6 MB | **2.9 MB** | 3.6 MB | **2.7 MB** | 4.0 MB |
| Speed-up p50 / one thread / start-up | **5.3× / 5.1× / 6.2×** | | **5.2× / 4.9× / 5.6×** | | **2.8× / 2.8× / 5.8×** | |

Workers: uccharan runs one batch call (`uccharan_phonemize_lines`, one pass over the 3 000
sentences) on N native threads sharing one handle; the reference library keeps a single global
voice and cannot be called from several threads, so it runs as N separate processes, the most
favourable way to parallelise it. On 4 workers uccharan reaches 2.4× its one-thread rate (en-US)
and the reference 3.9×; uccharan's batch of 3 000 sentences is short (about 15 ms), so thread
start-up weighs more. It is still 3.9× faster in absolute terms.

The cache of recent words is part of these numbers (the warm-up text fills it with frequent
words, as a running application would); its first sentences in a fresh process are slower.

Kokoro English against misaki (`tools/bench/kokoro_speed.py`) is in section 6.

### Size

```sh
cargo build --release -p uccharan-capi --no-default-features --target-dir target/nodata
python tools/bench/sizes.py --code target/nodata/release/libuccharan.so \
    --espeak-lib .../libespeak-ng.so.1.52.0 --espeak-data .../espeak-ng-data --out tools/bench/results/sizes.json
```

What a language needs, counted the same way on both sides: for uccharan its pack (`.ucd`, self-contained);
for espeak-ng the language's dictionary plus the phoneme files every voice loads at start-up
(`phontab`, `phonindex`, `phondata`, `intonations` — espeak-ng does not start without them, even when
it only produces phonemes). Code is each shared library without data (uccharan built with
`--no-default-features`, x86-64 Linux, release profile). Compressors: gzip -9, xz -9e, zstd -19.
Bytes ([`sizes.json`](../tools/bench/results/sizes.json)):

| | Raw | gzip | xz | zstd |
| --- | ---: | ---: | ---: | ---: |
| **en-US** — uccharan `en-us.ucd` | **727 151** | 650 957 | 604 156 | 624 621 |
| en-US — espeak-ng `en_dict` + phoneme files | 827 224 | **441 434** | **379 744** | **413 762** |
| ↳ of which `en_dict` alone | 168 204 | 106 731 | 95 580 | 100 801 |
| **en-GB** — uccharan `en-gb.ucd` | **707 225** | 634 131 | 587 384 | 609 362 |
| **Hindi** — uccharan `hi.ucd` | **2 964** | **2 515** | **2 404** | **2 446** |
| Hindi — espeak-ng `hi_dict` + phoneme files | 751 163 | 361 901 | 306 464 | 336 870 |
| Code — uccharan library, no data | 817 296 | 387 196 | 313 760 | 337 458 |
| Code — espeak-ng library | **661 568** | **280 309** | **230 116** | **250 218** |
| Kokoro English — uccharan `kokoro-en-us.ucd` / `kokoro-en-gb.ucd` | 1 914 413 / 2 066 805 | 1 578 859 / 1 693 505 | 1 443 800 / 1 544 704 | 1 483 646 / 1 586 319 |

What this shows, plainly:

- **Hindi: uccharan is far smaller** in every column (its rules are code; the pack is a 3 KB
  exception list).
- **English, raw: uccharan's pack is smaller** (727 KB against 827 KB) — **compressed it is
  larger** (604 KB against 380 KB with xz). uccharan's pack is already entropy-coded, so general
  compressors gain little on it, while espeak-ng's files compress well. The English lexicon holds
  51 k exceptions to the letter-to-sound tables; espeak-ng's English dictionary is a smaller
  exception list plus spelling rules.
- **Code: espeak-ng's library is smaller** (662 KB against 817 KB raw; about 200 KB of uccharan's
  is the Rust standard library's panic, formatting and backtrace support). Code plus en-US data: uccharan
  1.54 MB raw against espeak-ng 1.49 MB — about equal; compressed, espeak-ng is smaller.
- A smaller English pack that keeps only the 24 k words of misaki's dictionaries was tried: 378 KB
  raw, 301 KB with xz (smaller than espeak-ng's files compressed), but held-out agreement fell from
  1.19 % to 1.44 % PER and held-out names from 88 % to 68 %, so it is not shipped.

Compared with 0.1.0 the English packs are 2.8× smaller (2.0 MB → 0.73 MB) and the Kokoro English
data a third smaller (2.9 / 3.1 MB → 1.9 / 2.1 MB), with every word coming out as before the compression.

| Package | Size |
| --- | ---: |
| Python wheel (abi3, x86-64 Linux; en-US and Hindi inside, other packs fetched on first use) | 1.1 MB (0.1.0: 5.5 MB) |
| Native library with every pack built in (`libuccharan.so`, default features) | 6.2 MB (0.1.0: 11.2 MB) |
| WebAssembly core | 438 KiB (165 KiB gzipped), no data; packs load on demand |

### WebAssembly (Node 22, same sentences, one call each)

Warm-up on development sentences, then each held-out sentence timed once
([`wasm.json`](../tools/bench/results/wasm.json)):

| | en-US | en-GB | Hindi |
| --- | ---: | ---: | ---: |
| Latency p50 / p99 | 14.6 µs / 79 µs | 20.7 µs / 109 µs | 9.1 µs / 36 µs |
| Words/s | 413 k | 311 k | 565 k |
| Load pack + first sentence | 14 ms | 6 ms | 12 ms |
| Module initialisation | 56 ms (once) | | |

These were measured while the machine was shared with other jobs. A side-by-side run against the
espeak-ng WebAssembly build published in the npm package `phonemizer` was **not done**.

### Targets from the design notes

| Target | Result | Met? |
| --- | --- | --- |
| Lexicon path > 100 k words/s | 0.58 M words/s through ctypes | yes |
| WebAssembly within 2–3× of native | 14.6 µs vs 11.8 µs p50 for en-US (1.2×) | yes |
| Per-language data < 5 MB | en 0.73 MB (+ 1.9 MB for Kokoro English), hi 3 KB | yes |
| WebAssembly core < 2 MB | 439 KiB | yes |
| en-US sentence PER ≤ 5 % vs reference | 1.16 % | yes |
| Hindi PER ≤ 5 % vs reference | 0.09 % | yes |
| End-to-end WER within +0.5 (en) / +1.0 (hi) on two Piper voices per language | en-US: amy-low +0.02 (0.2.0; earlier 0.2.0 builds +0.21 and +0.30), lessac −0.09 (0.1.0); en-GB: one voice, +0.11 (interval up to +0.77); hi: one voice, −0.12 | en-US yes; en-GB and Hindi on one voice only |
| Zero out-of-table symbols | none, except U+0329 on 130-symbol Piper maps (shared with the reference) | yes, with that note |
| Faster than the reference on every axis | latency, throughput (one and several workers), start-up, memory: yes. Size: Hindi yes; English pack smaller raw but **larger compressed**, code larger | all speed axes yes; size only partly |

## 6. Kokoro

### English: agreement with misaki

```sh
python tools/eval/kokoro_eval.py --set heldout --out tools/eval/results   # with misaki installed
```

Reference: misaki 0.9.4 as Kokoro's own pipeline runs it (spaCy `en_core_web_sm` tagger, espeak-ng
fallback for words outside its dictionaries). uccharan: the Kokoro English front end with its built-in copy of
misaki 0.9.4's dictionaries (the same output, byte for byte, as when it reads misaki's JSON files:
checked again for 0.2.0 on the 4 000 held-out and prose sentences for both accents). Run again for
0.2.0. "Token error" = edit distance over Kokoro tokens / reference length.

| Accent | Set | Identical sentences | Token error | Words in misaki's dictionaries | of which POS-dependent | Words misaki sends to espeak-ng |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| en-US | Tatoeba held-out (3 000) | **94.9 %** | 0.25 % | **99.96 %** of 20 251 | 98.5 % of 3 843 | 75.0 % of 372 |
| en-US | Sherlock Holmes (1 000) | 87.2 % | 0.29 % | 99.83 % of 14 502 | 98.8 % of 3 245 | 43.8 % of 130 |
| en-GB | Tatoeba held-out | **92.3 %** | 0.36 % | **99.96 %** of 20 240 | 98.4 % of 3 843 | 56.7 % of 383 |
| en-GB | Sherlock Holmes | 88.6 % | 0.28 % | 99.83 % of 14 504 | 98.8 % of 3 245 | 55.5 % of 128 |

"POS-dependent" words are the roughly 790 entries whose pronunciation misaki chooses by part of
speech (*that*, *read*, *live*, *use*, *close* …). The rule-based tagger that replaces spaCy picks
the same entry 98.5 % of the time; the remaining 1.5 % are the only place where removing spaCy
changes the result for dictionary words. The most frequent remaining differences are the
stressed/unstressed *that* (demonstrative versus conjunction or relative), *in* at the end of a
clause, and *lives*/*live*.

Words outside misaki's dictionaries are where the two fallbacks differ (espeak-ng's spelling rules
against uccharan's CMUdict-based lexicon and rules), mostly foreign names.

### English: speed against misaki

```sh
python tools/bench/kokoro_speed.py --out tools/bench/results/kokoro_speed.json
```

Same 3 000 sentences, one call per sentence, same Python process (uccharan through its C library
with ctypes) after a warm-up on 300 development sentences; misaki without its espeak-ng fallback
(which would only slow it down):

| | misaki 0.9.4 (spaCy sm) | uccharan | |
| --- | ---: | ---: | ---: |
| Latency per sentence, p50 / p99 | 1 882 µs / 3 985 µs | **20.9 µs / 102 µs** | 90× |
| Words per second, one thread | 4.0 k | **289 k** | 73× |
| Start-up incl. dictionaries and first sentence | 1 338 ms | **0.8 ms** | |
| Memory after start-up | 194 MB | **2.3 MB** | |

uccharan's dictionaries are compiled into the library and used in place, so start-up costs almost
nothing; reading misaki's JSON instead (`load_misaki`) takes about 0.2 s and 50 MB.

### End-to-end through a Kokoro voice

```sh
python tools/eval/kokoro_e2e.py --model kokoro-v1.0-int8 --lang en-us --reference misaki-output.jsonl \
    --asr whisper-onnx:/models/whisper-base/base --sid 3 --n 300 --out tools/eval/results
```

300 held-out sentences per language (without digits) through the int8 Kokoro v1.0 multi-language
model in sherpa-onnx 2.0.1 (`phoneme_codepoints`), once from the reference front end's tokens and
once from uccharan's, transcribed by the same recogniser. Speaker ids are sherpa-onnx's indices into
the model's `voices.bin`.

Run for 0.1.0 and not repeated for 0.2.0 (the Kokoro English agreement above is unchanged).

| Language | Speaker id | Reference front end | Identical tokens | WER reference | WER uccharan | Difference [95 % CI] | Better / worse |
| --- | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| en-US | 3 | misaki 0.9.4 | 287 / 300 | 2.95 % | **2.91 %** | −0.04 [−0.16, +0.08] | 2 / 1 |
| en-GB | 21 | misaki 0.9.4 | 277 / 300 | **2.08 %** | 2.12 % | +0.04 [−0.08, +0.16] | 1 / 2 |
| Hindi | 32 | espeak-ng 1.52 `hi` + published mapping | 224 / 300 | 10.35 % | **10.25 %** | −0.10 [−0.55, +0.36] | 8 / 5 |

Hindi tokens differ more often than the agreement figures suggest because the reference side here
was rebuilt from espeak-ng's unpunctuated output (plus a final full stop), while uccharan keeps
clause punctuation: ignoring punctuation, 298 of the 300 token strings are identical. The uccharan English side was run
with misaki's JSON dictionaries loaded; the built-in compiled copy gives byte-identical tokens on
all 6 000 English evaluation sentences.

## 7. Sound-alike matching

```sh
python tools/eval/soundalike/build_items.py                    # sentences from the term sets
python tools/eval/soundalike/make_confusions.py --models /models --out raw.jsonl   # voices and recognisers
python tools/eval/soundalike/make_confusions.py --merge raw.jsonl --out tools/eval/soundalike/data/confusions.jsonl
python tools/eval/soundalike/eval.py --split dev               # thresholds are chosen here
python tools/eval/soundalike/eval.py --split heldout --systems uccharan,uccharan-plain,uccharan-words,chars,metaphone \
    --thresholds uccharan=0.80,uccharan-plain=0.80,uccharan-words=0.80,chars=0.84,metaphone=1.0 \
    --out tools/eval/results/soundalike_heldout.json
python tools/eval/soundalike/eval.py --speed --out tools/eval/results/soundalike_speed.json
```

Run on 2026-11-14. **What is measured.** [Sound-alike matching](soundalike.md) is given the whole
vocabulary and a transcript and corrects what it finds. Three things can happen: a term the
recogniser got wrong is put back (*recovery*), another span or another term is corrected (*wrong
correction*), or a sentence that has no term in it is changed (*false correction*).

**Terms** (hand-written, `tools/eval/soundalike/terms/`): 310 Indian personal names (given names
and surnames from across the country, each with its Devanagari form), 173 brand and product names
(Indian and international), 168 technical and domain terms (software, Indian finance and
administration, medicines, Hindi words used in English). 97 of the brands and terms carry a spoken
form (`pay T M`, `engine X`, `cube control`), as a word list would. Every term belongs to the
development or the held-out half by a hash of its spelling; all 651 are in the vocabulary for every
transcript of either half, as they would be in use.

**Transcripts** (`data/confusions.jsonl`): each term is put into an English sentence, a Hindi
sentence in Devanagari (a name in Devanagari; brands and jargon in Latin script, as they are
written in India) and a romanised Hindi sentence, from six to eight hand-written templates per
kind. The 1 953 sentences are spoken by synthetic voices through sherpa-onnx 2.0.1 (Piper
`en_US-amy-low` and `hi_IN-rohan-medium`; Kokoro v1.0 speakers 3, 11, 24 for English and 31, 33
for Hindi; two voices for a name in an English sentence, one otherwise) and transcribed by small
recognisers (Whisper tiny and base and Moonshine tiny for English; Whisper base told the language
is Hindi; for the romanised sentences also Whisper base told it is English): 2 263 recordings,
4 836 transcripts. Whisper base writes Hindi speech in Devanagari, in Arabic script or in Latin
letters as it pleases, and translates when told the speech is English; all of it is kept, because
a matcher behind a recogniser is handed exactly that. A further 259 mishearings were written by
hand (`terms/mishearings.tsv`: *Coat Harry*, *cube cuddle*, *ज़ोमैटो*). Transcripts that are equal
for the same term count once. The names are also scored the other way round: the vocabulary holds
their Devanagari forms and the transcripts are the Latin-script ones.

**Sentences without a term**: per half, 500 Tatoeba sentences (the held-out half has 100 Sherlock
Holmes sentences among its 300 English ones) and 30 hand-written romanised Hindi ones, as written,
plus what the recognisers made of half of them (265 and 270 recordings).

**Systems.** *uccharan*: the matcher at its defaults, with the term sets as a word list (spoken
forms included). *Plain list*: the same without the spoken forms. *Character edit distance*: one
minus the Levenshtein distance between the letters of a span and of a term (or its spoken form),
over the longer of the two. *Metaphone*: the same over Metaphone codes (the `jellyfish`
implementation). The baselines search the same spans of one to three words and settle overlaps the
same way; they have no notion of ordinary words and cannot cross scripts. Each baseline runs at the
lowest threshold that changes at most 1 % of the development sentences without a term (Metaphone
never gets there: 40 % at an exact match of codes). For uccharan that rule gave 0.65; its default
was set to 0.80, because below it wrong corrections on the development half rise quickly (26 at
0.80, 57 at 0.70, 119 at 0.65). The other two thresholds (0.90 for uncertain spans, 0.92 for
ordinary words), the costs and the pre-filter were also settled on the development half; the
held-out half was scored once, after that.

### Held-out results

| System | Threshold | Terms put back | Wrong corrections | Sentences without a term that were changed | Precision |
| --- | ---: | ---: | ---: | ---: | ---: |
| **uccharan**, word list with spoken forms | 0.80 | 43.9 % (1118 / 2546) | 23 | 0.3 % (2 / 775) | 97.8 % |
| uccharan, plain list of terms | 0.80 | 43.4 % (1105 / 2546) | 24 | 0.3 % (2 / 775) | 97.7 % |
| character edit distance | 0.84 | 8.8 % (224 / 2546) | 5 | 0.9 % (7 / 775) | 94.5 % |
| Metaphone codes | 1.00 | 18.7 % (477 / 2546) | 1455 | 38.2 % (296 / 775) | 19.9 % |

Of the sentences without a term, uccharan changed none of the 397 English ones and 2 of the 378
Hindi and romanised Hindi ones (*गाना* "song" became the brand "Gaana"; a recogniser's *سنائد* became
"Sunita"); on the development half 6 of 725. *Wrong corrections* count every correction to
something other than the sentence's term, including a term that sounds the same as the right one
(`Sentho` to "Sandhu" where "Senthil" was said).

Passing the words that occur twice or more in the development sentences as `ordinary_words`
(`uccharan-words`) changed nothing on the held-out half: the same 2 sentences.

### By kind of transcript (uccharan, held-out)

| Transcripts (Latin-script vocabulary unless said) | Term not written right | Put back | Wrong corrections |
| --- | ---: | ---: | ---: |
| English sentences | 857 | 64.2 % (550) | 11 |
| Hindi sentences written in Devanagari by the recogniser | 30 | 50.0 % (15) | 0 |
| Hindi sentences written in Arabic script by the recogniser | 431 | 19.5 % (84) | 0 |
| Hindi and Hinglish sentences written in Latin letters (or translated) | 375 | 25.1 % (94) | 7 |
| Latin-script transcripts against the names in Devanagari | 853 | 44.0 % (375) | 5 |
| *of the Latin-vocabulary rows:* | | | |
| Personal names | 904 | 46.8 % (423) | 14 |
| Brands and products | 385 | 36.6 % (141) | 1 |
| Technical and domain terms | 404 | 44.3 % (179) | 3 |
| Terms with a spoken form | 273 | 50.9 % (139) | 3 |
| Terms without one | 1420 | 42.5 % (604) | 15 |
| Hand-written mishearings | 132 | 76.5 % (101) | 0 |
| Recogniser transcripts | 2414 | 42.1 % (1017) | 23 |

- Spoken forms matter where the spelling hides the sound: for the terms that have one, recovery is
  46.2 % from the plain list and 50.9 % with them (development half: 38.0 % and 52.0 %).
- "Term not written right" includes transcripts no matcher could repair: Whisper told that
  romanised Hindi is English answers with a translation (*"We will come to the wedding in the
  meeting tomorrow"* for a sentence about Vinod), and the Arabic-script transcripts drop most short
  vowels. That, not the cross-script comparison, is why those rows are low: Devanagari transcripts
  recover half of their terms, and the Devanagari vocabulary 44 % from Latin transcripts.
- The pre-filter costs almost nothing: on the development half, comparing every span with every
  term recovers 1 281 terms against 1 278 (and makes 32 wrong corrections against 26).
- Without the romanised-Indic reading of Latin-script words, recovery on the development half
  falls from 43.6 % to 36.6 %.

### Precision and recall across thresholds (held-out)

`threshold` varies; uncertain spans still need 0.90 and ordinary words 0.92.

| `threshold` | 0.70 | 0.75 | 0.80 | 0.85 | 0.90 | 0.95 | 1.00 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Terms put back (recall) | 50.1 % | 47.6 % | 43.9 % | 39.2 % | 30.6 % | 13.8 % | 5.2 % |
| Precision | 94.2 % | 95.7 % | 97.8 % | 98.7 % | 99.4 % | 100.0 % | 100.0 % |
| Sentences without a term that were changed | 0.9 % | 0.7 % | 0.3 % | 0.3 % | 0.3 % | 0.0 % | 0.0 % |
| Wrong corrections | 71 | 49 | 23 | 11 | 3 | 0 | 0 |

The baselines at the same thresholds (terms put back / sentences without a term that were changed):

| | 0.70 | 0.75 | 0.80 | 0.85 | 0.90 | 0.95 | 1.00 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| character edit distance | 20.8 % / 22.8 % | 17.8 % / 19.2 % | 15.0 % / 6.5 % | 8.7 % / 0.9 % | 4.6 % / 0.4 % | 3.7 % / 0.4 % | 3.7 % / 0.4 % |
| Metaphone codes | 27.3 % / 59.7 % | 27.1 % / 59.7 % | 23.5 % / 43.6 % | 19.6 % / 38.2 % | 18.7 % / 38.2 % | 18.7 % / 38.2 % | 18.7 % / 38.2 % |

### Speed

`uccharan -l en-us --match-terms FILE --match-stats` on 3 000 held-out transcripts (7.8 words
each), one thread, best of three runs on the development machine while it was busy with other work.
The vocabularies are the 651 terms filled up with pairs of the names and with English words.

| Terms | Stored sequences | Building the index | Transcripts per second | Words per second |
| ---: | ---: | ---: | ---: | ---: |
| 1 000 | 3 289 | 0.04 s | 7 844 | 61 036 |
| 10 000 | 33 409 | 0.30 s | 1 468 | 11 425 |
| 50 000 | 182 094 | 2.60 s | 315 | 2 450 |

Every word starts up to three spans, each looked up in up to three readings, so the last row is
about 7 000 spans a second. Above 10 000 terms the time goes into reading the pre-filter's
lists, which grow with the vocabulary.

With these filler vocabularies the matcher also shows its limit: 1.4 % of the development
sentences without a term change against the 1 000 terms, and 11.6 % against the 10 000, because
rare English words (*alighted*, *likelier*) sound like common ones (*lighted*, *likely*). A large
vocabulary of ordinary-looking words needs recogniser confidences or higher thresholds.

### What this does not show

- The speech is synthetic and the voices read uccharan's own phonemes, so a term is pronounced the
  way one of its stored readings says. Real speakers, accents and larger recognisers make other
  errors, and a term uccharan reads wrongly would be missed where here it is found.
- The recognisers give no word confidences in this set-up, so the confidence gate is covered by
  unit tests only.
- Hindi precision rests on a hand-written list of about 950 frequent words; the 2 false
  corrections above are words outside it.

## 8. Romanised text into native script

```sh
XLIT_DATA=DIR python3 tools/eval/translit/prep.py hi                 # word pairs and the fixed split (per language)
XLIT_DATA=DIR python3 tools/eval/translit/corpus.py hi SENTENCES...  # word and word-pair counts
cargo run --release -p uccharan-lexgen -- xlit hi --data DIR         # data/packs/xlit-hi.ucd
python3 tools/eval/translit/score.py hinglish tools/eval/gold/hinglish/dev.tsv        # development
python3 tools/eval/translit/heldout.py --data DIR                    # tools/eval/results/translit_heldout.json
```

[`transliterate`](transliteration.md) writes romanised Hindi, Marathi, Bengali, Tamil, Telugu,
Gujarati, Punjabi, Kannada or Malayalam in its script and leaves English words as English. The
claims measured here: the first candidate of a single word is right about as often as a neural
transliterator's; in chat-style Hindi mixed with English the Hindi words are written correctly and
the English words are left alone; and it is small and fast enough to run in front of a voice.

### Data and split

- **Word pairs.** The Dakshina dataset's lexicons (Google Research, CC BY-SA 4.0) and Aksharantar
  (AI4Bharat). Development: Dakshina `dev`, Aksharantar `valid`. Held-out: Dakshina `test`, and the
  two hand-made parts of Aksharantar `test` that its authors report: native words (rows from
  `AK-Freq` / `AK-Uni`) and named entities (`AK-NEI` / `AK-NEF`). Training: Dakshina `train` and
  Aksharantar `train`, **minus every pair whose native word or whose romanised word occurs in a
  development or held-out file**, so each held-out input and reference is new to the model and
  absent from the spelling lexicon. One row per distinct romanised input; a candidate is right when
  it equals any reference of that input.
- **Dakshina romanised sentences.** The 5,000 held-out sentences per language (Wikipedia
  sentences romanised by annotators), every Latin word converted (`english = convert`: the
  references are fully native). Tokens are paired by position; the 10 to 265 sentences per language
  whose two sides differ in token count are skipped. Development: the first 1,500 `dev` sentences.
- **Hinglish.** `tools/eval/gold/hinglish/test.tsv`: 164 hand-written chat-style sentences, 868
  Hindi words and 130 English words, references in Devanagari with English left in Latin; among
  them a block in vowel-less chat spelling (`nhi`, `kr`, `h`). Development: `dev.tsv` (100
  sentences) and `dev-chat.tsv` (30 sentences in chat spelling). Loanwords typed in English
  spelling (`office`, `school`) count as English, by the convention of these references.
- **Strict and relaxed.** Strict is an exact match of the token. Relaxed (Devanagari) folds
  spelling variants of one word: chandrabindu = anusvara, nukta ignored, nasal + virama before a
  consonant = anusvara (हिन्दी = हिंदी), and the verb endings यी / ई and ये / ए after a written vowel
  (लिये = लिए; गयी / गई stay different). For the other scripts the two are practically the same.
- **Comparison.** IndicXlit (AI4Bharat's neural transliterator, 11 M parameters, one 127 MB
  download for 21 languages) was not run here; its numbers are the top-1 accuracies published in
  the README of https://github.com/AI4Bharat/IndicXlit (read on 2026-11-14) for the same test sets.
  Its evaluation scripts are its own, so small differences of protocol are possible. "No pack" is
  this library without a pack: the built-in Hindi list and rules, or the plain ITRANS-style
  converter.

### How the numbers were obtained, and which are clean

Weights and model sizes were chosen on the development sets. The held-out sets were then scored
once for every language. That run showed one failure in Hindi: the chat spellings of the
held-out sentences (`nhi`, `h`, `acha`) were being overruled by the model, because nothing in the
development data had exercised the hand-written chat list. `dev-chat.tsv` was written, the
failure reproduced and fixed on it, and the packs rebuilt. The packs that ship were then scored
again; they also differ from the first packs in the hand-written lists for the eight other
languages, the है / हैं rule (developed on the Dakshina development sentences) and the table
layout. So:

- the Hinglish number of the shipped Hindi pack, **98.2 %**, was obtained after a change made in
  response to held-out errors and is optimistic; the number of the one run made before that is
  **95.9 %**;
- every other number moved by about a point at most between the two runs, and both are in
  `tools/eval/results` (`translit_heldout_first.json`, `translit_heldout.json`).

### Hindi

Single words, held-out, top-1 % (top-3 in brackets):

| System | Dakshina test (4,442) | Aksharantar native words (3,643) | Aksharantar named entities (1,997) |
| --- | ---: | ---: | ---: |
| uccharan, `xlit-hi` (8.7 MB) | **68.69** (83.12) | 49.66 (69.42) | 57.04 (75.86) |
| its n-gram model alone (5.4 MB; first run) | 56.24 (78.01) | 47.32 (68.32) | 54.53 (74.06) |
| IndicXlit, published | 60.56 | 55.59 | 58.87 |
| uccharan without a pack (list, then rules) | 19.83 | 22.87 | 11.37 |
| plain ITRANS-style rules | 12.31 | 16.58 | 6.11 |

On Dakshina the first candidate is right 8.1 points more often than IndicXlit's published figure;
on Aksharantar's hand-made sets it is **behind**, by 5.9 points on native words and 1.8 on named
entities. The model by itself is behind on all three; the lead on Dakshina comes from re-ranking
with word frequency, and that test flatters a frequency prior (its words were sampled from
Wikipedia, and part of the frequency corpus is Wikipedia text). The Aksharantar words are rarer and
the frequency list knows fewer of them.

Sentences, held-out:

| System | Dakshina sentences (85,623 words): word accuracy, strict (relaxed) |
| --- | ---: |
| uccharan, `xlit-hi` | **86.47** (88.81) |
| each word alone (no context) | 85.33 (87.70) |
| first run | 86.22 (88.53) |
| without a pack (every word by list and rules) | 55.52 (57.78) |
| plain ITRANS-style rules | 35.64 (38.15) |

| System, Hinglish held-out (868 Hindi words, 130 English words) | Hindi words right, strict (relaxed) | English words converted by mistake | Hindi words left in Latin | Sentences fully right |
| --- | ---: | ---: | ---: | ---: |
| uccharan, `xlit-hi` as shipped (second scoring, see above) | 95.2 % (**98.2 %**) | 10 / 130 = 7.7 % | 3 / 868 = 0.3 % | 139 / 164 = 84.8 % |
| the same, each word alone | 94.6 % (97.5 %) | 10 / 130 = 7.7 % | 6 / 868 = 0.7 % | 135 / 164 = 82.3 % |
| first run (the clean number) | 93.1 % (**95.9 %**) | 10 / 130 = 7.7 % | 3 / 868 = 0.3 % | 124 / 164 = 75.6 % |
| without a pack (the `hinglish` list and rules) | 71.5 % (72.4 %) | 4 / 130 = 3.1 % | 174 / 868 = 20.0 % | 39 / 164 = 23.8 % |
| plain ITRANS-style rules, every word converted | 44.7 % (45.7 %) | 130 / 130 | 0 | 1 / 164 |

What is still wrong on the Hinglish set: the English words converted are homographs used as
English (`sir,`, `trust me`, `bus`, `charger use`, `me too`, `he is`) and loans the sound test
misses (`login`, `share`, `cricket`); the Hindi words are names (`delhi`, `priya`, `america`),
causatives the spelling does not distinguish (`nikalna` for निकलना / निकालना), `gaya` for गाया,
`ki` for कि, and गयी where the reference has गई. On the Dakshina sentences the commonest errors are
`hai` where the reference has हैं after a feminine form, `ki` for कि / की, and spelling variants.

### All nine languages

Single words, held-out, top-1 % (top-3 in brackets); IndicXlit = published top-1:

| Language | Pack | Dakshina test: top-1 (top-3) | IndicXlit | Aksharantar native words: top-1 (top-3) | IndicXlit | Aksharantar named entities: top-1 (top-3) | IndicXlit |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Hindi | 8.7 MB | 68.69 (83.12) | 60.56 | 49.66 (69.42) | 55.59 | 57.04 (75.86) | 58.87 |
| Marathi | 7.4 MB | 74.45 (82.87) | 64.85 | 57.08 (67.93) | 63.72 | 52.55 (69.67) | 53.57 |
| Bengali | 7.8 MB | 67.09 (78.69) | 55.49 | 60.65 (75.56) | 61.70 | 46.42 (64.28) | 37.12 |
| Tamil | 7.0 MB | 75.90 (82.50) | 68.10 | 72.65 (85.16) | 69.78 | 38.81 (58.29) | 42.12 |
| Telugu | 7.0 MB | 81.45 (89.17) | 73.38 | 83.88 (92.05) | 84.69 | 53.38 (71.54) | 51.82 |
| Gujarati | 6.0 MB | 65.62 (81.26) | 62.02 | 54.72 (73.87) | 61.89 | 49.67 (69.80) | 48.89 |
| Punjabi | 3.1 MB | 53.34 (71.89) | 47.24 | 34.43 (52.24) | 40.27 | 34.05 (52.28) | 36.08 |
| Kannada | 6.7 MB | 84.40 (91.16) | 77.18 | 75.68 (86.76) | 76.18 | 48.64 (71.69) | 49.92 |
| Malayalam | 6.4 MB | 71.66 (80.26) | 63.56 | 61.62 (74.43) | 64.73 | 30.90 (44.67) | 33.93 |

The first candidate beats IndicXlit's published figure on Dakshina in all nine languages (by 3.6
to 11.6 points), is behind it on Aksharantar's native words in eight (by 0.5 to 7.2) and ahead in
Tamil, and on named entities is ahead in three (Bengali, Telugu, Gujarati) and behind in six. The
nine packs together are 60 MB.

Dakshina romanised sentences, held-out, word accuracy % (strict; relaxed in brackets):

| Language | Dakshina test inputs | Aksharantar native / entities | Sentence words | Word accuracy, strict (relaxed) | Each word alone | No pack (rules) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Hindi | 4,442 | 3,643 / 1,997 | 85,623 | 86.47 (88.81) | 85.33 | 55.52 |
| Marathi | 5,667 | 4,459 / 1,998 | 46,789 | 85.48 (85.93) | 85.14 | 16.64 |
| Bengali | 9,146 | 2,267 / 1,999 | 57,471 | 80.82 (80.83) | 80.35 | 11.97 |
| Tamil | 6,833 | 2,607 / 1,997 | 46,460 | 76.68 (76.69) | 76.46 | 9.21 |
| Telugu | 5,726 | 2,240 / 1,999 | 43,544 | 82.91 (83.64) | 82.85 | 30.62 |
| Gujarati | 10,229 | 5,243 / 1,997 | 75,619 | 82.74 (82.75) | 82.64 | 48.28 |
| Punjabi | 6,880 | 2,184 / 1,997 | 84,376 | 79.19 (79.19) | 77.01 | 29.25 |
| Kannada | 5,013 | 4,207 / 1,992 | 53,919 | 83.02 (84.44) | 82.96 | 30.03 |
| Malayalam | 5,561 | 4,854 / 1,990 | 42,398 | 70.87 (71.90) | 70.27 | 5.45 |

Outside Hindi the English / native decision is **not measured**: there is no code-mixed test set
for the other eight languages, their homograph and chat lists were written by hand without review
by a native speaker, and they use the decision weights fitted on Hindi. Context adds little for
most of them because their word-pair tables come from small corpora (8 k to 67 k pairs against
165 k for Hindi).

### What the parts are worth (Hindi, development sets)

| Part | Effect |
| --- | --- |
| word frequency in the ranking | +6.5 points top-1 on the development words |
| phonetic agreement (the sound-alike distance between the romanised word and the candidate) | +0.5 to +0.7 points |
| word-pair table in the sentence pass | +1.4 points on Dakshina development sentences, +1.8 on the Hinglish development set |
| loan test and fitted decision weights | English words converted by mistake: 33 % → 8 % of the development set's |
| chat list taking effect (listed spellings the model cannot produce) | chat-spelling development set 72.3 → 98.1 % |
| है / हैं after a plural or singular verb form | +0.15 points on Dakshina development sentences |
| pruning the model from 15.9 MB to 5.3 MB of tables | −1.2 points top-1 |
| training without the Dakshina rows (a pack without share-alike word pairs) | −1.0 point on Dakshina development words, −0.5 on sentences |

### Size and speed

A pack is 3 to 9 MB; about two thirds of it is the n-gram tables. The code adds no dependency.
One thread, words per second, on the shared
development machine (load average 2 to 3 on 4 cores during the run, so these are lower bounds):

| Language | Running text, first pass | Same text again | New words only |
| --- | ---: | ---: | ---: |
| Hindi | 12,105 | 243,270 | 3,145 |
| Marathi | 7,700 | 308,278 | 4,138 |
| Bengali | 7,793 | 338,922 | 3,246 |
| Tamil | 5,580 | 755,409 | 4,634 |
| Telugu | 5,803 | 216,228 | 3,823 |
| Gujarati | 9,648 | 230,832 | 4,603 |
| Punjabi | 17,392 | 624,892 | 6,236 |
| Kannada | 5,999 | 303,021 | 4,224 |
| Malayalam | 3,482 | 203,986 | 2,895 |

"Running text, first pass" is the 5,000 Dakshina test sentences from a cold start, so it includes
the first encounter of every word; "same text again" is five more passes in the same process;
"new words" is the Dakshina test lexicon, where every word needs the beam search and the phonetic
comparison (about a third of a millisecond each). A chat message of ten words costs 2 to 3 ms the
first time its words are seen.

### What this does not show

- The Hinglish sets are small and were written by the author of the system, and one fix was made
  after seeing held-out errors (above).
- The split removes every training pair that collides with a held-out word, so common words such
  as `wahan` are missing from the measured packs' lexicon (it writes वाहन); packs trained on all
  pairs would know more spellings, but could not be measured this way.
- The frequency prior is partly Wikipedia text and the Dakshina words come from Wikipedia: the
  Dakshina word numbers are the favourable case, the Aksharantar native words the unfavourable
  one. Real chat text is closer to the Hinglish set.
- IndicXlit's figures are published ones, not reproduced here.
