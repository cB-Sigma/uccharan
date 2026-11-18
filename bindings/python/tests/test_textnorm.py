"""Normalise-only mode and word lists from Python."""

import pytest
import uccharan

TSV = (
    "# uccharan wordlist 1\n"
    "word\tlang\tipa\tsay\tmatch\tnote\n"
    "Kokoro\ten\tkəkˈɔːɹoʊ\t\t\tproduct\n"
    "SBI\thi\t\tएस बी आई\tcase\t\n"
)


def test_normalize_spans_use_code_points() -> None:
    text = "Pay ₹500 😀 by 10:30."
    n = uccharan.normalize(text, "en-us")
    assert n.text == "Pay five hundred rupees by ten thirty."  # the emoji is dropped
    money = next(s for s in n.spans if s.kind == "money")
    assert text[money.src[0] : money.src[1]] == "₹500"
    assert n.text[money.out[0] : money.out[1]] == "five hundred rupees"
    # Spans tile the input and the outputs rebuild the text.
    assert "".join(s.text for s in n.spans) == text
    assert "".join(s.spoken for s in n.spans) == n.text
    time = next(s for s in n.spans if s.kind == "time")
    assert time.src_utf16[0] == time.src[0] + 1  # the emoji is two UTF-16 units
    b = uccharan.normalize(text, "en-us", byte_offsets=True)
    m = next(s for s in b.spans if s.kind == "money")
    assert text.encode()[m.src[0] : m.src[1]].decode() == "₹500"
    assert [n.text[a:z] for a, z in n.sentences] == [n.text]


def test_normalize_options() -> None:
    assert uccharan.normalize("कुल 12 students", "hi").text == "कुल twelve students"
    assert (
        uccharan.normalize("कुल 12 students", "hi", digits="request").text
        == "कुल बारह students"
    )
    assert uccharan.normalize("UPI से", "hi", latin="transliterate").text == "यू पी आई से"
    assert uccharan.normalize("5,00,000 votes", "en-us").text == "five lakh votes"
    assert (
        uccharan.normalize("500000 votes", "en-us", en_scale="indian").text
        == "five lakh votes"
    )
    assert (
        uccharan.normalize("see www.a.com", "en-us", urls="keep").text
        == "see www.a.com"
    )
    with pytest.raises(ValueError):
        uccharan.normalize("x", "en-us", latin="bogus")


def test_word_lists() -> None:
    wl = uccharan.WordList(TSV)
    assert len(wl) == 2
    assert wl.entries()[1]["say"] == "एस बी आई"
    n = uccharan.normalize("SBI में", "hi", wordlists=[wl])
    assert n.text == "एस बी आई में"
    assert n.spans[0].kind == "wordlist" and n.spans[0].entry == (0, 1)
    ipa = uccharan.phonemize_wordlists("Kokoro", "en-us", wordlists=[wl])
    assert ipa.startswith("kəkˈɔːɹoʊ")
    pls = wl.to_pls("en-us")
    back = uccharan.WordList(pls)
    assert back.entries()[0]["ipa"] == "kəkˈɔːɹoʊ"
    inline = uccharan.WordList.from_entries([{"word": "Kokoro", "say": "koko"}])
    assert uccharan.normalize("Kokoro", "en-us", wordlists=[inline, wl]).text == "koko"
    with pytest.raises(ValueError, match="line 2"):
        uccharan.WordList("word\tlang\nx\tnot-a-lang\n")


def test_respell() -> None:
    r = uccharan.respell("kəkˈɔːɹoʊ", "hi", source="en-us")
    assert r is not None and r.text == "ककॉरो" and r.distance <= 0.1


def test_word_list_shapes_match_phonemize() -> None:
    wl = uccharan.WordList("Kokoro\ten\tkəkˈɔːɹoʊ\n")
    plain = uccharan.phonemize("Hi. Bye!", "en-us", "piper")
    assert (
        uccharan.phonemize_wordlists(
            "Hi. Bye!", "en-us", profile="piper", wordlists=[wl]
        )
        == plain
    )
    cps = uccharan.phonemize_wordlists(
        "Kokoro. Hi!", "en-us", profile="sherpa-onnx", wordlists=[wl]
    )
    assert isinstance(cps, list) and len(cps) == 2
    assert "".join(map(chr, cps[0])).startswith("kəkˈɔːɹoʊ")


