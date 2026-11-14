# Normalise-only mode

Some voices read text, not phonemes: LLM-based and character-based models such as Indic
Parler-TTS, IndicF5, Chatterbox, VoxCPM2, Veena, Svara or Maya. Most of them misread raw digits,
dates, currency and URLs, and none documents how it handles them. `normalize` gives such a voice
the **spoken form** of a text, with every change recorded as a typed span aligned to the input:

```text
in:   Pay ₹1,50,000 by 15/08/2025 at 10:30 बजे — www.example.com
out:  Pay one lakh fifty thousand rupees by …
```

It uses the same number, date and symbol readers as the phonemiser (English, Hindi and the
number words of every data-driven language), so a text voice and a phoneme voice say the same
thing.

## API

**Python**

```python
import uccharan

n = uccharan.normalize("Pay ₹500 by 10:30.", "en-us")
n.text        # 'Pay five hundred rupees by ten thirty.'
n.sentences   # [(0, 38)]
[(s.kind, s.text, s.spoken) for s in n.spans if s.kind != "punct"]
# [('text', 'Pay', 'Pay'), ('money', '₹500', 'five hundred rupees'),
#  ('text', 'by', 'by'), ('time', '10:30', 'ten thirty')]

uccharan.normalize("UPI से 12 students", "hi", latin="transliterate").text
# 'यू पी आई से बारह स्ट्यूडन्ट्स'
```

Offsets are code points (Python string indexes); pass `byte_offsets=True` for UTF-8 byte
offsets. Every span also carries `src_utf16` and `out_utf16`.

**JavaScript**

```js
import { normalize } from "uccharan";
const n = await normalize("Pay ₹500 by 10:30.", { lang: "en-us" });
n.spans[2]; // { kind: "money", start: 4, end: 8, outStart: 4, outEnd: 23, text: "₹500", ... }
```

`start`/`end` are UTF-16 offsets (JavaScript string indexes); `src.byte` and `src.char` give the
other two.

**Rust**

```rust
use uccharan::normalize_api::{normalize, NormOptions};
use uccharan::{Lang, Phonemizer};

let p = Phonemizer::new();
let n = normalize(&p, "Pay ₹500 by 10:30.", Lang::EnUs, &NormOptions::default());
assert_eq!(n.text, "Pay five hundred rupees by ten thirty.");
```

`Normalized::to_json(input)` gives the JSON form below; `normalize_json` takes the options as
JSON.

**C**

```c
char *json = uccharan_normalize(p, "Pay ₹500.", "hi", "{\"latin\":\"transliterate\"}");
/* {"text":"…","sentences":[…],"spans":[…]} */
uccharan_string_free(json);
```

**Command line**: `uccharan -l hi -p text "₹500"` prints the spoken form; `-p text-json` the JSON
with spans; `--norm-options '{"digits":"request"}'` sets options; `--wordlist FILE` applies a
[word list](wordlists.md).

### Result

```json
{
  "text": "Pay five hundred rupees.",
  "sentences": [{"byte": [0, 24], "char": [0, 24], "utf16": [0, 24]}],
  "spans": [
    {"kind": "text", "lang": "en-us", "src": {…}, "out": {…}, "text": "Pay", "spoken": "Pay"},
    {"kind": "punct", …, "text": " ", "spoken": " "},
    {"kind": "money", "lang": "en-us", "src": {"byte": [4, 10], "char": [4, 8], "utf16": [4, 8]},
     "out": {…}, "text": "₹500", "spoken": "five hundred rupees"},
    {"kind": "punct", …, "text": ".", "spoken": "."}
  ]
}
```

Guarantees, checked by the tests (`crates/uccharan/tests/normalize_api.rs`, including 1 200
fuzzed inputs):

- the spans tile the input: in order, without gaps or overlaps, every input byte in exactly one
  span;
- the spoken forms of the spans, in order, are the output text;
- `text`, `latin` and `punct` spans are copied unchanged (spacing and punctuation are kept);
- normalising the output again changes nothing;
- every `number` span reads back to the value of its digits (English and Hindi, sampled up to
  10⁷, with lakh/crore and international grouping);
- byte, code-point and UTF-16 offsets describe the same positions.

### Span kinds

