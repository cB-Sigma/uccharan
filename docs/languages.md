# Languages

uccharan phonemises 43 language tags. English (US and British) and Hindi have hand-built front
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
| Dogri | `doi` | `hi` ‡ | IndicCorp v2, 2,000 sentences | 1.40 % | 1.35 % | 1.9× § | none |
| Bodo | `brx` | `hi` ‡ | IndicCorp v2, 2,000 sentences | 0.37 % | 0.32 % | 2.3× § | none |
| Santali (Ol Chiki) | `sat` | none ¶ | IndicCorp v2, 2,000 sentences | — ¶ | — | — ¶ | none |
| Manipuri (Meetei Mayek, Bengali script) | `mni` | none ◊ | IndicCorp v2, 2,000 sentences | — ◊ | — | — ◊ | none |
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

‡ espeak-ng has no voice for Sanskrit, Bhojpuri, Maithili, Magahi, Dogri or Bodo; text in them is read
with its Hindi voice, and compatible mode is measured against that reading.

§ Measured on a Windows 11 laptop (500 held-out sentences, p50, best of 5 rounds,
[`compare_doi_windows.json`](../tools/bench/results/compare_doi_windows.json)); in the same run
Magahi gave 2.3× and Hindi 1.8×, against 2.8× each on the machine of the other rows. Bodo:
[`compare_brx_windows.json`](../tools/bench/results/compare_brx_windows.json), same method; in that
run Dogri gave 1.9× and Hindi 1.8×.

¶ espeak-ng cannot read Ol Chiki (it says the code point of each letter, under every voice), so
Santali has no reference reading: both modes give uccharan's Santali reading, checked on a
hand-written gold set instead (held-out half: 100 % of 100 words, sentence PER 0.00 %; see the
notes below for what that does and does not show). Speed, same laptop and method as §: 16.7 µs per
sentence at the median, 0.72 million words per second, one thread
([`compare_sat_windows.json`](../tools/bench/results/compare_sat_windows.json)); in that run Bodo
took 20.4 µs (2.4× espeak-ng) and Hindi 10.8 µs (1.8×).

◊ Manipuri: espeak-ng has no Meitei voice and cannot read Meetei Mayek, and its Bengali voice reads
Bengali-script Meitei as Bengali, so both modes give uccharan's reading, checked on a gold set
(held-out half: 92.2 % of 193 Meetei Mayek words, 95.2 % of 83 Bengali-script words against 28.9 %
for the Bengali voice). Speed, same laptop and method as §: 14.2 µs per sentence at the median, 0.55
million words per second, one thread
([`compare_mni_windows.json`](../tools/bench/results/compare_mni_windows.json)); in that run Bodo
took 20.3 µs (2.4× espeak-ng) and Hindi 11.1 µs (1.8×).

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

**Dogri** (`doi`, Devanagari) has no espeak-ng voice (1.52.0 has none); as for
Sanskrit, compatible mode is the Hindi voice's reading (a word written with the
apostrophe of a lost h, कि’यां, is read as one word, as the voice reads it), and improved mode reads
Dogri from its own language file. Dogri is tonal, like Punjabi: historic voiced aspirates and h
are no longer said as such and leave a tone. uccharan writes the tone as its Punjabi does, a grave
accent for low tone and an acute for high tone on the vowel: a word-initial voiced aspirate is
voiceless and unaspirated with low tone on the next vowel (घर kˈʌ̀ɾ, भाशा pˈàːʃaː); an h before a
vowel is lost with low tone on it (ज्हार ɟˈàːɾ, म्हीना mˈìːnaː, हत्थ ˈʌ̀tʰː); a voiced aspirate
inside the word stays voiced but loses its aspiration, and it or a lost h after a vowel — written
ह् (साह्ब), with an apostrophe (कि’यां, र’लाया; some text has ऱ there) or with the avagraha (रपेऽ,
also typed as a Latin S) — gives high tone to a stressed vowel before it (साह्ब sˈáːb, त्रीह् tɾˈíː,
सिद्धा sˈɪ́dːaː), otherwise low tone to a stressed vowel after it (बधाई bədˈàːiː). This follows Ved
Kumari Ghai's description of Dogri tone (Ghai 1991, *Studies in Phonetics and Phonology, with
special reference to Dogri*; her account of the devoicing of initial voiced aspirates); the
Wikipedia and Wiktionary articles on Dogri, after U. S. Bahri (2001, *Dogri: Phonology and
Grammatical Sketch*), call the tone after a vowel low-rising rather than high, and that has not been
settled here. The rest of the inventory follows Bahri: ऐ औ are ɛː ɔː, ष is ʃ, क़ ख़ ग़ are k kʰ ɡ;
inherent-vowel deletion is the Hindi type, a doubled consonant is long. Numbers: 0–30, the tens,
सौ, ज्हार (1000) and the ordinals पैह्ला, दूआ, त्रीया, चौथा, -मां from Bahri (2002, in E. Chan's
numeral tables); the other numbers below 100 are formed like the Punjabi ones in Dogri spelling and
have not been checked by a Dogri speaker; रपेऽ, मिंट, प्रतिशत and the month names from Unicode CLDR.
Improved mode and normalise-only mode read 12मीं, 5मां, 1ला as ordinals (पैह्ला, बारांमीं),
10:30 as दस बजियै त्रीह् मिंट, ₹25 as पंजी रपेऽ. Romanised Dogri (`hinglish=True`, and
`transliterate(..., "doi")`, which has no pack) goes through the romanised-Hindi path after a short
list of common Dogri words (`tusen kiyan ho` → तुसें कि’यां हो).

