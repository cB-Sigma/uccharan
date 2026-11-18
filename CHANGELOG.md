# Changelog

All notable changes are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow semantic versioning.

## [Unreleased]

### Added

- **Dogri** (`doi`, also `dogri`, `dgo`; Devanagari). espeak-ng has no Dogri voice, so compatible
  mode is the reading of its Hindi voice, as for Sanskrit: PER 1.40 % (1.35 % without digits) on
  2,000 held-out sentences of IndicCorp v2 Dogri text, 1.9× espeak-ng's speed (measured on a
  Windows 11 laptop). Improved mode reads Dogri from its own language file (`lang/doi.lang`),
  with tone written as uccharan's Punjabi writes it: a word-initial voiced aspirate is voiceless
  with low tone on the next vowel (घर kˈʌ̀ɾ), an h before a vowel is lost with low tone (ज्हार
  ɟˈàːɾ), and a voiced aspirate inside the word or an h after a vowel (ह्, the apostrophe of
  कि’यां, the avagraha of रपेऽ) gives high tone to the stressed vowel before it (साह्ब sˈáːb), after
  Ghai (1991) and Bahri (2001). On a hand-written gold set (held-out half: 70 words, 15 sentences)
  improved mode gets 97.1 % of words right against 44.3 % for the Hindi voice, sentence PER 0.43 %
  against 13.03 %. Numbers, ordinals (12मीं, 1ला), decimals, clock times (10:30 दस बजियै त्रीह्
  मिंट), the rupee (रपेऽ) and dates in Dogri words in improved mode and normalise-only mode
  (`lang/numbers/ord-doi.txt`; 0–30 and the tens after Bahri, the rest of 31–99 formed like the
  Punjabi numbers and not yet checked by a Dogri speaker). Romanised Dogri through the `hinglish`
  option and `transliterate(..., "doi")` without a pack, with a short list of common Dogri words.
  Python, WebAssembly and C take the new tag; evaluation sets in `tools/eval/data/doi_*.tsv` and
  `tools/eval/gold/doi`.
- **Bodo** (`brx`, also `bodo`, `boro`; Devanagari). espeak-ng has no Bodo voice, so compatible
  mode is the reading of its Hindi voice, as for Sanskrit and Dogri: PER 0.37 % (0.32 % without
  stress) on 2,000 held-out sentences of IndicCorp v2 Bodo text and 0.48 % on the 1,012 FLORES-200
  sentences of IndicGenBench, 2.3× espeak-ng's speed (measured on a Windows 11 laptop). Improved
  mode reads Bodo from its own language file (`lang/brx.lang`): ो is ɯ and ै ौ are ɯi ɯu (Unicode
  L2/08-197), the inherent vowel is ɔ, said where an apostrophe follows the consonant (बर’, दख’ना)
  and otherwise dropped as in Hindi, ं is ŋ, य व close diphthongs, and loanword letters take Bodo
  sounds (ज z, भ b, श s), after Bhattacharya (1977) as given by Sarmah (2004). Tone (two lexical
  tones, one per word) is not written in Bodo and not said. On a hand-written gold set (held-out
  half: 79 words, 16 sentences) improved mode gets 98.7 % of words right against 2.5 % for the
  Hindi voice, sentence PER 0.44 % against 52.91 %. Numbers in Bodo words in improved mode and
  normalise-only mode: native numerals up to the thousands, लाख and कौटि above (25 नैजिबा, 2021 नै
  रोजा नैजिसे), ordinals with -थि (`lang/numbers/ord-brx.txt`), the rupee (रां), clock times and
  dates. Romanised Bodo in its Latin orthography (`jwnwm`, `arw`) through the `hinglish` option and
  `transliterate(..., "brx")` without a pack, with rules and a word list of its own
  (`src/brx.rs`). Python, WebAssembly and C take the new tag; evaluation sets in
  `tools/eval/data/brx_*.tsv` (the second held-out set `brx_flores.tsv` comes from IndicGenBench)
  and `tools/eval/gold/brx`.
