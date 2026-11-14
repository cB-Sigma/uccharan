"""Pronunciation scoring: how far what was heard is from the reference.

>>> from uccharan.train import score
>>> s = score("मेरा घोड़ा तेज़ है", "मेरा कीड़ा तेज़", lang="hi")
>>> [(w.word, w.status) for w in s.words]
[('मेरा', 'ok'), ('घोड़ा', 'mispronounced'), ('तेज़', 'ok'), ('है', 'missing')]
>>> s.words[1].errors
('gʰ→k', 'oː→iː')

Both sides are read as phonemes and aligned with the weighted distance of
the sound-alike matcher: close sounds (voicing, aspiration, vowel length, a
weak vowel) cost little, unrelated sounds cost a whole phone. What was heard
is a recogniser's text (read by uccharan in its language) or a phoneme
string from a phone recogniser.
"""

from __future__ import annotations

from collections import Counter
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass, field
from typing import Any

import uccharan

from .. import _native
from ..textnorm import WordList
from ._phones import BREAKS

__all__ = ["Score", "WordResult", "score", "score_many"]


@dataclass(frozen=True)
class WordResult:
    """One word of the reference."""

    word: str
    ipa: str
    heard: str
    """The heard words aligned with it (empty for a phoneme string)."""
    heard_phones: str
    """The phones heard for it, in the matcher's broad notation."""
    score: float
    """Similarity, 0 to 1."""
    cost: float
    status: str
    """``ok``, ``mispronounced`` (below the threshold) or ``missing``
    (nothing was heard for it)."""
    errors: tuple[str, ...]
    """What differs: ``ɡʰ→k`` (heard as), ``-h`` (not heard), ``+ə``
    (extra)."""


@dataclass
class Score:
    reference: str
    heard: str
    words: list[WordResult]
    error_rate: float
    """Phoneme error rate: (substitutions + deletions + insertions) over
    the reference phones. Can exceed 1."""
    weighted_error: float
    """The same with each error counted at its weight (close sounds cost
    less), over the weight of the reference."""
    substitutions: int
    deletions: int
    insertions: int
    reference_phones: int
    heard_phones: int
    cost: float = 0.0
    """Summed cost of the errors, in phones."""
    weight: float = 0.0
    """Weight of the reference (the cost of losing all of it)."""
    extra: list[str] = field(default_factory=list)
    """Heard words that no reference word accounts for."""

    @property
    def score(self) -> float:
        """Overall similarity, 0 to 1."""
        return max(0.0, 1.0 - self.weighted_error)

    @property
    def mispronounced(self) -> list[WordResult]:
        """The words that are not ``ok``."""
        return [w for w in self.words if w.status != "ok"]

    def to_dict(self) -> dict[str, object]:
        return {
            "reference": self.reference,
            "heard": self.heard,
            "score": round(self.score, 4),
            "error_rate": round(self.error_rate, 4),
            "weighted_error": round(self.weighted_error, 4),
            "substitutions": self.substitutions,
            "deletions": self.deletions,
            "insertions": self.insertions,
            "reference_phones": self.reference_phones,
            "heard_phones": self.heard_phones,
            "extra": self.extra,
            "words": [
                {"word": w.word, "ipa": w.ipa, "heard": w.heard, "heard_phones": w.heard_phones, "score": round(w.score, 4), "status": w.status, "errors": list(w.errors)}
                for w in self.words
            ],
        }

    def feedback(self) -> str:
        """A few lines for a learner or a reviewer."""
        lines = [f"score {self.score:.0%}, phoneme error {self.error_rate:.1%}"]
        for w in self.mispronounced:
            if w.status == "missing":
                lines.append(f"  {w.word} [{w.ipa}]: not heard")
            else:
                said = f" heard as “{w.heard}”" if w.heard else ""
                lines.append(f"  {w.word} [{w.ipa}]:{said} {w.score:.0%} ({', '.join(w.errors)})")
        if self.extra:
            lines.append(f"  extra: {' '.join(self.extra)}")
        return "\n".join(lines) + "\n"


def _words(text: str, lang: str, improved: bool, lists: Sequence[WordList | str]) -> list[tuple[str, str]]:
    """``(word, ipa)`` for every spoken word of a text."""
    if lists:
        # Word lists change readings; take the words from the phoneme string.
        ipa = str(uccharan.phonemize_wordlists(text, lang, wordlists=lists, improved=improved, punctuation=False))
        norm = uccharan.normalize(text, lang, improved=improved, wordlists=lists).text.split()
        parts = ipa.split()
        if len(parts) == len(norm):
            return list(zip(norm, parts))
        return [(p, p) for p in parts]
    out: list[tuple[str, str]] = []
    for sentence in uccharan.analyze(text, lang, improved=improved):
        for words, _term in sentence:
            for w in words:
                if not w.ipa:
                    continue
                if w.glued and out:
                    out[-1] = (out[-1][0] + w.text, out[-1][1] + w.ipa)
                else:
                    out.append((w.text, w.ipa))
    return out


def _is_en_us(lang: str) -> bool:
    return lang.lower().replace("_", "-") in ("en", "en-us", "a")


