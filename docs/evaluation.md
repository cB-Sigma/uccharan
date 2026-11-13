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
| en-US (`en-us`) | Tatoeba held-out | **1.19 %** | 0.97 % | 76.2 % | 1.16 % | 76.1 % |
| en-US (`en-us`) | Sherlock Holmes | 1.41 % | 1.16 % | 49.1 % | 1.42 % | 47.8 % |
| en-GB (`en-gb-x-rp`) | Tatoeba held-out | **1.41 %** | 1.17 % | 72.1 % | 1.38 % | 72.0 % |
| en-GB (`en-gb-x-rp`) | Sherlock Holmes | 1.51 % | 1.26 % | 45.2 % | 1.52 % | 44.1 % |
| Hindi (`hi`) | Tatoeba held-out | **0.12 %** | 0.09 % | 98.8 % | 0.12 % | 98.8 % |

For comparison, the development sets (which the rules were tuned on) give en-US 1.18 %, en-GB 1.42 %,
Hindi 0.15 %: held-out and development agree, so the rules are not over-fitted to the development
sample.

### Breakdown

**By kind of edit** (share of all edit operations, held-out):

| | Vowel quality | Stress | Mixed / other | Consonant | Length | Word boundary | Schwa (hi) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| en-US | 40.7 % | 29.6 % | 14.6 % | 10.6 % | 2.5 % | 1.9 % | — |
| en-GB | 42.6 % | 28.2 % | 17.3 % | 9.4 % | 0.9 % | 1.6 % | — |
| Hindi | 16.1 % | 35.0 % | 10.5 % | 21.7 % | 3.5 % | 3.5 % | 9.1 % |

**By word class** (word-level disagreement in sentences whose word counts line up):

| | Function words (weak forms) | Content words in CMUdict | Words not in CMUdict |
| --- | ---: | ---: | ---: |
| en-US | 0.60 % of 9 063 | 5.2 % of 10 533 | 31.1 % of 196 |
| en-GB | 0.60 % of 9 063 | 6.3 % of 10 533 | 30.6 % of 196 |
| Hindi (all words) | — | 0.13 % of 18 139 | — |

**Numbers.** Sentences containing digits: en-US PER 3.6 % (31 sentences), en-GB 4.3 %, Hindi 3.1 %
(22 sentences), against 1.1 %, 1.4 % and 0.08 % for the rest. The reference's number readings have
many idiosyncrasies (digit grouping, "hundred" years, Hindi stress on number words) that are only
partly reproduced.

**Word lists** (isolated words):

| List | PER | Exact | Exact ignoring stress | Stress pattern wrong |
| --- | ---: | ---: | ---: | ---: |
| en-US frequent, in CMUdict | 2.3 % | 89.6 % | 91.0 % | 3.3 % |
| en-US rare, in CMUdict | 18.0 % | 46.6 % | 49.0 % | 21.6 % |
| en-US not in CMUdict (1 997) | 19.1 % | 43.8 % | 48.0 % | 28.9 % |
| en-GB frequent | 2.6 % | 89.1 % | 90.5 % | 3.5 % |
| en-GB rare / not in CMUdict | 17.8 % / 18.6 % | 46.7 % / 44.7 % | 49.0 % / 49.2 % | 21.7 % / 29.5 % |
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
| `en_US-amy-low` | Whisper base (int8, sherpa-onnx) | 377 / 500 | 3.17 % | **2.97 %** | −0.21 [−0.52, +0.11] | 14 / 11 |
| `en_US-lessac-medium` | Whisper base | 377 / 500 | 4.30 % | **4.21 %** | −0.09 [−0.50, +0.30] | 16 / 12 |
| `en_GB-alba-medium` | Whisper base | 347 / 500 | **7.36 %** | 7.47 % | +0.11 [−0.55, +0.77] | 29 / 25 |
| `hi_IN-rohan-medium` | Whisper tiny, Hindi fine-tune (faster-whisper) | 495 / 500 | 8.95 % | **8.82 %** | −0.12 [−0.32, 0.00] | 2 / 0 |

Reading the table honestly:

- In every run the confidence interval contains zero, which is the claim:
  voices driven by uccharan are understood as well as when driven by the reference. "Better" is
  not claimed; the small negative differences are not significant.
- The recognisers are small (base and tiny models) because the machine has no GPU. Larger
  recognisers would have lower absolute WER; the comparison between the two columns is what
  matters.
- **History of the English amy-low run.** It was run twice. The first run (before three
  development-set name corrections: *Sami*, *Ziri*, *Mennad*) gave 3.13 % against 3.56 %
  (difference +0.44, CI [+0.07, +0.83]), i.e. a significant loss, mostly from those names. After
  the correction, which was found on the development set, the second run gave the numbers above.
  Both result files are kept (`tools/eval/results/e2e_en-us_en_US-amy-low.json` is the second; the
  first is quoted here).
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
| Piper id sequence vs ids built from the reference (compatible mode, reference-style numbers) | amy-low: 76.0 % of texts identical, id error rate 0.78 %; lessac: 76.1 %, 0.78 %; alba: 72.0 %, 0.92 %; rohan: 98.8 %, 0.09 % |
| sherpa-onnx `tokens.txt` coverage | same as the Piper maps above |
| Kokoro: all tokens inside the 114-symbol vocabulary, `[0, …, 0]` framing | yes for en-US, en-GB and Hindi |
| Kokoro Hindi vs reference through the published espeak → Kokoro mapping | 98.8 % of sentences identical, token error rate 0.13 % |
| Kokoro English vs the misaki dictionary (misaki 0.7.4 `us_gold.json`, 3 603 frequent words) | 69.5 % identical (74.4 % ignoring stress) — **not** a drop-in for Kokoro English |
| Kitten 0.8: built-in table equals the model's `tokens.txt`, ids `[0, …, 10, 0]` | yes; no symbol outside the table |
| Kitten ids vs the `phonemizer`-package front end Kitten uses | 75.1 % of texts identical, id error rate 1.35 % |
| Matcha-TTS | table and framing unit-tested; no Matcha model was available, **untested** against a model |
| `phonemizer` compatibility shim vs `phonemizer` 3.6 `EspeakBackend`, 1 000 held-out utterances | identical output: en-US 76.5 % (with stress), 83.4 % (plain), 69.9 % (with tie); en-GB 72.7 %; Hindi 98.3 % |

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
| English proper nouns | 20 | **60.0 %** | 45.0 % | 45.0 % | 55.0 % |
| English numbers, dates, units | 20 | 35.0 % | 30.0 % | **85.0 %** | **85.0 %** |
| Hindi schwa / nasal / nukta words | 36 | 61.1 % | 61.1 % | 61.1 % | **97.2 %** |
| Romanised Hindi (experimental) | 12 | 0 % | 0 % | 0 % | **58.3 %** |
| **All unseen** | 128 | 51.6 % | 47.7 % | 56.2 % | **80.5 %** |

"Compatible" uses reference-style number reading; "default" is the library default (compatible
phonology, natural numbers).