- **Santali** (`sat`, also `santali`, `sat-olck`; Ol Chiki script, U+1C50–U+1C7F). espeak-ng has
  no Santali voice and cannot read Ol Chiki (it says each letter's code point under every voice),
  so there is no reference reading: both modes give uccharan's Santali reading, from its own
  language file (`lang/sat.lang`): ᱚ ᱟ ᱤ ᱩ ᱮ ᱳ = ɔ a i u e o, ᱟᱹ ᱮᱹ = ə ɛ, ᱸ ᱺ nasalise, ᱻ lengthens,
  the glottal letters ᱜ ᱡ ᱫ ᱵ are checked stops (k̚ʔ c̚ʔ t̚ʔ p̚ʔ) at the end of a word and before a
  voiceless consonant and voiced elsewhere, ᱼ (phaarkaa) keeps them checked and ᱽ (ahad) voices
  them, ᱷ aspirates, ᱭ ᱣ ᱶ close diphthongs, stress on the second syllable; after Everson's Ol
  Chiki proposal (Unicode L2/05-243R), Ghosh (2008) and Anderson and Horo (2021). A full stop typed
  for ᱹ and a hyphen typed for ᱼ are read as the marks; ᱾ ᱿ end sentences. On a hand-written gold
  set (held-out half: 100 words, 16 sentences) the reading gets every word right, sentence PER
  0.00 %, which shows the rules implement the published letter values, not more; every Ol Chiki
  word of four held-out sets (2,000 IndicCorp v2 sentences, 1,012 IndicGenBench FLORES-200
  sentences, 500 Santali Wikipedia sentences, 1,000 Tatoeba sentences) is read. 16.7 µs per
  sentence (p50) on a Windows 11 laptop, where Bodo took 20.4 µs. Numbers from ASCII or Ol Chiki
  digits in Santali words (25 ᱵᱟᱨ ᱜᱮᱞ ᱢᱚᱬᱮ, ᱥᱟᱭ, ᱦᱟᱡᱟᱨ, ᱞᱟᱠᱷ, ᱠᱳᱴᱤ), the rupee (ᱴᱟᱠᱟ) and dates
  (CLDR month names). Romanised Santali (Santali Latin alphabet and its ASCII form: `johar`,
  `menak'a`) in Ol Chiki through the `hinglish` option and `transliterate(..., "sat")` without a
  pack, with rules and a word list of its own (`src/sat.rs`). Santali in Bengali, Odia or
  Devanagari script is read as that script's language. Python, WebAssembly and C take the new
  tag; evaluation sets in `tools/eval/data/sat_*.tsv` (held-out: IndicCorp v2, IndicGenBench,
  Santali Wikipedia, Tatoeba), `tools/eval/gold/sat` and the coverage check
  `tools/eval/sat_check.py`.
- **Manipuri** (Meitei; `mni`, also `manipuri`, `meitei`, `meetei`, `mni-mtei`, `mni-beng`) in both
  its scripts. Meetei Mayek (U+ABC0–U+ABFF and the extensions U+AAE0–U+AAFF) gets its own block in
  the tokenizer (letters, ꯰–꯹ digits, ꯫ and ꫱ sentence ends, ꫰ a comma) and its own language file
  (`lang/mni.lang`): consonant letters carry ɐ, the lonsum letters ꯛ ꯜ ꯝ ꯞ ꯟ ꯠ ꯡ ꯢ close a syllable,
  apun iyek ꯭ makes a cluster, ꯩ ꯧ are ei ou, ꯏ ꯎ ꯌ after a vowel close a diphthong, a final letter
  keeps ɐ only after the suffix consonants (ꯂꯩꯕ leibɐ, ꯃꯅꯤꯄꯨꯔ mɐnipuɾ); after Everson's Meitei Mayek
  proposal (WG2 N3206R2, Unicode L2/07-005R2) and Chelliah (1997) and Bhat and Ningomba (1997) as
  summarised by Wikipedia. The two lexical tones are written in neither script and are not read
  (lum iyek ꯬ is skipped); words carry no stress marks (new option `stress = none`). What the
  spelling leaves to the reader is written out first (`mni::prepare`): a stem closed before a
  suffix (ꯃꯇꯃꯗꯥ mɐtɐmdɐ, মোমবি mombi) and a suffix vowel written ꯥ or া (ꯂꯥꯎꯕꯥ laubɐ). Bengali-script
  Meitei is written in Meetei Mayek letter for letter (`mni::bengali_to_meetei`: a final bare
  consonant closes the syllable, hasanta before a consonant is a lonsum letter, শ ষ স are s) and read
  the same way; Bengali and Assamese text read as before (outputs identical on their 8,000 dev and
  held-out sentences). espeak-ng has no Meitei voice and cannot read Meetei Mayek, so both modes
  give this reading (`Lang::no_reference`). On a gold set of 561 words with Wiktionary's
  pronunciations and 33 hand-written sentences, split by hash before the rules were run on it, the
  held-out half gets 92.2 % of 193 Meetei Mayek words and 95.2 % of 83 Bengali-script words right,
  sentence PER 1.25 % (Meetei Mayek, 10) and 0.52 % (Bengali script, 9); espeak-ng's Bengali voice
  gets 28.9 % of the Bengali-script words. Every word of the held-out IndicCorp v2, Manipuri
  Wikipedia and IndicGenBench sets is read (`tools/eval/mni_check.py`). 14.2 µs per sentence (p50)
  on a Windows 11 laptop, where Bodo took 20.3 µs. Numbers in Meitei words from ASCII, Meetei Mayek
  or Bengali digits, as Meetei Mayek text writes them (25 ꯀꯨꯟꯃꯉꯥ, 201 ꯆꯥꯅꯤꯒ ꯁꯨꯞꯅ ꯑꯃꯥ, 2000 ꯂꯤꯁꯤꯡ
  ꯑꯅꯤ with the scale word first: new option `scale-first`), the rupee (ꯂꯨꯄꯥ) and dates;
  normalise-only mode writes them in Bengali script for Bengali-script text (`mni::meetei_to_bengali`).
  Romanised Meitei (`eina nangbu nungsi`) in Meetei Mayek through the `hinglish` option and
  `transliterate(..., "mni")` without a pack (`src/mni.rs`). Python, WebAssembly and C take the new
  tag; evaluation sets in `tools/eval/data/mni_*.tsv` (held-out: IndicCorp v2, Manipuri Wikipedia,
  IndicGenBench in Bengali script), `tools/eval/gold/mni` and the coverage check
  `tools/eval/mni_check.py`.
- **Kashmiri** (`ks`, also `kashmiri`, `koshur`, `kas`, `ks-arab`, `ks-deva`) in the Perso-Arabic
  script of its official orthography, which writes nearly every vowel, and in Devanagari. A new
  language file (`lang/ks.lang`) reads the words after `ks::explicit` has spelled each vowel out from
  the marks and letters of the orthography (َ ِ ُ a i u, ٔ ə, ٕ ɨ, ٟ ɨː, ا aː, ٲ əː, وٗ uː, و oː, ۆ o,
  ۄ ɔ, یٖ iː, ی eː, یٚ e, ؠ palatalisation, ٛ a cluster, a final ہ carrying the mark before it), after
  the tables of the Wikipedia article "Kashmiri language" and Wiktionary's pronunciations; a word
  written without any vowel mark is read with the inherent a and Urdu-type deletion; stress on the
  leftmost non-final heavy syllable, long vowels first (Morén 2000; new option `stress =
  left-heavy`). Kashmiri Devanagari (the 2002 and 2009 proposals: ॅ ऺ ॉ ऻ ॖ ॗ ॆ ॊ ॏ, ्य) is written in
  Perso-Arabic script (`ks::devanagari_to_arabic`) and read the same way, so a word reads the same in
  both scripts; Urdu, Sindhi and every Devanagari language read as before (outputs identical on all
  their sentence sets, both modes, three profiles, romanised input). espeak-ng has no Kashmiri voice,
  so both modes give this reading (`Lang::no_reference`). On a gold set of 1,344 words with
  Wiktionary's pronunciations and 42 hand-written sentences, split by hash before the rules were run
  on it, the held-out half gets 95.8 % of 455 Perso-Arabic words, 92.6 % of 190 Devanagari words and
  37 of 37 numerals right, sentence PER 0.18 % (Perso-Arabic, 12) and 0.00 % (Devanagari, 6);
  espeak-ng's Urdu and Hindi voices, other languages, get 27.9 % and 30.5 % of the words. Every word
  of the held-out IndicCorp v2 and Kashmiri Wikipedia sets is read (`tools/eval/ks_check.py`).
  15.0 µs per sentence (p50) on a Windows 11 laptop, where Bodo took 20.1 µs. Numbers in Kashmiri
  words from ASCII, Extended Arabic-Indic or Devanagari digits, as Wiktionary spells them (25
  پٕنٛژٕہ, 800 ٲٹھ شَتھ, 1000 اَکھ ساس, لَچھ, کَرور), the rupee (رۄپَے), per cent and dates (CLDR month
  names); normalise-only mode writes them in Devanagari for Devanagari text
  (`ks::arabic_to_devanagari`). Romanised Kashmiri (`koshur chhu akh zabaan`) in Perso-Arabic script
  through the `hinglish` option and `transliterate(..., "ks")` without a pack (`src/ks.rs`). Python,
  WebAssembly and C take the new tag; evaluation sets in `tools/eval/data/ks_*.tsv` (held-out:
  IndicCorp v2, Kashmiri Wikipedia), `tools/eval/gold/ks` and the coverage check
  `tools/eval/ks_check.py`.
- Language-file options `stress = left-heavy` (primary stress on the leftmost non-final syllable
  with a long vowel, else the leftmost closed one, else the first) and `reader = ks` (a Rust
  function rewrites each word before the letter rules).
- Language-file options `stress = none` (no stress marks) and `scale-first yes` in `[numbers]` (the
  scale word before its count).
- Language-file option `stress = second` (primary stress on the second syllable).
- Language-file options `time-hour-minutes` and `time-minute` (the hour word before minutes and a
  word after them) and `apostrophe keep` (an apostrophe between Indic letters stays in the word);
  see `docs/adding-a-language.md`.

### Changed

- The phonemizer keeps its per-language word tables on the heap: with 43 languages the unoptimised
  command-line tool overflowed the 1 MB main-thread stack on Windows.

### Fixed

- **Tamil, Kannada, Telugu and Malayalam numbers** in improved mode are read with the written number
  words of Unicode CLDR, as normalise-only mode reads them (`improved-numbers = written`): Tamil
  ₹1,250 is *ஆயிரத்து இருநூற்று ஐம்பது ரூபாய்*, not *ஒன்று ஆயிரம் இரண்டு நூறு ஐம்பது*, and 10% says
  *சதவீதம்*. Kannada and Telugu no longer say "one thousand" and "one hundred" for ಸಾವಿರ, వెయ్యి
  and వంద. The other Indian languages keep their number table, which reads better there (Bengali
  একশো, Urdu 19 and 45). Compatible mode is unchanged.
- **The rupee sign** is said in improved mode in every Indian language and in Urdu and Sindhi
  (*ரூபாய்*, *টাকা*, *روپے*); the phonemiser dropped it.
- **Hindi clock times before बजे** say बजे once: *3:30 बजे* is *साढ़े तीन बजे* in every mode with
  natural normalisation, as in normalise-only mode, and *3:20 बजे* is *तीन बजकर बीस मिनट*
  (was *… मिनट बजे*; *4:00 बजे* was *चार बजे बजे*). Normalise-only mode reads *3:20 बजे* the same
  way.
- **English month abbreviations** without a full stop before a day are read in full in natural
  normalisation and normalise-only mode: *Jan 5* is *January five*, *Mar 3rd* *March third*. A
  lower-case *mar*, or one not before a day of the month, stays a word.
- **An unknown language** names every supported tag in the error in Python and the command line, as
  in JavaScript; it listed only en-us, en-gb and hi.

## [0.6.6] - 2026-11-17

### Changed

- README: Bhojpuri, Maithili, Magahi and the other languages without a word table need no pack (they looked
  missing from the list of packs); what was run on a Windows 11 laptop (the tests, UTF-16 input, the C library
  with MSVC, Hindi at 8.7 µs p50 per sentence).

## [0.6.5] - 2026-11-16

### Fixed

- **`tools/bench/compare.py` on Windows**: the memory measurement read `/proc/self/status` and
  stopped the run on Windows. It now uses `psutil` when it is installed, else `/proc` on Linux or
  `getrusage` on other POSIX systems, and on Windows without `psutil` leaves the memory columns
  empty with a note. The default `--lib` is the platform's library name (`uccharan.dll`,
  `libuccharan.dylib`, `libuccharan.so`).
- **Standard input on Windows**: `uccharan -l hi < file.txt` stopped with a `UnicodeDecodeError`
  traceback when the file was UTF-16 (as Notepad and PowerShell write it). The Python and Rust
  command lines read standard input as UTF-8, or as UTF-16 or UTF-32 when it starts with a byte
  order mark, and drop a UTF-8 byte order mark; input that does not decode is a one-line error
  naming the encoding, with exit status 2, instead of a traceback (Python) or silently stopping
  at the bad line (Rust). The Rust command line's `--wordlist` and `--match-terms` files are
  read the same way.
- **`python -m uccharan.packs`** printed a `RuntimeWarning` about `uccharan.packs` being in
  `sys.modules` before it ran. `uccharan.packs` is now a package with a `__main__` module; its
  API is unchanged.

### Changed

- The README shows the MSVC `cl.exe` commands for linking the C library on Windows, against the
  DLL and statically; a static link needs `ntdll.lib` and `synchronization.lib` among the system
  libraries.
- docs/evaluation.md notes the Hindi latency measured on a Windows 11 laptop (8.7 µs p50 per
  sentence, against 5.5 µs on the Linux machine of the table) and that no Windows-only cost was
  found in the Hindi path.
- The Python README says how to set up Windows PowerShell 5.1 for UTF-8 in pipes (cmd and
  PowerShell 7 need nothing).

## [0.6.4] - 2026-11-16

Bug fixes to the existing languages (no new languages): clock times, currency words that agree
with the amount, ordinals before a hyphen, and numbers before a full stop that ends a sentence.

### Fixed

