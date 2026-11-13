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
| Hindi (`hi`) | Tatoeba held-out | **0.12 %** | 0.09 % | 98.8 % | 0.12 % | 98.8 % |

For comparison, the development sets (which the rules were tuned on) give en-US 1.16 %, en-GB 1.41 %,
Hindi 0.15 %: held-out and development agree, so the rules are not over-fitted to the development
sample.

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

| Origin | Names | Reference (espeak-ng 1.52) | uccharan compatible | **uccharan improved** |
| --- | ---: | ---: | ---: | ---: |
| Indian | 48 | 43.8 % | 50.0 % | **77.1 %** |
| English | 52 | 84.6 % | 100 % | **100 %** |
| European | 17 | 70.6 % | 88.2 % | **100 %** |
| Hispanic | 19 | 52.6 % | 89.5 % | **89.5 %** |
| Chinese | 12 | 58.3 % | 91.7 % | **91.7 %** |
| Other East Asian | 10 | 50.0 % | 100 % | **100 %** |
| Arabic, Persian, Turkish | 6 | 50.0 % | 100 % | **100 %** |
| African | 8 | **50.0 %** | 37.5 % | 37.5 % |
| **Names not in CMUdict** | 41 | 34.1 % | 19.5 % | **56.1 %** |
| Names in CMUdict | 131 | 70.2 % | 99.2 % | 99.2 % |
| **All held-out** | 172 | 61.6 % | 80.2 % | **89.0 %** |

Read the rows with care: for names that are in CMUdict the reference pronunciation *is* CMUdict's,
and so is uccharan's lexicon, so 99 % there is expected rather than impressive. The fair comparison
is **names not in CMUdict: 56.1 % against 34.1 %**, and Indian names (77.1 % against 43.8 %).
African names are the one origin where the reference does better (4 against 3 of 8 — too few to
say much). The development half gives 92.8 % against 74.0 % overall and 90.3 % against 73.6 % on
Indian names.

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
| Hindi PER ≤ 5 % vs reference | 0.12 % | yes |
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
