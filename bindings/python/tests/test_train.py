"""The training tools (uccharan.train): no audio, no downloads."""

import doctest
import importlib
import json
import unicodedata
from pathlib import Path

import pytest
import uccharan
from uccharan import train
from uccharan.train import SymbolTable, Utterance
from uccharan.train.cli import main as cli

FIX = Path(__file__).parent / "fixtures" / "train"
ROOT = Path(__file__).resolve().parents[3]


# -- manifests ----------------------------------------------------------------


def test_manifest_layouts():
    lj = train.read_manifest(FIX / "metadata.csv")
    assert [u.id for u in lj] == ["LJ001-0001", "LJ001-0002", "LJ001-0003", "LJ001-0004"]
    assert lj[1].text == "घर में 25 घोड़े हैं।" and lj[1].line == 2
    assert train.read_manifest(FIX / "metadata.csv", text_column=2)[1].text == "घर में पच्चीस घोड़े हैं"

    piper = train.read_manifest(FIX / "piper.csv", layout="piper")
    assert [(u.id, u.audio, u.speaker) for u in piper] == [("a", "a.wav", "anita"), ("b", "b.wav", "ravi")]

    rows = train.read_manifest(FIX / "rows.jsonl")
    assert rows[0] == Utterance("one", "रउआ कइसन बानी?", audio="clips/one.flac", lang="bho", duration=1.9, line=1)
    assert rows[1].speaker == "7" and rows[1].lang == "hi"

    with pytest.raises(ValueError, match="no text column"):
        train.from_rows([{"audio": "x.wav"}])
    with pytest.raises(ValueError, match="line 1"):
        train.read_manifest(FIX / "tokens.txt", layout="ljspeech")


# -- symbols ------------------------------------------------------------------


def test_symbol_tables():
    built = SymbolTable.from_symbols("nə mˈ")
    assert list(built.ids)[:4] == ["_", "^", "$", " "]
    assert built.encode("nə", framing="piper") == [1, 0, built.ids["n"], 0, built.ids["ə"], 0, 2]
    assert built.encode("nə", framing="blank") == [0, built.ids["n"], 0, built.ids["ə"], 0]
    assert built.encode("nXə", framing="plain") == [built.ids["n"], built.ids["ə"]]

    voice = SymbolTable.from_file(FIX / "base_voice.json")
    assert voice.ids["ʰ"] == 145 and voice.num_symbols == 162
    assert voice.multi_codepoint() == ["aɪ"]
    assert voice.outside("bʱaː") == ["ʱ"]
    assert voice.extend(["ʱ", "a"]) == ["ʱ"] and voice.ids["ʱ"] == 162
    assert json.loads(voice.to_piper_json())["ʱ"] == [162]

    tokens = SymbolTable.from_file(FIX / "tokens.txt")
    assert tokens.ids == {"_": 0, "^": 1, "$": 2, " ": 3, "a": 4, "n": 5}
    assert tokens.to_tokens() == (FIX / "tokens.txt").read_text(encoding="utf-8")

    matcha = SymbolTable.matcha()
    assert matcha.ids["_"] == 0 and matcha.ids[" "] == 16
    assert matcha.encode("a", framing="blank") == uccharan.matcha_ids("a")
    assert matcha.outside("mẽː") == ["̃"]


# -- labelling ----------------------------------------------------------------


def test_label_warnings_and_symbols():
    done = train.label(train.read_manifest(FIX / "metadata.csv"), "hi")
    u1, u2, u3, u4 = done.utterances
    assert u1.phonemes == "nəmˈʌsteː dˈʊnɪjˌaː." and u1.warnings == []
    assert u2.normalized == "घर में पच्चीस घोड़े हैं।" and u2.warnings == []
    assert {w.code for w in u3.warnings} == {"unknown-character", "mixed-script"}
    assert "😀" in u3.warnings[0].detail and "Latin" in str(u3.warnings[1])
    assert u4.warnings == []
    # Every symbol is one code point, and the phoneme text is NFD.
    for u in done.utterances:
        assert u.phonemes == unicodedata.normalize("NFD", u.phonemes)
        assert not done.symbols.outside(u.phonemes)
    assert done.symbols.multi_codepoint() == [] and not done.outside
    assert done.voice()["symbols"]["single_codepoint"] is True


