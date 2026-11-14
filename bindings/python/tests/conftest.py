import os
from pathlib import Path

# Tests never download: the packs come from the source tree, and are still
# checked against the sums in uccharan/packs.py.
_CRATES = Path(__file__).resolve().parents[3] / "crates"
_DATA = _CRATES / "uccharan" / "data"
_TABLES = _CRATES / "uccharan-data" / "data"
os.environ.setdefault("UCCHARAN_DATA_DIR", os.pathsep.join([str(_DATA), str(_TABLES)]))
os.environ["UCCHARAN_OFFLINE"] = "1"
