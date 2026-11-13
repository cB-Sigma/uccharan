# misaki English dictionaries

`us_gold.json`, `us_silver.json`, `gb_gold.json` and `gb_silver.json` are copied unchanged from
the `misaki/data` directory of the **misaki 0.9.4** wheel published on PyPI
(`misaki-0.9.4-py3-none-any.whl`, project <https://github.com/hexgrad/misaki>, author hexgrad),
which is distributed under the Apache License 2.0 (`LICENSE` in this directory is the licence file
from that wheel).

| File | SHA-256 |
| --- | --- |
| `gb_gold.json` | `29e62f4b60261c88f7f3c2c7811ca3825978948090b72d2b27d565b729282f71` |
| `gb_silver.json` | `48131e2d92ccc41655f4543e87e0f938e71463eb5a54be7f0693bb712ebb6bce` |
| `us_gold.json` | `dc414872a49a28ae6c141463d502fd945f3b2fde040484fdc47d00cc4612686f` |
| `us_silver.json` | `de8f67be911bb6c659187b4a65fd966b6a30e56350e0f790d763210b053ac475` |

**Provenance.** Upstream does not document where the entries of these dictionaries originally come
from (open questions in hexgrad/misaki issues #107 and #113). This project relies on the upstream
Apache-2.0 licence statement for them.

`tools/lexgen` compiles them into `crates/uccharan/data/kokoro-en-us.ucd` and
`kokoro-en-gb.ucd`, which back the `kokoro` profile for English.