- **Clock times** were read with the letter *h* or as one number: French and Portuguese "10h30"
  were *dix h trente*, *dez h trinta*, and "10.30" was *a thousand and thirty* in every
  language with a decimal comma. Improved mode and normalise-only mode now read times: French
  *dix heures trente* ("10h30", "10 h 30", "10h", "10:30"; *une heure*, *vingt et une heures*),
  Portuguese *dez horas e trinta* (*uma hora*, *duas horas*), Spanish *diez horas y treinta*
  and Catalan *deu hores i trenta* ("10h30"), German *zehn Uhr dreißig* ("10:30", "10.30 Uhr",
  "um 10.30"; *ein Uhr*), Dutch *tien uur dertig*, and "10.30" / "10:30" as hours and minutes
  in Finnish (*kymmenen kolmekymmentä*), Swedish, Danish, Norwegian, Italian, Romanian,
  Hungarian, Czech, Polish and Turkish (*kymmenen nolla viisi* for "10.05"). `time…` lines in
  `[numbers]` of the language files.
- **A full stop before two digits** grouped thousands ("10.30" read as 1030). It groups only
  three digits now; compatible mode reads "10.30" with the voice's word for the full stop
  (Finnish *kymmenen piste kolmekymmentä*), as the reference voices do, and improved mode reads
  a time or a decimal.
- **A number before a full stop that ends the sentence** was read as an ordinal when the next
  sentence started with a word such as German *Am*: "iPhone 15. Am Montag …" was *fünfzehnter
  Am …*. "N." is now an ordinal only before a lower-case word or a month name, or after a word
  such as *am*, *der*, *den*, *zum* (German, Danish, Norwegian, Finnish, Hungarian, Czech,
  Polish, Turkish); otherwise it is a cardinal and the full stop ends the sentence.
- **Ordinals before a hyphen** were split: English "3rd-floor" was *three rd floor*, now *third
  floor* ("21st-century" *twenty first century*) in every mode, as the reference reads it;
  French "1er-mai" / "2e-étage", Spanish "2º-piso", Dutch "3e-klas" and the like likewise in
  improved mode and normalise-only mode.
- **Currency words** were always singular ("25 €" *vingt-cinq euro*). Improved mode and
  normalise-only mode now use the plural where the language has one: French *euros*,
  *dollars*, *livres*, Spanish *euros*, *dólares*, Portuguese *euros*, *libras*, *centavos*,
  Catalan *euros*, *dòlars*, Romanian *douăzeci și cinci de dolari*, Finnish *euroa*, Czech
  *dolary* / *dolarů*, *eura* / *eur*, Polish *dolary* / *dolarów*, German *Rupien*, English
  "25 €" *twenty five euros*; Italian *euro*, German *Euro* and the Dutch and Scandinavian
  words stay invariable. One is said in the form that goes with the currency word: *un euro*,
  *una libra*, *ein Euro*, *une livre*, *jedno euro*, Spanish *veintiún euros*. Compatible mode
  keeps the reference readings; English compatible mode now says *euros* and *cents* for `€`
  and `¢`, as the reference does.

Development PER (compatible mode): fi 1.09 → 1.02 %, nl 1.60 → 1.55 %, sv 1.26 → 1.23 %, de 1.05
→ 1.03 %, da 1.84 → 1.82 %, tr 0.54 → 0.52 %, nb 0.87 → 0.86 %, es 0.54 → 0.53 %; no language
got worse.

## [0.6.3] - 2026-11-15

Bug fixes to the existing languages (no new languages): dates, ordinals and decimals are read
in full in improved mode and normalise-only mode, Konkani and Sinhala numbers are written with
their proper words, and compatible mode follows the reference voices on more number formats.

### Fixed

- **Dates** written with digits (15/08/2024, 15.08.2024, 15-08-2024, 2024-08-15; English also
  MM/DD/YYYY in en-us) are read as day, month name and year in every language with a
  phonemiser and normalise-only mode, in improved mode and normalise-only mode: German
  *fünfzehnter August zweitausendvierundzwanzig* (*am fünfzehnten August* after "am"), French
  *quinze août …* (*premier mai*), Spanish and Portuguese *quince de agosto de …*, Polish
  *piętnastego sierpnia dwa tysiące dwudziestego czwartego roku*, Hungarian (year first)
  *… augusztus tizenötödike*, Czech, Finnish, Swedish, Danish, Norwegian, Dutch, Italian,
  Catalan, Romanian, Turkish, and the Indian languages, Urdu and Sindhi (day, month, year as
  numbers of the language). They were read piece by piece (*fifteen zero eight …*). Compatible
  mode keeps the piece-by-piece reading, as the reference voices read dates so. Month names
  from Unicode CLDR (`[dates]` in the language files).
- **Ordinals** are read as ordinals where the writing makes it clear: "3." before a lower-case
  word or next to a month or an article (German, Danish, Norwegian, Finnish, Hungarian, Czech,
  Polish, Turkish), French *1er*, *1re*, *2e*, Spanish, Italian and Portuguese *1º*, *2ª*,
  Catalan *1r*, *2n*, *1a*, Dutch *1e*, *2de*, Swedish *1:a*, *3:e*. Ordinal words from the CLDR
  rule sets (`lang/numbers/ord-*.txt`). Compatible mode reads as ordinals only what the
  reference voices read so (Danish, Norwegian and Hungarian "3.", French *1er*, the Spanish,
  Italian and Portuguese º/ª, Catalan *1r*/*2n*, Dutch *1e*, Swedish *:a*/*:e*).
- **Konkani numbers** were Marathi words. Normalise-only mode and improved mode now use the
  standard Goan Konkani spellings of the Goa Konkani Akademi (*पांच*, *स*, *णव*, *धा*,
  *इकरा*, *एकुणीस*, *णव्वद*, *णव्याण्णव*, *शंबर*, *एकशे …*, *तिनशे*, *हजार*, *लाख*,
  *कोटी*; `lang/numbers/kok.txt`). Compatible mode keeps the reference voice's readings.
- **Sinhala**: `%` was dropped; compatible mode now says *සියයට* before the number, as the
  reference voice does, and normalise-only and improved mode write *ප්‍රතිශතය* after it.
  100 and 1000 were *එක සිය*, *එක දහස*: normalise-only and improved mode write the standard
  forms *සියය*, *එකසිය …*, *දෙසියය*, *තුන්සියය*, *දහස*, *එක්දහස් …*, *දෙදහස*
  (`lang/numbers/si.txt`); compatible mode now reads 100, 200, 1000 and 1500 as the
  reference voice does (*siyaya*, *dekasiya*, *ahas*, *dahas pahasiya*).
- **Decimals and number signs in compatible mode** now follow the reference voices: German
  "-5" (*minus*) and "2.5" (*Punkt*); a full stop between digits that does not group thousands
  in Spanish, French, Italian, Turkish, Czech, Swedish, Catalan, Hungarian, Finnish,
  Norwegian and Danish; Czech and Hungarian "1.000" (the full stop never groups); Hungarian
  decimals with *tized*, *század*, *ezred*; the digits after the decimal comma read as one
  number in Italian, Portuguese, Polish, Czech, Turkish, Finnish, Catalan and Romanian
  ("3,25" *vinte e cinco*); no minus word in Polish, Turkish, Catalan and Danish. Improved
  mode and normalise-only mode read "2.5" as a decimal (*zwei Komma fünf*).
- **Money with cents** in the Latin-script languages ("25,50 €", "€25,50", also with the sign
  after a space): improved mode and normalise-only mode read the whole units, the currency and
  the cents (*fünfundzwanzig Euro fünfzig*, *veinticinco euro cincuenta*); they were read as a
  decimal number (*fünfundzwanzig Komma fünf null Euro*). Compatible mode keeps the reference
  reading.
- **French written numbers**: *cent* and *quatre-vingt* take no s before another number word
  (*deux cent mille*, *quatre-vingt mille*; still *deux cents millions*).
- **Vietnamese letters** (ạ, ễ, ộ … in names such as *Nguyễn*) split a word in two; they are
  now Latin letters, read by their base letter in every language (English included).

### Changed

- The WikiPron packs `wikipron-en-us.ucd` and `wikipron-en-gb.ucd` are rebuilt: which of their
  words need an entry depends on how accented letters are read (download them again for 0.6.3).

## [0.6.2] - 2026-11-15

Bug fixes. Normalise-only mode wrote numbers in Sinhala and Sindhi as internal keys instead of
words; every non-Latin language is now checked for numbers written in its own script.

### Fixed

- **Normalise-only mode in Sinhala and Sindhi** (`normalize` in Python, WASM and C,
  `normalize_api` in Rust): numbers came out as the keys the phonemiser uses to look up its
  number readings, strings of letters that are not words ("25" → *ණගඡ* in Sinhala, *ڙتج* in
  Sindhi). They are now written as Sinhala and Sindhi words, one for each number word the
  phonemiser reads (`[numbers.written]` in `si.lang` and `sd.lang`): "25" → *විසිපහ*,
  *پنجويهه*; "1,234" → *එක දහස දෙක සිය තිස්හතර*, *هڪ هزار ٻه سؤ چوٽيهه*.
- **Decimals in Sinhala, Sindhi, Konkani and Sanskrit (improved mode)** lost their decimal point word ("3.5"
  was read *three five*). The phonemiser now says the point as the reference voice does
  (*දශම*, *ڏهائي*, *दशमलव*) and normalise-only mode writes it; Sindhi `%` is *فيصد*.
- **The rupee sign** is written as a word in Sinhala, Urdu and Sindhi (*රුපියල්*, *روپے*,
  *رپيا*), as in the other Indian languages; it was left out.
