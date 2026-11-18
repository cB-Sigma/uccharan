"""The data-driven Indic languages need no pack and route Latin words to English."""

import uccharan


def test_indic_languages():
    assert uccharan.phonemize("কমলা", lang="bn") == uccharan.phonemize("কমলা", lang="bn-bd")
    assert uccharan.phonemize("मराठी", lang="mr").startswith("məɾ")
    assert "bn" in uccharan.LANGUAGES and "te" in uccharan.LANGUAGES
    ids = uccharan.phonemize("తెలుగు", lang="te", profile="sherpa-onnx")
    assert ids and ids[0]


def test_dogri():
    # Compatible mode is the Hindi reading; improved mode reads Dogri tone.
    assert "doi" in uccharan.LANGUAGES
    assert uccharan.phonemize("घर", lang="doi") == uccharan.phonemize("घर", lang="hi")
    assert uccharan.phonemize("घर", lang="dogri", improved=True) == "kˈʌ̀ɾ"
    assert uccharan.phonemize("ज्हार", lang="doi", improved=True) == "ɟˈàːɾ"
    assert uccharan.normalize("25", "doi", improved=True).text == "पंजी"
    assert uccharan.transliterate("tusen kiyan ho", "doi").text == "तुसें कि’यां हो"


def test_bodo():
    # Compatible mode is the Hindi reading; improved mode reads Bodo (ो = ɯ,
    # the apostrophe = the inherent vowel ɔ).
    assert "brx" in uccharan.LANGUAGES
    assert uccharan.phonemize("बोलो", lang="brx") == uccharan.phonemize("बोलो", lang="hi")
    assert uccharan.phonemize("बोलो", lang="bodo", improved=True) == "bˈɯlɯ"
    assert uccharan.phonemize("बर’", lang="brx", improved=True) == "bˈɔrɔ"
    assert uccharan.normalize("25", "brx", improved=True).text == "नैजिबा"
    assert uccharan.transliterate("ang boro raw bungw", "brx").text == "आं बर’ राव बुङो"