Held-out text: 2,000 sentences of Dogri from AI4Bharat's IndicCorp v2 (CC0; FLORES-200 and
Tatoeba have no Dogri), lines in Hindi left out. Most of the remaining disagreement with the
Hindi voice is in what uccharan's Hindi reading does not reproduce of the voice: ऎ, a chandrabindu
on ई (नेईँ), a slash read as English *slash*, abbreviations such as डॉ., hyphenated compounds.
A hand-written gold set ([tools/eval/gold/doi](../tools/eval/gold/doi): 115 words, 35 sentences,
split by a hash before the rules were run on it) measures improved mode; held-out half:

| Held-out | Hindi-voice fallback (espeak-ng `hi`) | uccharan compatible | **uccharan improved** |
| --- | ---: | ---: | ---: |
| Dogri words (70), exact | 44.3 % | 44.3 % | **97.1 %** |
| Dogri sentences (15): PER / identical | 13.03 % / 6.7 % | 13.03 % / 6.7 % | **0.43 % / 86.7 %** |

The gold set encodes the descriptions named above as written down for this project, so it shows
that improved mode implements them, not that a Dogri speaker would accept every form.

**Bodo** (`brx`, also `bodo`, `boro`; Devanagari, the official script since 1975) is a
Tibeto-Burman language with no espeak-ng voice (1.52.0 has none); as for Sanskrit and Dogri,
compatible mode is the Hindi voice's reading (a word written with an apostrophe inside it, कर’ना,
is read as one word, as the voice reads it), and improved mode reads Bodo from its own language
file, `lang/brx.lang`. Its spelling differs from Hindi's in ways the Hindi reading cannot see:

- ओ ो (and ऒ ॊ, which the Bodo Sahitya Sabha prefers) write the vowel ɯ, not o: बोलो bˈɯlɯ, मोसा
  mˈɯsa; ै ौ are ɯi ɯu (दै dˈɯi, मोसौ mˈɯsɯu); vowel length is not distinctive (ई ऊ = इ उ);
- the inherent vowel is ɔ. A bare consonant at the end of a word or before another consonant has
  no vowel, as in Hindi (मोनथाय mˈɯntʰai, जोनोम zˈɯnɯm), except in the first syllable (गसंना
  ɡˈɔsɔŋna); where the vowel is said all the same, Bodo writes an apostrophe after the consonant
  (गोजौ कमा: बर’ bˈɔrɔ, दख’ना dˈɔkʰɔna, लाथिख’), which improved mode reads as ɔ in any of its forms
  (`'`, `’`, `ʼ`). A closing quotation mark right after a consonant is read the same way;
- ं is always ŋ (सुबुं sˈubuŋ, संख्रि sˈɔŋkʰri), and a bare consonant before it keeps its vowel;
  य and व after a vowel and before a consonant or the end of the word close a diphthong (थाखाय
  tʰˈakʰai, दाव dˈau, मायरं mˈairɔŋ);
- the consonants are those of Bodo: ख थ फ kʰ tʰ pʰ, ज z, स s, र r; the Indo-Aryan letters of
  loanwords are read with Bodo sounds (घ झ ढ ध भ as ɡ z d d b, च छ श ष as s, ट ठ ड ण as t tʰ d n, ड़ as
  r: भारत bˈarɔt).

Sources: the phoneme inventory of P. C. Bhattacharya (1977, *A Descriptive Analysis of the Boro
Language*) as given by Priyankoo Sarmah (2004, *Some Aspects of the Tonal Phonology of Bodo*, M.Phil.
thesis, CIEFL Hyderabad) and the Wikipedia article "Boro language (India)"; the Government of India's
Unicode document L2/08-197 (2008, *Representation of Bodo, Dogri and Maithili languages in Unicode
Standard*) for ो = ɯ and the apostrophe; and the first article of the Universal Declaration of
Human Rights in Bodo Devanagari and in the older Latin orthography (quoted in the Wikipedia article)
for ै, ं and the bare consonants.

**Tone.** Bodo has two lexical tones, high and low (Burling 1959; Joseph and Burling 2001; Sarmah
2004, who shows from recordings that the third and fourth tones of Bhattacharya 1977 are not
distinct, and that a word carries one tone, on its last syllable). Bodo Devanagari does not write
tone, so improved mode does not say it: its output has no tone marks, and minimal pairs such as
Wikipedia's *ja* "eat" (high) and "be" (low) come out the same. L2/08-197 and Wiktionary call the
apostrophe a tone mark of the short o; in running text it stands after the first and middle
syllables too (क’कराझार, बिसायख’थि), which in Sarmah's account carry no lexical tone, so uccharan
reads it as the vowel it shows and not as a tone. A Bodo tone dictionary would be needed to say
tone; none with an open licence was found.