def test_label_modes_and_options():
    rows = [Utterance("a", "घोड़े भूखे हैं।")]
    compat = train.label(rows, "hi").utterances[0].phonemes
    improved = train.label(rows, "hi", improved=True).utterances[0].phonemes
    assert compat != improved and "ɽ" in improved
    assert compat == " ".join(uccharan.phonemize("घोड़े भूखे हैं।", "hi", "piper"))
    no_stress = train.label(rows, "hi", improved=True, stress=False).utterances[0].phonemes
    assert "ˈ" not in no_stress and no_stress == improved.replace("ˈ", "").replace("ˌ", "")
    replaced = train.label(rows, "hi", improved=True, replace={"ʰ": "ɦ"})
    assert "ʰ" not in replaced.utterances[0].phonemes and "ɡɦ" in replaced.utterances[0].phonemes
    assert replaced.voice()["phonemizer"]["replace"] == {"ʰ": "ɦ"}
    assert train.label(rows, "hi", profile="espeak-ipa").utterances[0].phonemes == unicodedata.normalize("NFD", uccharan.phonemize("घोड़े भूखे हैं।", "hi"))
    with pytest.raises(ValueError, match="unknown profile"):
        train.label(rows, "hi", profile="kokoro")


def test_label_numbers_and_empty():
    done = train.label([Utterance("n", "फ़ोन 98765432101234 पर करें"), Utterance("e", "…"), Utterance("x", "☃")], "hi")
    assert [w.code for w in done.utterances[0].warnings] == ["long-number"]
    assert [w.code for w in done.utterances[1].warnings] == ["no-phonemes"]
    assert {w.code for w in done.utterances[2].warnings} == {"unknown-character", "no-phonemes"}
    assert done.warning_counts() == {"long-number": 1, "no-phonemes": 2, "unknown-character": 1}
    # With normalisation off, digits are not read: the utterance says so.
    off = train.label([Utterance("n", "घर में 25 घोड़े")], "hi", normalize="off").utterances[0]
    assert [str(w) for w in off.warnings] == ["unread-number: 25"] and off.normalized == "घर में 25 घोड़े"


def test_label_target_set():
    rows = train.read_manifest(FIX / "piper.csv", layout="piper") + [Utterance("z", "झंडा ऊँचा रहे।")]
    done = train.label(rows, "hi", symbols=FIX / "base_voice.json")
    # What the base voice's table lacks is reported, per symbol and per utterance.
    assert set(done.outside) == {c for u in done.utterances for c in u.phonemes if c not in done.symbols}
    assert {"ɟ", "ɳ", "ɖ"} <= set(done.outside)
    flagged = [u for u in done.utterances if any(w.code == "outside-symbols" for w in u.warnings)]
    assert [u.id for u in flagged] == ["z"] and "ɟ (U+025F)" in flagged[0].warnings[-1].detail
    ext = train.label(rows, "hi", symbols=FIX / "base_voice.json", extend=True)
    assert not ext.outside and sorted(ext.added) == sorted(done.outside)
    assert all(ext.symbols.ids[s] >= 162 for s in ext.added)
    assert ext.symbols.ids["n"] == 26


def test_label_wordlist_and_languages():
    rows = [Utterance("p", "Paytm से भेजो"), Utterance("b", "रउआ कइसन बानी?", lang="bho")]
    plain = train.label(rows, "hi", improved=True)
    listed = train.label(rows, "hi", improved=True, wordlists=[FIX / "words.tsv"])
    assert plain.utterances[0].phonemes != listed.utterances[0].phonemes
    assert listed.utterances[0].phonemes.startswith("peːʈiːeːm ")
    assert listed.voice()["phonemizer"]["word_lists"][0]["file"] == "words.tsv"
    assert len(listed.voice()["phonemizer"]["word_lists"][0]["sha256"]) == 64
    assert plain.languages() == ["bho", "hi"]
    assert plain.utterances[1].phonemes == " ".join(uccharan.phonemize("रउआ कइसन बानी?", "bho", "piper", improved=True))
    assert plain.utterances[1].phonemes != " ".join(uccharan.phonemize("रउआ कइसन बानी?", "hi", "piper", improved=True))


