# Output profiles

A profile is the exact shape a family of voices was trained on. All profiles are rendered from the
same analysed sentences; they differ in framing, not in pronunciation. For the measured agreement
of each profile see [evaluation.md](evaluation.md).

| Profile | Result | For |
| --- | --- | --- |
| `espeak-ipa` | one string, words separated by spaces, clause punctuation attached | anything that used `phonemizer`'s espeak backend |
| `piper` | one NFD string per sentence; ids through the voice's `phoneme_id_map` | Piper voices with `phoneme_type: espeak` |
| `sherpa-onnx` | one list of Unicode codepoints per sentence | sherpa-onnx ≥ 2.0 `GenerationConfig.phoneme_codepoints` |
| `kokoro` | Kokoro token string; ids `[0, …, 0]` | Kokoro-82M |
| `kitten` | phoneme string; ids `[0, …, 10, 0]` | Kitten TTS 0.8 ONNX models |
| `matcha` | phoneme string; ids interspersed with 0 | Matcha-TTS |

## espeak-ipa

`həlˈoʊ, wˈɜːld.` Stress marks `ˈ` `ˌ` stand immediately before the stressed vowel, length is `ː`,
affricates are two letters, there are no tie bars. With `punctuation = false` the terminators are
left out (the shape `phonemizer` returns by default). With `keep_language_flags`, words read by
another language's rules are bracketed as the reference does: `mɛ̃ (en)ˈɒfɪs(hi) ɟˈaː`.

## piper

Piper phonemizes clause by clause, keeps each clause's terminator, adds a space after `,` `:` `;`,
and produces one phoneme list per sentence. The unit is one Unicode codepoint after canonical
decomposition (NFD), so a nasalised vowel is two tokens (vowel + U+0303) and ASCII `g` is written
`ɡ` (U+0261). Language-switch flags never appear.

Ids: `[BOS, PAD, p1, PAD, p2, PAD, …, EOS]` with the ids of `^`, `_`, `$` from the voice's
`phoneme_id_map` (`PhonemeIdMap::from_json`, `uccharan.piper_ids`, `piperIds`). Symbols that are not
in the map are skipped, as Piper does, and returned so that you can see them. Older voices with the
130-symbol map have no entry for the syllabic mark U+0329 (as in `n̩`); it is dropped for those
voices whichever phonemizer produced it.

## sherpa-onnx

`list[list[int]]`: the codepoints of each `piper` sentence string. sherpa-onnx maps each codepoint
through the model's `tokens.txt` and adds the model's own BOS, EOS and padding. See
[integrations/sherpa-onnx.md](integrations/sherpa-onnx.md).

## kokoro

Kokoro's vocabulary (114 symbols) uses single letters for English diphthongs and affricates
(`A` = eɪ, `I` = aɪ, `O` = oʊ, `Q` = əʊ, `W` = aʊ, `Y` = ɔɪ, `ʤ`, `ʧ`), `T` for the American flap
and `ᵊ` for the reduced vowel before a syllabic consonant.

- **Hindi**: Kokoro's Hindi was trained on espeak-ng `hi` output passed through a short published
  mapping. uccharan's Hindi output through the same mapping is what this profile emits, and it
  agrees closely with the reference at token level.
- **English**: Kokoro's English was trained on misaki's output, not on espeak-ng. The profile runs
  uccharan's port of misaki's English G2P with misaki 0.9.4's dictionaries (built in) and matches
  misaki on 94.9 % of held-out sentences. A build without the dictionaries falls back to
  converting uccharan's espeak-style English, which stays in the vocabulary but follows a
  different convention. See [integrations/kokoro.md](integrations/kokoro.md).

Anything outside the vocabulary is dropped, as the model's tokenizer does. `kokoro_ids` wraps the
ids in the pad id 0 and stops at 510 symbols.

## kitten and matcha

Both use the StyleTTS2 symbol table (pad, punctuation, A–Z a–z, an IPA block) and the
`phonemizer` shape with punctuation and stress. They differ only in two quote characters in the
table and in framing: Kitten 0.8 is `[0, ids…, 10, 0]` (10 is `…`), Matcha intersperses the blank
id 0 between symbols. Kitten's front end re-tokenises the phoneme string with `\w+|[^\w\s]` and
joins with single spaces, which puts a space before punctuation; do the same before calling
`kitten_ids` if you want the model's exact input.

## Choosing options per profile

| You are feeding | Mode | Normalisation |
| --- | --- | --- |
| an existing espeak-trained voice | compatible | `natural` (numbers are read sensibly; voices were trained on spelled-out corpus text, so the reference's own number readings carry no advantage) |
| a pipeline that compares against espeak-ng output | compatible | `espeak` |
| a new voice you are training | improved | `natural` |