**Numbers.** Improved mode and normalise-only mode read numbers in Bodo words: the native numerals
for the units, tens, hundreds and thousands (से नै थाम ब्रै बा द’ स्नि दाइन गु, जि 10, जिसे 11,
नैजि 20, नैजिबा 25, जौसे 100, नै जौ 200, रोजासे 1000, नै रोजा 2000), and the Indo-Aryan लाख and कौटि
for 10^5 and 10^7 (से लाख, बा कौटि). This follows the numerals of the Wikipedia article and how Bodo
text writes them (IndicCorp v2: नैजिसे, नैरोजा नैजिसे, द’जिदाइन
रोजा, से लाख रां); the corpus uses लाख and कौटि, not a native word, above the thousands. Zero is
लाथिख’ (Wiktionary). Ordinals take -थि (1थि सेथि, 21थि नैजिसेथि, as in Bodo text), ₹25 is नैजिबा
रां, 10:30 जि रिंगा थामजि मिनिट (रिंगा and मिनिट as in Bodo text; the word order has not been
checked by a Bodo speaker), percent प्रतिशत, the decimal point पइन्ट, and the month names come from
Unicode CLDR (`brx.xml`). Bodo also counts with classifiers (मोनसे "one thing", सासे "one person");
numbers written with digits are read without one. Compatible mode reads numbers in Hindi, as the
reference voice does.

**Romanised Bodo** follows the older Latin orthography (w for ɯ, o for the inherent vowel, ng for
ŋ), not romanised Hindi, so it has its own rules (`crates/uccharan/src/brx.rs`): a list of 32
common words that are also English words or have irregular spellings (`ang` आं, `arw` आरो, `boro`
बर’, `raw` राव, `man` मान), then a letter converter for the other words that the English lexicon
does not know (`jwnwm` जोनोम, `dokhona` दख’ना, `kokrajhar` कक्राझार). `transliterate(..., "brx")` (no
pack) and the `hinglish` option use it: `ang boro raw bungw` → आं बर’ राव बुङो. The list holds the
words of the first article of the Declaration, so that sentence is no test of the converter; there
is no romanised Bodo test set, so this is not measured.

Held-out text: 2,000 sentences of Bodo from AI4Bharat's IndicCorp v2 (CC0; Tatoeba has too few and
FLORES-200 itself has no Bodo), lines in Hindi left out, and as a second held-out set the 1,012
FLORES-200 `devtest` sentences in Bodo from Google's IndicGenBench (Flores-IN, CC BY-SA 4.0):
compatible mode against the Hindi voice gives PER 0.37 % (0.32 % without stress) on the first and
0.48 % (0.35 %) on the second. Bodo Wikipedia exists only in the Wikimedia Incubator (`Wp/brx`,
189 pages; `bxr` is Buryat) and is not used. A hand-written gold set
([tools/eval/gold/brx](../tools/eval/gold/brx): 150 words and 35 sentences, split by a hash before
the rules were run on it; words from Wiktionary, L2/08-197, the Wikipedia numerals, CLDR and
IndicCorp v2; the Declaration sentence with its pronunciation from the Latin version, the others
from IndicCorp v2 lines outside the evaluation sets) measures improved mode; held-out half:

| Held-out | Hindi-voice fallback (espeak-ng `hi`) | uccharan compatible | **uccharan improved** |
| --- | ---: | ---: | ---: |
| Bodo words (79), exact | 2.5 % | 2.5 % | **98.7 %** |
| Bodo sentences (16): PER / identical | 52.91 % / 0.0 % | 52.91 % / 0.0 % | **0.44 % / 87.5 %** |

The Hindi voice scores so low because it reads Bodo with Hindi's vowels (ə for ɔ, oː for ɯ) and
consonants (ɟ for z). The gold set writes down the descriptions named above for this project, so it
shows that improved mode implements them, not that a Bodo speaker would accept every form; the
remaining held-out word error is ककबरक (Kokborok), where the Hindi-type deletion keeps a vowel the
word does not have.

**Santali** (`sat`, also `santali`, `sat-olck`; Ol Chiki, U+1C50–U+1C7F) is a Munda
(Austroasiatic) language written in its own alphabet, Ol Chiki, which writes every vowel as a
letter and has no inherent vowel and no virama. espeak-ng 1.52.0 has no Santali voice and cannot
read the script at all: under every voice it says the code point of each letter (for ᱥ, U+1C65,
`-v hi` says "akshar ek si chhah panch", `-v en` "letter 1c65"). No voice was trained on that, so there
is nothing for compatible mode to reproduce, and comparing with it would measure nothing: **both
modes give the same reading**, uccharan's Santali rules (`lang/sat.lang`, read by the Brahmic
engine with every consonant marked as having no vowel of its own), and the language-switch flag is
`(sat)`. The reading:

- vowels ᱚ ᱟ ᱤ ᱩ ᱮ ᱳ = ɔ a i u e o; with ᱹ (gaahlaa ttuddaag) ᱟᱹ = ə and ᱮᱹ = ɛ (ᱯᱟᱹᱨᱥᱤ pərsˈi, ᱨᱮᱹᱞ
  rˈɛl); ᱚᱹ, whose contrast with ᱚ is marginal, is ɔ; ᱸ (mu ttuddag) nasalises the vowel before it
  (ᱦᱚᱸ hɔ̃, ᱛᱟᱦᱮᱸ tahˈẽ), ᱺ (mu-gaahlaa ttuddaag) is both marks (ᱥᱟᱺᱜᱤᱧ sə̃ɡˈiɲ), ᱻ (relaa)
  lengthens the vowel (ᱨᱮᱻᱞ rˈeːl);
