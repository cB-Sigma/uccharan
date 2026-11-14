#!/usr/bin/env python3
"""Build the sentences of the sound-alike evaluation.

Reads the term sets in ``terms/`` and writes ``data/items.jsonl``: one
sentence per term and context (English, Hindi in Devanagari, romanised
Hindi), plus sentences without any vocabulary term (Tatoeba and Sherlock
Holmes sentences from ``tools/eval/data`` and hand-written romanised Hindi
ones). Every term goes to the development or the held-out split by a hash of
its spelling, and every sentence follows its term; sentences without a term
come from the matching development / held-out files and keep the id of
their source sentence (`tatoeba-<id>`, `pg1661-<n>`).

    python tools/eval/soundalike/build_items.py
"""

from __future__ import annotations

import hashlib
import json
import random
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
EVAL = HERE.parent

EN_NAME_G = [
    "Please call {t} tomorrow morning.",
    "I spoke to {t} about the project yesterday.",
    "{t} will join the meeting at three.",
    "Can you ask {t} to send the file?",
    "The email from {t} arrived last night.",
    "We are waiting for {t} to approve the budget.",
    "{t} said the report is almost ready.",
    "Schedule a call with {t} for next week.",
]
EN_NAME_S = [
    "Please send the invoice to Mister {t} by Friday.",
    "Doctor {t} will see you at four.",
    "I have a meeting with Miss {t} this afternoon.",
    "The {t} family moved to Pune last year.",
    "Mister {t} asked for a copy of the contract.",
    "Tell Professor {t} that the class is cancelled.",
    "This order was placed by Mister {t} on Monday.",
    "Forward the message to Miss {t} right away.",
]
EN_BRAND = [
    "Have you heard about {t}?",
    "They mentioned {t} in the meeting today.",
    "I read an article about {t} this morning.",
    "My brother has been working with {t} for two years.",
    "{t} was the first thing she talked about.",
    "We compared {t} with the other options.",
    "Everyone in the office is talking about {t} now.",
    "Can you tell me more about {t}?",
]
EN_JARGON = [
    "Can you explain {t} to the new team?",
    "The notes from the meeting mention {t} twice.",
    "We need to talk about {t} before the review.",
    "She wrote a long document about {t} last week.",
    "{t} came up again in the discussion.",
    "I am still learning about {t} these days.",
    "Please add {t} to the list of topics.",
    "Nobody told me about {t} until yesterday.",
]
HI_NAME_G = [
    "{t} को कल सुबह फ़ोन करना है।",
    "मैंने {t} से इस बारे में बात की थी।",
    "{t} आज दफ़्तर नहीं आ पाएंगे।",
    "यह रिपोर्ट {t} को भेज दीजिए।",
    "क्या आप {t} को जानते हैं?",
    "{t} ने बताया कि काम लगभग पूरा हो गया है।",
    "कल की बैठक में {t} भी आएंगे।",
    "मुझे {t} का नंबर चाहिए।",
]
HI_NAME_S = [
    "{t} जी को कल सुबह फ़ोन करना है।",
    "मैंने {t} साहब से इस बारे में बात की थी।",
    "डॉक्टर {t} आज नहीं आएंगे।",
    "यह फ़ाइल {t} जी को भेज दीजिए।",
    "क्या आप {t} परिवार को जानते हैं?",
    "{t} जी ने बताया कि काम पूरा हो गया है।",
    "कल की बैठक में {t} साहब भी आएंगे।",
    "मुझे {t} जी का पता चाहिए।",
]
HI_OTHER = [
    "आज की बैठक में {t} के बारे में बात हुई।",
    "क्या आपने {t} के बारे में सुना है?",
    "मुझे {t} की पूरी जानकारी चाहिए।",
    "हम लोग अब {t} का इस्तेमाल करते हैं।",
    "मेरे भाई ने कल {t} का नाम लिया था।",
    "{t} के बारे में आज अख़बार में ख़बर आई है।",
    "उन्होंने {t} पर एक लंबा लेख लिखा है।",
    "सब लोग आजकल {t} की बात कर रहे हैं।",
]
HING_NAME_G = [
    "{t} ko kal subah phone karna hai",
    "maine {t} se is baare mein baat ki thi",
    "{t} aaj office nahi aa payenge",
    "yeh report {t} ko bhej dijiye",
    "kya aap {t} ko jaante hain",
    "kal ki meeting mein {t} bhi aayenge",
]
HING_NAME_S = [
    "{t} ji ko kal subah phone karna hai",
    "maine {t} sahab se is baare mein baat ki thi",
    "doctor {t} aaj nahi aayenge",
    "yeh file {t} ji ko bhej dijiye",
    "kya aap {t} parivaar ko jaante hain",
    "kal ki meeting mein {t} sahab bhi aayenge",
]
HING_OTHER = [
    "aaj ki meeting mein {t} ke baare mein baat hui thi",
    "kya aapne {t} ke baare mein suna hai",
    "mujhe {t} ki poori jaankari chahiye",
    "hum log ab {t} ka istemaal karte hain",
    "mere bhai ne kal {t} ka naam liya tha",
    "sab log aajkal {t} ki baat kar rahe hain",
]

