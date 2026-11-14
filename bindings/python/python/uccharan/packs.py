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
    "en-gb": ("c267d028790da5440ae90df87da7b131800f7932124afe356193e387aaab13ad", 707290),
    "en-us": ("337e2c2c25c3d1a376f857634c20e2a220bcc91e65876ec58623f2feaf997aac", 727189),
    "hi": ("970df73a97f846c7089d8794acce6f54f745ffcbfa208b77a5710ad84a2e3a31", 2964),
    "kokoro-en-gb": ("dcd66185dfe6f43c0d244287ffbd895a0f17465e533d1d0394a7d9ed9d7ccea0", 2066805),
    "kokoro-en-us": ("05a70e6f2314c04fcfc7b0a50387025a4919436551cb69a6074417df0641aa76", 1914413),
    "sd": ("03df601bf97c4588a8be6c3a01cb3c0d7678309fd819e4dd91027e60b8d67707", 187440),
    "ur": ("3b5fd97fb49d2cb21ae069e461269ba9e3c5f1c6dc5f7dd7f0e3ad4065de078a", 378180),
    "wikipron-en-gb": ("53718c5458145f7a49cb0bc29c534f1eddf4f6424f240e654bc469f4884c8d4f", 728104),
    "wikipron-en-us": ("85946bafb7e1e9daab9909138c7ba38437ca6d42d8e673122a8eba76dfdf814b", 741599),
    "wikipron-hi": ("8f5d330038f296d4e712d3a17a0837268bd2a91033d7b029d8fad0952b10a889", 321723),
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