def test_label_files(tmp_path):
    rows = train.read_manifest(FIX / "piper.csv", layout="piper") + [Utterance("c", "…", audio="c.wav", speaker="ravi")]
    done = train.label(rows, "hi", improved=True)
    written = done.write(tmp_path, validation=1)
    names = sorted(str(p.relative_to(tmp_path)) for p in written)
    assert names == sorted(
        [
            "labels.jsonl", "voice.json", "report.json",
            "piper/metadata.csv", "piper/metadata_phoneme_ids.csv", "piper/phonemes.json",
            "matcha/filelist.txt", "matcha/train.txt", "matcha/val.txt", "matcha/tokens.txt",
            "icefall/tokens.txt", "icefall/utterances.jsonl",
        ]
    )  # fmt: skip
    # Piper, --data.phoneme_type text: file|speaker|phonemes; the phonemes
    # are what Piper will split with list(unicodedata.normalize("NFD", …)).
    lines = (tmp_path / "piper/metadata.csv").read_text(encoding="utf-8").splitlines()
    assert len(lines) == 2  # the utterance without phonemes is left out
    name, speaker, phonemes = lines[0].split("|")
    assert (name, speaker) == ("a.wav", "anita")
    assert phonemes == done.utterances[0].phonemes
    id_map = json.loads((tmp_path / "piper/phonemes.json").read_text(encoding="utf-8"))
    assert id_map["_"] == [0] and id_map["^"] == [1] and id_map["$"] == [2]
    assert all(len(k) == 1 and len(v) == 1 for k, v in id_map.items())
    # Piper, --data.data_type phoneme_ids: file|speaker|text|ids, framed.
    cols = (tmp_path / "piper/metadata_phoneme_ids.csv").read_text(encoding="utf-8").splitlines()[0].split("|")
    ids = [int(x) for x in cols[3].split()]
    expect = [1, 0]
    for ch in unicodedata.normalize("NFD", phonemes):
        expect += [id_map[ch][0], 0]
    assert ids == expect + [2] and cols[2] == "नमस्ते दुनिया।"
    # The same ids as uccharan.piper_ids gives with this map at run time.
    assert ids == uccharan.piper_ids(phonemes, {"phoneme_id_map": id_map})[0]
    # Matcha: path|speaker number|phonemes, and a held-out file.
    fl = (tmp_path / "matcha/filelist.txt").read_text(encoding="utf-8").splitlines()
    assert fl[0] == f"a.wav|0|{phonemes}" and fl[1].startswith("b.wav|1|")
    train_, val = [(tmp_path / f"matcha/{n}.txt").read_text(encoding="utf-8").splitlines() for n in ("train", "val")]
    assert len(val) == 1 and sorted(train_ + val) == sorted(fl)
    # tokens.txt reads back as the same table.
    assert SymbolTable.from_file(tmp_path / "icefall/tokens.txt").ids == done.symbols.ids
    rec = json.loads((tmp_path / "icefall/utterances.jsonl").read_text(encoding="utf-8").splitlines()[0])
    assert rec["tokens"] == list(phonemes) and len(rec["token_ids"]) == len(rec["tokens"])
    # voice.json records how the phonemes were made.
    voice = json.loads((tmp_path / "voice.json").read_text(encoding="utf-8"))
    assert voice["phonemizer"] == {
        "name": "uccharan", "version": uccharan.__version__, "mode": "improved", "profile": "piper",
        "normalize": "natural", "hinglish": False, "stress": True, "replace": {}, "word_lists": [],
    }  # fmt: skip
    assert voice["language"] == "hi" and voice["phoneme_type"] == "text"
    assert voice["num_speakers"] == 2 and voice["speakers"] == ["anita", "ravi"]
    assert voice["phoneme_id_map"] == id_map and voice["dataset"]["written"] == 2
    report = json.loads((tmp_path / "report.json").read_text(encoding="utf-8"))
    assert report["warnings"] == {"no-phonemes": 1} and report["utterances_with_warnings"][0]["id"] == "c"
    assert len((tmp_path / "labels.jsonl").read_text(encoding="utf-8").splitlines()) == 3