# Romanised Hindi sentences without any vocabulary term (hand-written).
HING_NONE = """
kal subah jaldi uthna hai
mujhe aaj bahut kaam karna hai
kya tumne khana kha liya
baarish ki wajah se train late hai
aap kahan se bol rahe hain
mera phone charge nahi ho raha
shaam ko bazaar chalte hain
yeh kitaab bahut acchi hai
bachche school se aa gaye hain
mujhe thoda paani chahiye
aaj mausam bahut suhana hai
kal hum log picnic par jayenge
tumhara ghar kitni door hai
main abhi raaste mein hoon
darwaza band kar dijiye
usne mujhe kuch nahi bataya
yeh kaam kal tak ho jayega
mujhe neend aa rahi hai
aapka naam kya hai
hum agle hafte milte hain
bijli phir se chali gayi
chai mein cheeni kam daalna
mujhe yeh rang pasand nahi hai
woh log abhi tak nahi pahunche
dukaan raat ko das baje band hoti hai
kya main andar aa sakta hoon
meri gaadi kharab ho gayi hai
tum itni der se kyon aaye
sab kuch theek ho jayega
mujhe hindi thodi thodi aati hai
aaj raat khane mein kya bana hai
uska beta bahar padhne gaya hai
yeh rasta seedha station jaata hai
main kal doctor ke paas gaya tha
garmi mein yahan bahut bheed hoti hai
tumhari awaaz saaf nahi aa rahi
is mahine kharcha bahut ho gaya
hum bachpan mein yahan khelte the
kripya thoda intezaar kijiye
mujhe apni galti ka ehsaas hai
aap chinta mat kijiye
woh hamesha sach bolta hai
kal raat der tak jaagna pada
is sawaal ka jawaab mushkil hai
main tumhe baad mein phone karta hoon
unki shaadi agle saal hogi
yahan se hawai adda door hai
mujhe yeh geet bahut pasand hai
pehle haath dho lo phir khana khao
kya yeh seat khaali hai
subah ki sair sehat ke liye acchi hai
maine saara paisa bank mein jama kar diya
tum kab wapas aaoge
aaj daftar mein chhutti hai
uski beti bahut samajhdar hai
mera sir dard kar raha hai
hum sab milkar kaam karenge
yeh sheher bahut purana hai
zara dheere boliye
mujhe aapse ek zaroori baat karni hai
""".split("\n")


def split_of(term: str) -> str:
    """Development or held-out, by the term's spelling."""
    return "dev" if hashlib.sha1(term.lower().encode()).digest()[0] % 2 == 0 else "heldout"


def read_terms() -> list[dict]:
    out = []
    for kind, name in (("name", "names.tsv"), ("brand", "brands.tsv"), ("jargon", "jargon.tsv")):
        for line in (HERE / "terms" / name).read_text(encoding="utf-8").splitlines():
            if not line.strip() or line.startswith("#"):
                continue
            cols = line.split("\t")
            t = {"term": cols[0], "kind": kind, "split": split_of(cols[0])}
            if kind == "name":
                t["type"], t["hi"] = cols[1], cols[2]
            elif len(cols) > 1 and cols[1].strip():
                t["say"] = cols[1].strip()
            out.append(t)
    return out


def pick(templates: list[str], key: str) -> str:
    return templates[int(hashlib.sha1(key.encode()).hexdigest(), 16) % len(templates)]


