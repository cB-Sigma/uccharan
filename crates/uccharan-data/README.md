# uccharan-data

Word tables of [uccharan](https://crates.io/crates/uccharan)'s languages other than English and
Hindi: Urdu, Sindhi, German, Dutch, French, Italian, Portuguese (Brazil and Portugal), Polish,
Turkish, Czech, Swedish, Catalan, Romanian, Hungarian, Finnish, Norwegian Bokmål and Danish.

You do not use this crate directly. uccharan depends on it when you ask for word tables to be
compiled into your program:

```toml
uccharan = { version = "1", features = ["bundle"] }      # every word table
uccharan = { version = "1", features = ["embed-de"] }    # only German
```

Without those features uccharan reads a language's table from a pack file at its first use (see
the `uccharan::packs` module).

## Licence

Apache-2.0. Every table, its source and its licence are listed in `DATA_LICENSES.md`.