def test_label_single_speaker_layout(tmp_path):
    done = train.label(train.read_manifest(FIX / "metadata.csv"), "hi")
    done.write(tmp_path, formats=["piper", "matcha"])
    first = (tmp_path / "piper/metadata.csv").read_text(encoding="utf-8").splitlines()[0]
    assert first == "LJ001-0001|nəmˈʌsteː dˈʊnɪjˌaː."
    assert (tmp_path / "matcha/filelist.txt").read_text(encoding="utf-8").splitlines()[0] == first
    assert not (tmp_path / "icefall").exists()
    with pytest.raises(ValueError, match="unknown format"):
        done.write(tmp_path, formats=["vits"])


# -- phones and sentences -----------------------------------------------------


def test_phones_and_sentences():
    assert train.split_phones("bʰˈaːɾət mẽː, hɛ̃.") == [["bʰ", "aː", "ɾ", "ə", "t", "m", "e\u0303ː"], ["h", "ɛ\u0303"]]
    assert train.split_phones("ˈaːɟ", stress=True) == [["ˈaː", "ɟ"]]
    assert train.split_phones("t͡ʃaɪ") == [["t͡ʃ", "a", "ɪ"]]
    assert train.split_phones("ɡʰˈoːr.eː hɛː.") == [["ɡʰ", "oː", "r.", "eː", "h", "ɛː"]]
    assert train.split_sentences("आज छुट्टी है। कल नहीं! क्या 3.5 ठीक है? हाँ\nA. Rao came. He left.") == [
        "आज छुट्टी है।", "कल नहीं!", "क्या 3.5 ठीक है?", "हाँ", "A. Rao came.", "He left.",
    ]  # fmt: skip


# -- script selection ---------------------------------------------------------

CORPUS = [
    "यह घर है।",
    "वह घर है।",
    "यह घर बड़ा है।",
    "खाना अच्छा है।",
    "खाना बहुत अच्छा है और घर बड़ा है।",
    "आज 3 बजे आना।",
    "Hello दोस्त कैसे हो?",
    "हाँ।",
    "यह बहुत ही लंबा वाक्य है जो बार बार वही बात कहता रहता है और कभी खत्म नहीं होता है क्योंकि इसमें शब्द बहुत हैं।",
    "झ़ॅ घर है यह।",
]


def test_select_script_covers_with_few_sentences():
    s = train.select_script(CORPUS, "hi", min_symbol_sentences=1, min_words=3, max_words=12)
    st = s.stats
    assert st["rejected"] == {"digits": 1, "mixed-script": 1, "too-long": 1, "too-short": 1}
    assert st["phones"]["covered"] == st["phones"]["total"] and not st["phones"]["missing"]
    assert st["diphones"]["covered"] == st["diphones"]["total"]
    # The long sentence carries most of the inventory and is taken first;
    # sentences that add nothing are not taken.
    assert s.sentences[0].text == "खाना बहुत अच्छा है और घर बड़ा है।"
    assert "वह घर है।" in [x.text for x in s.sentences] and "यह घर बड़ा है।" not in [x.text for x in s.sentences]
    assert len(s.sentences) < st["candidates"]
    assert [x.phones_covered for x in s.sentences] == sorted(x.phones_covered for x in s.sentences)
    assert s.sentences[-1].diphones_covered == st["diphones"]["covered"]
    assert sum(len(x.new_phones) for x in s.sentences) == st["phones"]["total"]


