"""Language packs that are not inside the wheel.

The wheel carries the en-us and Hindi data. British English (``en-gb``) and
the Kokoro English dictionaries (``kokoro-en-us``, ``kokoro-en-gb``) are packs:
``.ucd`` files attached to the GitHub release of the same version. They are
fetched on first use, checked against the SHA-256 sums below (which are part
of the wheel), and kept in a cache directory, so each is downloaded once.

Where a pack comes from, first match wins:

1. ``$UCCHARAN_DATA_DIR/<name>.ucd`` (for offline machines and containers);
2. the cache, ``$XDG_CACHE_HOME/uccharan/<version>/`` or
   ``~/.cache/uccharan/<version>/``;
3. the release download (skipped when ``UCCHARAN_OFFLINE=1``).

Every file is checked against its SHA-256 sum before it is used.

Optional packs are never fetched automatically. ``wikipron-en-us``,
``wikipron-en-gb`` and ``wikipron-hi`` hold pronunciations from Wiktionary
(via WikiPron) under **CC BY-SA 4.0**, not Apache-2.0: load one with
``uccharan.load_pack("wikipron-en-us")`` only if that licence suits your use.
They are used by the improved mode for words outside uccharan's own lexicon.
The transliteration packs ``xlit-hi``, ``xlit-mr``, ``xlit-bn``, ``xlit-ta``,
``xlit-te``, ``xlit-gu``, ``xlit-pa``, ``xlit-kn`` and ``xlit-ml`` (romanised
text into native script, see :mod:`uccharan.translit`) are **CC BY-SA 4.0** as
well and are loaded the same way: ``uccharan.load_pack("xlit-hi")``.
``uccharan.load_pack(path)`` loads any ``.ucd`` file directly, unchecked.
"""

from __future__ import annotations

import hashlib
import os
import tempfile
import urllib.request
from pathlib import Path

from . import _native

URL = "https://github.com/cB-Sigma/uccharan/releases/download/v{version}/{name}.ucd"

# name -> (sha256, size in bytes); written by tools/packs.py from crates/uccharan/data.
PACKS: dict[str, tuple[str, int]] = {
    # @@PACKS@@
    "ca": ("fa3bade0cdab6f861e50f0f382813bb75e22828bf60dc2dcbc3f48ac4d540c68", 195196),
    "cs": ("bd78333a5484f149de56975043d1a8b18d16356ee9ead3ad915b6fa17cc59a09", 77647),
    "da": ("b94a426b4dc1b6bacf611eeb34e39b71a85cb725ec9ca747198195cf30d89804", 489926),
    "de": ("5a604a6503873a2d7f1e53c647e39a463b7696d8a8b20baf6be4ad5c610ea9cf", 372646),
    "en-gb": ("c267d028790da5440ae90df87da7b131800f7932124afe356193e387aaab13ad", 707290),
    "en-us": ("337e2c2c25c3d1a376f857634c20e2a220bcc91e65876ec58623f2feaf997aac", 727189),
    "fi": ("cdf6c318663f16d41680139dcfd969415ef08804b689a26c25f77cbe8ddd2f52", 137291),
    "fr": ("6640eaed0e5a0a260dc84bf547fd10f9857bd83b61365e7699eb016072851949", 181315),
    "hi": ("970df73a97f846c7089d8794acce6f54f745ffcbfa208b77a5710ad84a2e3a31", 2964),
    "hu": ("9616e25cbd24cf9588cc6e2b2e777d0ad87b2fa51ae21db030fbbd612a2956e3", 45656),
    "it": ("f500dc08197da28721172218146ba468939964a8e7265f7b81ae6cf6d4449819", 188304),
    "kokoro-en-gb": ("dcd66185dfe6f43c0d244287ffbd895a0f17465e533d1d0394a7d9ed9d7ccea0", 2066805),
    "kokoro-en-us": ("05a70e6f2314c04fcfc7b0a50387025a4919436551cb69a6074417df0641aa76", 1914413),
    "nb": ("ae2b27494d29c31042a0d12d9120e216753942b30bb9534b40cfaa5c192c788f", 99555),
    "nl": ("74ac5c59314639fcc79c0b8880c25c5036c27c5acea4e762ce6cf1e876bd6ca0", 242384),
    "pl": ("e9012ae209810da0a5bd640963f72cb16f3c0d785a40eec27b7b050c27c91fa9", 239118),
    "pt-br": ("181a522f2df06ed6897ed67b2f0235f4162390ab215d15d72182caa863fb199c", 170933),
    "pt": ("7662c0faf867e2dff2da29f1a4b927ca2f9bfc9a979dab308e0ab44d80cdaec8", 215118),
    "ro": ("8f60693549f155fa6a30accbda95e8750e296668a9f14cf073f2b79bda6fd9d3", 308230),
    "sd": ("92f45a588217d4f677ce113239d50e2916c5c30cd27c01d4671009c5af909e13", 209454),
    "sv": ("0db60d4b4ab24d2e1d373eb185c346631bfd812797dc65a6383ad867b69e9af9", 171085),
    "tr": ("c52bbb871db4aa0f9df2a4be2b59a109962a316b02a73d156b62165ac76ce35b", 230491),
    "ur": ("acd79fd0c91e2feb76d64115ae24f5e68fc841866381185bc6dfeb36af9f7936", 542615),
    "wikipron-en-gb": ("53718c5458145f7a49cb0bc29c534f1eddf4f6424f240e654bc469f4884c8d4f", 728104),
    "wikipron-en-us": ("85946bafb7e1e9daab9909138c7ba38437ca6d42d8e673122a8eba76dfdf814b", 741599),
    "wikipron-hi": ("8f5d330038f296d4e712d3a17a0837268bd2a91033d7b029d8fad0952b10a889", 321723),
    "xlit-bn": ("0273a468778c752d8a070852abfb39cd0cd0a61b2148a58c20626a06e99fded1", 7836025),
    "xlit-gu": ("4f5c4e575c3d465643b8a3ef2ade0b4b1206736d156914f99213558c9cfd0e39", 5951140),
    "xlit-hi": ("59e5dde0354e4d4c55530baedc3034d5ed7eff0a382056a1d269d012b0c9ca52", 8666025),
    "xlit-kn": ("1ecd0a56e9d6fe4729554b25994c417d96fe98464f35d008a6f66482dfaca6b5", 6651694),
    "xlit-ml": ("cb87b660c5c662510ec5d690465072c84b782e56b31483a48e089576fcfa493c", 6378093),
    "xlit-mr": ("623f1a488324d18819d84177f21f3a900bd47025904fb80d5f7dbda2efb4bec8", 7355539),
    "xlit-pa": ("4938d0597c56d7de5e91d791ab2819ef8a3e402f14324ba76c75c479c521f5c4", 3132326),
    "xlit-ta": ("4cc9b3ea2400ee9ad689a09023ee8dd9286861819827e38fda7d8a3cc9048dc8", 7039209),
    "xlit-te": ("c6d63469c0184646d0ff4f12198313f409a536f2fbf269a127648ad488e59c83", 7025004),
    # @@END@@
}