- the four glottal letters ᱜ ᱡ ᱫ ᱵ are voiced stops ɡ ɟ d b before a vowel (ᱜᱚ ɡˈɔ) and checked
  stops at the end of a word: glottalised and unreleased, written k̚ʔ c̚ʔ t̚ʔ p̚ʔ (ᱢᱤᱫ mˈit̚ʔ, ᱟᱡᱟᱜ
  aɟˈak̚ʔ); ᱼ (phaarkaa) keeps the checked value before a vowel (ᱢᱮᱱᱟᱜᱼᱟ menˈak̚ʔa, ᱜᱼᱚ k̚ʔˈɔ), ᱽ
  (ahad) gives the voiced value where the checked one would be read (ᱨᱟᱡᱽ rˈaɟ, ᱜᱷᱟᱨᱚᱸᱡᱽ ɡʱarˈɔ̃ɟ,
  ᱜᱤᱫᱽᱨᱟᱹ ɡidrˈə). Before another consonant the sources leave the value open (Zide: checked "at
  certain word-internal preconsonantal junctures"); uccharan reads a glottal letter as checked
  before a voiceless consonant (ᱢᱤᱫᱴᱟᱹᱝ mit̚ʔʈˈəŋ, ᱦᱤᱥᱟᱹᱵᱛᱮ) and voiced before a voiced one (ᱜᱤᱫᱨᱟᱹ
  ɡidrˈə, ᱰᱤᱜᱨᱤ), which is how Ol Chiki text spells these words. A loanword whose voiced stop
  stands before a voiceless consonant without ᱽ (ᱟᱹᱵᱠᱷᱟᱡᱤᱭᱟᱹᱱ "Abkhazian") is therefore read checked;
- ᱷ after a consonant writes aspiration (ᱞᱮᱠᱷᱟ lekʰˈa, ᱵᱷᱟᱨᱚᱛ bʱarˈɔt: the voiced aspirates are
  breathy ɡʱ ɟʱ dʱ ɖʱ bʱ ɽʱ), and after a vowel or at the start of a word it is h;
- the other consonants are ᱛ ᱠ ᱪ ᱯ ᱴ t k c p ʈ, ᱰ ɖ, ᱝ ᱢ ᱱ ᱧ ᱬ ŋ m n ɲ ɳ, ᱥ ᱦ s h, ᱨ ᱲ r ɽ, ᱞ l;
  ᱭ ᱣ are j w before a vowel and close a diphthong after one (ᱠᱟᱱᱟᱭ kanˈai, ᱵᱮᱱᱟᱣ benˈau), and ᱶ is
  the nasal glide w̃ (ᱱᱚᱶᱟ nɔw̃ˈa), ũ after a vowel (ᱥᱟᱶ sˈaũ);
- primary stress is on the second syllable (Santali is "strongly second-syllable prominent in
  uninflected disyllables", Anderson and Horo 2021); the particles and postpositions ᱫᱚ ᱨᱮ ᱛᱮ
  ᱠᱷᱚᱱ ᱠᱚ ᱜᱮ ᱟᱨ ᱦᱚᱸ ᱥᱮ are unstressed. Loanwords that keep their own stress are not told apart;
- written habits: a full stop typed for ᱹ, which it resembles (ᱟ.ᱲᱟ for ᱟᱹᱲᱟ), and a hyphen typed
  for ᱼ (ᱢᱮᱱᱟᱜ-ᱟ) are read as the marks; ᱻ after a glottal letter, a common slip for ᱼ, is read as
  ᱼ; ᱾ and ᱿ end a sentence. Marks after a letter that does not take them are not said.

Sources: Michael Everson, *Final proposal to encode the Ol Chiki script in the UCS* (ISO/IEC
JTC1/SC2/WG2 N2984R, Unicode L2/05-243R, 2005, after Norman Zide 1996, "Scripts for Munda
languages", and Raghunath Murmu's *Ranar*) for the letter values and the marks; Arun Ghosh,
"Santali" (in G. D. S. Anderson, ed., *The Munda Languages*, Routledge 2008, pp. 11–98) and Lukas
Neukom, *Santali* (LINCOM 2001), as summarised in the Wikipedia articles "Santali language" and
"Ol Chiki script", for the phonemes and the checked stops (Sebeok 1943 and Ghosh 2008 treat them
as allophones of p t c k in the coda); Gregory D. S. Anderson and Luke Horo (2021), "Towards a
prosodic typology of the Kherwarian languages: Santali of Assam" (JSEALS Special Publication 8),
for the stress. Not modelled: vowel harmony and the vowel differences between dialects, which
Ol Chiki does not write (the Wikipedia transcription of the numerals has ɛ where the spelling has
ᱮ: pɛ, arɛ, ɡɛl), and the deletion of a second vowel in words of three syllables (Wikipedia's
/hapaɽam/ → [hapˈɽám]); every letter keeps its value.

**Numbers.** Both modes, and normalise-only mode, read numbers written with ASCII or Ol Chiki
digits (᱐–᱙) in Santali words: the units ᱢᱤᱫ ᱵᱟᱨ ᱯᱮ ᱯᱩᱱ ᱢᱚᱬᱮ ᱛᱩᱨᱩᱭ ᱮᱭᱟᱭ ᱤᱨᱟᱹᱞ ᱟᱨᱮ, ᱜᱮᱞ 10, 11–19
as ᱜᱮᱞ and the unit (ᱜᱮᱞ ᱢᱤᱫ), the tens as a unit and ᱜᱮᱞ (25 ᱵᱟᱨ ᱜᱮᱞ ᱢᱚᱬᱮ), ᱥᱟᱭ for hundreds
(ᱢᱤᱫ ᱥᱟᱭ), and the borrowed ᱦᱟᱡᱟᱨ, ᱞᱟᱠᱷ and ᱠᱳᱴᱤ above (2021 ᱵᱟᱨ ᱦᱟᱡᱟᱨ ᱵᱟᱨ ᱜᱮᱞ ᱢᱤᱫ). This follows
the numerals of the Wikipedia article and how Santali text in IndicCorp v2 writes numbers (ᱯᱮ ᱜᱮᱞ
ᱛᱩᱨᱩᱭ 36, ᱵᱟᱨ ᱜᱮᱞ ᱯᱮ 23, ᱢᱤᱫ ᱦᱟᱡᱟᱨ, ᱵᱟᱨ ᱞᱟᱠᱷ); the same text also writes ᱯᱚᱱ for 4 and ᱢᱚᱲᱮ for 5,
and counts thousands as "ten hundreds" (ᱢᱤᱫ ᱜᱮᱞ ᱥᱟᱭ 1000), which uccharan does not. The twenty
ᱤᱥᱤ is not used for numbers written with digits. Zero is ᱡᱤᱨᱳ, the decimal point ᱯᱚᱭᱮᱱᱴ, minus
ᱢᱤᱱᱟᱥ, per cent ᱥᱟᱭ ᱠᱟᱲᱟ and ₹ ᱴᱟᱠᱟ after the amount (all as in the same text); month names come
from Unicode CLDR (`sat.xml`). There are no ordinals and no clock times: the corpus gives no
settled written form for either. As for the other Indic scripts, a date written with native digits
(᱑᱕/᱐᱘/᱒᱐᱒᱔) is read number by number, not as a date.

**Romanised Santali** follows the Santali Latin alphabet (Bodding 1930: ạ for ə, k' c' t' p' for
the checked stops, ṭ ḍ ṇ ṛ, ñ ṅ, a tilde for a nasal vowel) and its casual ASCII form, with rules of
its own (`crates/uccharan/src/sat.rs`), written in Ol Chiki: a list of 25 common words (`johar`
ᱡᱚᱦᱟᱨ, `inj` ᱤᱧ, `disom` ᱫᱤᱥᱚᱢ, `do` ᱫᱚ, `santali` ᱥᱟᱱᱛᱟᱲᱤ, `menak'a` ᱢᱮᱱᱟᱜᱼᱟ), then a letter
converter: c = ᱪ, j = ᱡ, y = ᱭ, o = ᱚ (ō ô = ᱳ), ng = ᱝ (ᱝᱜ before a vowel), h after a stop = ᱷ;
k' c' t' p', and k c t p at the end of a word (a final stop of a Santali word is checked), are
ᱜ ᱡ ᱫ ᱵ, with ᱼ before a vowel; g j d b where ᱜ ᱡ ᱫ ᱵ would be read checked take ᱽ (`raj` ᱨᱟᱡᱽ).
`transliterate(..., "sat")` (no pack) and the `hinglish` option use it; a clause is rewritten only
when it has one of 17 distinctive listed words or a letter that only the Santali alphabet uses, so
English text is left alone, and English words stay in Latin letters (there is no Ol Chiki
respelling of English). `johar inj disom do santali kana` → ᱡᱚᱦᱟᱨ ᱤᱧ ᱫᱤᱥᱚᱢ ᱫᱚ ᱥᱟᱱᱛᱟᱲᱤ ᱠᱟᱱᱟ. Casual
romanisation does not mark ə, ɛ, ɔ against o, or the checked stops consistently, so the converter
guesses there; there is no romanised Santali test set, and this is not measured.

**Other scripts.** Santali is also written in Bengali, Odia, Devanagari and Latin script. uccharan
reads Santali only in Ol Chiki (and romanised as above): a Bengali, Odia or Devanagari word in a
`sat` request is read by that script's language (Bengali, Odia, Hindi), not as Santali. Those
spellings follow each script's own conventions and mark the checked stops and ə differently from
writer to writer, and all the text available for testing (IndicCorp v2, IndicGenBench, Santali
Wikipedia, Tatoeba) is in Ol Chiki.

Held-out text: 2,000 sentences of Santali from AI4Bharat's IndicCorp v2 (CC0; in Ol Chiki; lines
with wiki markup left out), and three more held-out sets, never used for the rules: the 1,012
FLORES-200 `devtest` sentences in Santali from Google's IndicGenBench (Flores-IN, CC BY-SA 4.0),
500 sentences from random articles of the Santali Wikipedia (sat.wikipedia.org, CC BY-SA 4.0;
none of them in the IndicCorp text) and 1,000 Tatoeba sentences (CC BY 2.0 FR). With no reference
reading they cannot give a phoneme error rate; `tools/eval/sat_check.py` measures what can be
measured without one:

| Held-out set | Ol Chiki words | read to Santali phonemes | marks not said (per 1,000 Ol Chiki characters) |
| --- | ---: | ---: | ---: |
| IndicCorp v2, 2,000 sentences | 25,262 | 100.00 % | 1.47 |
| IndicGenBench Flores-IN, 1,012 sentences | 23,935 | 100.00 % | 1.53 |
| Santali Wikipedia, 500 sentences | 7,197 | 100.00 % | 0.36 |
| Tatoeba, 1,000 sentences | 4,865 | 100.00 % | 0.29 |

"Read" means a non-empty pronunciation made only of the phonemes above; the marks not said are
mostly ᱹ after ᱤ or a consonant and ᱼ ᱽ after other letters, typing slips the rules leave out. This
shows the rules cover the text, not that the readings are right. That is what the hand-written
gold set checks ([tools/eval/gold/sat](../tools/eval/gold/sat): 164 words and 35 sentences, split
by a hash before the rules were run on it; words from the Ol Chiki descriptions, the Wikipedia
numerals, CLDR and IndicCorp v2; the first article of the Declaration as quoted in the Wikipedia
article and 33 IndicCorp v2 lines outside all the evaluation sets). Held-out half:

| Held-out | espeak-ng | uccharan (both modes) |
| --- | ---: | ---: |
| Santali words (100), exact | — (reads code points) | **100.0 %** |
| Santali sentences (16): PER / identical | — | **0.00 % / 100.0 %** |

Development half: 63 of 64 words (the loanword ᱟᱹᱵᱠᱷᱟᱡᱤᱭᱟᱹᱱ, read with a checked p̚ʔ before kʰ),
sentence PER 0.00 %. The perfect held-out score is not a measure of how Santali sounds: Ol Chiki
is a phonemic alphabet, the gold set was written by applying the published letter values by hand,
and the rules apply the same values, so the score shows that the rules implement the description
without slips. What the description leaves out (vowel harmony, dialect vowels, loanword stress
and voicing) is in neither. No Santali voice or speech recogniser has been tried.

Speed: there is no espeak-ng reading to time against. On the Windows 11 laptop of the § rows
(500 held-out IndicCorp v2 sentences, one thread, best of 5 rounds,
[`compare_sat_windows.json`](../tools/bench/results/compare_sat_windows.json)) uccharan takes
16.7 µs per sentence at the median (p99 38.4 µs), 0.72 million words per second; in the same run
Bodo took 20.4 µs (2.4× espeak-ng's speed) and Hindi 10.8 µs (1.8×).

**Manipuri** (`mni`, also `manipuri`, `meitei`, `meetei`, `mni-mtei`, `mni-beng`) is Meitei
(Meeteilon), a Tibeto-Burman language with two lexical tones, written in two scripts: **Meetei
Mayek** (U+ABC0–U+ABFF, extensions U+AAE0–U+AAFF), its own Brahmic-type script, official since the
2000s, and the **Bengali script**, long the usual one and still common. uccharan reads both.
espeak-ng 1.52.0 has no Meitei voice (its `bpy` voice is Bishnupriya Manipuri, an Indo-Aryan
language) and cannot read Meetei Mayek (it says each letter's code point), so **both modes give the
same reading** (`Lang::no_reference`) and the language-switch flag is `(mni)`. Its Bengali voice can
read Bengali-script Meitei, but as Bengali (inherent ɔ, Bengali vowels and finals, ৱ spelt out by
name): a compatible mode reproducing it would reproduce another language, so there is none; the gold
set below shows how far it is.

Meetei Mayek is read by `lang/mni.lang` (Brahmic engine, `block = abc0`):

- a consonant letter carries the inherent vowel ɐ (written ə in much recent work, said [ɐ]);
  ꯥ ꯤ ꯨ ꯦ ꯣ = a i u e o, and ꯩ ꯧ, /ɐj/ /ɐw/, are said [ej] [ow], written ei ou (ꯃꯤꯇꯩꯂꯣꯟ miteilon);
  ꯑ carries the vowel signs, ꯏ ꯎ are i u;
- the lonsum letters ꯛ ꯜ ꯝ ꯞ ꯟ ꯠ ꯡ write k l m p n t ŋ closing a syllable, ꯢ a closing i; apun iyek
  ꯭ kills a vowel inside a cluster (ꯀ꯭ꯂꯥꯁ klas) and is ignored after a lonsum letter; ꯪ (nung) is ŋ;
- the consonants ꯀ ꯈ ꯒ ꯘ ꯉ = k kʰ ɡ ɡʱ ŋ, ꯆ ꯖ ꯓ = tʃ dʒ dʒʱ, ꯇ ꯊ ꯗ ꯙ ꯅ = t tʰ d dʱ n, ꯄ ꯐ ꯕ ꯚ ꯃ
  = p pʰ b bʱ m, ꯌ ꯔ ꯂ ꯋ = j ɾ l w, ꯁ ꯍ = s h; the extension letters for Sanskrit and Pali sounds
  keep their Unicode values;
- ꯏ ꯎ ꯌ after a vowel sign and before a consonant or the end of a word close a diphthong (ꯂꯥꯏ lai,
  ꯆꯥꯎꯕ tʃaubɐ, ꯃꯈꯣꯌ mɐkʰoi), and ꯏ ꯎ right after a consonant letter make ꯩ ꯧ with its vowel (ꯄꯨꯔꯀꯏ
  puɾɐkei); elsewhere they are vowels (ꯃꯤꯑꯣꯏ mioi, ꯉꯥꯡꯏ ŋaŋi);
- a word-final consonant letter keeps ɐ after the consonants of the suffixes -bɐ -nɐ -pɐ -dɐ -ɡɐ -mɐ
  -tɐ -kɐ (ꯂꯩꯕ leibɐ, ꯑꯁꯤꯗ ɐsidɐ) and has none after the others, which end loanwords (ꯃꯅꯤꯄꯨꯔ
  mɐnipuɾ; Wiktionary has mɐnipuɾɐ for this one);
- no stress marks (`stress = none`): Meitei is a tone language, and the sources give no stress
  accent to mark. The tones (two, some say three) are written in neither script, so they are not
  read; lum iyek ꯬, a heavy-tone mark the proposal encodes but text rarely uses (76 times in
  IndicCorp v2), is skipped. The proposal also notes that modern spelling sometimes tells words apart
  by tone with or without ꯥ; uccharan reads ꯥ as a there.

Two things neither script writes are made explicit first (`mni::prepare`): a stem ending in a
consonant is often spelt without a lonsum letter or hasanta before a suffix (ꯃꯇꯃꯗꯥ, মতমদা
mɐtɐmdɐ, ꯇꯃꯅꯥ tɐmnɐ), so a k t p m n ŋ l letter without a vowel sign, after a vowel and before a
consonant that has one, closes the syllable (right to left; not after ꯑ: ꯑꯃꯁꯨꯡ ɐmɐsuŋ, and not at
the start of a number word after a ten: ꯇꯔꯥꯃꯔꯤ); and many writers spell the suffix vowel ɐ at the
end of a word with ꯥ or া (ꯂꯥꯎꯕꯥ, লৈবা), so after ꯅ, after ꯕ ꯗ ꯒ following a vowel or voiced
consonant, and after ꯄ ꯇ ꯀ following ꯛ ꯠ ꯞ (the suffixes alternate p t k / b d ɡ), a final ꯥ of a
longer word is ɐ (laubɐ, leibɐ), except in six listed words with a final a (ꯀꯅꯥ "who", ꯃꯄꯥ "father").
Both are guesses about morphology the spelling hides: ꯁꯅꯥꯃꯍꯤ (Sanamahi) is read with a closed
syllable it does not have.

**Bengali script.** Bengali-script Meitei is written in Meetei Mayek letter for letter
(`mni::bengali_to_meetei`) and then read the same way, so the two scripts give the same reading of
the same word (লৈবাক অমসুং = ꯂꯩꯕꯥꯛ ꯑꯃꯁꯨꯡ). The correspondences follow how Bengali-script Meitei is
spelt: a final consonant without a vowel sign closes the syllable (মতম mɐtɐm, থবক tʰɐbɐk; শহর gets ꯁꯍꯔ꯭
with apun iyek, as র has no lonsum form); hasanta before a consonant is a lonsum letter (মহাক্না
ꯃꯍꯥꯛꯅꯥ), except before র য ৱ, ব after a stop (খ্বাই kʰwai) and ল at the start of a word, which make a
cluster; শ ষ স are s, ছ is tʃ, the retroflex letters are the dental ones, ড় ঢ় are ɾ, য য় are j, ৱ is
w, ং is ŋ, ৎ is t; Bengali digits are digits. This is not how the Bengali or Assamese readers read
the script: only a request for `mni` goes this way, and Bengali and Assamese text read exactly as
before (byte-identical output on their 8,000 development and held-out sentences, both modes, three
profiles). A Bengali-script word in a request for another language is read as Bengali.

Sources: Michael Everson, *Proposal for encoding the Meitei Mayek script in the BMP of the UCS*
(ISO/IEC JTC1/SC2/WG2 N3206R2, Unicode L2/07-005R2, 2007) and the Irish and US national bodies'
*Proposed Encoding for Meetei Mayek Block* (N3473, L2/08-232, 2008) for the letters, the lonsum
letters, apun iyek and lum iyek; Shobhana L. Chelliah, *A Grammar of Meithei* (Mouton de Gruyter,
1997) and D. N. S. Bhat and M. S. Ningomba, *Manipuri Grammar* (LINCOM, 1997), as summarised in the
Wikipedia articles "Meitei language" and "Meitei script", for the phonemes, the [ɐ] and the
diphthongs, the tones and the voicing of suffixes; English Wiktionary's Manipuri entries for the
pronunciation of final letters. Not modelled: tone; the dissimilation of aspirates and the velar
deletion that Wikipedia describes (they are mostly written); a and ɐ in closed syllables where the
spelling does not show them (ꯂꯝꯕꯤ lambi); clusters in loanwords written without apun iyek (ꯙꯔꯃ,
Wiktionary dʱɐɾmɐ, is read dʱɐɾɐmɐ).

**Numbers.** Both modes, and normalise-only mode, read numbers written with ASCII, Meetei Mayek
(꯰–꯹) or Bengali digits in Meitei words, as Meetei Mayek text spells them (IndicCorp v2, whose number
articles spell out 1–999; the units, tens, 100 and 1000 as the Wikipedia article lists them): the
units ꯑꯃꯥ ꯑꯅꯤ ꯑꯍꯨꯝ ꯃꯔꯤ ꯃꯉꯥ ꯇꯔꯨꯛ ꯇꯔꯦꯠ ꯅꯤꯄꯥꯟ ꯃꯥꯄꯟ, ꯇꯔꯥ 10, ꯀꯨꯟ 20, ꯀꯨꯟꯊ꯭ꯔꯥ 30, ꯅꯤꯐꯨ 40, ꯌꯥꯡꯈꯩ 50,
ꯍꯨꯝꯐꯨ 60, ꯍꯨꯝꯐꯨꯇꯔꯥ 70, ꯃꯔꯤꯐꯨ 80, ꯃꯔꯤꯐꯨꯇꯔꯥ 90 (scores of twenty), a unit glued to its ten (ꯀꯨꯟꯃꯉꯥ 25,
1–3 as ꯃꯊꯣꯏ ꯅꯤꯊꯣꯏ ꯍꯨꯝꯊꯣꯏ: ꯀꯨꯟꯃꯊꯣꯏ 21; 11 and 13 as ꯇꯔꯥꯃꯥꯊꯣꯏ ꯇꯔꯥꯍꯨꯝꯗꯣꯏ), the hundreds ꯆꯥ- with
the unit (ꯆꯥꯝꯃꯥ 100, ꯆꯥꯅꯤ 200, ꯆꯥꯝꯉꯥ 500) and -ꯒ ꯁꯨꯞꯅ before the rest (ꯆꯥꯅꯤꯒ ꯁꯨꯞꯅ ꯑꯃꯥ 201); above,
the scale word before its count, as the text writes it (`scale-first`: ꯂꯤꯁꯤꯡ ꯑꯅꯤ 2000, ꯂꯥꯈ ꯇꯔꯥ,
ꯀ꯭ꯔꯣꯔ). Zero ꯐꯨꯟ (Wiktionary), point ꯄꯣꯏꯟꯇ and the rupee ꯂꯨꯄꯥ after the amount (Meitei text often
puts it first; uccharan does not); minus and per cent are the English loans ꯃꯥꯏꯅꯁ ꯄꯔꯁꯦꯟꯇ, as no
Meitei word was found in the sources; months as IndicCorp v2 spells them, in CLDR's day-month-year
order. In normalise-only mode, text written in Bengali script gets the same words in Bengali script
(`mni::meetei_to_bengali`: অমা 25 → অমা কুন্মঙা, as Bengali-script Meitei writes অমা অনি তরা লিশিং).
No ordinals or clock times.

**Romanised Meitei** (`eina nangbu nungsi`) is written in Meetei Mayek through the `hinglish` option
and `transliterate(..., "mni")` without a pack ([transliteration](transliteration.md#manipuri)).

Held-out text: 2,000 IndicCorp v2 sentences (CC0; Meetei Mayek; lines with wiki markup left out),
370 sentences from random articles of the Manipuri Wikipedia (mni.wikipedia.org, CC BY-SA 4.0, in
Meetei Mayek; none in the IndicCorp text; the 300 random articles the builder fetches gave 370
sentences), and, in Bengali script, the 1,012 FLORES-200 `devtest` sentences of IndicGenBench
(Flores-IN, CC BY-SA 4.0), whose `dev` split is the Bengali-script development set. FLORES+ also has
Manipuri in both scripts (`mni_Beng`, `mni_Mtei`) but is gated; Tatoeba has eleven Manipuri
sentences (used in the gold set). With no reference reading, `tools/eval/mni_check.py` checks
coverage: every word of every set is read (100.00 %), and 71.4 % of the Bengali-script word tokens
of the held-out IndicGenBench set are read the same as a Meetei Mayek word that occurs in IndicCorp
v2. The gold set ([tools/eval/gold/mni](../tools/eval/gold/mni)): Wiktionary pronunciations for
375 Meetei Mayek words and 150 of their Bengali-script spellings, 36 numerals, 33 sentences
transcribed by hand; held-out half:

| Held-out | espeak-ng `bn` | uccharan (both modes) |
| --- | ---: | ---: |
| Meetei Mayek words (193), exact | — (reads code points) | **92.2 %** |
| Bengali-script words (83), exact | 28.9 % | **95.2 %** |
| Meetei Mayek sentences (10): PER | — | **1.25 %** |
| Bengali-script sentences (9): PER | 23.32 % | **0.52 %** |

Details and the development half in [evaluation.md](evaluation.md). No Meitei voice or speech
recogniser has been tried.

Speed: there is no espeak-ng reading to time against. On the Windows 11 laptop of the § rows (500
held-out IndicCorp v2 sentences, Meetei Mayek, one thread, best of 5 rounds,
[`compare_mni_windows.json`](../tools/bench/results/compare_mni_windows.json)) uccharan takes 14.2 µs
per sentence at the median (p99 44.9 µs), 0.55 million words per second; in the same run Bodo took
20.3 µs (2.4× espeak-ng's speed) and Hindi 11.1 µs (1.8×).

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
