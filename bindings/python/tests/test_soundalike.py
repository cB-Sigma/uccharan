"""Sound-alike matching from Python."""

import pytest
import uccharan


def test_apply_reports_every_change() -> None:
    index = uccharan.SoundIndex(["Kothari", "Paytm", "Natarajan"])
    assert len(index) == 3 and index.terms()[0] == "Kothari"
    text = "Mr. Nutter Rajan and Coat Harry use pay tm 😀 daily"
    fixed = index.apply(text, "en-us")
    assert fixed.text == "Mr. Natarajan and Kothari use Paytm 😀 daily"
    assert [(c.old, c.new) for c in fixed.changes] == [("Nutter Rajan", "Natarajan"), ("Coat Harry", "Kothari"), ("pay tm", "Paytm")]
    for c in fixed.changes:
        assert text[c.src[0] : c.src[1]] == c.old
        assert fixed.text[c.out[0] : c.out[1]] == c.new
        assert 0.8 <= c.score <= 1.0 and not c.case_only
    # Ordinary text and exact terms stay as they are.
    same = index.apply("Mr. Kothari walked to the station.", "en-us")
    assert same.changes == [] and same.candidates[0].exact


def test_find_offsets_and_cross_script() -> None:
    cands = uccharan.match_terms("😀 कोठारी जी", ["Kothari"], "hi")
    assert [(c.term, c.text) for c in cands] == [("Kothari", "कोठारी")]
    assert cands[0].src == (2, 8) and cands[0].src_utf16 == (3, 9) and not cands[0].exact
    by_bytes = uccharan.match_terms("😀 कोठारी जी", ["Kothari"], "hi", byte_offsets=True)
    assert by_bytes[0].src == (5, 23)


def test_word_list_and_confidence() -> None:
    wl = uccharan.WordList("# uccharan wordlist 1\nword\tlang\tipa\tsay\nnginx\ten\t\tengine X\n")
    index = uccharan.SoundIndex(wl)
    text = "restart engine x now"
    assert index.apply(text, "en-us").text == "restart nginx now"
    sure = index.apply(text, "en-us", confidence=[0.99, 0.98, 0.97, 0.99], max_confidence=0.6)
    assert sure.text == text and len(sure.candidates) == 1
    unsure = index.apply(text, "en-us", confidence=[0.99, 0.4, 0.5, 0.99], max_confidence=0.6)
    assert unsure.text == "restart nginx now"
    with pytest.raises(ValueError):
        index.apply(text, "en-us", confidence=[0.5])
    assert uccharan.SoundIndex([{"word": "Jio", "ipa": "dʒˈiːoʊ"}]).sounds(0) == ["dʒiːoː"]
