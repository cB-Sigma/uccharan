import os
from pathlib import Path

# Tests never download: the packs come from the source tree, and are still
# checked against the sums in uccharan/packs.py.
_DATA = Path(__file__).resolve().parents[3] / "crates" / "uccharan" / "data"
os.environ.setdefault("UCCHARAN_DATA_DIR", str(_DATA))
os.environ["UCCHARAN_OFFLINE"] = "1"
