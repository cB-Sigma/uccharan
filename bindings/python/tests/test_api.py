import json
import subprocess
import sys
import unicodedata

import pytest

import uccharan
from uccharan.compat import EspeakBackend


def test_version_and_constants():
    assert uccharan.__version__.count(".") == 2
    assert "hi" in uccharan.LANGUAGES and "sherpa-onnx" in uccharan.PROFILES


def test_english_basic():
    assert uccharan.phonemize("Hello, world.") == "həlˈoʊ, wˈɜːld."
    assert uccharan.phonemize("Hello, world.", punctuation=False) == "həlˈoʊ wˈɜːld"
    assert uccharan.phonemize("The car park is far.", "en-gb") == "ðə kˈɑː pˈɑːk ɪz fˈɑː."


def test_hindi_and_code_mixing():
    nfd = lambda s: unicodedata.normalize("NFD", s)  # noqa: E731
    assert nfd(uccharan.phonemize("नमस्ते दुनिया", "hi")) == nfd("nəmˈʌsteː dˈʊnɪjˌaː")
    mixed = uccharan.analyze("मैं office जा रहा हूँ", "hi")
    langs = [w.lang for w in mixed[0][0][0]]
    assert langs == ["hi", "en-gb", "hi", "hi", "hi"]
    flagged = uccharan.phonemize("मैं office जा रहा हूँ", "hi", language_flags=True)
    assert "(en)" in flagged and "(hi)" in flagged
    roman = uccharan.phonemize("main ghar ja raha hoon", "hi", hinglish=True)
    assert nfd(roman) == nfd(uccharan.phonemize("मैं घर जा रहा हूँ", "hi"))


def test_profiles_shapes():
    s = uccharan.phonemize("Hi there. Bye!", "en-us", "piper")
    assert s == ["hˈaɪ ðˈɛɹ.", "bˈaɪ!"]
    cps = uccharan.phonemize("Hi there. Bye!", "en-us", "sherpa-onnx")
    assert cps == [[ord(c) for c in x] for x in s]
    k = uccharan.phonemize("The butter is better.", "en-us", "kokoro")
    assert "T" in k and uccharan.unmapped(k, "kokoro") == []
    ids = uccharan.kokoro_ids(k)
    assert ids[0] == 0 and ids[-1] == 0 and len(ids) == len(k) + 2
    m = uccharan.phonemize("Hello, world.", "en-us", "matcha")
    assert uccharan.matcha_ids(m)[0::2] == [0] * (len(m) + 1)
    assert uccharan.kitten_ids(m)[-2:] == [10, 0]


def test_piper_ids_from_dict():
    cfg = {"phoneme_id_map": {"_": [0], "^": [1], "$": [2], " ": [3], "h": [20], "a": [14], "ɪ": [74], "ˈ": [120], ".": [10]}}
    ids, missing = uccharan.piper_ids("hˈaɪ.", cfg)
    assert ids == [1, 0, 20, 0, 120, 0, 14, 0, 74, 0, 10, 0, 2] and missing == []
    ids, missing = uccharan.piper_ids("hˈaɪ.", json.dumps(cfg))
    assert missing == []
    _, missing = uccharan.piper_ids("xa", cfg)
    assert missing == ["x"]


def test_normalisation_modes():
    assert uccharan.phonemize("$5", normalize="natural") == uccharan.phonemize("five dollars")
    assert uccharan.phonemize("5", "hi") == uccharan.phonemize("पांच", "hi")
    assert uccharan.phonemize("abc 123", normalize="off") == uccharan.phonemize("abc")


def test_batch_matches_single():
    texts = [f"Sentence number {i} is here." for i in range(40)]
    assert uccharan.phonemize_batch(texts) == [uccharan.phonemize(t) for t in texts]


def test_errors():
    with pytest.raises(ValueError):
        uccharan.phonemize("x", lang="xx")
    with pytest.raises(ValueError):
        uccharan.phonemize("x", profile="nope")  # type: ignore[call-overload]
    with pytest.raises(ValueError):
        uccharan.phonemize("x", normalize="nope")
    assert uccharan.phonemize("") == ""
    assert uccharan.phonemize("...", profile="piper") == []


def test_phonemizer_shim():
    b = EspeakBackend("en-us", preserve_punctuation=True, with_stress=True)
    assert b.phonemize(["Hello, world."], strip=True) == ["həlˈoʊ, wˈɜːld."]
    assert b.phonemize(["Hello"]) == ["həlˈoʊ "]
    plain = EspeakBackend("en-us")
    assert plain.phonemize(["Hello, world."], strip=True) == ["həloʊ wɜːld"]
    tied = EspeakBackend("en-us", with_stress=True, tie="^")
    assert tied.phonemize(["church"], strip=True) == ["t^ʃˈɜːt^ʃ"]
    with pytest.raises(ValueError):
        EspeakBackend("fr-fr")
    with pytest.raises(TypeError):
        b.phonemize("Hello")  # type: ignore[arg-type]