def test_select_script_filters_and_limits(tmp_path):
    one = train.select_script(CORPUS, "hi", min_symbol_sentences=1, max_sentences=1)
    assert len(one.sentences) == 1 and one.stats["phones"]["missing"]
    # A phone seen in one sentence only is rare: that sentence is dropped.
    rare = train.select_script(CORPUS, "hi", min_symbol_sentences=2, max_words=12)
    assert rare.stats["rejected"]["rare-symbol"] >= 1
    assert all(n < 2 for n in rare.stats["corpus_only_phones"].values())
    digits = train.select_script(CORPUS, "hi", min_symbol_sentences=1, allow_digits=True, allow_mixed_script=True, max_words=12)
    assert "digits" not in digits.stats["rejected"] and "mixed-script" not in digits.stats["rejected"]
    # Sentences outside a symbol table are not offered.
    table = SymbolTable.from_symbols(c for c in unicodedata.normalize("NFD", uccharan.phonemize("यह घर है। वह घर है।", "hi")) if c not in "ˈˌ")
    inside = train.select_script(CORPUS, "hi", min_symbol_sentences=1, symbols=table, min_words=3)
    assert {x.text for x in inside.sentences} == {"यह घर है।", "वह घर है।"}
    by_length = train.select_script(CORPUS, "hi", min_symbol_sentences=1, max_words=12, prefer="length")
    assert by_length.stats["phones"]["covered"] == by_length.stats["phones"]["total"]
    assert by_length.sentences[0].text != "खाना बहुत अच्छा है और घर बड़ा है।"
    files = by_length.write(tmp_path)
    assert [p.name for p in files] == ["script.txt", "script.tsv", "coverage.json"]
    assert (tmp_path / "script.txt").read_text(encoding="utf-8").splitlines() == [x.text for x in by_length.sentences]
    assert (tmp_path / "script.tsv").read_text(encoding="utf-8").count("\n") == len(by_length.sentences) + 1
    assert json.loads((tmp_path / "coverage.json").read_text(encoding="utf-8"))["language"] == "hi"
    with pytest.raises(ValueError):
        train.select_script(CORPUS, "hi", prefer="shortest")


# -- coverage -----------------------------------------------------------------


def test_coverage_counts_and_missing():
    rows = [Utterance("u1", "घर है।", duration=0.9), Utterance("u2", "यह घर है।", duration=1.2)]
    r = train.coverage(rows, "hi", reference=["यह घर है।", "वह पानी है।"], min_count=2)
    assert r.utterances == 2 and r.phones["ɡʰ"] == 2 and r.phones["j"] == 1
    assert r.diphones["ɡʰ-ʌ"] == 2
    assert set(r.missing_phones) == {"ʋ", "p", "aː", "n", "i"}
    assert ("p-aː", 1) in r.missing_diphones and all(d not in r.diphones for d, _ in r.missing_diphones)
    assert r.rare_phones == {"j": 1, "ə": 1}
    assert 0 < r.totals["diphone_coverage"] < 1
    assert abs(r.totals["hours"] - 2.1 / 3600) < 1e-9
    assert "missing phones" in r.summary() and json.loads(r.to_json())["totals"]["utterances"] == 2
    # Against a symbol table: its unused symbols.
    table = SymbolTable.from_symbols("ɡʰʌɾhɛːz.")
    assert train.coverage(rows[:1], "hi", symbols=table).missing_phones == ["z"]


def test_coverage_durations():
    al = train.read_alignments(FIX)
    assert al == {"u1": [("", 0.0, 0.1), ("ɡʰ", 0.1, 0.3), ("ʌ", 0.3, 0.6), ("ɾ", 0.6, 0.9)]}
    r = train.coverage([Utterance("u1", "घर")], "hi", alignments=al)
    assert sorted(r.durations) == sorted(["ʌ", "ɾ", "ɡʰ"]) and list(r.durations)[-1] == "ɡʰ"
    assert r.durations["ɡʰ"] == {"count": 1, "total": 0.2, "mean": 0.2, "median": 0.2, "min": 0.2, "max": 0.2}
    assert r.totals["aligned_seconds"] == 0.8


def test_alignment_formats(tmp_path):
    (tmp_path / "a.jsonl").write_text('{"id": "u1", "phones": [{"phone": "a", "start": 0.0, "end": 0.25}, {"phone": "n", "start": 0.25, "duration": 0.05}]}\n', encoding="utf-8")
    assert train.read_alignments(tmp_path / "a.jsonl") == {"u1": [("a", 0.0, 0.25), ("n", 0.25, 0.3)]}
    (tmp_path / "a.ctm").write_text("u1 1 0.00 0.25 a\nu1 1 0.25 0.05 n\nu2 1 0.10 0.10 sil\n", encoding="utf-8")
    got = train.read_alignments(tmp_path / "a.ctm")
    assert got["u1"] == [("a", 0.0, 0.25), ("n", 0.25, 0.3)] and got["u2"][0][0] == "sil"
    assert train.read_alignments(FIX / "u1.TextGrid", tier="words")["u1"][1][0] == "घर"