| kind | example | kind | example |
| --- | --- | --- | --- |
| `number` | 12,000 | `phone` | +91 98765 43210, 555-123-4567 |
| `decimal` | 3.14 | `digits` | 007 |
| `ordinal` | 21st, 5वां | `range` | 10-20 |
| `year` | 1984, 1990s | `fraction` | 3/4 |
| `money` | ₹500, Rs. 250, ₹5 करोड़ | `measure` | 5km, 25°C |
| `percent` | 18% | `abbreviation` | Dr., e.g., डॉ. |
| `time` | 10:30, 9:05am | `acronym` | FBI (spelled) |
| `date` | 15/08/2025, 2024-01-05 | `symbol` | & |
| `url` | www.example.com | `email` | ram@gmail.com |
| `latin` | Latin word kept in Indic text | `transliterated` | Latin word respelled |
| `wordlist` | a word-list entry | `dropped` | text left out (emoji, a dropped URL) |
| `text` | an unchanged word | `punct` | spaces and punctuation |

Number spans carry `value` (the digits read).

## Options

| Option (Python / JSON) | Values | Default |
| --- | --- | --- |
| `digits` | `context`, `request`, or a language tag | `context`: the request language, except that a number next to a Latin-script word in Indic text is read in English ("कुल 12 students" → "कुल twelve students") |
| `en_scale` (`enScale` in JS) | `auto`, `indian`, `international` | `auto`: lakh and crore for numbers written with Indian grouping (`5,00,000`) and for English inside Indic text, else thousand/million |
| `latin` | `keep`, `transliterate`, `spell` | `keep`: Latin-script words stay in Latin, as Indian TTS models are trained; `transliterate` respells them in the request script ("office" → "ऑफ़िस", "UPI" → "यू पी आई") for voices without Latin letters; `spell` reads them letter by letter |
| `urls` | `read`, `keep`, `drop` | `read`: "w w w dot example dot com", "ram ऐट द रेट gmail डॉट com" |
| `improved` (`mode` in JSON) | bool | keeps known brand names whole ("iPhone") |
| `respell` | `native`, `latin-hyphen`, `latin-plain` | respelling style for word-list entries that have IPA but no spoken form |
| `wordlists` | word lists | applied first (see [word lists](wordlists.md)) |

## Readings per language

- **English**: cardinals to 10¹⁵, decimals, signs, ordinals, years and decades, `$ £ € ₹` amounts
  (also `Rs.`, `Rs`, `INR`), `bn`/`m`/`k` multipliers, times with am/pm, ISO and slash dates
  (month first in en-US, day first in en-GB), telephone numbers digit by digit, ranges, simple
  fractions, units, abbreviations, acronyms, URLs and email addresses.
- **Hindi** (and Bhojpuri, Maithili, Sanskrit): Indian grouping with हज़ार/लाख/करोड़/अरब, दशमलव,
  रुपये/पैसे, `रु.`, dates with month names, बजकर … मिनट times, the clock idioms before बजे
  (साढ़े दस, सवा चार, पौने पाँच), fractional idioms before a scale word (डेढ़ लाख, ढाई करोड़),
  10-digit mobile numbers and `+91` numbers digit by digit, ordinals with -वां/-वीं, Devanagari digits.
- **Other Indian languages** (bn, mr, ne, gu, pa, or, te, kn, ta, ml, as, kok, si): numbers in
  any of the Indic digit blocks or ASCII, with the language's number words, `%` and `₹`. Dates,
  times and phone numbers are read part by part. The number words are those the phonemiser uses,
  which follow how espeak-ng voices read them; for Tamil, Nepali and Bengali these are
  sometimes not the standard written forms (see the results below).

## Accuracy

Hand-written gold sets in `tools/eval/gold/norm/` (input, accepted spoken forms, class), split
into development and held-out halves when they were written, before the readers were tuned;
`tools/eval/norm_eval.py` scores them (sentence exact match after folding case, punctuation,
nukta and chandrabindu/anusvara). Held-out, scored once:

| Language | Sentences | Exact | WER |
| --- | ---: | ---: | ---: |
| English (en-US) | 45 | 45 | 0.0 % |
| Hindi | 42 | 42 | 0.0 % |
| Marathi | 10 | 9 | 5.0 % |
| Bengali | 10 | 9 | 7.7 % |
| Nepali | 10 | 8 | 15.4 % |
| Tamil | 10 | 5 | 53.3 % |

The English and Hindi sets were written by the same people as the readers, so they measure the
cases we thought of; the misses elsewhere are the number words noted above (Tamil compounds
such as இருபத்தைந்து and இரண்டாயிரத்து, Nepali long vowels, Marathi टक्के for %). Results:
`tools/eval/results/norm_{dev,heldout}.json`.