def test_respelling_loads_british_data_in_a_fresh_process() -> None:
    # English words in Indic text are respelled from the British reading; a
    # fresh interpreter must load that data without phonemize() running first.
    import subprocess
    import sys

    code = (
        "import uccharan\n"
        "t = uccharan.normalize('office computer', 'hi', latin='transliterate').text\n"
        "assert t == '\\u0911\\u092b\\u093c\\u093f\\u0938 \\u0915\\u092e\\u094d\\u092a\\u094d\\u092f\\u0942\\u091f\\u0930', ascii(t)\n"
        "t = uccharan.transliterate('office', 'hi', english='native').text\n"
        "assert t == '\\u0911\\u092b\\u093c\\u093f\\u0938', ascii(t)\n"
        "print('ok')\n"
    )
    out = subprocess.run([sys.executable, "-c", code], capture_output=True, text=True, encoding="utf-8")
    assert out.returncode == 0 and out.stdout.strip() == "ok", out.stderr


def test_normalize_latin_languages() -> None:
    # Each Latin-script language reads its numbers in its own words.
    assert uccharan.normalize("Ich habe 23 Euro.", "de").text == "Ich habe dreiundzwanzig Euro."
    assert uccharan.normalize("Il a 71 ans.", "fr").text == "Il a soixante et onze ans."
    assert uccharan.normalize("Il a 71 ans.", "fr-be").text == "Il a septante et un ans."
    assert uccharan.normalize("Il a 71 ans.", "fr-ch").text == "Il a septante et un ans."
    assert uccharan.normalize("Jeg har 95 kroner.", "da").text == "Jeg har femoghalvfems kroner."
    # "1 500": the groups apart in compatible mode, as the reference voice
    # reads them; one number in improved mode.
    assert uccharan.normalize("Tengo 1 500 libros.", "es").text == "Tengo uno quinientos libros."
    assert (
        uccharan.normalize("Tengo 1 500 libros.", "es", improved=True).text
        == "Tengo mil quinientos libros."
    )
    n = uccharan.normalize("Ich habe 23 Euro.", "de", digits="request")
    assert n.text == "Ich habe dreiundzwanzig Euro."
    number = next(s for s in n.spans if s.kind == "number")
    assert number.lang == "de" and number.value == "23"


# Expected Unicode block of each non-Latin language's output.
_BLOCKS = {
    **dict.fromkeys(["hi", "mr", "ne", "kok", "sa", "bho", "mai", "mag", "doi", "brx"], (0x0900, 0x097F)),
    **dict.fromkeys(["bn", "as"], (0x0980, 0x09FF)),
    "pa": (0x0A00, 0x0A7F),
    "gu": (0x0A80, 0x0AFF),
    "or": (0x0B00, 0x0B7F),
    "ta": (0x0B80, 0x0BFF),
    "te": (0x0C00, 0x0C7F),
    "kn": (0x0C80, 0x0CFF),
    "ml": (0x0D00, 0x0D7F),
    "si": (0x0D80, 0x0DFF),
    **dict.fromkeys(["ur", "sd"], (0x0600, 0x06FF)),
    "sat": (0x1C50, 0x1C7F),
    "mni": (0xABC0, 0xABFF),
}

_TWENTY_FIVE = {
    "hi": "पच्चीस",
    "bn": "পঁচিশ",
    "mr": "पंचवीस",
    "te": "ఇరవై ఐదు",
    "ne": "पच्चीस",
    "kn": "ಇಪ್ಪತ್ತೈದು",
    "ta": "இருபத்து ஐந்து",
    "ml": "ഇരുപത്തിയഞ്ച്",
    "as": "পঁচিশ",
    "gu": "પચ્ચીસ",
    "pa": "ਪੱਚੀ",
    "or": "ପଚିଶ",
    "kok": "पंचवीस",
    "si": "විසිපහ",
    "sa": "पच्चीस",
    "ur": "پچیس",
    "sd": "پنجويهه",
    "bho": "पच्चीस",
    "mai": "पच्चीस",
    "mag": "पच्चीस",
    "doi": "पच्चीस",
    "brx": "पच्चीस",
    "sat": "ᱵᱟᱨ ᱜᱮᱞ ᱢᱚᱬᱮ",
    "mni": "ꯀꯨꯟꯃꯉꯥ",
}