class PackError(RuntimeError):
    """A pack could not be found, downloaded or verified."""


def cache_dir() -> Path:
    base = os.environ.get("XDG_CACHE_HOME") or os.path.join(os.path.expanduser("~"), ".cache")
    return Path(base) / "uccharan" / _native.__version__


def _verify(name: str, blob: bytes, where: str) -> bytes:
    want, size = PACKS[name]
    got = hashlib.sha256(blob).hexdigest()
    if got != want or len(blob) != size:
        raise PackError(f"{where}: SHA-256 {got} does not match pack {name!r} of uccharan {_native.__version__}")
    return blob


def fetch(name: str) -> bytes:
    """The verified bytes of a pack, from the data directory, the cache or the release."""
    if name not in PACKS:
        raise PackError(f"unknown pack {name!r}; packs: {', '.join(sorted(PACKS))}")
    data_dir = os.environ.get("UCCHARAN_DATA_DIR")
    if data_dir and (Path(data_dir) / f"{name}.ucd").is_file():
        p = Path(data_dir) / f"{name}.ucd"
        return _verify(name, p.read_bytes(), str(p))
    cached = cache_dir() / f"{name}.ucd"
    if cached.is_file():
        try:
            return _verify(name, cached.read_bytes(), str(cached))
        except PackError:
            cached.unlink(missing_ok=True)
    url = URL.format(version=_native.__version__, name=name)
    if os.environ.get("UCCHARAN_OFFLINE") == "1":
        raise PackError(
            f"pack {name!r} is not available offline: download {url} into $UCCHARAN_DATA_DIR "
            "or unset UCCHARAN_OFFLINE"
        )
    try:
        with urllib.request.urlopen(url, timeout=60) as r:  # noqa: S310 (fixed https URL)
            blob = r.read(PACKS[name][1] + 1)
    except OSError as e:
        raise PackError(f"could not download pack {name!r} from {url}: {e}") from e
    _verify(name, blob, url)
    try:
        cached.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.NamedTemporaryFile(dir=cached.parent, delete=False) as tmp:
            tmp.write(blob)
        os.replace(tmp.name, cached)
    except OSError:
        pass  # a read-only home still works, just without a cache
    return blob


def ensure(lang: str, profile: str = "espeak-ipa") -> None:
    """Load whatever packs ``lang`` with ``profile`` needs and are not loaded yet."""
    for name in _native.missing_packs(lang, profile):
        _native.load_pack(fetch(name))
