# Labelling a dataset and choosing a reading script

Ten Hindi and ten Bhojpuri sentences, no audio. `metadata.jsonl` is the manifest (audio path, text,
language and speaker per row); `hi.txt` and `bho.txt` hold the same sentences as plain text.

```sh
pip install uccharan
python examples/train/run.py out/
```

or step by step with the command line:

```sh
# 1. Phonemes for training (Piper, Matcha-TTS, icefall), the symbol table and voice.json.
uccharan-train label examples/train/metadata.jsonl -l hi --improved --validation 2 -o out/labels

# 2. A reading script: the fewest sentences that cover the phones and diphones.
uccharan-train select examples/train/bho.txt -l bho --improved --lines --min-symbol-sentences 1 -n 5 -o out/script-bho

# 3. What the dataset covers, judged against a (here very small) text corpus.
uccharan-train coverage out/labels/labels.jsonl -l hi --improved --reference examples/train/hi.txt

# 4. A pronunciation dictionary for a forced aligner.
uccharan-train mfa-dict examples/train/bho.txt -l bho --improved -o out/bho.dict
```

`out/labels/piper/metadata.csv` then starts

```text
wavs/hi_01.wav|hi_f1|nəmˈʌsteː, ˌaːp kˈɛːseː hɛ̃?
wavs/hi_02.wav|hi_f1|mˌeːɾaː nˈaːm ənˈiːtaː hɛː.
```

and `out/labels/voice.json` records the uccharan version, the mode (`improved`), the languages and
the symbol table. See [docs/training.md](../../docs/training.md).