def test_normalize_indic_languages() -> None:
    # Every non-Latin language writes numbers as words of its own script.
    for lang, (lo, hi) in _BLOCKS.items():
        assert uccharan.normalize("25", lang).text == _TWENTY_FIVE[lang], lang
        for text in ["25", "1,234", "3.5", "₹500", "15/08/2024"]:
            for improved in (False, True):
                out = uccharan.normalize(text, lang, improved=improved).text
                assert out, (lang, text)
                for c in out:
                    assert c in " ‌‍" or lo <= ord(c) <= hi, (lang, text, out, c)
    # Sinhala and Sindhi: words, not keys of the phonemiser's word table.
    assert uccharan.normalize("1,234", "si").text == "එක්දහස් දෙසිය තිස්හතර"
    assert uccharan.normalize("100 1000 50%", "si").text == "සියය දහස පනහ ප්\u200dරතිශතය"
    # Konkani: Goan Konkani number words, not Marathi ones.
    assert uccharan.normalize("25 9 100", "kok").text == "पंचवीस णव शंबर"
    assert uccharan.normalize("3.5", "si").text == "තුන දශම පහ"
    assert uccharan.normalize("1,234", "sd").text == "هڪ هزار ٻه سؤ چوٽيهه"
    assert uccharan.normalize("₹500", "sd").text == "پنج سؤ رپيا"
    # The words read as the digits do.
    assert uccharan.phonemize(uccharan.normalize("13", "si").text, "si") == uccharan.phonemize("13", "si")


def test_normalize_dates_and_ordinals() -> None:
    # Dates and ordinals are read in full in every mode of normalise-only mode.
    assert uccharan.normalize("15.08.2024", "de").text == "fünfzehnter August zweitausend vierundzwanzig"
    assert uccharan.normalize("le 1er mai", "fr").text == "le premier mai"
    assert uccharan.normalize("200 000", "fr").text == "deux cent mille"
    assert uccharan.normalize("el 2º", "es").text == "el segundo"
    assert uccharan.normalize("15/08/2024", "es").text == "quince de agosto de dos mil veinticuatro"
    assert uccharan.normalize("Es kostet 25,50 €.", "de").text == "Es kostet fünfundzwanzig Euro fünfzig."
    assert uccharan.normalize("Am 3. Mai", "de").text == "Am dritten Mai"


def test_normalize_times_money_and_sentence_ends() -> None:
    # Clock times, plural currency words, ordinals before a hyphen, and a
    # number before a full stop that ends the sentence.
    assert uccharan.normalize("Il est 10h30.", "fr").text == "Il est dix heures trente."
    assert uccharan.normalize("às 10h30", "pt").text == "às dez horas e trinta"
    assert uccharan.normalize("um 10.30 Uhr", "de").text == "um zehn Uhr dreißig"
    assert uccharan.normalize("Kello on 10.30.", "fi").text == "Kello on kymmenen kolmekymmentä."
    assert (
        uccharan.normalize("Ich kaufe das iPhone 15. Am Montag kommt es.", "de").text
        == "Ich kaufe das iPhone fünfzehn. Am Montag kommt es."
    )
    assert uccharan.normalize("the 3rd-floor flat", "en-us").text == "the third floor flat"
    assert uccharan.normalize("25 €", "fr").text == "vingt-cinq euros"
    assert uccharan.normalize("25 €", "es").text == "veinticinco euros"
    assert uccharan.normalize("25 €", "it").text == "venticinque euro"
    assert uccharan.normalize("25 €", "pt").text == "vinte e cinco euros"
    spans = uccharan.normalize("à 10 h 30", "fr").spans
    assert any(s.kind == "time" for s in spans)
