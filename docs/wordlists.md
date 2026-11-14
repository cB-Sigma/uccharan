# Pronunciation word lists

One word list serves every kind of voice:

- **phoneme voices** (Piper, Kokoro, Kitten, Matcha, sherpa-onnx) get the entry's IPA, converted
  to the voice's phone set like any other word;
- **text voices** (normalise-only mode, see [normalize.md](normalize.md)) get the entry's spoken
  form, or a respelling generated from its IPA.

## Format

Tab-separated text, UTF-8. Lines starting with `#` are comments, except the two headers shown.
The column header row is optional (without it the columns come in this order) and may list the
columns in any order.

```text
# uccharan wordlist 1
# lang: hi
# alphabet: ipa
word	lang	ipa	say	match	note
Kokoro	en	kəkˈɔːɹoʊ			product name
SBI	hi		एस बी आई	case
Ishaan	hi	iːʃaːn	ईशान
UPI	*		यू पी आई	case	all Indic voices
2G	en		two G	exact
New York	en	nuː jˈɔːk
MH\d{1,2}	*		maharashtra	regex
```

| Column | Meaning |
| --- | --- |
| `word` | The written form: a word or a phrase (whitespace in it matches any run of whitespace). For `regex` rows, the pattern. |
| `lang` | The language the entry applies in; `*` for any; empty for the `# lang:` default. An English entry also applies to Latin-script words inside other languages' text; a Hindi entry also to Bhojpuri and Maithili. |
| `ipa` | Phonemes in uccharan's espeak-style IPA (as `uccharan -l <lang> <word>` prints them, with stress marks). Optional. |
| `say` | The spoken form for text voices (PLS *alias*). Optional. |
| `match` | `word` (default): whole words, any case. `case`: whole words, same case. `exact`: any substring, same case (for tokens such as `2G` inside `2G/3G`). `regex`: see below. |
| `note` | Free text. |

Word boundaries: a letter, digit or Indic sign next to the match prevents it, except at a change
between Latin and another script (`मैंSBI` matches `SBI`); the danda is a boundary. Precomposed
and decomposed nukta letters match each other.

**Regular expressions** (`regex` rows): literals, `.`, classes `[a-z]` `[^…]`, `\d \w \s \b`
and their negations, groups `( )` `(?: )`, `|`, `* + ? {m} {m,} {m,n}` (add `?` for lazy), `^ $`,
and a leading `(?i)` for any case. A pattern that backtracks too much stops matching instead
of hanging.

Errors carry the line number (`line 4: unknown language "xx"`). Rows with neither `ipa` nor `say`,
and `say` values in a script other than the language's, are reported as warnings.

The same entries can be given as JSON (`[{"word": "Kokoro", "lang": "en", "ipa": "…"}]`), and
`parse_any` / `WordList(text)` accepts TSV, PLS or JSON by content.

## PLS (W3C Pronunciation Lexicon 1.0)

Import: `alphabet="ipa"` lexicons; every `<grapheme>` of a `<lexeme>` becomes an entry with the
lexeme's preferred (or first) `<phoneme>` and first `<alias>`; `xml:lang` sets the language.
Other alphabets are rejected.

Export: PLS holds one language per document, so a list is exported per language
(`WordList::languages()`, then `to_pls(lang)`); entries for any language are included in each.
Match rules and notes, which PLS has no field for, go into a comment in the lexeme that import
reads back, so TSV → PLS → TSV keeps word, language, IPA, spoken form, match rule and note.
`regex` rows are not exported.

## Applying lists

Lists are given in order, highest priority first. For any stretch of text:

1. an earlier list wins over a later one wherever their matches overlap, even when the later
   match is longer (so a single-word entry in a request can override a phrase in a stored list);
2. within one list, the leftmost-longest match wins;
3. for the same span in one list, an entry for the request language beats one for any language,
   which beats an English entry applied to a Latin word inside another language;
4. lists apply before number reading, so `2G`, `MH12` or `UPI` entries win over the normaliser.

Within one entry: a text voice uses `say` when present (respelled into the request script with
`latin="transliterate"` when it is Latin), else a respelling of `ipa`; a phoneme voice uses `ipa`
when present, else the phonemes of `say`.

**Python**

```python
import uccharan

wl = uccharan.WordList.from_file("names.tsv")          # TSV, PLS or JSON
uccharan.phonemize_wordlists("Kokoro", "en-us", profile="kokoro", wordlists=[wl])
uccharan.normalize("SBI में Ishaan", "hi", wordlists=[wl]).text   # 'एस बी आई में ईशान'
wl.to_pls("en-us")                                        # PLS text
inline = uccharan.WordList.from_entries([{"word": "York", "ipa": "jˈɔːk"}])
uccharan.normalize("New York", "en-us", wordlists=[inline, wl])  # inline wins
```

**JavaScript**: `phonemize(text, { wordlists: [tsvText] })`, `normalize(text, { wordlists })`,
`convertWordlist(text, "pls", "en-us")`.

**Rust**: `uccharan::wordlist::WordList::parse_tsv` / `parse_pls` / `parse_any`,
`wordlist::select` (the priority rules above), `normalize_api::render_with_wordlists` and
`NormOptions::wordlists`.

**C**: `uccharan_phonemize_wordlists(p, text, lang, profile, flags, "[\"<tsv>\"]")`,
`uccharan_normalize` with `"wordlists"` in the options, `uccharan_wordlist_convert`.

**Command line**: `uccharan --wordlist names.tsv -p kokoro "Kokoro"`.

## Respelling

An entry with IPA but no spoken form is respelled for text voices
(`uccharan::respell::respell`, `uccharan.respell(ipa, lang, source=…)`):

- **Indian scripts** (Devanagari languages, Bengali, Assamese, Gujarati, Gurmukhi, Odia, Telugu,
  Kannada, Malayalam, Tamil): a writer produces aksharas with choices at every point where a
  reader could go wrong (virama or inherent vowel in clusters and at word end, anusvara or nasal
  consonant, nukta letters such as ज़ फ़ or their plain forms, ऑ/ऐ or ओ/ए for English vowels);
  other scripts use the Devanagari candidate moved to their block, with the letters they lack
  replaced (Tamil writes one stop per place of articulation). Sinhala has no writer yet.
- **English**: `latin-plain` (one lower-case word, for voices that lower-case text or pause at
  hyphens) or `latin-hyphen` (syllables with the stressed one in capitals, "kuh-KAW-roh").

Every candidate is **read back with uccharan's own phonemiser** in the target language and the
one closest to the entry's IPA wins (phoneme error rate after folding stress, length and close
allophones, and mapping vowels a script cannot write to the nearest it can, such as schwa in
Telugu). The result reports that reading and its distance. uccharan's reading stands in for the
voice's: a respelling uccharan reads correctly is a good bet, not a guarantee. Examples:
`kəkˈɔːɹoʊ` → ककॉरो (hi), `ɪʃˈɑːn` → इशान (hi), `kəmpjˈuːtəɹ` → कम्प्यूटर (hi).

Latin-script words in Indic text are respelled the same way with `latin="transliterate"`: the
word's British reading with the r of the American one (Indian English is rhotic), so "office"
becomes ऑफ़िस and "computer" कम्प्यूटर.
