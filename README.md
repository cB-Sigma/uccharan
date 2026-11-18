<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/assets/brand/banner-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="docs/assets/brand/banner-light.png">
    <img alt="uccharan · उच्चारण — phonemiser, text normaliser, transliterator and pronunciation engine" src="docs/assets/brand/banner-light.png" width="800">
  </picture>
</p>

<h3 align="center">Phonemiser, text normaliser, transliterator and pronunciation engine.</h3>

<p align="center">
  The text side of text-to-speech and speech recognition in one library: 44 languages,
  in Rust, Python, JavaScript and C. Apache-2.0, no GPL code or data.
</p>

<p align="center">
  <a href="https://github.com/cB-Sigma/uccharan/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/cB-Sigma/uccharan/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="Licence: Apache-2.0" src="https://img.shields.io/badge/licence-Apache--2.0-1f62b3"></a>
  <a href="https://pypi.org/project/uccharan/"><img alt="PyPI" src="https://img.shields.io/pypi/v/uccharan?color=1f62b3"></a>
  <a href="https://crates.io/crates/uccharan"><img alt="crates.io" src="https://img.shields.io/crates/v/uccharan?color=1f62b3"></a>
  <a href="https://www.npmjs.com/package/uccharan"><img alt="npm" src="https://img.shields.io/npm/v/uccharan?color=1f62b3"></a>
</p>

<p align="center">
  <a href="https://cb-sigma.github.io/uccharan/"><img alt="Try the live demo" src="docs/assets/brand/button-live-demo.png" width="257"></a>
</p>

<table>
<tr>
<td width="58%" valign="top">

**uccharan** (उच्चारण, "pronunciation") does the text work that sits in front of a neural voice
and behind a speech recogniser:

- **Phonemiser**: text into the phonemes that Piper, Kokoro, Kitten TTS, Matcha-TTS and
  sherpa-onnx voices were trained on, in 44 languages, measured against espeak-ng's output.
- **Text normaliser**: the spoken form of numbers, money, dates, times and URLs for voices that
  read plain text, with every change aligned to the input.
- **Transliterator**: romanised Hindi, Marathi, Bengali, Tamil, Telugu, Gujarati, Punjabi,
  Kannada and Malayalam (`aap kaise ho`) in the native script (`आप कैसे हो`).
- **Pronunciation engine**: your word lists applied to every kind of voice, names a recogniser
  misheard put back by sound, and tools to label and check data for training voices.

Written in Rust with no runtime dependencies, it replaces espeak-ng's text-to-phoneme step.
Every accuracy and speed figure here comes from a measurement described in
[docs/evaluation.md](docs/evaluation.md).

</td>
<td width="42%" valign="top">

**Install**

Python 3.9+

```sh
pip install uccharan
```

Rust

```sh
cargo add uccharan
```

JavaScript (browser, Node 18+)

```sh
npm install uccharan
```

Command line (macOS, Linux)

```sh
brew install cb-sigma/tap/uccharan
```