def test_cli():
    out = subprocess.run([sys.executable, "-m", "uccharan", "-l", "hi", "-p", "piper", "नमस्ते।"], capture_output=True, text=True, encoding="utf-8", check=True)
    assert json.loads(out.stdout) == [unicodedata.normalize("NFD", "nəmˈʌsteː.")]
    out = subprocess.run([sys.executable, "-m", "uccharan"], input="Hello\nworld\n", capture_output=True, text=True, encoding="utf-8", check=True)
    assert out.stdout.splitlines() == ["həlˈoʊ", "wˈɜːld"]


def test_kokoro_english_pack_and_override(tmp_path):
    assert uccharan.phonemize("I read the book yesterday.", "en-us", "kokoro") == "ˌI ɹˈɛd ðə bˈʊk jˈɛstəɹdˌA."
    assert uccharan.phonemize("Hello.", "en-gb", "kokoro") == "həlˈQ."
    assert uccharan.has_misaki("en-us") and uccharan.has_misaki("en-gb")
    # A different copy of misaki's dictionaries can replace the built-in one.
    (tmp_path / "gb_gold.json").write_text(json.dumps({"hello": "hɛlˈQ"}, ensure_ascii=False), encoding="utf-8")
    (tmp_path / "gb_silver.json").write_text("{}", encoding="utf-8")
    uccharan.load_misaki(tmp_path, "en-gb")
    assert uccharan.phonemize("Hello.", "en-gb", "kokoro") == "hɛlˈQ."
    with pytest.raises(ValueError):
        uccharan.load_misaki(tmp_path, "en-gb")


def test_packs_are_verified(tmp_path, monkeypatch):
    from uccharan import packs

    assert set(packs.PACKS) >= {"en-gb", "kokoro-en-us", "kokoro-en-gb"}
    assert uccharan.load_pack("en-gb") == "en-gb"
    # A file with the right name but the wrong content is refused.
    (tmp_path / "kokoro-en-gb.ucd").write_bytes(b"UCD1\0\0\0\0")
    monkeypatch.setenv("UCCHARAN_DATA_DIR", str(tmp_path))
    monkeypatch.setenv("XDG_CACHE_HOME", str(tmp_path / "cache"))
    with pytest.raises(uccharan.PackError, match="SHA-256"):
        packs.fetch("kokoro-en-gb")
    # Offline and nowhere to be found: a clear error that names the URL.
    (tmp_path / "kokoro-en-gb.ucd").unlink()
    with pytest.raises(uccharan.PackError, match="releases/download"):
        packs.fetch("kokoro-en-gb")
    with pytest.raises(uccharan.PackError, match="unknown pack"):
        packs.fetch("zz")


def test_a_missing_word_table_is_an_error_not_a_silent_fallback(tmp_path):
    # In a fresh process without the German table: phonemize() raises an
    # error that names the pack; English and Hindi still work.
    import os
    import subprocess
    import sys

    code = (
        "import uccharan\n"
        "from uccharan.packs import PackError\n"
        "assert uccharan.phonemize('hello', lang='en-us')\n"
        "assert uccharan.phonemize('नमस्ते', lang='hi')\n"
        "try:\n"
        "    uccharan.phonemize('Guten Tag', lang='de')\n"
        "except PackError as e:\n"
        "    assert \"'de'\" in str(e) and 'de.ucd' in str(e) and 'UCCHARAN_DATA_DIR' in str(e), e\n"
        "else:\n"
        "    raise SystemExit('German was read without its word table')\n"
    )
    env = {**os.environ, "UCCHARAN_DATA_DIR": str(tmp_path), "XDG_CACHE_HOME": str(tmp_path), "UCCHARAN_OFFLINE": "1"}
    r = subprocess.run([sys.executable, "-c", code], env=env, capture_output=True, text=True)
    assert r.returncode == 0, r.stderr


def test_packs_command_copies_packs_for_offline_use(tmp_path, capsys):
    from uccharan import packs

    assert packs.main(["--dir", str(tmp_path), "de", "fr"]) == 0
    assert sorted(f.name for f in tmp_path.iterdir()) == ["de.ucd", "fr.ucd"]
    assert packs.main([]) == 0
    listing = capsys.readouterr().out
    assert "de " in listing and "xlit-hi" in listing and "CC BY-SA 4.0" in listing
    assert packs.installed("de") and not packs.installed("no-such")