def score(
    reference: str,
    heard: str | None = None,
    *,
    lang: str = "en-us",
    heard_lang: str | None = None,
    heard_phonemes: str | None = None,
    reference_phonemes: str | None = None,
    improved: bool = True,
    threshold: float = 0.85,
    wordlists: Sequence[WordList | str] = (),
) -> Score:
    """Score what was heard against a reference text.

    ``heard`` is a recogniser's transcript, read in ``heard_lang`` (default
    ``lang``); ``heard_phonemes`` is an IPA string from a phone recogniser
    (spaces and stress marks are ignored). ``reference_phonemes`` replaces
    uccharan's reading of the reference (one IPA word per reference word).
    ``improved`` selects the reading mode of both texts; ``wordlists``
    apply to both.

    A word scoring under ``threshold`` is ``mispronounced``. In a word of
    four or five phones, a wrong vowel length scores about 0.97, a lost
    aspiration 0.96, a voicing error 0.9 and an unrelated consonant 0.8 or
    less: the default 0.85 passes what recognisers commonly confuse; a
    learner's exercise wants 0.95 or more."""
    if (heard is None) == (heard_phonemes is None):
        raise ValueError("give either heard (text) or heard_phonemes (IPA)")
    ref = _words(reference, lang, improved, wordlists)
    if reference_phonemes is not None:
        given = reference_phonemes.split()
        if len(given) != len(ref):
            ref = [(g, g) for g in given]
        else:
            ref = [(w, g) for (w, _), g in zip(ref, given)]
    if heard is not None:
        hl = heard_lang or lang
        hyp = _words(heard, hl, improved, wordlists)
        heard_english = _is_en_us(hl)
        heard_text = heard
    else:
        assert heard_phonemes is not None
        joined = "".join(c for c in heard_phonemes if not c.isspace() and c not in BREAKS)
        hyp = [("", joined)] if joined else []
        heard_english = False
        heard_text = heard_phonemes
    words, total_cost, total_weight, n_ref, n_hyp, subs, dels, ins, extra_words = _native.align_words([i for _, i in ref], [i for _, i in hyp], _is_en_us(lang), heard_english)

    results: list[WordResult] = []
    for (word, ipa), (cost, weight, phones, h0, h1, ops) in zip(ref, words):
        errors: list[str] = []
        said: list[str] = []
        for kind, r, h, _c in ops:
            if h:
                said.append(h)
            if kind == "substitute":
                errors.append(f"{r}→{h}")
            elif kind == "delete":
                errors.append(f"-{r}")
            elif kind == "insert":
                errors.append(f"+{h}")
        sc = 1.0 if weight <= 0 and cost <= 0 else 0.0 if weight <= 0 else max(0.0, min(1.0, 1.0 - cost / weight))
        if phones and not said:
            status = "missing"
        elif sc < threshold:
            status = "mispronounced"
        else:
            status = "ok"
        results.append(
            WordResult(
                word=word,
                ipa=ipa,
                heard=" ".join(hyp[k][0] for k in range(h0, h1) if hyp[k][0] and k not in extra_words),
                heard_phones="".join(said),
                score=sc,
                cost=cost,
                status=status,
                errors=tuple(errors),
            )
        )
    extra = [hyp[k][0] for k in extra_words if hyp[k][0]]
    errs = subs + dels + ins
    return Score(
        reference=reference,
        heard=heard_text,
        words=results,
        error_rate=(errs / n_ref) if n_ref else (1.0 if errs else 0.0),
        weighted_error=(total_cost / total_weight) if total_weight > 0 else (1.0 if total_cost > 0 else 0.0),
        cost=total_cost,
        weight=total_weight,
        substitutions=subs,
        deletions=dels,
        insertions=ins,
        reference_phones=n_ref,
        heard_phones=n_hyp,
        extra=extra,
    )


def score_many(
    pairs: Iterable[Mapping[str, object]],
    *,
    lang: str = "en-us",
    heard_lang: str | None = None,
    improved: bool = True,
    threshold: float = 0.85,
    wordlists: Sequence[WordList | str] = (),
) -> dict[str, Any]:
    """Score many utterances (a TTS → ASR round trip over a test set, or a
    learner's session).

    Each item has ``reference`` and ``heard`` or ``heard_phonemes``, and
    optionally ``id``, ``lang`` and ``heard_lang``. Returns ``utterances``
    (each :meth:`Score.to_dict` with its ``id``), the pooled ``error_rate``
    and ``weighted_error`` (errors summed over all utterances, not the mean
    of the rates), and ``words``: the reference words that were most often
    not ``ok``, with how often they occurred."""
    out: list[dict[str, object]] = []
    errs = n_ref = 0
    cost = weight = 0.0
    bad: Counter[str] = Counter()
    seen: Counter[str] = Counter()
    scores: list[float] = []
    exact = 0
    for n, item in enumerate(pairs, start=1):
        lg = str(item.get("lang") or lang)
        hp = item.get("heard_phonemes")
        s = score(
            str(item["reference"]),
            None if hp is not None else str(item.get("heard", "")),
            lang=lg,
            heard_lang=str(item.get("heard_lang") or heard_lang or lg),
            heard_phonemes=None if hp is None else str(hp),
            improved=improved,
            threshold=threshold,
            wordlists=wordlists,
        )
        errs += s.substitutions + s.deletions + s.insertions
        n_ref += s.reference_phones
        cost += s.cost
        weight += s.weight
        scores.append(s.score)
        exact += not (s.substitutions or s.deletions or s.insertions)
        for w in s.words:
            seen[w.word] += 1
            if w.status != "ok":
                bad[w.word] += 1
        d = s.to_dict()
        d["id"] = str(item.get("id", n))
        out.append(d)
    return {
        "utterances": out,
        "count": len(out),
        "exact": exact,
        "reference_phones": n_ref,
        "error_rate": errs / n_ref if n_ref else 0.0,
        "weighted_error": cost / weight if weight else 0.0,
        "mean_score": sum(scores) / len(scores) if scores else 1.0,
        "words": [{"word": w, "errors": n, "occurrences": seen[w]} for w, n in bad.most_common()],
    }