def test_coverage_outliers():
    rows = [Utterance(f"u{i}", "यह घर है।", duration=1.0) for i in range(10)]
    rows.append(Utterance("long", "यह घर है और वह घर है और यह घर बड़ा है और वह घर भी बड़ा है।" * 3, duration=30.0))
    rows.append(Utterance("fast", "यह घर है।", duration=0.1))
    rows.append(Utterance("latin", "this is not Hindi at all"))
    rows.append(Utterance("empty", "…"))
    rows.append(Utterance("mixed", "मेरा favourite game cricket है"))
    r = train.coverage(rows, "hi")
    reasons = {(o.id, o.reason) for o in r.outliers}
    assert ("long", "too-long") in reasons and ("fast", "speaking-rate") in reasons
    assert ("latin", "wrong-script") in reasons and ("latin", "out-of-vocabulary") in reasons
    assert ("empty", "no-phonemes") in reasons and ("mixed", "out-of-vocabulary") in reasons
    assert not any(o.id == "u0" for o in r.outliers)
    # A vocabulary decides what is out of vocabulary; English uses the lexicon.
    v = train.coverage(rows[:1], "hi", vocabulary=["यह", "है"], max_oov=0.3)
    assert [(o.reason, o.detail) for o in v.outliers] == [("out-of-vocabulary", "घर")]
    en = train.coverage([Utterance("e", "the zxqvy blorptang sat"), Utterance("f", "the cat sat down")], "en-us")
    assert [(o.id, o.reason) for o in en.outliers] == [("e", "out-of-vocabulary")]
    assert train.coverage(rows, "hi", max_phones=5, max_oov=1.0).totals["utterances"] == 15


# -- scoring ------------------------------------------------------------------


def test_score_same_and_close():
    same = train.score("नमस्ते दुनिया", "नमस्ते दुनिया", lang="hi")
    assert same.error_rate == 0 and same.score == 1 and not same.mispronounced and not same.extra
    # A spelling variant of the same sound costs nothing or next to nothing.
    variant = train.score("हिंदी में ज़रूर", "हिन्दी में जरूर", lang="hi")
    assert variant.score > 0.9 and not variant.mispronounced
    # Aspiration lost: reported as an error, cheap, not a mispronunciation.
    asp = train.score("भारत", "बारत", lang="hi")
    assert asp.words[0].errors == ("bʰ→b",) and asp.words[0].status == "ok" and 0.9 < asp.words[0].score < 1
    assert asp.substitutions == 1 and asp.error_rate == pytest.approx(1 / asp.reference_phones)
    assert asp.weighted_error < asp.error_rate
    # … unless the listener is strict.
    assert train.score("भारत", "बारत", lang="hi", threshold=0.99).words[0].status == "mispronounced"


def test_score_words():
    s = train.score("मेरा घोड़ा तेज़ है", "मेरा कीड़ा तेज़", lang="hi")
    assert [(w.word, w.status) for w in s.words] == [("मेरा", "ok"), ("घोड़ा", "mispronounced"), ("तेज़", "ok"), ("है", "missing")]
    assert s.words[1].heard == "कीड़ा" and s.words[1].errors == ("gʰ→k", "oː→iː")
    assert [w.word for w in s.mispronounced] == ["घोड़ा", "है"]
    assert s.deletions == 2 and s.substitutions == 2 and s.insertions == 0
    assert "घोड़ा" in s.feedback() and "not heard" in s.feedback()
    assert s.to_dict()["words"][3]["status"] == "missing"
    # An extra word is reported as such and blamed on no reference word.
    extra = train.score("मेरा घोड़ा तेज़ है", "अरे मेरा घोड़ा बहुत तेज़ है", lang="hi")
    assert extra.extra == ["अरे", "बहुत"] and not extra.mispronounced and extra.insertions > 0 and extra.score < 1
    # A word list changes the reference's reading (here to what was heard).
    plain = train.score("Paytm से भेजो", "पेटीएम से भेजो", lang="hi")
    listed = train.score("Paytm से भेजो", "पेटीएम से भेजो", lang="hi", wordlists=[(FIX / "words.tsv").read_text(encoding="utf-8")])
    assert plain.score < 1 and listed.score == 1
    # Numbers and symbols are compared as they are spoken.
    assert train.score("₹500 दो", "पाँच सौ रुपये दो", lang="hi").score > 0.95


