# Romanised text into Indian scripts

People type `aap kaise ho` or `mujhe kal office jana hai`. Voices and speech recognisers do
better with `आप कैसे हो` and `मुझे कल office जाना है`. `transliterate` writes the romanised
words of a language in its script, leaves English words as English, and returns spans aligned
to the source text.

```python
import uccharan

uccharan.load_pack("xlit-hi")                       # the Hindi pack, CC BY-SA 4.0 (see below)
t = uccharan.transliterate("mujhe kal office jana hai", "hi")
t.text                                              # 'मुझे कल office जाना है'
[(s.kind, s.text, s.written) for s in t.spans if s.kind != "other"]
# [('indic', 'mujhe', 'मुझे'), ('indic', 'kal', 'कल'), ('english', 'office', 'office'),
#  ('indic', 'jana', 'जाना'), ('indic', 'hai', 'है')]

uccharan.transliterate("mujhe kal office jana hai", "hi", english="native").text
# 'मुझे कल ऑफ़िस जाना है'
```

```js
import { loadTranslit, transliterate } from "uccharan";
await loadTranslit("hi", bytesOfXlitHiUcd);          // or place the file in the data directory
(await transliterate("aap kaise ho", { lang: "hi" })).text;   // "आप कैसे हो"
```

```rust
use uccharan::translit::{transliterate, TranslitOptions};
use uccharan::{Lang, Phonemizer};

let p = Phonemizer::new();
p.load_pack(std::fs::read("xlit-hi.ucd")?)?;
let t = transliterate(&p, "aap kaise ho", Lang::Hi, &TranslitOptions::default());
assert_eq!(t.text, "आप कैसे हो");
```

```c
char *json = uccharan_transliterate(p, "aap kaise ho", "hi", NULL);   /* after uccharan_load_pack */
```

```sh
uccharan -l hi -p native --pack xlit-hi.ucd "aap kaise ho"            # आप कैसे हो
uccharan -l hi -p native-json --pack xlit-hi.ucd "aap kaise ho"       # text, spans, alternatives
```

## Languages and packs

The work is done with a language pack, one `.ucd` file per language:

| Pack | Language | Size | Chat-spelling list | Homograph list |
| --- | --- | ---: | --- | --- |
| `xlit-hi` | Hindi | 8.7 MB | 240 entries, checked on development sentences | 100 entries, checked on development sentences |
| `xlit-mr` | Marathi | 7.4 MB | 108, not yet reviewed by a native speaker | 38, not yet reviewed |
| `xlit-bn` | Bengali | 7.8 MB | 103, not yet reviewed | 45, not yet reviewed |
| `xlit-ta` | Tamil | 7.0 MB | 101, not yet reviewed | 26, not yet reviewed |
| `xlit-te` | Telugu | 7.0 MB | 96, not yet reviewed | 25, not yet reviewed |
| `xlit-gu` | Gujarati | 6.0 MB | 99, not yet reviewed | 48, not yet reviewed |
| `xlit-pa` | Punjabi (Gurmukhi) | 3.1 MB | 111, not yet reviewed | 48, not yet reviewed |
| `xlit-kn` | Kannada | 6.7 MB | 98, not yet reviewed | 21, not yet reviewed |
| `xlit-ml` | Malayalam | 6.4 MB | 101, not yet reviewed | 28, not yet reviewed |

All nine packs have the transliteration model, the lexicon of attested spellings and the
word counts. The two hand-written lists differ: the Hindi lists were developed against Hindi
sentences; the lists of the other eight languages were written by hand and have not been
checked by a native speaker or against code-mixed text (there is no such test set for them
yet). The weights that decide between English and the language were fitted on Hindi and are
used unchanged for the others.

**Licence.** The packs are **CC BY-SA 4.0**, not Apache-2.0: they are trained on the Dakshina
and Aksharantar word pairs and use counts from corpora that include Wikipedia text
([DATA_LICENSES.md](../DATA_LICENSES.md)). They are therefore not inside the crate, the wheel,
the npm package or the C library, and nothing fetches them on its own. They are assets of each
GitHub release, with the licence text and attributions next to them. Load one when that
licence suits your use:

* Python: `uccharan.load_pack("xlit-hi")` (downloaded once, SHA-256 checked, cached; or put the
  file in `$UCCHARAN_DATA_DIR`), or `uccharan.load_pack("/path/xlit-hi.ucd")`;