- **Sanskrit in improved mode** wrote numbers as Hindi words while the phonemiser reads them in
  Sanskrit; it now writes the Sanskrit words (*पञ्चविंशतिः*). Compatible mode keeps the Hindi
  words, as the reference voice reads digits in Hindi.
- **Phonemiser, Sinhala and Sindhi digits**: Sinhala 6 and 7 (alone and in 106, 1006 …) were
  read as the letters of their internal key (*ɳɐdʒə*) and are now *hɐjə*, *hɐtə*; the Sindhi
  hundred was *basaʊ* after every count ("500" *panɟa basaʊ*) and is now *saʊ*, as the
  reference voice reads it.

## [0.6.1] - 2026-11-15

Bug fixes. Normalise-only mode reads numbers in the Latin-script languages with their own
number words (it read them in English); currency signs and the Spanish decimal comma are read
as the reference voices read them; letters of other alphabets in names no longer pass into the
phonemes.

### Added

- Language-file options for the written form of number words in normalise-only mode
  (`[numbers.written]`, `written-hyphen`, `written-glue`, `written-glue-after`,
  `written-plural`), for currency signs (`currency`) and `group space-improved`
  (see `docs/adding-a-language.md`).

### Fixed

- **Normalise-only mode in the Latin-script languages** (`normalize` in Python, WASM and C,
  `normalize_api` in Rust): numbers in Spanish, German, Dutch, French (also fr-be, fr-ch),
  Italian, Portuguese (pt-br, pt), Polish, Turkish, Czech, Swedish, Catalan, Romanian,
  Hungarian, Finnish, Norwegian and Danish text were read in English (in Hindi with
  `digits="request"`), and words with hyphens or apostrophes lost them. They are now read with
  the language's own number rules, the ones the phonemiser uses, and written in the language's
  spelling: "Ich habe 23 Euro." → *Ich habe dreiundzwanzig Euro.*, "Il a 71 ans." →
  *septante et un* (fr-be, fr-ch), "Jeg har 95 kroner." → *femoghalvfems*, "1 280" → *mille
  deux cent quatre-vingts*; decimals with the language's decimal sign, `%` and currency signs.
  Numbers written in groups of three digits ("1 500") follow the phonemiser: joined in
  compatible mode where the reference voice joins them, and in improved mode. Abbreviations of
  the language stay as written (German "Dr." was "Doctor").
- **Currency signs** (`€ $ £`, in some languages `¥ ₹ ¢`) were left out in the Latin-script
  languages; they are read where they stand, with the reference voice's word ("5 €" *fünf Euro*,
  "$5" *dollar cinq*); improved mode says the word after the amount.
- **Spanish decimal comma**: "3,5" is *tres coma cinco*, "1.000" *mil* and "12,50" *doce coma
  cincuenta*, as the reference voice reads them ("12,50" was read as 1250). Development PER
  0.62 → 0.55 %.
- **Letters of other alphabets in names** (Łódź, Dvořák, Erdoğan, Gödel, Straße) passed into the
  phonemes unchanged in the Latin-script languages that do not have them (Spanish, French,
  Romanian, Finnish, Portuguese and others), and Piper voices skipped them. A letter that
  neither the language's letter rules nor its word table's model know is now read as its base
  letter (ł l, ř r, ğ g, ø o, ß ss), as the reference voices read them. Development PER is
  unchanged or slightly better in every Latin-script language (Danish 1.94 → 1.91 %, Dutch 1.64
  → 1.62 %, French 0.63 → 0.62 %, with the currency signs). Letters of Vietnamese
  (ễ, ộ, ...) still split the word they stand in. The Finnish word table, rebuilt, no longer
  needs entries for a few such words (`fi.ucd` 136,697 → 136,680 bytes; same readings).
- **WASM:** Bhojpuri, Maithili and Magahi asked for data files of their own (`bho.ucd`,
  `mai.ucd`, `mag.ucd`) that do not exist, and failed. They now load the Hindi data, as the
  Rust core and the Python package already did.

### Changed

- **Improved mode** reads numbers written with spaces between groups of three digits ("1 500")
  as one number in Spanish, Italian, Portuguese, Czech and Catalan too (compatible mode reads
  the groups apart, as the reference voices do).

## [0.6.0] - 2026-11-14