def test_score_phoneme_string_and_languages():
    s = train.score("The quick brown fox", heard_phonemes="ð ə k w ɪ k b ɹ aʊ n f ɔ k s", lang="en-us")
    assert [w.status for w in s.words] == ["ok"] * 4 and s.words[3].errors == ("ɑː→ɔ",)
    assert s.words[1].heard == "" and s.words[1].heard_phones == "kwɪk"
    bad = train.score("The quick brown fox", heard_phonemes="ðə kwɪk daʊn fɑːks", lang="en-us")
    assert [w.word for w in bad.mispronounced] == ["brown"]
    # A recogniser that writes Latin for Hindi speech: both sides are phonemes.
    cross = train.score("नमस्ते", "namaste", lang="hi", heard_lang="en-us")
    assert cross.score > 0.6
    with pytest.raises(ValueError):
        train.score("a", lang="en-us")
    with pytest.raises(ValueError):
        train.score("a", "a", heard_phonemes="a", lang="en-us")
    given = train.score("Kothari", "kothari", lang="en-us", reference_phonemes="koːʈʰaːɾiː")
    assert given.words[0].ipa == "koːʈʰaːɾiː"


def test_score_many():
    r = train.score_many(
        [
            {"id": "a", "reference": "नमस्ते दुनिया", "heard": "नमस्ते दुनिया"},
            {"id": "b", "reference": "मेरा घोड़ा तेज़ है", "heard": "मेरा कीड़ा तेज़ है"},
            {"reference": "घोड़ा", "heard_phonemes": "koːɽaː"},
        ],
        lang="hi",
        threshold=0.95,
    )
    assert r["count"] == 3 and r["exact"] == 1
    assert r["words"][0] == {"word": "घोड़ा", "errors": 2, "occurrences": 2}
    assert 0 < r["weighted_error"] < r["error_rate"] < 1
    assert [u["id"] for u in r["utterances"]] == ["a", "b", "3"]


# -- MFA dictionary -----------------------------------------------------------


def test_mfa_dictionary():
    text = train.mfa_dictionary(["घर में पानी है।", "पानी, घर! Hello 123"], "hi")
    assert text == "घर\tɡʰ ʌ ɾ\nपानी\tp aː n i\nमें\tm e\u0303ː\nहै\th ɛː\n"
    for line in text.splitlines():
        word, phones = line.split("\t")
        assert " " not in word and all(phones.split(" "))
    assert "hello\t" in train.mfa_dictionary(["Hello घर"], "hi", foreign=True)
    both = train.mfa_dictionary(["घोड़ा", "पानी"], "hi", both_modes=True).splitlines()
    assert [line.split("\t")[0] for line in both] == ["घोड़ा", "घोड़ा", "पानी", "पानी"]
    assert train.mfa_dictionary(["घर"], "hi", stress=True) == "घर\tɡʰ ˈʌ ɾ\n"
    assert train.mfa_dictionary(["The cat"], "en-us") == "cat\tk æ t\nthe\tð ə\n"


# -- command line -------------------------------------------------------------