* JavaScript: `loadTranslit("hi", bytes)`, or `loadTranslit("hi")` with the file in the data
  directory;
* Rust: `Phonemizer::load_pack(bytes)`; C: `uccharan_load_pack`; command line: `--pack FILE`.

**Without a pack** the function still answers, by rule: for Hindi (and Bhojpuri, Maithili,
Magahi) with the small built-in list of common romanised words and the rules behind the
`hinglish` option, for the other languages with a plain ITRANS-style converter that treats
every word outside the English lexicon as romanised. This is far less accurate (see the
"no pack" rows in [evaluation.md](evaluation.md#8-romanised-text-into-native-script)); the
result says whether a pack did the work (`Transliterated.pack` in Python and JavaScript,
`Phonemizer::has_translit` in Rust).

## Options

| Option | Values | Meaning |
| --- | --- | --- |
| `english` | `keep` (default) | English words stay in Latin letters |
| | `native` | English words are respelled in the script (`office` → `ऑफ़िस`), with the same respeller as `normalize(latin="transliterate")` |
| | `convert` | no language decision: every Latin word is taken for a romanised word. For text known to be fully romanised |
| `alternatives` | number, default 3 | candidates listed per word |
| `context` | default on | neighbouring words decide between candidates and between English and the language; off = each word alone |

In Rust these are the fields of `TranslitOptions`; in C, the command line (`--translit-options`)
and `transliterate_json` they are members of a JSON object.

## The result

`text` is the converted text. `spans` cover the input in order without gaps. Each span has

* `kind`: `indic` (a romanised word, written in the script), `english` (kept, or respelled
  when asked), or `other` (numbers, punctuation, spaces, emoji, text already in a native
  script, and words glued to digits such as `b4`);
* its range in the source and in the output: bytes in Rust; code points in Python
  (`byte_offsets=True` for bytes, `src_utf16` / `out_utf16` for UTF-16); UTF-16 in JavaScript
  (`start`, `end`, `outStart`, `outEnd`, with `src` / `out` holding all three);
* for a Latin word, `alternatives`: native candidates, the chosen one first, each with a share
  of belief between 0 and 1. An English span lists them too: they are what the word would
  have been had it been taken for a romanised word.

The JSON form (C, command line, `transliterate_json`):

```json
{"text": "कल office",
 "spans": [
  {"kind": "indic", "src": {"byte": [0,3], "char": [0,3], "utf16": [0,3]},
   "out": {"byte": [0,6], "char": [0,2], "utf16": [0,2]}, "text": "kal", "written": "कल",
   "alternatives": [{"text": "कल", "score": 0.956}, {"text": "काल", "score": 0.038}]},
  {"kind": "other", "src": {"byte": [3,4], ...}, "out": {...}, "text": " ", "written": " "},
  {"kind": "english", "src": {...}, "out": {...}, "text": "office", "written": "office",
   "alternatives": [{"text": "ऑफिस", "score": 0.879}]}]}
```

## With voices

* **Phoneme voices.** `phonemize(text, "hi", hinglish=True)` uses the Hindi pack when it is
  loaded: romanised words are written in Devanagari by the transliterator and then read by
  the Hindi front end; English words are read as English. Without the pack the option works
  as before, from the built-in list.
* **Text voices.** Transliterate, then normalise:
  `uccharan::translit::normalize_romanised` in Rust does both and returns the spoken form
  with spans that point into the original romanised text; from the other languages call
  `transliterate` and pass its text to `normalize`. Choose `english="native"` for a voice that
  cannot read Latin letters.
* **Speech recognition output or training text** that mixes scripts: `english="keep"` gives
  one script per language.

## How it decides

1. **Candidates.** A lexicon of attested romanised spellings of frequent words (for Hindi,
   58 k spellings of 20 k words, plus the hand-written chat spellings such as `nhi`, `kr`,
   `h`), and for any other word a statistical model: the romanised word is read letter by
   letter, each letter writing zero, one or two native characters, and a 6-gram model over
   these (letter, characters) steps — trained on 0.5 to 3.2 million word pairs per language —
   scores the outcomes. A beam search lists the best eight.
2. **Ranking.** Each candidate is scored by the model, by how often the spelling is attested,
   by how frequent the native word is in a corpus, and by sound: the romanised word and the
   candidate are both turned into phonemes by uccharan and compared with the weighted
   phoneme distance of the [sound-alike matcher](soundalike.md).
3. **English or not.** For each word the choice is between "English" and its native
   candidates, in one pass over the sentence. The English side is scored by the word's
   frequency in English; the native side by the frequency of the native word and how usual
   that spelling of it is; neighbours pull towards the same language, and pairs of native
   words that often occur together pull towards each other (this is what separates मैं from
   में for `me` / `main`, and की from कि).
4. **Loanwords typed in English spelling stay English.** `office` has a perfectly good
   Devanagari candidate, ऑफिस. When the chosen candidate sounds like the English word itself,
   the token is that English word and is kept (or respelled with `english="native"`).
5. **Homographs.** A hand-written list per language of spellings that are both an English
   word and an unrelated native word (`to` तो, `is` इस, `me` में, `bus` बस): for these only
   the neighbouring words decide.
6. **Hindi है / हैं.** Both are typed `hai`. After a plural verb form (`karte hai`, `gaye
   hai`) the word is written हैं, after a masculine singular one (`karta hai`) है; elsewhere
   the choice of step 3 stands.

Three or more of the same letter count as two (`yaaar`). Sentence punctuation ends the
stretch over which words influence each other.

## Bhojpuri, Maithili and Magahi

These languages are written in Devanagari and typed with the romanisation habits of Hindi,
and there are no public word pairs for them. They have no pack yet. Today:

* `transliterate(text, "bho")` (or `mai`, `mag`) without a pack uses the built-in Hindi list
  and rules;
* for better results on mostly-Hindi vocabulary, transliterate with the Hindi pack
  (`lang="hi"`) and read the result with the Bhojpuri, Maithili or Magahi front end: the
  letters-to-Devanagari step is the same problem in all four languages.

A pack of their own is the Hindi model with their own word data: the Hindi word pairs
together with word counts, word-pair counts, chat spellings (`ba` बा, `baani` बानी, `rahal`
रहल, `hamar` हमार) and homographs of the language. The ranking then prefers the words of the
language (रहल over रहाल) because the frequencies are its own. The pack builder supports this
(`lang=bho` in a recipe next to the Hindi `train.tsv`, see
`data/sources/translit/README.md`); what is missing is a large enough corpus per language to
count from and a test set to measure on.

## Dogri

Dogri is written in Devanagari and typed with the habits of romanised Hindi. It has no pack:
`transliterate(text, "doi")` and the `hinglish` option first write a short hand-written list of
common Dogri words in Devanagari (`tusen` तुसें, `kiyan` कि’यां, `eh` एह्, `ae` ऐ, `kanne` कन्नै,
`nein` नेईं, `ch` च, `gi` गी; 27 entries, not yet checked by a Dogri speaker or on chat text) and
then use the Hindi list and rules: `tusen kiyan ho` → तुसें कि’यां हो. There is no Dogri test set
for this yet, so it is not measured. A pack of its own would need romanised–Devanagari Dogri word
pairs; AI4Bharat's Aksharantar has a Dogri part (`doi.zip`) that the pack builder could use.

## Bodo

Romanised Bodo follows the older Latin orthography of Bodo (in use before Devanagari was adopted in
1975, and in the Wikipedia article's quotation of the Universal Declaration of Human Rights): `w`
for the vowel ɯ (ो), `wi` `wu` for ै ौ, `o` for the inherent vowel, `ng` for ŋ, `kh` `th` `ph` for
ख थ फ, `j` and `z` for ज. It is not romanised Hindi, so `transliterate(text, "brx")` and the
`hinglish` option use rules of their own (`crates/uccharan/src/brx.rs`), without a pack: a
hand-written list of 32 words that are also English words or are spelt irregularly (`ang` आं,
`arw` आरो, `boro` बर’, `raw` राव, `man` मान, `be` बे), then a letter converter for the other words
that the English lexicon does not know. The converter writes the inherent vowel of the first
syllable as a bare consonant and a later one with the apostrophe, as Bodo spelling does (`boro`
बर’, `dokhona` दख’ना, `lathikho` लाथिख’), joins consonants without a vowel between them with a
virama (`sni` स्नि, `kokrajhar` कक्राझार), and writes ŋ as ं at the end of a syllable (`subung`
सुबुं) and ङ before a vowel (`bithanga` बिथाङा). With the `hinglish` option it rewrites a clause only
when the clause has a listed word or one with the vowel `w` after a consonant (`jwnwm`, `arw`), so
English text is left alone. `ang boro raw bungw` → आं बर’ राव बुङो. There is no romanised Bodo test
set, so it is not measured. AI4Bharat's Aksharantar has a Bodo part (`brx.zip`) that the pack
builder could use; whether its romanisations follow the Latin orthography above has not been checked.

## Santali

Romanised Santali is written in the Santali Latin alphabet (Bodding 1930; ạ for ə, k' c' t' p' for
the checked stops, ṭ ḍ ṇ ṛ, ñ ṅ, a tilde for a nasal vowel) or in a casual ASCII form of it.
`transliterate(text, "sat")` and the `hinglish` option write it in Ol Chiki with rules of their own
(`crates/uccharan/src/sat.rs`), without a pack: a hand-written list of 25 common words (`johar`
ᱡᱚᱦᱟᱨ, `inj` ᱤᱧ, `disom` ᱫᱤᱥᱚᱢ, `do` ᱫᱚ, `re` ᱨᱮ, `santali` ᱥᱟᱱᱛᱟᱲᱤ, `menak'a` ᱢᱮᱱᱟᱜᱼᱟ, `mit` ᱢᱤᱫ),
then a letter converter for the other words that the English lexicon does not know: a ạ e ẹ i o ō
u = ᱟ ᱟᱹ ᱮ ᱮᱹ ᱤ ᱚ ᱳ ᱩ (a doubled vowel is one), c ᱪ, j ᱡ, y ᱭ, w v ᱣ, ñ ny ᱧ, ṅ ng ᱝ (ᱝᱜ before a
vowel: `sengel` ᱥᱮᱝᱜᱮᱞ), h after a stop ᱷ (`dharti` ᱫᱷᱟᱨᱛᱤ); k' c' t' p', and k c t p at the end of a
word, are the glottal letters ᱜ ᱡ ᱫ ᱵ, with ᱼ before a vowel (`mit'` ᱢᱤᱫ, `menak'a` ᱢᱮᱱᱟᱜᱼᱟ), and g j d
b where those letters would be read checked take ᱽ (`raj` ᱨᱟᱡᱽ). A clause is rewritten only when it
has one of 17 distinctive listed words or a letter that only the Santali alphabet uses (ạ ṛ ṭ ḍ ṇ ñ
ṅ, an apostrophe after k c t p), so English text is left alone; English words stay in Latin
letters, as there is no Ol Chiki respelling of English. `johar inj disom do santali kana` →
ᱡᱚᱦᱟᱨ ᱤᱧ ᱫᱤᱥᱚᱢ ᱫᱚ ᱥᱟᱱᱛᱟᱲᱤ ᱠᱟᱱᱟ. Casual romanisation leaves ə, ɛ, o against ɔ and the checked
stops unmarked, so the converter guesses there (o is ᱚ, the commoner vowel); there is no romanised
Santali test set, so it is not measured. Santali written in Bengali, Odia or Devanagari script is
not converted.

## Manipuri

Romanised Meitei, as people type it (a for ɐ, aa for a, ei, ou, kh th ph, ng; `eina nangbu nungsi`),
is written in **Meetei Mayek**, the official script, by `transliterate(text, "mni")` and the
`hinglish` option, without a pack (`crates/uccharan/src/mni.rs`): a hand-written list of 28 common
words in their usual Meetei Mayek spelling (`eina` ꯑꯩꯅꯥ, `nangbu` ꯅꯪꯕꯨ, `amasung` ꯑꯃꯁꯨꯡ, `leibak`
ꯂꯩꯕꯥꯛ, `ngasi` ꯉꯁꯤ, as in IndicCorp v2 text), then a letter converter for the words the English
lexicon does not know: a is the inherent vowel, aa ꯥ, i ee ꯤ, u oo ꯨ, e ꯦ, o ꯣ, ei ꯩ, ou ꯧ; i and u
(o after a) after a vowel close a diphthong (`lai` ꯂꯥꯏ, `chaoba` ꯆꯥꯎꯕ); kh gh ng ch jh th dh ph bh
sh have their letters; a consonant not followed by a vowel is a lonsum letter (k l m p n t ng: ꯛ ꯜ ꯝ
ꯞ ꯟ ꯠ ꯡ, `imphal` ꯏꯝꯐꯜ) or takes apun iyek ꯭ (`thr` ꯊ꯭ꯔ); an apostrophe separates syllables
(`laak'i` ꯂꯥꯛꯏ). A clause is rewritten only when it has one of 21 distinctive listed words, so
English text is left alone, and English words stay in Latin letters. Romanised Meitei does not tell
a from ɐ before a consonant (Meetei Mayek ꯂꯩꯕꯥꯛ, romanised `leibak`), so the converter writes ɐ there
unless the word is listed; there is no romanised Meitei test set (Tatoeba has three romanised
sentences), so this is not measured.

**Between the two scripts.** Bengali-script Meitei is read through `mni::bengali_to_meetei`, which
writes it in Meetei Mayek letter for letter, and normalise-only mode writes its number words back
with `mni::meetei_to_bengali`. Both are public Rust functions; the Python, WebAssembly and C
interfaces have no script-conversion call (`transliterate` writes romanised text in a script), so
they do not offer one. The conversion follows the letters and the habits of each script's spelling
(a final consonant without a vowel sign in Bengali script closes the syllable, a lonsum letter in
Meetei Mayek); it is not a spelling standardiser, and a round trip can change a spelling (a final
ꯥ written for the vowel ɐ).

## Accuracy, size and speed

Measured on held-out word pairs and sentences; method, baselines and all numbers are in
[evaluation.md](evaluation.md#8-romanised-text-into-native-script).

On 164 hand-written Hindi chat sentences with English mixed in (held-out), the Hindi pack
writes 98.2 % of the 868 Hindi words correctly (spelling variants such as हूँ / हूं counted as
equal), leaves 0.3 % of them in Latin letters, and converts 10 of the 130 English words by
mistake. That figure was obtained after a fix prompted by the first run on the same sentences,
which gave 95.9 %; the evaluation explains both. Without a pack the same sentences come out
72.4 % right with 20 % left in Latin.

| Language | First candidate right: Dakshina test words (IndicXlit, published) | Aksharantar native words (IndicXlit) | Words right in Dakshina test sentences | New words per second | Running text, words per second |
| --- | ---: | ---: | ---: | ---: | ---: |
| Hindi | 68.69 % (60.56) | 49.66 % (55.59) | 86.47 % | 3,145 | 12,105 |
| Marathi | 74.45 % (64.85) | 57.08 % (63.72) | 85.48 % | 4,138 | 7,700 |
| Bengali | 67.09 % (55.49) | 60.65 % (61.70) | 80.82 % | 3,246 | 7,793 |
| Tamil | 75.90 % (68.10) | 72.65 % (69.78) | 76.68 % | 4,634 | 5,580 |
| Telugu | 81.45 % (73.38) | 83.88 % (84.69) | 82.91 % | 3,823 | 5,803 |
| Gujarati | 65.62 % (62.02) | 54.72 % (61.89) | 82.74 % | 4,603 | 9,648 |
| Punjabi | 53.34 % (47.24) | 34.43 % (40.27) | 79.19 % | 6,236 | 17,392 |
| Kannada | 84.40 % (77.18) | 75.68 % (76.18) | 83.02 % | 4,224 | 5,999 |
| Malayalam | 71.66 % (63.56) | 61.62 % (64.73) | 70.87 % | 2,895 | 3,482 |

IndicXlit is AI4Bharat's neural transliterator (one 127 MB download for 21 languages); its
figures are the published ones. uccharan's first candidate is right more often on the Dakshina
test words in all nine languages and less often on Aksharantar's hand-made native words in
eight of them: it leans on word frequency, which helps most with common words. Speed is one
thread on a shared machine.

## Limits

* The romanisation does not always carry the difference: `ki` for कि / की, `gaya` for गया /
  गाया, `hai` after a feminine verb form. Context gets some of these.
* A homograph used as English inside a sentence of the language (`bus stand`, `log out`,
  `exam pass`) is the main source of English words written in the script.
* Names are converted like any word; a capital letter is not used as a cue. All-capital
  words lean English.
* Tokens are converted one by one: words typed together or split differently from the
  native spelling are not rejoined.
* Only Hindi has been measured on code-mixed text. For the other eight languages the word
  and sentence accuracy is measured, the English / native decision is not.
* A word the pack has never seen costs a beam search (about a third of a millisecond);
  repeated words are remembered.

## Building the packs

`cargo run --release -p uccharan-lexgen -- xlit <lang>` builds a pack from
`data/sources/translit/<lang>/` (recipe and hand-written lists in the repository) and the
large inputs described in `data/sources/translit/README.md`. The scripts that prepare the
inputs and score the result are in `tools/eval/translit/`.