Where the reference still wins: **proper nouns** (12 against 11 of 20). The names list is short and
the letter-to-sound tables are weaker than the reference's spelling rules on foreign names
(*Sinead*, *Ciaran*, *Reykjavik*, *Wojciech*). Romanised Hindi is experimental and fails on
spellings outside its word list (*rehte*, *kripya*).

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
python tools/bench/compare.py --rounds 7 --langs en-us,en-gb,hi --out tools/bench/results/compare.json
cargo bench -p uccharan
node tools/bench/wasm_bench.mjs --out tools/bench/results/wasm.json
```

**Method.** Both libraries are loaded as shared libraries into one Python process through ctypes and
called once per sentence on the 3 000 held-out sentences, producing the same thing (an IPA string
per sentence, reference-style numbers, no punctuation). The ctypes call overhead (a few µs) is
included on both sides. Throughput is input words per second. Best of 7 rounds.
Start-up is measured in a fresh process: loading the library, initialising it and phonemizing one
sentence. Memory is resident-set growth in a fresh process.

**The machine.** 4 nominal vCPUs shared with other work. A control measurement taken in the same
run (N busy Python processes, total work relative to one) gave 1.0 / 1.88 / 2.15 for 1 / 2 / 4
processes: the machine could deliver a little over two cores' worth of work at most.

### Library, native (x86-64)

| | en-US uccharan | en-US reference | en-GB uccharan | en-GB reference | Hindi uccharan | Hindi reference |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Latency per sentence, p50 | **10.4 µs** | 53.2 µs | **11.4 µs** | 56.4 µs | **6.9 µs** | 14.8 µs |
| Latency per sentence, p99 | **37 µs** | 230 µs | **41 µs** | 217 µs | **27 µs** | 49 µs |
| Words/s, one thread | **677 k** | 128 k | **625 k** | 126 k | **783 k** | 366 k |
| Words/s, 2 workers | **1 404 k** | 202 k | **1 200 k** | 214 k | **1 805 k** | 528 k |
| Words/s, 4 workers | **1 706 k** | 202 k | **1 326 k** | 275 k | **1 744 k** | 609 k |
| Init + first sentence | **0.55 ms** | 4.3 ms | **0.52 ms** | 3.8 ms | **0.46 ms** | 4.9 ms |
| RSS after init | **2.2 MB** | 3.0 MB | **2.2 MB** | 3.0 MB | **1.4 MB** | 2.9 MB |
| RSS after 3 000 sentences | 3.8 MB | **3.6 MB** | 3.7 MB | **3.6 MB** | **2.6 MB** | 4.1 MB |
| Speed-up p50 / one thread / start-up | **5.1× / 5.3× / 7.8×** | | **5.0× / 5.0× / 7.3×** | | **2.1× / 2.1× / 10.6×** | |

Workers: uccharan runs one batch call (`uccharan_phonemize_lines`) on N native threads sharing one
handle; the reference library keeps a single global voice and cannot be called from several
threads, so it runs as N separate processes, the most favourable way to parallelise it. uccharan
scales with what the machine offers (en-US 2.2× on 4 workers, against the control's 2.15×); the
reference reaches 1.8× (en-US), 2.0× (en-GB) and 2.3× (Hindi). Scaling beyond two cores' worth
could not be measured on this machine.

Memory after a whole run is about the same for English (uccharan 0.1–0.2 MB more) and lower for
Hindi.

Rule path (made-up words, which only letter-to-sound rules can read; 1 500 sentences of 8 such
words, earlier run in the same harness): uccharan 69 k words/s, reference 38 k words/s (**1.8×**).

Criterion (`cargo bench -p uccharan`, in-process Rust, medians, earlier run on the busier
machine; [`criterion.json`](../tools/bench/results/criterion.json)):

| Benchmark | Throughput |
| --- | ---: |
| English running text, lexicon path (en-US / en-GB / en-US improved) | 1.14 M / 1.01 M / 0.80 M words/s |
| English made-up words, rule path | 74 k words/s |
| Hindi running text (compatible / improved) | 1.41 M / 1.23 M words/s |
| Text with numbers, money, dates | 0.57 M words/s |
| Rendering only: Piper strings / sherpa codepoints / Kokoro tokens | 34 M / 23 M / 1.2 M words/s |
| Building a `Phonemizer` from the embedded data | 14.5 µs |

### Size

| | uccharan | reference |
| --- | ---: | ---: |
| Native library | 11.2 MB (`libuccharan.so`, with all data inside: en-US and en-GB lexicons 2.0 MB each, misaki dictionaries for Kokoro 2.9 MB + 3.1 MB, Hindi 4 KB) | 0.66 MB library + a data directory |
| Data for these languages | included above (en-US 2.0 MB, en-GB 2.0 MB, Hindi 4 KB) | about 0.8 MB for English (dictionary 168 KB + shared phoneme data 0.66 MB), 18.4 MB for the full directory |
| Python wheel | 5.5 MB (abi3, all data) | — |
| WebAssembly core | **414 KiB** (155 KiB gzipped), no data | — |
| WebAssembly data | en-US 2.0 MiB (1.0 MiB gzipped), en-GB 2.0 MiB, Hindi 4 KiB, Kokoro English 2.9 / 3.1 MiB (1.3 / 1.4 MiB gzipped), each loaded on demand | — |

**uccharan is larger than the reference for English** (its lexicon holds 126 k CMUdict-derived
entries; the reference's English dictionary is a small exception list plus spelling rules). It is
much smaller for Hindi.

### WebAssembly (Node 22, same sentences, one call each)

| | en-US | en-GB | Hindi |
| --- | ---: | ---: | ---: |
| Latency p50 / p99 | 17 µs / 76 µs | 13 µs / 54 µs | 9 µs / 32 µs |
| Words/s | 386 k | 535 k | 679 k |
| Load data + first sentence | 14 ms | 5 ms | 1.3 ms |
| Module initialisation | 37 ms (once) | | |

A side-by-side run against the espeak-ng WebAssembly build published in the npm package
`phonemizer` was **not done** (that package was not installed in the measurement environment).

### Targets from the design notes

| Target | Result | Met? |
| --- | --- | --- |
| Lexicon path > 100 k words/s | 0.68 M (through ctypes) – 1.1 M (in Rust) words/s | yes |
| WebAssembly within 2–3× of native | 17 µs vs 10.4 µs p50 for en-US (1.6×) | yes |
| Per-language data < 5 MB | en 2.0 MB (+ 3 MB for Kokoro English), hi 4 KB | yes |
| WebAssembly core < 2 MB | 414 KiB | yes |
| en-US sentence PER ≤ 5 % vs reference | 1.19 % | yes |
| Hindi PER ≤ 5 % vs reference | 0.12 % | yes |
| End-to-end WER within +0.5 (en) / +1.0 (hi) on two Piper voices per language | en-US: two voices, −0.21 and −0.09; en-GB: one voice, +0.11 (interval up to +0.77); hi: one voice, −0.12 | en-US yes; en-GB and Hindi on one voice only |
| Zero out-of-table symbols | none, except U+0329 on 130-symbol Piper maps (shared with the reference) | yes, with that note |
| Faster than the reference on every axis | latency, throughput (one and several workers), start-up: yes. Memory: lower at start-up, about equal after a run for English. Size on disk for English: **no** | all speed axes yes; size no |

## 6. Kokoro

### English: agreement with misaki

```sh
python tools/eval/kokoro_eval.py --set heldout --out tools/eval/results   # with misaki installed
```

Reference: misaki 0.9.4 as Kokoro's own pipeline runs it (spaCy `en_core_web_sm` tagger, espeak-ng
fallback for words outside its dictionaries). uccharan: the Kokoro English front end with its built-in copy of
misaki 0.9.4's dictionaries (the same output, byte for byte, as when it reads misaki's JSON files:
checked on all 6 000 English evaluation sentences for both accents). "Token error" = edit distance over Kokoro tokens / reference length.

| Accent | Set | Identical sentences | Token error | Words in misaki's dictionaries | of which POS-dependent | Words misaki sends to espeak-ng |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| en-US | Tatoeba held-out (3 000) | **94.9 %** | 0.25 % | **99.96 %** of 20 251 | 98.5 % of 3 843 | 75.3 % of 372 |
| en-US | Sherlock Holmes (1 000) | 87.2 % | 0.29 % | 99.83 % of 14 502 | 98.8 % of 3 245 | 43.8 % of 130 |
| en-GB | Tatoeba held-out | **92.3 %** | 0.36 % | **99.96 %** of 20 240 | 98.4 % of 3 843 | 56.9 % of 383 |
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
with ctypes); misaki without its espeak-ng fallback (which would only slow it down):

| | misaki 0.9.4 (spaCy sm) | uccharan | |
| --- | ---: | ---: | ---: |
| Latency per sentence, p50 / p99 | 1 958 µs / 4 257 µs | **25.9 µs / 104 µs** | 76× |
| Words per second, one thread | 3.8 k | **259 k** | 68× |
| Start-up incl. dictionaries and first sentence | 1 452 ms | **0.6 ms** | |
| Memory after start-up | 204 MB | **2.5 MB** | |

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
