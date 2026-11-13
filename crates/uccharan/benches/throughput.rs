//! Throughput, latency and start-up benchmarks.
//!
//! ```text
//! cargo bench -p uccharan
//! ```
//!
//! * `lexicon_path/*`  — running text whose words are in the lexicon;
//! * `rule_path/*`     — made-up words that only the letter-to-sound rules can read;
//! * `hindi/*`         — Devanagari text (rules only);
//! * `batch/*`         — 2 000 sentences on 1, 2 and 4 threads;
//! * `cold_start`      — building a phonemizer from the embedded data;
//! * `profiles/*`      — rendering cost of each output profile.
//!
//! Throughput is reported in input words per second ("elements").

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use uccharan::profile::{self, Profile};
use uccharan::{Lang, Mode, Normalize, Options, Phonemizer};

const EN: &str =
    "The quick brown fox jumps over the lazy dog while the children watch from the window. \
She said that the weather would improve before the end of the week, but nobody believed her. \
We have been waiting for almost an hour, and the train still has not arrived at the station. \
Could you tell me where the nearest post office is, please? It is just around the corner.";

const HI: &str = "आज मौसम बहुत अच्छा है और हम सब बाहर घूमने जा रहे हैं। \
क्या आप मुझे बता सकते हैं कि रेलवे स्टेशन कहाँ है? वह यहाँ से ज़्यादा दूर नहीं है। \
मेरे पिताजी हर सुबह अख़बार पढ़ते हैं और चाय पीते हैं। बच्चे स्कूल से लौटकर खेलने चले गए।";

/// Pronounceable strings that are in no dictionary.
fn nonsense(n: usize) -> String {
    let onsets = [
        "bl", "tr", "sk", "m", "d", "fr", "gl", "p", "v", "z", "st", "kr", "n", "sh", "th",
    ];
    let nuclei = ["a", "e", "i", "o", "u", "ai", "ou", "ee", "oa"];
    let codas = ["nt", "rk", "s", "m", "ld", "p", "x", "ng", "t", ""];
    let mut state = 0x2545_f491u32;
    let mut next = |m: usize| {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        state as usize % m
    };
    let mut out = String::new();
    for _ in 0..n {
        for _ in 0..2 + next(2) {
            out.push_str(onsets[next(onsets.len())]);
            out.push_str(nuclei[next(nuclei.len())]);
            out.push_str(codas[next(codas.len())]);
        }
        out.push(' ');
    }
    out
}

fn words(s: &str) -> u64 {
    s.split_whitespace().count() as u64
}

fn benches(c: &mut Criterion) {
    let p = Phonemizer::new();
    let us = Options::new(Lang::EnUs);
    let gb = Options::new(Lang::EnGb);
    let hi = Options::new(Lang::Hi);

    let mut g = c.benchmark_group("lexicon_path");
    g.throughput(Throughput::Elements(words(EN)));
    g.bench_function("en-us", |b| b.iter(|| p.phonemize(black_box(EN), &us)));
    g.bench_function("en-gb", |b| b.iter(|| p.phonemize(black_box(EN), &gb)));
    let mut improved = us.clone();
    improved.mode = Mode::Improved;
    g.bench_function("en-us-improved", |b| {
        b.iter(|| p.phonemize(black_box(EN), &improved))
    });
    g.finish();

    let rare = nonsense(64);
    let mut g = c.benchmark_group("rule_path");
    g.throughput(Throughput::Elements(words(&rare)));
    g.bench_function("en-us", |b| b.iter(|| p.phonemize(black_box(&rare), &us)));
    g.finish();

    let mut g = c.benchmark_group("hindi");
    g.throughput(Throughput::Elements(words(HI)));
    g.bench_function("compatible", |b| b.iter(|| p.phonemize(black_box(HI), &hi)));
    let mut hi_improved = hi.clone();
    hi_improved.mode = Mode::Improved;
    g.bench_function("improved", |b| {
        b.iter(|| p.phonemize(black_box(HI), &hi_improved))
    });
    g.finish();

    let mut g = c.benchmark_group("numbers");
    let nums = "It cost $1,234.56 on 2024-01-05 at 3:30pm, about 25% more than the 1999 price of 950 dollars.";
    g.throughput(Throughput::Elements(words(nums)));
    g.bench_function("natural", |b| b.iter(|| p.phonemize(black_box(nums), &us)));
    let mut like = us.clone();
    like.normalize = Normalize::EspeakLike;
    g.bench_function("reference-style", |b| {
        b.iter(|| p.phonemize(black_box(nums), &like))
    });
    g.finish();

    let sentences: Vec<&str> = EN
        .split_inclusive(['.', '?'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let batch: Vec<&str> = sentences.iter().cycle().take(2000).copied().collect();
    let batch_words: u64 = batch.iter().map(|s| words(s)).sum();
    let mut g = c.benchmark_group("batch");
    g.throughput(Throughput::Elements(batch_words));
    g.sample_size(20);
    for threads in [1usize, 2, 4] {
        g.bench_function(format!("{threads}-threads"), |b| {
            b.iter(|| p.render_batch(black_box(&batch), &us, Profile::EspeakIpa, true, threads))
        });
    }
    g.finish();

    c.bench_function("cold_start", |b| {
        b.iter_batched(
            || (),
            |_| black_box(Phonemizer::new()).phonemize("Hello.", &us),
            BatchSize::SmallInput,
        )
    });

    let analysed = p.analyze(EN, &us);
    let mut g = c.benchmark_group("profiles");
    g.throughput(Throughput::Elements(words(EN)));
    g.bench_function("analyze-only", |b| b.iter(|| p.analyze(black_box(EN), &us)));
    g.bench_function("render-piper", |b| {
        b.iter(|| profile::piper_sentences(black_box(&analysed)))
    });
    g.bench_function("render-sherpa-codepoints", |b| {
        b.iter(|| profile::sherpa_codepoints(black_box(&analysed)))
    });
    g.bench_function("render-kokoro", |b| {
        b.iter(|| profile::kokoro_tokens(black_box(&analysed), Lang::EnUs))
    });
    g.finish();
}

criterion_group!(all, benches);
criterion_main!(all);