def read_tsv(name: str) -> list[tuple[str, str]]:
    """(source id, sentence) pairs of an evaluation file."""
    rows = []
    for line in (EVAL / "data" / name).read_text(encoding="utf-8").splitlines():
        if "\t" in line:
            i, s = line.split("\t", 1)
            rows.append((i if i.startswith("pg") else f"tatoeba-{i}", s.strip()))
    return rows


def main() -> None:
    terms = read_terms()
    items = []
    for n, t in enumerate(terms):
        term, kind = t["term"], t["kind"]
        spoken = t.get("say", term)
        if kind == "name":
            en = EN_NAME_G if t["type"] == "g" else EN_NAME_S
            hi = HI_NAME_G if t["type"] == "g" else HI_NAME_S
            hing = HING_NAME_G if t["type"] == "g" else HING_NAME_S
        else:
            en = EN_BRAND if kind == "brand" else EN_JARGON
            hi, hing = HI_OTHER, HING_OTHER
        base = {"term": term, "kind": kind, "split": t["split"]}
        for ctx, tpl in (("en", pick(en, "en" + term)), ("hi", pick(hi, "hi" + term)), ("hing", pick(hing, "hg" + term))):
            # The reference sentence shows the term as the vocabulary writes
            # it; the voice reads the spoken form (and, in Hindi sentences,
            # the Devanagari form of a personal name).
            written = term
            voiced = spoken
            if ctx == "hi" and kind == "name":
                voiced = t["hi"]
            items.append({**base, "id": f"{ctx}-{kind}-{n}", "ctx": ctx, "ref": tpl.format(t=written), "tts": tpl.format(t=voiced),
                          **({"hi": t["hi"]} if kind == "name" else {}), **({"say": t["say"]} if "say" in t else {})})
    # Sentences without a vocabulary term.
    vocab = [re.compile(r"(?<!\w)" + re.escape(t["term"].lower()) + r"(?!\w)") for t in terms]
    vocab_hi = [t["hi"] for t in terms if "hi" in t]

    def clean(s: str) -> bool:
        low = s.lower()
        return not any(p.search(low) for p in vocab) and not any(h in s for h in vocab_hi)

    rng = random.Random(20261114)
    for split, en_file, hi_file, n_en, n_hi in (("dev", "en_dev.tsv", "hi_dev.tsv", 300, 200), ("heldout", "en_heldout.tsv", "hi_heldout.tsv", 300, 200)):
        en = [r for r in read_tsv(en_file) if 5 <= len(r[1].split()) <= 16 and r[1].isascii() and not re.search(r"\d", r[1]) and clean(r[1])]
        if split == "heldout":
            prose = [r for r in read_tsv("en_prose.tsv") if 5 <= len(r[1].split()) <= 16 and r[1].isascii() and not re.search(r"\d", r[1]) and clean(r[1])]
            en = rng.sample(en, n_en - 100) + rng.sample(prose, 100)
        else:
            en = rng.sample(en, n_en)
        hi = [r for r in read_tsv(hi_file) if 4 <= len(r[1].split()) <= 14 and not re.search(r"[A-Za-z0-9०-९]", r[1]) and clean(r[1])]
        hi = rng.sample(hi, n_hi)
        for k, (src, s) in enumerate(en):
            items.append({"id": f"en-none-{split}-{k}", "ctx": "en", "kind": "none", "split": split, "ref": s, "tts": s, "source": src})
        for k, (src, s) in enumerate(hi):
            items.append({"id": f"hi-none-{split}-{k}", "ctx": "hi", "kind": "none", "split": split, "ref": s, "tts": s, "source": src})
    hing = [s.strip() for s in HING_NONE if s.strip()]
    for k, s in enumerate(hing):
        split = "dev" if k % 2 == 0 else "heldout"
        items.append({"id": f"hing-none-{split}-{k}", "ctx": "hing", "kind": "none", "split": split, "ref": s, "tts": s})
    out = HERE / "data" / "items.jsonl"
    out.parent.mkdir(exist_ok=True)
    with out.open("w", encoding="utf-8") as f:
        for it in items:
            f.write(json.dumps(it, ensure_ascii=False) + "\n")
    by: dict[str, int] = {}
    for it in items:
        k = f"{it['split']}/{it['ctx']}/{it['kind']}"
        by[k] = by.get(k, 0) + 1
    print(len(items), "items", json.dumps(by, sort_keys=True))


if __name__ == "__main__":
    main()