Fixes to existing languages and two new French voices: Belgian and Swiss French (`fr-be`,
`fr-ch`). French liaison before h, number words and *ont*/*sont* follow the reference voice more
closely; numbers written with spaces between groups of three digits are read as the reference
voices read them; Danish reads words outside its list better; Catalan links words as the
reference voice does. Improved mode of Spanish, German, French, Italian and Portuguese has new
gold sets and follows the standard descriptions more closely.

### Added

- **Belgian and Swiss French** (`fr-be`, `fr-ch`, the espeak-ng voices of the same names): read
  with the French word table; 70–99 as *septante*, *huitante* (Swiss) and *nonante*. Held-out
  PER 0.63 % and 0.62 %.
- **Improved-mode gold sets for Spanish, German, French, Italian and European Portuguese**
  (`tools/eval/gold/improved/`, 25 development and 25 test words per language, written from the
  JIPA illustrations and pronouncing dictionaries; `gold_eval.py --improved dev|test`). Test words
  right: improved mode 88.8 %, espeak-ng 64.0 %.
- Language-file sections `[words.improved]` (readings for improved mode only),
  `[spelling.improved]` (a phoneme change tied to letters of the spelling), `[context.compat]`
  (a word's reading after given words or word endings) and `[h-muet.compat]`; `[meta]` options
  `clitic-verbs`, `clitic-subjects`, `group-stress-spaced`, `vowel-mark-except`; lexgen reads an
  optional `data/sources/<lang>/train.tsv` of words that only train the letter-to-sound model.

### Fixed

- **French**: in compatible mode a word written with h first blocks liaison unless it is an
  h-muet word, as the reference voice does (`les haricots`, `des hiboux`); improved mode uses the
  h-aspiré words of the dictionaries. Number words keep or drop their final consonant by what
  follows (`six enfants`, `six livres`, `dix ans`, `neuf heures`); *ont* and *sont* lose their
  stress after a clitic pronoun (`ils ont`, `n'en ont`, `y sont`). Improved mode: gn is ɲ,
  *jouent* is ʒu.
- **Numbers with spaces between groups of three digits**: compatible mode joins the groups only
  across an ordinary space, as the reference voices do; after a no-break or thin space (common in
  news text) each group is read by itself, a group with a leading zero as the voices read it
  (`000`). Improved mode joins groups across all of these spaces. Held-out PER in compatible mode:
  Finnish 1.57 → 1.36 %, Swedish 1.44 → 1.32 %, Polish 1.19 → 1.12 %, Norwegian 1.03 → 0.96 %,
  Hungarian 1.37 → 1.34 %.
- **Catalan cross-word sandhi**: the conjunction *i*, *hi*, *ho*, *lo* and final unstressed i and
  u as glides by the neighbouring words, *o* as w between vowels, final r as a tap before a vowel
  and a trill after a stressed vowel before a consonant. Held-out PER 1.51 → 1.44 %.

### Changed

- **French** held-out PER 0.79 → 0.63 % (development 0.80 → 0.63 %).
- **Danish words outside the word list**: 13,600 more words in the list and 41,000 that only
  train the letter-to-sound model; held-out PER 2.22 → 1.95 %, outside-list words wrong 68.1 →
  62.3 %; `da.ucd` 490 → 658 KB.
- **Improved mode** of Spanish, German, Italian and European Portuguese follows the standard
  descriptions more closely (Spanish final ð, ɱ and voiced s; German ʁ, ɐ after a long vowel and
  short unstressed vowels; Italian j w i u for ɪ ʊ; Portuguese final u, ɐj, nasal final vowels).
- `tools/eval/e2e_tts_asr.py` works for every language with a held-out set (it read English or
  Hindi sentences only, and transcribed in English).

## [0.5.0] - 2026-11-14

Language data on demand: the word tables of sixteen Latin-script languages, Urdu and Sindhi
are now packs that are downloaded and checked the first time a language is used, which makes the
wheel, the crate and the C library much smaller; the tables themselves are in the new crate
`uccharan-data`. Roman numerals such as IV and IX were read one step too high in every
Latin-script language; that is fixed. Capitals, Roman numerals and single letters now follow each
language's reading, which lowers the error rate in all seventeen Latin-script languages.

### Fixed

- **Roman numerals with a subtracted letter** (`IV`, `IX`, `XL`, `XIV` …) were read one step too
  high in every Latin-script language (`Carlos IV` as "cinco"). Spellings that are not numerals
  (`IIII`, `IL`) are no longer read as numbers.
- **Spanish and Catalan single letters** (`plan B`, `vitamina C`) are said by their names instead
  of as a bare consonant.

### Changed

- **Capitals, Roman numerals and Catalan sandhi in compatible mode** (all seventeen Latin-script
  languages; held-out halves scored again, see docs/evaluation.md): a token in capitals is spelled
  when it has two or three letters or no vowel and read as a word otherwise (`NATO`, `COVID`);
  Finnish, Hungarian and Czech stress the first letter of a spelled abbreviation; Turkish capital
  I is the dotless letter. Roman numerals follow each voice: cardinals (Spanish, Portuguese,
  Catalan, Czech, Romanian), "römisch" + cardinal (German), cardinal + "romain" (French), ordinals
  (Italian, Danish), letters (Dutch, Polish, Turkish, Swedish, Hungarian, Finnish, Norwegian);
  improved mode reads them as numbers everywhere. Catalan voices final s, ʃ and z before voiced
  sounds and before h-words and reads the conjunction *i* as a glide before a vowel; Finnish tens
  after a hundred carry secondary stress. Held-out PER: Spanish 0.74 → 0.55 %, Italian 0.66 →
  0.53 %, Catalan 1.86 → 1.51 %, Polish 1.33 → 1.19 %, German 1.01 → 0.95 %, Dutch 1.77 → 1.69 %,
  French 0.85 → 0.79 %, Portuguese 0.67 → 0.61 % (Brazil) and 0.89 → 0.84 % (Portugal), Turkish
  0.72 → 0.68 %, Czech 1.40 → 1.32 %, Swedish 1.51 → 1.44 %, Romanian 1.00 → 0.97 %, Hungarian
  1.44 → 1.37 %, Finnish 1.64 → 1.57 %, Norwegian 1.11 → 1.03 %, Danish 2.26 → 2.22 %.
  New language-file options: `roman`, `roman-skip`, `roman-before`, `roman-after`, `ordN`,
  `romanN`, `Nh` number forms, `caps-words`, `caps-entries`, `spell-stress`, `letter-names`,
  `dotless-i`, `vowel-mark = h`.
- **Numbers written in groups of three digits with spaces** (`10 000`, `1 250 000`) are read as
  one number in German, Dutch, French, Polish, Swedish, Romanian, Hungarian, Finnish and
  Norwegian. French makes no liaison before words with h aspiré (`les haricots`) or before
  *huit* and *onze*.

- **Word tables are fetched when a language is first used** instead of being part of every
  package. English, Hindi and the languages read by rules alone stay built in; the word tables
  of German, Dutch, French, Italian, Portuguese (both), Polish, Turkish, Czech, Swedish, Catalan,
  Romanian, Hungarian, Finnish, Norwegian, Danish, Urdu and Sindhi (46 KB to 0.5 MB each) are
  packs attached to the GitHub release, checked against SHA-256 sums built into the package and
  cached in `~/.cache/uccharan/<version>/`. Sizes: wheel 5.6 → 2.0 MB, crate 8.4 → 4.9 MB
  compressed, C library 11.6 → 7.6 MB; the npm package already loaded each language's file on
  first use.
  - A language is never read without its table: Python raises `PackError`, the C calls return
    NULL and the new `uccharan_last_error()` gives the reason (`uccharan_ensure()` checks ahead),
    Rust's new `Phonemizer::ensure` returns the error (`phonemize` panics with the same message
    when `ensure` was not called and the table cannot be loaded), and `phonemizeSync` in
    JavaScript throws. Every message names the pack, its URL and where the file is looked for.
  - Offline: `python -m uccharan.packs --dir DIR de fr …` (or `--all`) fetches packs for
    `UCCHARAN_DATA_DIR`, which may now name several directories; `UCCHARAN_OFFLINE=1` forbids
    downloads.
  - Rust: `features = ["bundle"]` compiles every table in (they live in the new `uccharan-data`
    crate; `embed-<tag>` features still select single tables), `features = ["download"]` fetches
    a missing table on first use; without either, tables are read from `UCCHARAN_DATA_DIR` or
    the cache. New module `uccharan::packs`, `Phonemizer::missing_packs`,
    `Phonemizer::rules_fallback`. `Phonemizer::has` is now false for a language whose table is
    not loaded, and `Phonemizer::empty()` reads such a language with its letter rules.
  - Command line: `uccharan --packs` lists the packs, `uccharan --download NAME…` fetches them
    (with the `download` feature). The C library builds with `--features bundle` or `download`
    as well.

- **Hungarian**: a list of 2.4 k words with only secondary stress at the end of a clause, and
  number words with one stress per group of three digits. Held-out PER 2.08 → 1.44 % on
  FLORES-200 (1.02 % without digits; second scoring of that half); words in the table 1.8 % wrong,
  other listed words 0.2 %, words outside the word list 3.8 % (1121 words).
- **Sindhi**, words outside the word list: `sd.ucd` now holds a model that corrects the letter
  rules' reading of a word (trained on the readings of the 61 k-word list) and the words that
  rules and model together get wrong (6.9 k entries, 120 KB; was 18 k entries, 209 KB). Held-out
  PER 1.52 → 1.30 % (third scoring of that half, see docs/evaluation.md); words outside the list
  29.8 → 21.9 % wrong. The same model made Urdu's unlisted words worse on the development
  sentences (45.7 → 49.6 % wrong), so Urdu keeps its table of exceptions.

## [0.4.0] - 2026-11-14

### Added

- **Italian** (`it`): compatible with the espeak-ng voice `it`; word table `it.ucd` (9.2 k entries and a letter-to-sound model from 104 k words, 188 KB, feature `embed-it`), number words, letter names and clause rules. Held-out PER 0.66 % on FLORES-200 (0.48 % without digits); words in the table 0.5 % wrong, other listed words 0.1 %, words outside the word list 32.2 % (503 words); 4.6× the speed of espeak-ng.
- **Portuguese (Brazil)** (`pt-br`): compatible with the espeak-ng voice `pt-br`; word table `pt-br.ucd` (5.1 k entries and a letter-to-sound model from 82.5 k words, 171 KB, feature `embed-pt-br`), number words, letter names and clause rules. Held-out PER 0.67 % on FLORES-200 (0.46 % without digits); words in the table 0.7 % wrong, other listed words 0.8 %, words outside the word list 42.0 % (490 words); 5.1× the speed of espeak-ng.
- **Portuguese (Portugal)** (`pt`): compatible with the espeak-ng voice `pt`; word table `pt.ucd` (6.9 k entries and a letter-to-sound model from 81.6 k words, 215 KB, feature `embed-pt`), number words, letter names and clause rules. Held-out PER 0.89 % on FLORES-200 (0.64 % without digits); words in the table 1.3 % wrong, other listed words 1.8 %, words outside the word list 50.2 % (484 words); 5.2× the speed of espeak-ng.
- **Polish** (`pl`): compatible with the espeak-ng voice `pl`; word table `pl.ucd` (18.8 k entries and a letter-to-sound model from 127 k words, 239 KB, feature `embed-pl`), number words, letter names and clause rules. Held-out PER 1.33 % on FLORES-200 (1.05 % without digits); words in the table 3.6 % wrong, other listed words 2.7 %, words outside the word list 30.7 % (1058 words); 3.1× the speed of espeak-ng.
- **Turkish** (`tr`): compatible with the espeak-ng voice `tr`; word table `tr.ucd` (16.3 k entries and a letter-to-sound model from 151 k words, 230 KB, feature `embed-tr`), number words, letter names and clause rules. Held-out PER 0.72 % on FLORES-200 (0.51 % without digits); words in the table 0.1 % wrong, other listed words 0.0 %, words outside the word list 22.3 % (1087 words); 2.7× the speed of espeak-ng.
- **Czech** (`cs`): compatible with the espeak-ng voice `cs`; word table `cs.ucd` (4.9 k entries and a letter-to-sound model from 133 k words, 78 KB, feature `embed-cs`), number words, letter names and clause rules. Held-out PER 1.40 % on FLORES-200 (1.13 % without digits); words in the table 0.9 % wrong, other listed words 2.9 %, words outside the word list 11.4 % (501 words); 1.8× the speed of espeak-ng.
- **Swedish** (`sv`): compatible with the espeak-ng voice `sv`; word table `sv.ucd` (7.0 k entries and a letter-to-sound model from 94.8 k words, 171 KB, feature `embed-sv`), number words, letter names and clause rules. Held-out PER 1.51 % on FLORES-200 (0.88 % without digits); words in the table 0.1 % wrong, other listed words 0.0 %, words outside the word list 32.7 % (1161 words); 3.7× the speed of espeak-ng.
- **Catalan** (`ca`): compatible with the espeak-ng voice `ca`; word table `ca.ucd` (8.1 k entries and a letter-to-sound model from 71.0 k words, 195 KB, feature `embed-ca`), number words, letter names and clause rules. Held-out PER 1.86 % on FLORES-200 (1.53 % without digits); words in the table 7.3 % wrong, other listed words 4.4 %, words outside the word list 41.8 % (553 words); 4.9× the speed of espeak-ng.
- **Romanian** (`ro`): compatible with the espeak-ng voice `ro`; word table `ro.ucd` (14.4 k entries and a letter-to-sound model from 92.4 k words, 308 KB, feature `embed-ro`), number words, letter names and clause rules. Held-out PER 1.00 % on FLORES-200 (0.89 % without digits); words in the table 0.1 % wrong, other listed words 0.0 %, words outside the word list 49.1 % (629 words); 3.3× the speed of espeak-ng.
- **Hungarian** (`hu`): compatible with the espeak-ng voice `hu`; word table `hu.ucd` (3.2 k entries and a letter-to-sound model from 152 k words, 46 KB, feature `embed-hu`), number words, letter names and clause rules. Held-out PER 2.08 % on FLORES-200 (1.61 % without digits); words in the table 2.7 % wrong, other listed words 6.6 %, words outside the word list 3.8 % (1119 words); 13.1× the speed of espeak-ng.
- **Finnish** (`fi`): compatible with the espeak-ng voice `fi`; word table `fi.ucd` (5.7 k entries and a letter-to-sound model from 153 k words, 137 KB, feature `embed-fi`), number words, letter names and clause rules. Held-out PER 1.64 % on FLORES-200 (1.13 % without digits); words in the table 5.7 % wrong, other listed words 4.5 %, words outside the word list 24.6 % (1938 words); 1.7× the speed of espeak-ng.
- **Norwegian Bokmål** (`nb`): compatible with the espeak-ng voice `nb`; word table `nb.ucd` (5.2 k entries and a letter-to-sound model from 85.8 k words, 100 KB, feature `embed-nb`), number words, letter names and clause rules. Held-out PER 1.11 % on FLORES-200 (0.52 % without digits); words in the table 1.3 % wrong, other listed words 0.0 %, words outside the word list 15.9 % (1119 words); 2.3× the speed of espeak-ng.
- **Danish** (`da`): compatible with the espeak-ng voice `da`; word table `da.ucd` (26.6 k entries and a letter-to-sound model from 81.4 k words, 490 KB, feature `embed-da`), number words, letter names and clause rules. Held-out PER 2.26 % on FLORES-200 (1.81 % without digits); words in the table 0.7 % wrong, other listed words 0.4 %, words outside the word list 68.2 % (1025 words); 27.1× the speed of espeak-ng.

- **Romanised text into Indian scripts** (`uccharan::translit`): `aap kaise ho` → `आप कैसे हो`,
  `mujhe kal office jana hai` → `मुझे कल office जाना है`. Each Latin word gets candidates from a
  lexicon of attested spellings and from an n-gram transliteration model, ranked by the model,
  by word frequency and by how alike the romanised word and the candidate sound (the
  sound-alike matcher's phoneme distance); one pass over the sentence decides per word between
  English and the language, with word-pair statistics for cases like मैं / में. English words
  stay in Latin letters by default, loanwords typed in English spelling included; `english`
  = `native` respells them in the script and `convert` takes every word for a romanised one.
  The result has spans with byte, code-point and UTF-16 offsets, a kind (`indic`, `english`,
  `other`) and the alternatives of each word. Rust (`translit::transliterate`,
  `normalize_romanised`), Python (`uccharan.transliterate`), JavaScript (`transliterate`,
  `loadTranslit`), C (`uccharan_transliterate`) and `uccharan -p native` / `-p native-json`.
  The data is in nine optional packs under **CC BY-SA 4.0** (`xlit-hi`, `xlit-mr`, `xlit-bn`,
  `xlit-ta`, `xlit-te`, `xlit-gu`, `xlit-pa`, `xlit-kn`, `xlit-ml`; 3 to 9 MB each, release
  assets, never fetched automatically), built by `lexgen xlit` from the Dakshina and
  Aksharantar word pairs; the library contains none of it and converts by rule without a
  pack. With the Hindi pack loaded, the `hinglish` option of `phonemize` uses it. Hindi has
  hand-written chat-spelling and homograph lists checked on development sentences; the lists
  of the other eight languages are hand-written and not yet reviewed by native speakers.
  See docs/transliteration.md and docs/evaluation.md.

### Changed

- **Faster word-table models.** The letter-to-sound model's run-time was rewritten: the hashes of
  a letter's contexts are computed together and shared by the sound and stress tables, a bit set
  built on first use (one bit per table entry present, about 0.3 MB of memory per language in
  use) answers for the contexts a table does not have, buckets are searched by bisection, and no
  text is checked per letter. A word of running text is remembered once instead of three times,
  and each word table has a key filter (`BLM1`, one byte per entry: files grow by 4 to 10 %) so
  that a word the table does not list costs no block decode. Output is unchanged on every
  development and held-out sentence of every language. Throughput against espeak-ng 1.52.0, one
  thread: German 2.0 → 5.8×, Dutch 1.4 → 3.2×, French 3.0 → 5.9×; with the
  earlier run-time Turkish, Czech, Finnish and Norwegian were slower than espeak-ng
  (0.9×, 0.7×, 0.7×, 0.9×) and are now 2.7×, 1.8×, 1.7× and 2.3×.
- `lexgen build` and `lexgen check` build the word tables side by side (`LEXGEN_JOBS`) and take
  about two minutes instead of twenty; `check` remembers under `target/lexgen-check` which tables
  it found up to date with unchanged inputs (`LEXGEN_NO_CACHE=1` checks afresh).
- The README and the package descriptions describe everything uccharan does: phonemiser, text
  normaliser, transliterator and pronunciation engine. The per-language notes moved to
  docs/languages.md and the comparison with other phonemisers to docs/comparison.md.
- The test suite also runs on Windows and macOS on every change.

### Fixed

- Python: `normalize(..., latin="transliterate")` and `transliterate(..., english="native")`
  load the British English data when the text has Latin-script words, as `phonemize` does.
  Without it English words were respelled from the American reading alone (`computer` came out
  as कम्प्यूरर instead of कम्प्यूटर) unless something else had loaded that data first.

## [0.3.0] - 2026-11-14

### Added

- **Tools for training voices and checking pronunciation** (`uccharan.train` in the Python
  package, and the `uccharan-train` command): `label` turns a dataset manifest (LJSpeech-style
  `metadata.csv`, Piper CSV, JSON lines, Hugging Face-style rows) into training-ready phonemes for
  Piper (`--data.phoneme_type text` CSV, phoneme-id CSV, `phonemes.json`), Matcha-TTS filelists
  and icefall-style recipes (`tokens.txt`, token lists), with a symbol table, a `voice.json` that
  records the uccharan version, mode, profile and language, single-code-point symbols throughout,
  a report of symbols outside a target table (`--symbols`, `--replace`, `--extend`), word lists,
  and per-utterance warnings (unknown characters, unread or long numbers, mixed scripts);
  `select` picks a compact reading script that covers every phone and as many diphones as
  possible (greedy set cover with length, digit, script and rare-symbol filters) and writes its
  coverage statistics; `coverage` reports phone and diphone counts, what is missing against a
  reference corpus, the duration per phone from TextGrid, JSON-lines or CTM alignments, and
  outlier utterances (too long, too many out-of-vocabulary words, wrong script, speaking rate);
  `score` computes the phoneme error of a recogniser's text or a phone recogniser's output
  against a reference with the sound-alike matcher's weighted distance, with a score per word and
  the mispronounced, missing and extra words; `mfa-dict` writes a Montreal Forced Aligner
  dictionary for a word list or corpus. In Rust, `uccharan::soundalike::score::align_words` gives
  the weighted alignment per reference word. See docs/training.md and examples/train.
- **Sound-alike matching** (`uccharan::soundalike`): correct speech-recognition output against a
  vocabulary of names, products and jargon by comparing phonemes ("Coat Harry" → "Kothari",
  "pay tm" → "Paytm", "कोठारी" → "Kothari"). A vocabulary index stores each term as its IPA, the
  reading of its spoken form, or uccharan's readings of its spelling (English in both modes,
  romanised Hindi, inner capitals split, initialisms spelled out); spans of one to three transcript
  words are compared with a weighted phoneme edit distance (voicing, aspiration, vowel length and
  retroflex / dental pairs cheap, schwa and stress nearly free, normalised by length) behind a
  consonant-skeleton pre-filter. Candidates carry byte, code-point and UTF-16 offsets, a score,
  whether the term is already there, and the kind of span (unknown word, uncertain, ordinary
  words); `apply` returns the corrected text with every change, with separate thresholds per kind
  and an optional per-word confidence gate. Rust, Python (`uccharan.SoundIndex`,
  `uccharan.match_terms`), JavaScript (`createSoundIndex`, `matchTerms`), C
  (`uccharan_sound_index_new`, `uccharan_match_terms`) and `uccharan --match-terms FILE`. A new
  evaluation (`tools/eval/soundalike`): 651 hand-written terms in English, Hindi and Hinglish
  sentences, spoken by synthetic voices and transcribed by small recognisers, plus hand-written
  mishearings and sentences without a term. Held-out, at the default thresholds: 43.9 % of
  the terms the recognisers got wrong are put back (64.2 % in English sentences) with 97.8 % of
  the corrections right, and 2 of 775 sentences without a term are changed; character edit
  distance puts back 8.8 %, Metaphone codes 18.7 % while changing 38 % of the sentences without a
  term. 7 800 transcripts a second against 1 000 terms, 1 500 against 10 000, 300 against 50 000.
- **Written number words in normalise-only mode** for Bengali, Gujarati, Kannada, Malayalam,
  Marathi, Nepali, Odia, Punjabi, Tamil, Telugu and Urdu: `normalize()` now spells numbers with
  the standard orthography of Unicode CLDR's spell-out rules (new `uccharan::rbnf`, rule files in
  `crates/uccharan/lang/numbers/`, written by `tools/cldr_rbnf.py`) instead of the phonemiser's
  voice-oriented number words: Tamil இருபத்து ஐந்து and இரண்டாயிரத்து இருபத்து நான்கு, Nepali तीन, Marathi
  टक्के, Bengali শতাংশ. Phonemisation is unchanged. Gold sets extended (bn, mr, ne, ta) and added
  (te, kn, ml, gu, pa, or, ur; `tools/eval/gold/norm`); held-out exact match after the change:
  bn 14/14, mr 14/14, ne 14/14, ta 14/14 (the first ten rows of each were scored for 0.2.0),
  te 7/7, kn 7/7, ml 7/7, gu 7/7, pa 7/7, ur 7/7, or 6/7.
- **Spanish** (`es`) on a new Latin-script engine (`latin.rs`; language files with letter rules,
  a stress module, clause-level rules across word boundaries, `[spell]` letter names, Roman
  numerals, `+`-joined number words): held-out PER 0.74 % against espeak-ng 1.52.0 `es` on
  FLORES-200 (0.45 % without digits), 2.3× faster. Improved mode: e for the voice's ɛ, ai ei oi
  au eu, d after l and n.
- **German** (`de`): on the Latin-script engine with German letter rules (vowel length from the spelling, unstressed prefixes, stressed particles and loan suffixes, final devoicing, r allophones), number words with units before tens and the decimal comma, and a word table (`de.ucd`, 16 k entries, 357 KB, feature `embed-de`) made by running espeak-ng 1.52.0 on a 139 k-word list. Held-out PER 1.01 % on FLORES-200 (0.82 % without digits); words of the word list 0.1 % wrong, words outside it 46.7 % (811 of 17,344 scored words); 2.4× faster. Improved mode writes ɐ for -er and aː for the voice's ɑː.
- **Dutch** (`nl`): Dutch letter rules (vowel length by open and closed syllable, unstressed prefixes and endings, -lijk, final devoicing, g and ch), stop and s assimilation across words inside a clause, number words (one-word tens, years counted in hundreds, decimal comma) and a word table (`nl.ucd`, 12 k entries, 231 KB, feature `embed-nl`; 80 k-word list). Held-out PER 1.77 % on FLORES-200 (1.39 % without digits); words of the word list 1.8 % wrong, words outside it 54.7 % (803 of 19,173 scored words); 1.8× faster. Improved mode marks i, y and u long before r.
- **French** (`fr`): French letter rules (silent final letters, nasal vowels, digraphs, schwa in open syllables, stress on the last full syllable), elided words read with the next word (`l'école`), liaison consonants kept as latent marks in the word table and said before a vowel inside a clause (`les amis` → `le-z amˈi`), the voice's clitic hyphen (`ʒə- nə- sˈɛ pˈa`), one-word number readings with stress on the last word, and a word table (`fr.ucd`, 12 k entries, 170 KB, feature `embed-fr`; 100 k-word list). Held-out PER 0.85 % on FLORES-200 (0.65 % without digits); words of the word list 1.6 % wrong, words outside it 32.4 % (330 of 21,347 scored words); 2.8× faster. Improved mode writes ɥ before i.
- **A letter-to-sound model in the word tables** of German, Dutch and French: back-off tables over
  letter contexts, trained on each language's full word list, with stress marks chosen so that the
  language's own stress rule gives the reference pattern. It reads the words a table does not
  list, and the tables keep only the words the model and the rules get wrong, which makes them
  half the size or less (German 748 → 357 KB). Words held out of the model's training list:
  German 62.1 → 69.0 % right, Dutch 48.6 → 59.4 %, French 83.7 → 86.4 %. The held-out figures
  above include the model (before it: German 1.29 %, Dutch 1.91 %, French 0.98 %; see
  docs/evaluation.md).

### Changed

- **Urdu and Sindhi**, words outside the tables: consonant pairs said without a vowel (نٹ, رڈ, ہ before
  a consonant, …), final ہ after a consonant as a weak vowel that keeps the short vowel before it,
  izafat kasra as eː, ی before و as eː, unstressed iː short, Sindhi on the Hindi-type deletion. The
  tables now hold only the words the rules get wrong, taken from 120 k Urdu and 61 k Sindhi corpus
  words (`ur.ucd` 47 k entries, 495 KB; `sd.ucd` 18 k entries, 192 KB). Held-out PER Urdu 2.18 →
  1.44 %, Sindhi 2.79 → 1.52 %. New letter attribute `weak` in language files.

### Fixed

- Python: `bho`, `mai` and `mag` raised "unknown pack" on first use (they are read with the
  Hindi data, which the wheel carries); they now work without a pack.
- Command line (the `uccharan` program, `python -m uccharan` and `uccharan-train`): standard input
  and output are UTF-8 on every platform; on Windows, phonemes could not be written to a console
  or pipe that used a legacy code page.

## [0.2.0] - 2026-11-14

Nineteen more South Asian languages; smaller, with modular language packs; and better at names.

### Added

- **Normalise-only mode** for voices that read text: `normalize()` in Rust, Python, JavaScript
  and C (`uccharan_normalize`), and `-p text` / `-p text-json` on the command line. Returns the
  spoken form, sentence ranges and typed spans aligned to the input (byte, code-point and UTF-16
  offsets). Options: digit language (context, request or fixed), lakh/crore or international
  English, Latin words kept, respelled or spelled, URLs and emails read, kept or dropped. New
  readings: `Rs.`/`INR`/`रु.` amounts, amounts with scale words (`₹5 करोड़`), Hindi clock and
  fractional idioms (साढ़े दस बजे, डेढ़ लाख), `+91` numbers, Indian-English lakh/crore, day-first
  dates in en-US when the first part cannot be a month. Gold sets for en, hi, mr, bn, ne and ta
  (`tools/eval/gold/norm`, `tools/eval/norm_eval.py`); held-out exact match en 45/45, hi 42/42,
  mr 9/10, bn 9/10, ne 8/10, ta 5/10.
- **Pronunciation word lists**: TSV format with word, language, IPA, spoken form and match rule
  (whole word, case, exact substring, regular expression), W3C PLS 1.0 import and export, ordered
  lists with first-list-wins priority. Applied in phonemisation (IPA converted to the profile's
  phone set, including Kokoro) and in normalise-only mode (spoken form, or a respelling chosen by
  reading candidates back with uccharan, for 13 scripts and English).

- **African names in English text** (improved mode): names spelled in African-language
  orthographies are recognised by spelling cues and read with their own letter values and
  penultimate stress, mapped to English phonemes (`uccharan::african`). A new hand-written gold
  set of 388 African names from 37 languages with sources per language
  (`tools/eval/gold/african_names.src`, built by `tools/eval/build_african_names.py`;
  `names_eval.py --gold`). Held-out: 70.8 % against espeak-ng's 45.6 % (41.0 % before); the
  general names set rises to 91.9 % (African 7/8 against espeak-ng's 4/8). Compatible mode is
  unchanged.
- **Egyptian, Chichewa, Shona, Tsonga and Tigrinya names** (improved mode): a list of 1 508
  Arabic given names from Wikidata (CC0) read with Arabic stress even where CMUdict has them with
  English stress; Egyptian spelling conventions; Ethiopian name elements; more Bantu onsets and
  clusters, Tsonga *x* and *rh*. Two new hand-written gold sets (167 Egyptian names; 224 Chichewa,
  Shona, Tsonga and Tigrinya names, the Tigrinya development half grown to 59). Held-out (second scoring): Egyptian 69.4 % against espeak-ng's 56.5 %;
  Chichewa/Shona/Tsonga/Tigrinya 87.5/73.1/73.7/70.4 % against 41.7/42.3/42.1/63.0 %. A list of
  Eritrean and Ethiopian name words from Wikidata (CC0) for Horn of Africa names.

- **Hindi number words in compatible mode** now follow espeak-ng's own readings of 0–99 (a table
  derived from running espeak-ng 1.52.0 on those numbers) and its one-word "एक सौ": held-out PER
  Hindi 0.12 → 0.09 %, Bhojpuri 0.80 → 0.51 %, Maithili 0.75 → 0.46 %, Sanskrit 0.95 → 0.64 %.
- **Magahi** (`mag`): compatible mode is the Hindi voice's reading (held-out PER 0.40 % against
  `hi` on FLORES-200); improved mode adds Magahi rules to the Bhojpuri ones (s for श ष, n for ण,
  -əi verb endings, final short ि ु, nasal vowel for ं after a long vowel, reduplication). Magahi
  gold set (512 words, 152 sentences; Wiktionary, FLORES-200 and grammar sources): held-out words
  98.5 % (Hindi voice 64.4 %), sentence PER 0.71 % (7.67 %).
- **Urdu and Sindhi** (`ur`, `sd`), the first Perso-Arabic-script languages: letter rules on the
  shared engine plus word tables (`ur.ucd`, 40 k words; `sd.ucd`, 20 k words; compiled in by the
  `embed-ur`/`embed-sd` features, fetched on demand in the browser) with espeak-ng 1.52.0's
  readings of frequent corpus words in context. Held-out PER 2.18 % (Urdu) and 2.79 % (Sindhi);
  4.6× and 2.7× faster than espeak-ng. Improved mode reads ص ظ as s z and the Urdu digits ۰–۹.
  Arabic-script punctuation (، ؛ ؟ ۔) ends clauses.
- **Punjabi, Odia, Sinhala, Konkani and Sanskrit** (`pa`, `or`, `si`, `kok`, `sa`) on the shared
  Brahmic engine. Held-out PER against espeak-ng 1.52.0: Punjabi 0.41 %, Odia 0.50 %, Sinhala
  0.86 % (FLORES-200), Konkani 0.20 % (Sangraha); Sanskrit has no espeak-ng voice, so compatible
  mode is the Hindi voice's reading (0.64 % against `hi`) and improved mode reads it as Sanskrit
  (no schwa deletion, classical stress rule, visarga echo, Sanskrit number words). 2.3–5.1× faster
  than espeak-ng. Engine additions: letter names with their own stress, spelled-out words triggered
  by a letter sequence, explicit letter kinds for scripts outside the common Indic layout, a word
  break after ZWJ conjuncts, more anusvara and deletion options, a number-word table per language.
- **Bhojpuri and Maithili** (`bho`, `mai`): compatible mode reproduces espeak-ng's Hindi-voice
  reading (no espeak-ng voice exists; held-out PER 0.51 % and 0.46 % against `hi` on FLORES-200),
  improved mode reads them with their own phonology (ऐ औ = əi əu, इ उ after a vowel, avagraha and
  its Latin-*s* spelling, flaps, nasal vowels, past -ल- forms, unstressed copula, Bhojpuri number
  words), Kaithi script support (`uccharan::kaithi`) and an experimental romanised-Bhojpuri word
  list. New hand-written gold sets (Bhojpuri 437 words and 153 sentences; Maithili 410 words, 134 with
  Wiktionary pronunciations, and 151 sentences; `tools/eval/build_bho_gold.py`,
  `tools/eval/bho_eval.py`). Held-out words: Bhojpuri 96.4 % against 62.9 % for the Hindi-voice
  fallback, Maithili 93.4 % against 68.6 %; sentence PER 0.43 % and 0.29 % against 5.54 % and
  5.66 %. 3.0–3.3× faster than espeak-ng's `hi` voice on the same text.
- **Bengali, Marathi, Telugu, Kannada, Malayalam, Nepali, Tamil, Gujarati and Assamese** (`bn`,
  `mr`, `te`, `kn`, `ml`, `ne`, `ta`, `gu`, `as`), compatible with the espeak-ng voices of the same names (held-out PER 0.37 %,
  0.27 %, 0.87 %, 1.58 %, 2.01 %, 2.67 %, 4.09 %, 2.41 %, 7.21 %), with improved-mode fixes for known reference
  quirks, every output profile, number reading, and routing of each script to its own language
  inside mixed text (with `(bn)`-style flags in the `espeak-ipa` profile). 2–37× faster than
  espeak-ng on the same sentences.
- **A data-driven language engine**: ordered context-sensitive rewrite rules
  (`uccharan::rules`), a language-file format (`uccharan::langdef`: letters, rule passes per
  mode, exceptions, unstressed words, number words with scale and combining forms) and one
  decoder for all Brahmic scripts (`uccharan::indic`: stress schemes, inherent-vowel deletion
  schemes, anusvara/visarga/geminate options). A new language is a `languages!` row plus a
  `.lang` file. `tools/eval/build_lang_sets.py` builds held-out sets per language (Tatoeba, or
  FLORES-200 for evaluation only); `tools/cldr_numbers.py` drafts number words from Unicode CLDR.

- **Optional WikiPron packs (CC BY-SA 4.0, not Apache-2.0)**: `wikipron-en-us`, `wikipron-en-gb`
  and `wikipron-hi`, built by `uccharan-lexgen wikipron` from Wiktionary pronunciations
  (via WikiPron) and rewritten into uccharan's symbols. They are separate release assets, never
  inside a package and never fetched unless asked for; each names its licence in its metadata.
  The improved mode uses them for words and names outside uccharan's own lexicon, taking the
  stress from uccharan's own reading (WikiPron's transcriptions carry none) and, where Wiktionary
  lists several pronunciations, the one closest to uccharan's. Compatible mode does not use them.
  Loading: `Phonemizer::load_pack` / `has_wikipron` (Rust), `uccharan.load_pack("wikipron-en-us")`
  (Python), `loadWikipron` (JavaScript), `uccharan_load_pack` (C), `--pack FILE` (command line).
  Held-out names with the pack: 88.4 % (89.0 % without); unseen gold sets 82.8 % (82.0 %).
- **Indian names in English text** (improved mode): a character-trigram classifier (trained on a
  hand-written list of Indian names against CMUdict's words) picks out unknown capitalised words
  that look Indian; they are read through uccharan's romanised-Hindi and Hindi rules and mapped
  to English phones. A names gold set of 380 names of many origins, with a held-out half, and
  `tools/eval/names_eval.py`, which reports accuracy per origin. Held-out names: 89.0 % against
  espeak-ng's 61.6 %; Indian names 77.1 % against 43.8 %.
- **Language packs.** The Python wheel carries en-US and Hindi only (1.1 MB instead of 5.5 MB);
  en-GB and the Kokoro English data are fetched from the GitHub release on first use, checked
  against SHA-256 sums that ship inside the wheel, and cached (`uccharan.load_pack`,
  `UCCHARAN_DATA_DIR`, `UCCHARAN_OFFLINE`). The npm package checks the packs it fetches the same
  way. Rust: `Phonemizer::load_pack(&self, bytes)` loads a pack into a shared phonemizer. The C
  library gained the same `embed-*` features as the Rust crate.
- `tools/bench/sizes.py`: sizes of the packs and the code next to espeak-ng's files, raw and
  with gzip, xz and zstd.
- The release workflow attaches the `.ucd` packs and their SHA-256 sums to the GitHub release.
- Benchmarks time each sentence once per fresh process after a warm-up on other text, with the
  two libraries taking turns (`tools/bench/compare.py`), so that uccharan's cache cannot flatter
  it.

### Changed

- **Data format 2** (packs from 0.1.0 are not readable): the lexicon is entropy-coded and leaves
  out every entry that the letter-to-sound tables reproduce exactly (the build checks that every
  word still comes out identical, and a compact set remembers which words were left out); the
  letter-to-sound tables are hash tables with 16-bit fingerprints. en-US data: 2.0 MB → 0.73 MB,
  en-GB 2.0 MB → 0.71 MB, Kokoro English 2.9 / 3.1 MB → 1.9 / 2.1 MB. Every word comes out as
  before the compression.
- A small per-thread cache of recent lexicon answers (4 096 words), a fast path for plain words
  in normalisation, and fenced lexicon blocks: faster than 0.1.0 despite the compressed data.
- Compatible mode, closer to espeak-ng (both accents): *couldn't / wouldn't / shouldn't / mustn't
  have* are joined as espeak-ng joins them; *beyond* has its /j/; *realize(d/s)*, *realization*,
  *strength(s)*, *length(s)* take the forms espeak-ng uses; *de-* before a vowel (*deodorant*) and
  possessives of names in *-y* (*Mary's*) keep a full /i/. Held-out en-US PER 1.19 % → 1.16 %,
  en-GB 1.41 % → 1.39 %.

### Fixed

- Tamil compatible mode: a hand-written list of Sanskrit and foreign loan stems that keep a voiced
  initial stop (பூமி, தேசிய, கவனம், பிரிட்டிஷ், …), compound boundaries before படு and after
  முன், the பிர-/கிர- cluster limited to k and p, and a word-initial glide ʲ merged into a preceding
  i or j (also Malayalam, Telugu). Held-out PER 4.41 % → 4.09 % (Malayalam 2.04 % → 2.01 %).
- Hindi compatible mode: before a hyphen, a word of two or more syllables ending in …CəC keeps its
  final inherent vowel as the reference does (अलग-अलग), and a vowel letter अ inside a word is ə
  unless stressed (जंतुअन). Hindi development PER 0.15 % → 0.14 %.
- A doubled *k* outside the English lexicon was dropped by the letter-to-sound model (*Mekki*
  read as *mˈɪi*); it is now read like *ck*.

## [0.1.0]

First version.

### Added

- Rust core (`uccharan`): text segmentation, number and symbol normalisation, English (General
  American and Received Pronunciation) from a CMUdict-derived lexicon with morphology,
  letter-to-sound tables and clause-level weak forms; Hindi from Devanagari rules with schwa
  deletion; code-mixed Hindi and English routed word by word; experimental romanised Hindi.
- Two behaviours: `compatible` (matches the phoneme streams espeak-trained voices expect) and
  `improved` (homographs in context, a names and brands list, context-aware abbreviations, natural
  numbers, morpheme-aware Hindi schwa deletion, homorganic nasals, plain-IPA Hindi symbols).
- Output profiles: espeak-style IPA, Piper (sentence strings and ids from a voice's
  `phoneme_id_map`), sherpa-onnx codepoints, Kokoro tokens and ids, Kitten TTS and Matcha-TTS ids.
- Kokoro English front end: a port of misaki's English G2P (Apache-2.0) with a rule-based
  tagger in place of spaCy and uccharan's own rules for words outside misaki's dictionaries.
  misaki 0.9.4's dictionaries are built in; another copy can be loaded with `load_misaki`,
  `--misaki`, `loadMisaki` or `uccharan_load_misaki`.
- Bindings: C ABI with a generated header, Python (abi3 wheels, typed, a small stand-in for
  `phonemizer`'s `EspeakBackend`), WebAssembly (npm package for browsers and Node).
- Command-line program `uccharan` (with `--ids`, `--threads`, `--misaki`).
- Batch API on native threads (`render_batch`, `uccharan_phonemize_lines`).
- Evaluation tools: agreement with a reference phonemizer, hand-curated gold sets, per-profile
  conformance checks against real model files, an end-to-end synthesis and recognition check, and
  speed comparisons. Results of the runs behind the published numbers are in `tools/eval/results`
  and `tools/bench/results`.
- Live demo (static site, WebAssembly).
