//! End-to-end fault-localization tests for `base64` properties.
//!
//! Each `#[test]` runs `crabcheck::quickcheck_with_locate!` on one property
//! from `etna-faultloc.rs`. Tests never panic — they print the LocateResult
//! and emit one `@@LOCATE@@ <json>` line per property so a harness can
//! collect machine-readable suspect summaries.

use std::fmt;

use base64::etna::{
    property_binhex_alphabet_matches_spec, property_decoded_len_estimate_does_not_panic,
    PropertyResult,
};
use crabcheck::quickcheck::{Arbitrary, Mutate};
use rand_etna::Rng;

#[derive(Clone)]
struct Bytes(Vec<u8>);
impl fmt::Debug for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Copy)]
struct Usize(usize);
impl fmt::Debug for Usize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<R: Rng> Arbitrary<R> for Bytes {
    fn generate(rng: &mut R, _n: usize) -> Self {
        let len = rng.random_range(0usize..=64);
        Bytes((0..len).map(|_| rng.random()).collect())
    }
}

const LEN_POOL: &[usize] = &[
    usize::MAX,
    usize::MAX - 1,
    usize::MAX - 2,
    usize::MAX - 3,
    usize::MAX - 4,
    usize::MAX - 7,
    usize::MAX / 2,
    usize::MAX / 4,
    1usize << 62,
    0,
    1,
    2,
];

impl<R: Rng> Arbitrary<R> for Usize {
    fn generate(rng: &mut R, _n: usize) -> Self {
        Usize(LEN_POOL[rng.random_range(0..LEN_POOL.len())])
    }
}

impl<R: Rng> Mutate<R> for Bytes {
    fn mutate(&self, rng: &mut R, _n: usize) -> Self {
        let mut out = self.0.clone();
        match rng.random_range(0u8..3) {
            0 if !out.is_empty() => {
                let i = rng.random_range(0..out.len());
                let b = rng.random_range(0u32..8);
                out[i] ^= 1u8 << b;
            }
            1 if out.len() < 64 => out.push(rng.random()),
            _ if !out.is_empty() => {
                out.pop();
            }
            _ => {}
        }
        Bytes(out)
    }
}

impl<R: Rng> Mutate<R> for Usize {
    fn mutate(&self, rng: &mut R, _n: usize) -> Self {
        let bit = rng.random_range(0u32..(usize::BITS));
        Usize(self.0 ^ (1usize << bit))
    }
}

fn to_opt(r: PropertyResult) -> Option<bool> {
    match r {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

fn binhex_alphabet_matches_spec_wrapper(Bytes(v): Bytes) -> Option<bool> {
    to_opt(property_binhex_alphabet_matches_spec(v))
}

fn decoded_len_estimate_does_not_panic_wrapper(Usize(n): Usize) -> Option<bool> {
    to_opt(property_decoded_len_estimate_does_not_panic(n))
}

fn emit_locate_json(r: &crabcheck::profiling::LocateResult) {
    use crabcheck::quickcheck::ResultStatus;
    let status = match &r.run.status {
        ResultStatus::Failed { .. } => "Failed",
        ResultStatus::Finished => "Finished",
        ResultStatus::GaveUp => "GaveUp",
        ResultStatus::TimedOut => "TimedOut",
        ResultStatus::Aborted { .. } => "Aborted",
    };
    let top = if let Some(s) = r.top() {
        serde_json::json!({
            "rank": s.rank,
            "file": s.region.file,
            "function": s.region.function,
            "start_line": s.region.start_line,
            "end_line": s.region.end_line,
            "ochiai": s.region.suspiciousness.ochiai,
            "delta": s.region.delta,
            "panic_overlap": s.panic_overlap,
            "confidence": format!("{}", s.confidence),
            "confidence_rule": s.confidence_rule,
        })
    } else {
        serde_json::Value::Null
    };
    let top_5: Vec<_> = r
        .suspects
        .iter()
        .take(5)
        .map(|s| {
            serde_json::json!({
                "rank": s.rank,
                "file": s.region.file,
                "function": s.region.function,
                "start_line": s.region.start_line,
                "end_line": s.region.end_line,
                "confidence": format!("{}", s.confidence),
                "confidence_rule": s.confidence_rule,
                "panic_overlap": s.panic_overlap,
            })
        })
        .collect();
    let diags: Vec<_> = r.diagnostics.iter().map(|d| d.tag()).collect();
    let out = serde_json::json!({
        "status": status,
        "passed": r.run.passed,
        "discarded": r.run.discarded,
        "n_panics": r.n_panics,
        "n_suspects": r.suspects.len(),
        "top": top,
        "top_5": top_5,
        "diagnostics": diags,
    });
    println!("@@LOCATE@@ {}", out);
}

#[test]
fn locate_binhex_alphabet_matches_spec() {
    let report =
        crabcheck::quickcheck_with_locate!(binhex_alphabet_matches_spec_wrapper, "base64");
    eprintln!("{report}");
    emit_locate_json(&report);
}

#[test]
fn locate_decoded_len_estimate_does_not_panic() {
    let report = crabcheck::quickcheck_with_locate!(
        decoded_len_estimate_does_not_panic_wrapper,
        "base64"
    );
    eprintln!("{report}");
    emit_locate_json(&report);
}
