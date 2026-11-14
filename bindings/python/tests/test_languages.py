"""The data-driven Indic languages need no pack and route Latin words to English."""

import uccharan


def test_indic_languages():
    assert uccharan.phonemize("কমলা", lang="bn") == uccharan.phonemize("কমলা", lang="bn-bd")
    assert uccharan.phonemize("मराठी", lang="mr").startswith("məɾ")
    assert "bn" in uccharan.LANGUAGES and "te" in uccharan.LANGUAGES
    ids = uccharan.phonemize("తెలుగు", lang="te", profile="sherpa-onnx")
    assert ids and ids[0]