C: `libuccharan` + `uccharan.h` on each
[release](https://github.com/cB-Sigma/uccharan/releases).
[More install options →](#install)

</td>
</tr>
</table>

<p align="center">
  <img alt="The live demo: English, Hindi and Tamil read in compatible and then improved mode (a homograph, a name, rupee amounts and a clock time), the spoken form of a sentence with money and a date, and German phonemes in the espeak, Piper and sherpa-onnx profiles" src="docs/assets/demo/demo.gif" width="760">
</p>

## Contents

- [Install](#install)
- [Quick start](#quick-start)
- [Compatible or improved: choosing a mode](#compatible-or-improved-choosing-a-mode)
- [What is in it](#what-is-in-it)
- [Languages](#languages)
- [Output profiles](#output-profiles)
- [Normalise-only mode](#normalise-only-mode) · [Word lists](#pronunciation-word-lists) ·
  [Sound-alike matching](#sound-alike-matching) · [Transliteration](#transliteration) ·
  [Training tools](#training-tools)
- [Packs and their licences](#packs-and-their-licences)
- [Versions and stability](#versions-and-stability)
- [What is not tested](#what-is-not-tested)
- [Licence](#licence) · [Documentation](#documentation)

## Install

| | |
| --- | --- |
| **Python** 3.9+ (Linux, macOS, Windows) | `pip install uccharan` or `uv add uccharan`; installs the `uccharan` and `uccharan-train` commands too |
| **Rust** | `cargo add uccharan`; the command line with `cargo install uccharan` |
| **JavaScript** (browser, Node 18+) | `npm install uccharan` |
| **Homebrew** (macOS, Linux) | `brew install cb-sigma/tap/uccharan`: the command, `libuccharan` and `uccharan.h`, built from source |
| **C** | `libuccharan` and `uccharan.h` for Linux, macOS and Windows on each [release](https://github.com/cB-Sigma/uccharan/releases), or `cargo build --release -p uccharan-capi` |

<details>
<summary><b>Building against the C library with MSVC on Windows</b></summary>

In a Developer Command Prompt in the unpacked release folder (`hello.c`, `include\`, `lib\`):

```bat
:: against the DLL: link its import library, then keep uccharan.dll next to hello.exe (or on PATH)
cl /nologo /MD /Iinclude hello.c lib\uccharan.dll.lib /Fe:hello.exe
copy lib\uccharan.dll .

:: static: uccharan.lib and the system libraries Rust's standard library uses
cl /nologo /MD /Iinclude hello.c lib\uccharan.lib ntdll.lib synchronization.lib kernel32.lib ^
   userenv.lib ws2_32.lib dbghelp.lib /Fe:hello-static.exe
```

Link statically with `/MD`: `uccharan.lib` is built against the DLL C runtime. Without `ntdll.lib` and
`synchronization.lib` the static link fails with unresolved externals. Built from source, the
system libraries a static link needs are printed by
`cargo rustc --release -p uccharan-capi --lib --crate-type staticlib -- --print native-static-libs`
(the list above is from Rust 1.97, with `synchronization.lib` added).

</details>

**Language data.** English and Hindi are built in, and so is every language read by rules alone:
the other Indian languages, Bhojpuri, Maithili and Magahi (which use Hindi's data), Dogri, Bodo,
Santali, Manipuri and Kashmiri. The word tables of the European languages, Urdu and Sindhi are
small packs (46 KB to 0.66 MB) that the Python and npm packages download the first time the
language is used, check against a SHA-256 sum and cache. For offline machines:

```sh
python -m uccharan.packs                 # list the packs and which are installed
python -m uccharan.packs --dir DIR --all # download into DIR, then set UCCHARAN_DATA_DIR=DIR
```

In Rust, the `download` feature fetches packs on first use and `bundle` compiles every word table
in. See [Packs and their licences](#packs-and-their-licences).

<a id="thirty-seconds-of-each-function"></a>

## Quick start

```python
import uccharan

# 1. Phonemes for a voice
uccharan.phonemize("Hello, world.")                          # 'həlˈoʊ, wˈɜːld.'
uccharan.phonemize("नमस्ते दुनिया", "hi")                       # 'nəmˈʌsteː dˈʊnɪjˌaː'
uccharan.phonemize("Guten Morgen, wie geht es dir?", "de")   # 'ɡˈuːtən mˈɔɾɡən, viː ɡˈeːt ɛs dˈiːɾ?'
uccharan.phonemize("Hi there. Bye!", "en-us", "piper")       # ['hˈaɪ ðˈɛɹ.', 'bˈaɪ!']  one string per sentence
uccharan.phonemize("I read it yesterday.", improved=True)    # 'aɪ ɹˈɛd ɪt jˈɛstɚdˌeɪ.'  (see "choosing a mode")

# 2. The spoken form of a text, for voices that read text
uccharan.normalize("Rs. 250 on 3/4/2024", "en-us").text      # 'two hundred fifty rupees on March fourth twenty twenty four'
uccharan.normalize("₹5 करोड़ का सौदा 10:30 बजे", "hi").text     # 'पांच करोड़ रुपये का सौदा साढ़े दस बजे'

# 3. Romanised text into native script (optional CC BY-SA 4.0 pack, downloaded once)
uccharan.load_pack("xlit-hi")
uccharan.transliterate("mujhe kal office jana hai", "hi").text   # 'मुझे कल office जाना है'

# 4. Your own pronunciations, for every kind of voice
words = uccharan.WordList.from_entries([
    {"word": "Kokoro", "lang": "en", "ipa": "kəkˈɔːɹoʊ"},
    {"word": "SBI", "lang": "hi", "say": "एस बी आई", "match": "case"},
])
uccharan.phonemize_wordlists("Kokoro speaks", "en-us", wordlists=[words])   # 'kəkˈɔːɹoʊ spˈiːks'
uccharan.normalize("SBI में खाता", "hi", wordlists=[words]).text             # 'एस बी आई में खाता'

# 5. Names a speech recogniser misheard, put back by sound
uccharan.match_terms("send it to Coat Harry", ["Kothari"], "en-us")    # [Candidate(text='Coat Harry', term='Kothari', score=0.93, …)]
uccharan.SoundIndex(["Kothari", "Paytm"]).apply("send it to Coat Harry", "en-us").text   # 'send it to Kothari'
```

```sh
# The command line
uccharan -l hi -p piper "नमस्ते दुनिया।"
uccharan -l ta --improved "விலை ₹1,250"

# Training data for a voice: phonemes, symbol table and voice.json from a dataset manifest
uccharan-train label metadata.csv -l hi --improved -o labels/
```

Code written for `phonemizer` can switch with `from uccharan.compat import EspeakBackend`.

<details>
<summary><b>The same in Rust, JavaScript and C</b></summary>

```rust
use uccharan::normalize_api::{normalize, NormOptions};
use uccharan::soundalike::{ApplyOptions, IndexOptions, SoundIndex};
use uccharan::translit::{transliterate, TranslitOptions};
use uccharan::{Lang, Options, Phonemizer};

let p = Phonemizer::new();
assert_eq!(p.phonemize("Hello, world.", &Options::new(Lang::EnUs)), "həlˈoʊ, wˈɜːld.");

let n = normalize(&p, "₹5 करोड़ का सौदा 10:30 बजे", Lang::Hi, &NormOptions::default());
assert_eq!(n.text, "पांच करोड़ रुपये का सौदा साढ़े दस बजे");

p.load_pack(std::fs::read("xlit-hi.ucd")?)?;          // CC BY-SA 4.0 pack from the release page
let t = transliterate(&p, "aap kaise ho", Lang::Hi, &TranslitOptions::default());
assert_eq!(t.text, "आप कैसे हो");

let index = SoundIndex::from_terms(&p, &["Kothari", "Paytm"], &IndexOptions::default());
let fixed = index.apply(&p, "send it to Coat Harry", Lang::EnUs, &ApplyOptions::default());
assert_eq!(fixed.text, "send it to Kothari");
```

```js
import { phonemize, normalize, loadTranslit, transliterate, createSoundIndex } from "uccharan";

await phonemize("Hello, world.");                                          // "həlˈoʊ, wˈɜːld."
await phonemize("Hi there. Bye!", { profile: "piper" });                   // ["hˈaɪ ðˈɛɹ.", "bˈaɪ!"]
(await normalize("₹5 करोड़ का सौदा 10:30 बजे", { lang: "hi" })).text;        // "पांच करोड़ रुपये का सौदा साढ़े दस बजे"
await loadTranslit("hi", bytesOfXlitHiUcd);                                // CC BY-SA 4.0 pack from the release page
(await transliterate("aap kaise ho", { lang: "hi" })).text;                // "आप कैसे हो"
const index = await createSoundIndex(["Kothari", "Paytm"]);
index.apply("send it to Coat Harry", { lang: "en-us" }).text;              // "send it to Kothari"
```

```c
#include "uccharan.h"

UccharanPhonemizer *p = uccharan_new();
char *ipa = uccharan_phonemize(p, "Hello, world.", "en-us", "espeak-ipa", 0);    /* "həlˈoʊ, wˈɜːld." */
char *spoken = uccharan_normalize(p, "Rs. 250 on 3/4/2024", "en-us", NULL);      /* JSON: text and spans */
UccharanSoundIndex *names = uccharan_sound_index_new(p, "[\"Kothari\", \"Paytm\"]", NULL);
char *fixed = uccharan_match_terms(p, names, "send it to Coat Harry", "en-us", NULL);   /* JSON: text, changes */
uccharan_string_free(ipa); uccharan_string_free(spoken); uccharan_string_free(fixed);
uccharan_sound_index_free(names);
uccharan_free(p);
```

</details>

<a id="improved-mode-and-names"></a>

## Compatible or improved: choosing a mode

Every call that reads text takes a **mode**. The two modes answer different questions:

- **Compatible mode** (the default) answers *"what phonemes was this voice trained on?"* It
  reproduces espeak-ng's reading, mistakes included, because that is what the existing Piper,
  Kokoro, Kitten TTS, Matcha-TTS and sherpa-onnx voices learned. Feed a voice anything else and it
  can mispronounce or garble the sounds it never saw.
- **Improved mode** answers *"how is this actually said?"* It fixes what espeak-ng gets wrong:
  homographs in context, names, Hindi schwa deletion and nasals, the languages espeak-ng can only
  read with its Hindi voice, and number words in the South Indian languages.

### Which one to use

| You are… | Use |
| --- | --- |
| driving an existing voice (Piper, Kokoro, Kitten TTS, Matcha-TTS, sherpa-onnx models trained on espeak-ng phonemes) | **compatible** |
| replacing espeak-ng or `phonemizer` in a pipeline and want the same output | **compatible** |
| training a new voice, or fine-tuning one on new labels | **improved** (and drive that voice with improved mode from then on; `uccharan-train` records the mode in `voice.json`) |
| building pronunciation lexicons, scoring pronunciation, aligning speech, or matching names by sound | **improved** |
| feeding a text-reading voice (Parler-TTS, IndicF5, Chatterbox, LLM-based TTS) through `normalize` | either; improved also keeps brand names whole |

Rule of thumb: **use the mode the voice was trained with.** For a voice you did not train, that is
compatible mode.

### What changes

| | Compatible (espeak-ng's reading) | Improved |
| --- | --- | --- |
| English homographs | `I read it yesterday.` → `aɪ ɹˈiːd …` | `aɪ ɹˈɛd …`, from tense, time words and the words around it |
| Names and brands | `Siobhan`, `iPhone` → `ʃˈaʊbɑːn`, `aɪ fˈoʊn` | `ʃəvˈɔːn`, `ˈaɪfoʊn`, from a hand-written list first |
| Hindi schwa deletion | `लड़कपन` → `lˈʌr.əkˌəpən` | `lˈʌɽəkpən` |
| Hindi nasals and long vowels | `हिंदी` → `hˈĩdi`, `पानी` → `pˈaːni`, `गाँव` → `ɡˈãnʋ` | `hˈɪndiː`, `pˈaːniː`, `ɡˈãʋ` |
| Hindi flaps ड़ ढ़ | `r.` (an ASCII stand-in) | `ɽ` `ɽʰ` |
| English words in Hindi text | British English phones a Hindi voice never saw (`ɒ`, `ɹ`, `w`) | mapped into the Hindi phone set |
| Romanised Hindi (`main kal office jaunga`) | read as English | read as Hindi with `hinglish=True` (experimental) |
| Bhojpuri, Maithili, Magahi, Dogri, Bodo, Sanskrit | read as Hindi (espeak-ng has no voice for them) | read as themselves, from each language's own rules |
| Tamil, Kannada, Telugu, Malayalam numbers | `₹1,250` (Tamil) → *ஒன்று ஆயிரம் இரண்டு நூறு ஐம்பது*, the rupee dropped | *ஆயிரத்து இருநூற்று ஐம்பது ரூபாய்*, written number words |
| The rupee sign in the Indian languages | dropped | said |

Santali, Manipuri and Kashmiri read the same in both modes: espeak-ng cannot read them, so there is
no reference to be compatible with. The full list per language is in
[docs/improved-mode.md](docs/improved-mode.md).

**How much better.** On hand-written sets whose expected pronunciations come from dictionaries and
standard phonology (never from a phonemiser), held out from the work on the rules:

| Held-out set | espeak-ng 1.52 | uccharan compatible | uccharan improved |
| --- | ---: | ---: | ---: |
| English homographs, proper nouns, numbers; Hindi schwa and nasals; romanised Hindi (128 items) | 51.6 % | 48.4 % | **82.0 %** |
| Personal names of many origins (172) | 61.6 % | 80.2 % | **91.3 %** |
| — Indian names (48) | 43.8 % | 50.0 % | **79.2 %** |
| — names not in CMUdict (41) | 34.1 % | 19.5 % | **68.3 %** |
| African names (195) | 45.6 % | 41.5 % | **75.9 %** |
| Bhojpuri words (224) | 62.9 % | 63.8 % | **96.4 %** |
| Maithili words (226) | 68.6 % | 66.4 % | **93.4 %** |
| Magahi words (270) | 64.4 % | 70.0 % | **98.5 %** |
| Dogri words (70) | 44.3 % | 44.3 % | **97.1 %** |
| Bodo words (79) | 2.5 % | 2.5 % | **98.7 %** |

For names that are in CMUdict the reference *is* CMUdict, which uccharan's lexicon comes from, so
the fair name rows are Indian names and names not in CMUdict. The Bhojpuri, Maithili, Magahi, Dogri
and Bodo sets mostly encode grammars' descriptions as written down for this project: they show
that improved mode implements them, not that a native speaker would accept every form
([items to check](docs/bhojpuri-maithili-review.md)). More sets (North African, Egyptian,
Chichewa, Shona, Tsonga and Tigrinya names; Santali, Manipuri and Kashmiri words) are in
[docs/evaluation.md](docs/evaluation.md#4-correctness-improved-mode-against-hand-curated-gold-sets).

### Setting the mode

```python
uccharan.phonemize(text, "ta", improved=True)
uccharan.phonemize_batch(lines, "ta", improved=True)
uccharan.normalize(text, "ta", improved=True)
uccharan.phonemize("main kal office jaunga", "hi", improved=True, hinglish=True)
```

```js
await phonemize(text, { lang: "ta", improved: true });
await normalize(text, { lang: "ta", improved: true });
```

```rust
use uccharan::{Lang, Mode, Options, Phonemizer};
let mut opt = Options::new(Lang::Ta);   // Mode::Compatible by default
opt.mode = Mode::Improved;
let ipa = Phonemizer::new().phonemize(text, &opt);
```

```c
uccharan_phonemize(p, text, "ta", "espeak-ipa", UCCHARAN_IMPROVED);   /* | UCCHARAN_HINGLISH */
uccharan_normalize(p, text, "ta", "{\"mode\": \"improved\"}");
```

```sh
uccharan -l ta --improved "விலை ₹1,250"
uccharan-train label metadata.csv -l hi --improved -o labels/
```

### Number reading is a separate switch

How digits, money, dates and times are read is chosen by `normalize`, independently of the mode:

| `normalize=` | Reads `$5 on 3/4/2024` (en-us) as | Use it for |
| --- | --- | --- |
| `"natural"` (default) | *five dollars on March fourth twenty twenty four* | speech: what a person would say |
| `"espeak"` | *dollar five on three slash four slash two thousand twenty four* | reproducing espeak-ng exactly, number for number |
| `"off"` | *on* (digits and symbols are left out) | text that is already written out |

The languages read by their own script rules (Bengali, Tamil, Telugu, Urdu and the rest outside
the Hindi family) read numbers as espeak-ng does in compatible mode, even with `"natural"`;
improved mode uses each language's own number words.

## What is in it

| | What it does | Guide |
| --- | --- | --- |
| [Phonemiser](#languages) | text to phonemes in 44 languages, in the shape each voice family expects | [profiles](docs/profiles.md), [languages](docs/languages.md) |
| [Normalise-only mode](#normalise-only-mode) | the spoken form of a text, with typed spans, for voices that read text | [normalize](docs/normalize.md) |
| [Pronunciation word lists](#pronunciation-word-lists) | your pronunciations as TSV, PLS or JSON, applied to phoneme and text voices alike | [word lists](docs/wordlists.md) |
| [Sound-alike matching](#sound-alike-matching) | corrects a recogniser's transcript against your vocabulary by sound | [soundalike](docs/soundalike.md) |
| [Transliteration](#transliteration) | romanised text of nine Indian languages into native script | [transliteration](docs/transliteration.md) |
| [Training tools](#training-tools) | labels datasets, picks reading scripts, reports coverage, scores pronunciation | [training](docs/training.md) |
| [Improved mode](#compatible-or-improved-choosing-a-mode) | pronunciations more correct than the espeak-ng reading, for voices you train yourself | [improved mode](docs/improved-mode.md) |
| [Packs](#packs-and-their-licences) | optional data under its own licence, downloaded only on request | [data licences](DATA_LICENSES.md) |

<a id="phonemiser-and-languages"></a>

## Languages

Most open voices were trained on phonemes from espeak-ng, which is GPL-licensed; sherpa-onnx 2.0
removed espeak-ng for that reason and expects the caller to bring phonemes. uccharan replaces only
espeak-ng's text-to-phoneme front end; the neural voice still does the speaking.

| Group | Languages (tag) |
| --- | --- |
| English | American `en-us`, British (RP) `en-gb` |
| Hindi and its neighbours | Hindi `hi` (with English mixed in), Bhojpuri `bho`, Maithili `mai`, Magahi `mag`, Sanskrit `sa`, Dogri `doi` |
| Other South Asian | Bengali `bn`, Marathi `mr`, Gujarati `gu`, Punjabi (Gurmukhi) `pa`, Odia `or`, Assamese `as`, Nepali `ne`, Konkani `kok`, Sinhala `si`, Urdu `ur`, Sindhi `sd`, Kashmiri `ks` (Perso-Arabic, Devanagari), Bodo `brx`, Santali `sat` (Ol Chiki), Manipuri `mni` (Meetei Mayek, Bengali script) |
| South Indian | Tamil `ta`, Telugu `te`, Kannada `kn`, Malayalam `ml` |
| European | Spanish `es`, German `de`, Dutch `nl`, French `fr` (also `fr-be`, `fr-ch`), Italian `it`, Portuguese `pt-br` and `pt`, Polish `pl`, Turkish `tr`, Czech `cs`, Swedish `sv`, Catalan `ca`, Romanian `ro`, Hungarian `hu`, Finnish `fi`, Norwegian Bokmål `nb`, Danish `da` |

Text in another script inside a sentence is read by that script's language, so Hindi with English
words, or Bengali inside Hindi, needs no splitting.

**Agreement with espeak-ng** (compatible mode, phoneme error rate on held-out sentences never used
to write rules or build tables) and **speed** (one thread, both called as libraries):

| | Phoneme error rate | Speed against espeak-ng |
| --- | ---: | ---: |
| English (US, British) | 1.16 %, 1.39 % (3,000 sentences) | 5.3×, 5.2× |
| Hindi | 0.09 % (3,000 sentences) | 2.8× |
| 21 other South Asian languages | 0.20 % (Konkani) to 7.21 % (Assamese); 12 of them under 1 % | 1.9× to 37× |
| 19 European languages and accents | 0.53 % (Italian) to 1.95 % (Danish) | 1.7× to 27.1× |

In absolute terms American English takes 11.8 µs per sentence against espeak-ng's 62.1 µs (p50),
580 k words per second on one thread against 114 k, and starts in 0.65 ms against 4.0 ms. Where
uccharan does **not** win is size: its library code is larger than espeak-ng's, and its English
data compresses worse (604 KB against 380 KB with xz).

End to end (voice plus speech recogniser, 200 to 500 held-out sentences per voice) the word error
rate with uccharan's phonemes is within the confidence interval of espeak-ng's for every voice
tried: eight Piper voices (English, Hindi, Spanish, German, French, Italian) and one Kokoro speaker
each for American English, British English and Hindi. Kokoro English goes through a built-in port
of misaki, the front end Kokoro was trained on, which gives the same tokens as misaki for 94.9 % of
held-out sentences at 21 µs per sentence against misaki's 1.88 ms.

The table for every language (data set, error rate with and without digits, speed, and which open
voices use those phonemes), how to read it, and per-language notes are in
[docs/languages.md](docs/languages.md); the methods are in [docs/evaluation.md](docs/evaluation.md);
to add a language, see [docs/adding-a-language.md](docs/adding-a-language.md).

## Output profiles

Each voice family wants phonemes in a different shape. Pick it with the profile argument (`-p` on
the command line):

| Profile | Gives you | For |
| --- | --- | --- |
| `espeak-ipa` (default) | IPA string, words separated by spaces | anything that used `phonemizer`'s espeak backend |
| `piper` | one string per sentence; `piper_ids()` maps it through a voice's `phoneme_id_map` | Piper voices |
| `sherpa-onnx` | one codepoint list per sentence | sherpa-onnx ≥ 2.0 `phoneme_codepoints` |
| `kokoro` | Kokoro tokens; `kokoro_ids()` | Kokoro-82M (English through the built-in misaki port) |
| `kitten`, `matcha` | phoneme string; `kitten_ids()`, `matcha_ids()` | Kitten TTS 0.8, Matcha-TTS |

Integration guides: [sherpa-onnx](docs/integrations/sherpa-onnx.md),
[Piper](docs/integrations/piper.md), [Kokoro](docs/integrations/kokoro.md),
[bolo](docs/integrations/bolo.md). Details of each profile: [docs/profiles.md](docs/profiles.md).

## Normalise-only mode

Voices that read text instead of phonemes (Indic Parler-TTS, IndicF5, Chatterbox, VoxCPM2, Veena
and other LLM-based models) still need numbers, money, dates, times, phone numbers and URLs
written out. `normalize` returns that spoken form with typed spans (number, money, date, time,
phone, URL, Latin word in Indic text, …) aligned to the input in byte, code-point and UTF-16
offsets. Options choose which language reads digits, lakh/crore or international grouping in
English, whether Latin words are kept, respelled in the script or spelled out, and whether URLs
are read.

```python
uccharan.normalize("UPI से भुगतान", "hi", latin="transliterate").text   # 'यू पी आई से भुगतान'
```

It uses the same number, date and currency readers as the phonemiser, so a text voice and a
phoneme voice say the same thing. See [docs/normalize.md](docs/normalize.md).

## Pronunciation word lists

One list of your names, brands and jargon serves every voice: phoneme voices get an entry's IPA
converted to their phone set, text voices get its spoken form, or a respelling chosen by reading
candidates back with uccharan's own phonemiser. Lists are tab-separated text with the columns
`word lang ipa say match note`, or JSON, or W3C PLS (import and export); matching is by whole
word, by case, by exact substring or by regular expression. See
[docs/wordlists.md](docs/wordlists.md).

## Sound-alike matching

A recogniser writes a name it does not know as the nearest words it does know: "Kothari" as
*Coat Harry*, "Paytm" as *pay tm*. `match_terms` and `SoundIndex` compare a transcript with your
vocabulary as phonemes and put the terms back. Close sounds are cheap to exchange (voicing,
aspiration, vowel length, retroflex / dental), unrelated ones are not, and because everything is
phonemes a Latin-script vocabulary matches a Devanagari transcript and the other way round.
Every change is reported; none is silent.

Held out, with 651 terms (Indian names, brands, technical and domain terms) in English, Hindi and
Hinglish sentences spoken by synthetic voices and transcribed by small recognisers: at the default
thresholds 43.9 % of the terms the recognisers got wrong are put back (64.2 % in English
sentences), 97.8 % of the corrections are right, and 2 of 775 sentences without any term are
changed. Character edit distance puts back 8.8 %; Metaphone codes 18.7 % while changing 38 % of the
term-free sentences. The speech in this test is synthetic; human speakers and larger recognisers
were not measured. See [docs/soundalike.md](docs/soundalike.md) and
[the evaluation](docs/evaluation.md#7-sound-alike-matching).

## Transliteration

People type `aap kaise ho` or `mujhe kal office jana hai`; voices and recognisers do better with
`आप कैसे हो` and `मुझे कल office जाना है`. `transliterate` writes the romanised words of **Hindi,
Marathi, Bengali, Tamil, Telugu, Gujarati, Punjabi, Kannada or Malayalam** in the language's
script, leaves English words as English (or respells them in the script on request), and returns
spans aligned to the source with the alternatives of each word. There is no neural network to
run: a word it has never seen takes about a third of a millisecond.

- **Hindi mixed with English** (164 held-out chat sentences): 95.9 % of the Hindi words written
  correctly on the first run, 98.2 % with the pack that ships (after a fix prompted by that run, so
  optimistic); 7.7 % of English words converted by mistake.
- **Single words**, first candidate right, against the figures published for AI4Bharat's
  IndicXlit: ahead on the Dakshina test words in all nine languages (by 3.6 to 11.6 points),
  behind on Aksharantar's native words in eight of nine (by 0.5 to 7.2 points), and mixed on named
  entities.

The data is one optional pack per language under CC BY-SA 4.0 (see
[Packs](#packs-and-their-licences)); without a pack the function converts by rule, far less
accurately (72.4 % of the Hindi words above). Full tables and caveats:
[docs/transliteration.md](docs/transliteration.md),
[the evaluation](docs/evaluation.md#8-romanised-text-into-native-script).

## Training tools

`uccharan.train` (Python) and the `uccharan-train` command are for people who train voices and
check pronunciation. Nothing in them reads audio.

```sh
uccharan-train label metadata.csv -l hi --improved -o labels/        # phonemes for Piper, Matcha-TTS, icefall
uccharan-train select corpus.txt -l bho --improved -n 500 -o script/ # what to record
uccharan-train coverage labels/labels.jsonl -l hi --reference corpus.txt --alignments aligned/
uccharan-train score -l hi --reference "मेरा घोड़ा तेज़ है" --heard "मेरा कीड़ा तेज़ है"
uccharan-train mfa-dict corpus.txt -l bho -o bho.dict                # dictionary for a forced aligner
```

- **Labelling** turns a manifest (LJSpeech `metadata.csv`, JSON lines, Hugging Face rows) into
  Piper's `--data.phoneme_type text` CSV, a phoneme-id CSV and id map, Matcha-TTS filelists,
  `tokens.txt` and token lists for icefall-style recipes, and a `voice.json` that records the
  uccharan version, mode, profile and language the voice must be driven with. Symbols outside a
  target table are reported, and each utterance gets warnings for unknown characters, unread
  numbers and mixed scripts. Word lists apply.
- **Script selection** picks the few sentences that cover every phone and as many diphones as
  possible.
- **Coverage** counts phones and diphones, lists what a dataset lacks against a reference
  corpus, gives the duration per phone from alignments, and names outlier utterances.
- **Scoring** aligns a reference with a recogniser's text or a phone recogniser's output using
  the weighted phoneme distance of the sound-alike matcher, and reports the error rate, a score
  per word, and the words that were mispronounced, missing or extra.

The file layouts were checked against the Piper, Matcha-TTS, icefall and Montreal Forced Aligner
sources and documentation; **no voice has been trained with them yet**. See
[docs/training.md](docs/training.md) and the [example](examples/train).

## Packs and their licences

The packages (crate, wheel, npm package, C library) are Apache-2.0 and carry only data that is
Apache-2.0 or BSD-licensed. Data under a **share-alike** licence is kept out of them: it ships as
separate files attached to each [GitHub release](https://github.com/cB-Sigma/uccharan/releases),
and nothing downloads it unless you ask.

| Pack | What it is | Size | Licence | In the packages? |
| --- | --- | ---: | --- | --- |
| `en-us`, `en-gb`, `hi` | English lexicons and letter-to-sound models, Hindi exceptions | 0.7 MB, 0.7 MB, 3 KB | Apache-2.0 (includes CMUdict, BSD-2-Clause) | yes (the wheel fetches `en-gb` on first use) |
| `de`, `nl`, `fr`, `it`, `pt-br`, `pt`, `pl`, `tr`, `cs`, `sv`, `ca`, `ro`, `hu`, `fi`, `nb`, `da`, `ur`, `sd` | word table of the language: a letter-to-sound model and its exceptions | 46 KB to 0.66 MB each, 4.2 MB together | Apache-2.0 | **fetched the first time the language is used** (wheel, npm package; crate with the `download` feature), or compiled in with the crate's `bundle` feature |
| `kokoro-en-us`, `kokoro-en-gb` | misaki 0.9.4's English dictionaries, for the `kokoro` profile | 1.9 and 2.1 MB | Apache-2.0 | crate and C library yes; wheel and npm fetch them on first use |
| `wikipron-en-us`, `wikipron-en-gb`, `wikipron-hi` | Wiktionary pronunciations (80.8 k, 81.3 k and 30.4 k words) for improved mode's words outside the built-in lexicon | 0.3 to 0.7 MB | **CC BY-SA 4.0** (Wiktionary contributors, via WikiPron) | **no**: on request only |
| `xlit-hi`, `xlit-mr`, `xlit-bn`, `xlit-ta`, `xlit-te`, `xlit-gu`, `xlit-pa`, `xlit-kn`, `xlit-ml` | transliteration model, spelling lexicon and word counts per language | 3.1 to 8.7 MB, 60 MB together | **CC BY-SA 4.0** (trained on the Dakshina and Aksharantar word pairs; counts from corpora that include Wikipedia text) | **no**: on request only |

```python
uccharan.load_pack("xlit-hi")              # downloads from the release of the installed version, checks SHA-256, caches
uccharan.load_pack("/path/to/xlit-hi.ucd") # or a file you fetched yourself
```

Downloads are checked against SHA-256 sums built into the package and cached in
`~/.cache/uccharan/<version>/`. A language is never read without its word table: when the pack
cannot be had, Python raises `PackError`, the C calls return NULL with the reason in
`uccharan_last_error()`, Rust's `Phonemizer::ensure` returns the error, and the synchronous
JavaScript calls throw; each message names the pack and its URL.

Offline: fetch the files once (`python -m uccharan.packs --dir DIR de fr ur`, or `--all`, or take
them from the release page), name the directory in `UCCHARAN_DATA_DIR` and set
`UCCHARAN_OFFLINE=1`. In Rust, `features = ["bundle"]` compiles every word table in and
`features = ["download"]` fetches on first use; without either, the tables are read from
`UCCHARAN_DATA_DIR` or the cache. In Rust and C, `load_pack` also takes a file's bytes; in
JavaScript, the npm package carries the files and loads each on first use, and `loadTranslit` and
`loadWikipron` take the bytes or read the file from the data directory.
Compatible mode never uses the WikiPron packs. Every file, its source and its licence are listed
in [DATA_LICENSES.md](DATA_LICENSES.md).

## Versions and stability

uccharan follows [semantic versioning](https://semver.org/) from 1.0.0:

- **Compatible mode's output** changes within a major version only to match espeak-ng's reading
  more closely, so a voice keeps getting the phonemes it was trained on. Any other change to it is
  a new major version.
- **Improved mode and normalise-only mode** get more correct in minor versions. A voice trained on
  improved-mode labels should pin the version it was trained with; `uccharan-train label` writes
  it into `voice.json`.
- **The Python, JavaScript, Rust and C interfaces** change incompatibly only in a major version.

Changes are listed in [CHANGELOG.md](CHANGELOG.md).

## What is not tested

- Voices of the languages other than English, Hindi, Spanish, German, French and Italian end to
  end (Telugu, Marathi, Bengali and Urdu had no offline voice or recogniser that works); en-GB and
  Hindi on more than one Piper voice each; Kokoro on more than one speaker per language.
- Kitten TTS and Matcha-TTS end to end (only symbol tables and framing are checked).
- The `piper` command-line program (it embeds espeak-ng and has no hook) and running Piper voices
  directly with onnxruntime.
- A human listening test, and review by native speakers of most languages. The transliteration
  lists for eight of the nine languages are unreviewed.
- The choice between English and the language in transliteration, for any language but Hindi.
- Sound-alike matching on human speech and with large recognisers.
- Training a voice from the files `uccharan-train label` writes, an aligner run with the exported
  dictionary, and pronunciation scores against human judgements.
- Multi-core scaling beyond about two cores, and speed on any machine but the shared one the
  figures come from.
- Side-by-side speed against espeak-ng's WebAssembly build.
- macOS beyond the test suite, which runs there on every change. Windows was also run on one
  Windows 11 laptop (Ryzen 7 5800H): the Python and Rust tests passed, the command line read UTF-16
  input from Notepad and PowerShell, and the C library linked with MSVC; Hindi took 8.7 µs p50 per
  sentence there against 5.5 µs on the Linux machine ([evaluation](docs/evaluation.md)). Agreement
  figures come from Linux only.

## Licence

Apache-2.0. Includes data from the CMU Pronouncing Dictionary (BSD-2-Clause) and misaki
(Apache-2.0); see [NOTICE](NOTICE). [DATA_LICENSES.md](DATA_LICENSES.md) lists every data file
with its source and licence. The optional `wikipron-*` and `xlit-*` packs are CC BY-SA 4.0; they
are not part of any package and are loaded only on request.

uccharan contains no espeak-ng code, rules or dictionary data. The Kokoro English front end is a
port of misaki's Apache-2.0 code and ships misaki 0.9.4's English dictionaries unchanged under
their upstream Apache-2.0 licence statement; upstream does not document where the dictionary
entries originally come from (details in [DATA_LICENSES.md](DATA_LICENSES.md)).

## Documentation

[Languages](docs/languages.md) · [Profiles](docs/profiles.md) ·
[Evaluation](docs/evaluation.md) · [Improved mode](docs/improved-mode.md) ·
[Normalise-only mode](docs/normalize.md) · [Word lists](docs/wordlists.md) ·
[Sound-alike matching](docs/soundalike.md) ·
[Transliteration](docs/transliteration.md) ·
[Training voices and checking pronunciation](docs/training.md) ·
[Architecture](docs/architecture.md) · [Adding a language](docs/adding-a-language.md) ·
[How it compares with other phonemisers](docs/comparison.md) ·
Integrations: [sherpa-onnx](docs/integrations/sherpa-onnx.md), [Piper](docs/integrations/piper.md),
[Kokoro](docs/integrations/kokoro.md), [bolo](docs/integrations/bolo.md) ·
[Contributing](CONTRIBUTING.md) · [Changelog](CHANGELOG.md) · [Security](SECURITY.md)

uccharan is a sibling of [bolo](https://github.com/cB-Sigma/bolo-speech). Made by Abhinav Gautam.
