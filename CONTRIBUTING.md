# Contributing

Thanks for helping. This file covers the rules that are specific to this project; the rest is
ordinary GitHub practice (fork, branch, pull request).

## Data

- Don't copy code, rule files or dictionary files from GPL-licensed projects such as espeak-ng.
- Every new data file needs its source and licence listed in `DATA_LICENSES.md`. Data under a
  share-alike, copyleft or non-commercial licence goes in a separate downloadable pack, never in
  the core packages.

## Setting up

```sh
cargo test --workspace                           # core, C API and tools
cargo run --release -p uccharan-lexgen           # rebuild crates/uccharan/data/*.ucd after changing data/sources
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings

# Python
pip install maturin pytest
cd bindings/python && maturin develop --release && pytest

# WebAssembly (needs the wasm32-unknown-unknown target and wasm-bindgen-cli 0.2.130)
node bindings/wasm/build.mjs && (cd bindings/wasm && node --test test/node.test.mjs)
node demo/build.mjs && node demo/serve.mjs       # http://127.0.0.1:8080/
```

## Changing pronunciations

1. Decide which behaviour you are changing. `compatible` output must keep matching what existing
   voices were trained on; if the reference is wrong, the fix belongs in `improved` mode.
2. Use the development sets only (`tools/eval/data/*_dev.tsv`, `tools/eval/gold/*.tsv`). The
   held-out sets and `tools/eval/gold/unseen` are for reporting, never for tuning; do not print
   their disagreements while you work on rules.
3. Run `python tools/eval/run_eval.py --lang … --set dev` and `python tools/eval/gold_eval.py`
   before and after, and put both numbers in the pull request.
4. Add a line to the unit or integration tests for the behaviour you fixed.

## Adding a language

See `docs/adding-a-language.md`. A language is accepted when it comes with held-out evaluation
sets, measured agreement for the profiles it claims, and provenance for its data.

## Style

- Rust: `rustfmt` defaults, no `unsafe` outside `crates/uccharan-capi`, no new runtime
  dependencies in the core without discussion.
- Documentation: plain language, British spelling, numbers only from runs that are in the
  repository's result files.
- Commits: one logical change each, written in the imperative ("Add Marathi schwa rules").

## Licence

By contributing you agree that your contribution is licensed under the Apache License 2.0.

## Releasing (maintainers)

1. Bump the version in `Cargo.toml` (`[workspace.package]`), `bindings/python/pyproject.toml`,
   `bindings/wasm/package.json` and the `uccharan_core` dependency of `crates/uccharan-capi`, and
   add a `CHANGELOG.md` section.
2. Actions → Release → Run workflow on `main`. The `targets` input chooses the registries
   (`pypi`, `crates`, `npm`; default all three). Each registry job is independent and skips a
   version that is already published.
3. The GitHub release and its tag `vX.Y.Z` are created only by a run that includes `pypi`, after
   the PyPI upload succeeded; the wheels, the sdist, the C library for each platform and the npm
   tarball are attached to it.

Secrets: `PYPI_API_TOKEN`, `CARGO_REGISTRY_TOKEN`, `NPM_TOKEN` (organisation secrets).
