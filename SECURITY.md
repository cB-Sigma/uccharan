# Security policy

## Supported versions

Only the latest released version receives fixes.

## Reporting a vulnerability

Please do not open a public issue. Email Abhinav Gautam at <abhinav@codebuckets.in> with a
description, a way to reproduce the problem and the version you used. You should get a reply within
seven days. Once a fix is released the report is credited in the changelog unless you prefer
otherwise.

## What counts

uccharan takes untrusted text and, optionally, untrusted data files (`Phonemizer::load`,
`uccharan_load`, `loadLanguage(lang, bytes)`) and voice configuration JSON
(`PhonemeIdMap::from_json`). The core crate forbids `unsafe` code; the only `unsafe` is at the C
boundary in `crates/uccharan-capi`. Reports of the following are in scope:

- a crash, panic or hang on any input text;
- memory unsafety through the C API when it is used as documented;
- a malformed data file or voice configuration that causes anything worse than an error;
- output that grows without bound relative to the input.

Wrong pronunciations are bugs, not vulnerabilities; please use the issue tracker.
