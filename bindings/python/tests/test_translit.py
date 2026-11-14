"""Romanised text into native script from Python."""

from pathlib import Path

import pytest
import uccharan

TOY = Path(__file__).resolve().parents[3] / "crates" / "uccharan" / "tests" / "data" / "xlit-toy.ucd"


def test_without_a_pack_rules_answer() -> None:
    # Bengali has no pack in this test run.
    t = uccharan.transliterate("kamal", "bn", english="convert")
    assert t.text == "কমল"
    assert not t.pack
    assert not uccharan.has_translit("bn")


def test_transliterate_with_a_pack() -> None:
    assert uccharan.load_pack(TOY) == "xlit-hi"
    assert uccharan.has_translit("hi")
    text = "mujhe kal the jana hai 😀!"
    t = uccharan.transliterate(text, "hi")
    assert t.pack
    assert t.text == "मुझे कल the जाना है 😀!"
    # Spans tile the input and the outputs rebuild the text.
    assert "".join(s.text for s in t.spans) == text
    assert "".join(s.written for s in t.spans) == t.text
    kal = next(s for s in t.spans if s.text == "kal")
    assert kal.kind == "indic" and kal.written == "कल"
    assert text[kal.src[0] : kal.src[1]] == "kal" and t.text[kal.out[0] : kal.out[1]] == "कल"
    assert kal.alternatives and kal.alternatives[0].text == "कल"
    assert next(s for s in t.spans if s.text == "the").kind == "english"
    tail = t.spans[-1]
    assert tail.kind == "other" and tail.src_utf16[1] == tail.src[1] + 1  # the emoji is two UTF-16 units
    b = uccharan.transliterate(text, "hi", byte_offsets=True)
    k = next(s for s in b.spans if s.text == "kal")
    assert text.encode()[k.src[0] : k.src[1]].decode() == "kal"


def test_english_policies_and_errors() -> None:
    uccharan.load_pack(TOY)
    assert "the" not in uccharan.transliterate("kal the jana hai", "hi", english="convert").text
    native = uccharan.transliterate("kal the jana hai", "hi", english="native").text
    assert not any(c.isascii() and c.isalpha() for c in native)
    with pytest.raises(ValueError):
        uccharan.transliterate("kal", "hi", english="maybe")
    with pytest.raises(ValueError):
        uccharan.transliterate("kal", "xx")
