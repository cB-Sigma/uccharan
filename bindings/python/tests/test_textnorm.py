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