def test_cli_label_select_coverage(tmp_path, capsys):
    out = tmp_path / "labels"
    assert cli(["label", str(FIX / "metadata.csv"), "-l", "hi", "--improved", "-o", str(out), "--wordlist", str(FIX / "words.tsv")]) == 0
    text = capsys.readouterr()
    assert "4 utterances" in text.out and "LJ001-0003: unknown-character: 😀" in text.err
    voice = json.loads((out / "voice.json").read_text(encoding="utf-8"))
    assert voice["phonemizer"]["mode"] == "improved" and voice["phonemizer"]["word_lists"][0]["file"] == "words.tsv"
    # A base voice's table: symbols outside it fail the run with --strict.
    assert cli(["label", str(FIX / "metadata.csv"), "-l", "hi", "-o", str(tmp_path / "s"), "--symbols", str(FIX / "base_voice.json"), "--strict", "--format", "piper"]) == 2
    assert "outside the target set" in capsys.readouterr().err
    assert cli(["label", str(FIX / "metadata.csv"), "-l", "hi", "-o", str(tmp_path / "s"), "--symbols", str(FIX / "base_voice.json"), "--strict", "--extend"]) == 0
    capsys.readouterr()

    assert cli(["coverage", str(out / "labels.jsonl"), "-l", "hi", "--json", str(tmp_path / "cov.json"), "--alignments", str(FIX)]) == 0
    assert "utterances        4" in capsys.readouterr().out
    assert json.loads((tmp_path / "cov.json").read_text(encoding="utf-8"))["durations"]["ʌ"]["count"] == 1
    assert cli(["coverage", str(FIX / "metadata.csv"), "-l", "hi", "--reference", str(ROOT / "examples/train/hi.txt")]) == 0
    assert "missing diphones" in capsys.readouterr().out

    corpus = tmp_path / "corpus.txt"
    corpus.write_text("यह घर है। वह घर है। खाना अच्छा है।\nखाना बहुत अच्छा है।\n", encoding="utf-8")
    assert cli(["select", str(corpus), "-l", "hi", "--min-symbol-sentences", "1", "-o", str(tmp_path / "script")]) == 0
    assert "phones" in capsys.readouterr().err
    assert (tmp_path / "script/script.txt").read_text(encoding="utf-8").strip()
    assert cli(["select", str(corpus), "-l", "hi", "--min-symbol-sentences", "1", "--lines", "-n", "1"]) == 0
    assert capsys.readouterr().out.count("\n") == 1


def test_cli_score_and_mfa(tmp_path, capsys):
    assert cli(["score", "-l", "hi", "--reference", "मेरा घोड़ा तेज़ है", "--heard", "मेरा घोड़ा तेज़ है"]) == 0
    assert capsys.readouterr().out.startswith("score 100%")
    assert cli(["score", "-l", "hi", "--reference", "मेरा घोड़ा तेज़ है", "--heard", "मेरा कीड़ा", "--json", str(tmp_path / "s.json")]) == 1
    assert "घोड़ा" in capsys.readouterr().out and json.loads((tmp_path / "s.json").read_text(encoding="utf-8"))["deletions"] > 0
    pairs = tmp_path / "pairs.tsv"
    pairs.write_text("a\tनमस्ते दुनिया\tनमस्ते दुनिया\nb\tघर\tगर\n", encoding="utf-8")
    assert cli(["score", "-l", "hi", "--pairs", str(pairs), "--threshold", "0.99"]) == 0
    assert "2 utterances" in capsys.readouterr().out
    with pytest.raises(SystemExit):
        cli(["score", "-l", "hi", "--reference", "घर"])
    words = tmp_path / "words.txt"
    words.write_text("घर\nपानी\n", encoding="utf-8")
    assert cli(["mfa-dict", str(words), "-l", "hi", "-o", str(tmp_path / "hi.dict")]) == 0
    assert (tmp_path / "hi.dict").read_text(encoding="utf-8") == "घर\tɡʰ ʌ ɾ\nपानी\tp aː n i\n"
    assert cli(["label", str(tmp_path / "missing.csv"), "-o", str(tmp_path / "x")]) == 2


# -- the example and the doctests ---------------------------------------------


def test_example_runs(tmp_path, capsys):
    import runpy

    mod = runpy.run_path(str(ROOT / "examples/train/run.py"))
    mod["main"](str(tmp_path))
    out = capsys.readouterr().out
    assert "20 utterances" in out and "warning:" not in out
    assert "hi: 5 sentences" in out and "bho: 5 sentences" in out
    meta = (tmp_path / "labels/piper/metadata.csv").read_text(encoding="utf-8").splitlines()
    assert len(meta) == 20 and meta[10].startswith("wavs/bho_01.wav|bho_f1|")
    voice = json.loads((tmp_path / "labels/voice.json").read_text(encoding="utf-8"))
    assert voice["languages"] == ["bho", "hi"] and voice["symbols"]["single_codepoint"]


@pytest.mark.parametrize("name", ["_phones", "label", "script", "score", "mfa"])
def test_doctests(name):
    result = doctest.testmod(importlib.import_module(f"uccharan.train.{name}"))
    assert result.attempted > 0 and result.failed == 0


def test_hindi_family_languages_need_no_pack():
    # Bhojpuri, Maithili and Magahi are read with the Hindi data.
    for lang in ("bho", "mai", "mag"):
        assert uccharan.phonemize("घर", lang)
