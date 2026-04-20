// ETNA workload runner for rust-base64.
//
// Usage: cargo run --release --bin etna -- <tool> <property>
//   tool:     etna | proptest | quickcheck | crabcheck | hegel
//   property: BinhexAlphabetMatchesSpec | DecodedLenEstimateDoesNotPanic | All
//
// Every invocation prints exactly one JSON line to stdout and exits 0
// (except argv parsing, which exits 2).

use base64::etna::{
    property_binhex_alphabet_matches_spec, property_decoded_len_estimate_does_not_panic,
    PropertyResult,
};
use crabcheck::quickcheck as crabcheck_qc;
use crabcheck::quickcheck::Arbitrary as CcArbitrary;
use hegel::{generators as hgen, HealthCheck, Hegel, Settings as HegelSettings, TestCase};
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestCaseError, TestError, TestRunner};
use quickcheck_etna::{Arbitrary as QcArbitrary, Gen, QuickCheck, ResultStatus, TestResult};
use rand::Rng;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Default, Clone, Copy)]
struct Metrics {
    inputs: u64,
    elapsed_us: u128,
}

impl Metrics {
    fn combine(self, other: Metrics) -> Metrics {
        Metrics {
            inputs: self.inputs + other.inputs,
            elapsed_us: self.elapsed_us + other.elapsed_us,
        }
    }
}

type Outcome = (Result<(), String>, Metrics);

fn to_err(r: PropertyResult) -> Result<(), String> {
    match r {
        PropertyResult::Pass | PropertyResult::Discard => Ok(()),
        PropertyResult::Fail(m) => Err(m),
    }
}

const ALL_PROPERTIES: &[&str] = &["BinhexAlphabetMatchesSpec", "DecodedLenEstimateDoesNotPanic"];

fn cases_budget() -> u64 {
    std::env::var("ETNA_CASES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(200)
}

fn run_all<F: FnMut(&str) -> Outcome>(mut f: F) -> Outcome {
    let mut total = Metrics::default();
    for p in ALL_PROPERTIES {
        let (r, m) = f(p);
        total = total.combine(m);
        if let Err(e) = r {
            return (Err(e), total);
        }
    }
    (Ok(()), total)
}

// ============================================================================
// Canonical witness inputs — used by `tool=etna` to deterministically replay
// the single most-reliable counterexample per property.
// ============================================================================

// "Hello, world!" encodes to BinHex positions that traverse the 7/f swap.
fn canonical_binhex_sample() -> Vec<u8> {
    b"Hello, world!".to_vec()
}

// usize::MAX is the canonical input that trips the `checked_add(3).expect(..)`
// panic in the buggy formula.
fn canonical_encoded_len_overflow() -> usize {
    usize::MAX
}

fn check_binhex_alphabet_matches_spec() -> Result<(), String> {
    to_err(property_binhex_alphabet_matches_spec(canonical_binhex_sample()))
}

fn check_decoded_len_estimate_does_not_panic() -> Result<(), String> {
    to_err(property_decoded_len_estimate_does_not_panic(
        canonical_encoded_len_overflow(),
    ))
}

fn run_etna_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_etna_property);
    }
    let t0 = Instant::now();
    let result = match property {
        "BinhexAlphabetMatchesSpec" => check_binhex_alphabet_matches_spec(),
        "DecodedLenEstimateDoesNotPanic" => check_decoded_len_estimate_does_not_panic(),
        _ => {
            return (
                Err(format!("Unknown property for etna: {property}")),
                Metrics::default(),
            );
        }
    };
    (
        result,
        Metrics {
            inputs: 1,
            elapsed_us: t0.elapsed().as_micros(),
        },
    )
}

// ============================================================================
// Shared biased generators
// ============================================================================

// A byte pool skewed toward printable ASCII + a few non-ASCII bytes to
// exercise alphabet indices that overlap the BIN_HEX 7/f disagreement window.
const BYTE_POOL: &[u8] = &[
    b'H', b'e', b'l', b'o', b',', b' ', b'w', b'r', b'd', b'!', b'f', b'F', b'7', b'8', b'9', b'a',
    b'b', b'c', b'd', b'e', b'0', b'1', b'2', b'3', b'4', b'5', b'6', 0x00, 0x7F, 0xFF,
];

#[derive(Clone)]
struct BinhexInput(Vec<u8>);

impl fmt::Debug for BinhexInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
impl fmt::Display for BinhexInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

fn random_binhex_input<R: Rng>(rng: &mut R) -> Vec<u8> {
    let len = rng.random_range(3usize..=32);
    (0..len)
        .map(|_| BYTE_POOL[rng.random_range(0..BYTE_POOL.len())])
        .collect()
}

impl QcArbitrary for BinhexInput {
    fn arbitrary(g: &mut Gen) -> Self {
        let len = g.random_range(3usize..=32);
        let v = (0..len)
            .map(|_| BYTE_POOL[g.random_range(0..BYTE_POOL.len())])
            .collect();
        BinhexInput(v)
    }
}

impl<R: Rng> CcArbitrary<R> for BinhexInput {
    fn generate(rng: &mut R, _n: usize) -> Self {
        BinhexInput(random_binhex_input(rng))
    }
}

// For the overflow property, we want `usize` values that are plausibly
// overflow-triggering. The pool is heavily skewed toward `usize::MAX -
// small_k`, with a few zero-ish values mixed in so the generator doesn't pin
// to a single tight bucket.
const LEN_POOL: &[usize] = &[
    usize::MAX,
    usize::MAX - 1,
    usize::MAX - 2,
    usize::MAX - 3,
    usize::MAX - 4,
    usize::MAX - 7,
    usize::MAX / 2,
    usize::MAX / 2 + 1,
    1 << 62,
    1 << 63,
    1_000_000,
    1024,
    0,
    1,
    4,
    255,
];

#[derive(Clone, Copy)]
struct OverflowLen(usize);

impl fmt::Debug for OverflowLen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl fmt::Display for OverflowLen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn random_overflow_len<R: Rng>(rng: &mut R) -> usize {
    LEN_POOL[rng.random_range(0..LEN_POOL.len())]
}

impl QcArbitrary for OverflowLen {
    fn arbitrary(g: &mut Gen) -> Self {
        OverflowLen(LEN_POOL[g.random_range(0..LEN_POOL.len())])
    }
}

impl<R: Rng> CcArbitrary<R> for OverflowLen {
    fn generate(rng: &mut R, _n: usize) -> Self {
        OverflowLen(random_overflow_len(rng))
    }
}

// ============================================================================
// proptest adapter
// ============================================================================

fn binhex_input_strategy() -> BoxedStrategy<Vec<u8>> {
    proptest::collection::vec(proptest::sample::select(BYTE_POOL.to_vec()), 3..=32).boxed()
}

fn overflow_len_strategy() -> BoxedStrategy<usize> {
    proptest::sample::select(LEN_POOL.to_vec()).boxed()
}

fn run_proptest_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_proptest_property);
    }
    let counter = Arc::new(AtomicU64::new(0));
    let t0 = Instant::now();
    let cfg = ProptestConfig {
        cases: cases_budget().min(u32::MAX as u64) as u32,
        max_shrink_iters: 32,
        failure_persistence: None,
        ..ProptestConfig::default()
    };
    let mut runner = TestRunner::new(cfg);
    let c = counter.clone();
    let result: Result<(), String> = match property {
        "BinhexAlphabetMatchesSpec" => runner
            .run(&binhex_input_strategy(), move |v| {
                c.fetch_add(1, Ordering::Relaxed);
                let cex = format!("({:?})", v);
                let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_binhex_alphabet_matches_spec(v)
                }));
                match out {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => Ok(()),
                    Ok(PropertyResult::Fail(_)) | Err(_) => Err(TestCaseError::fail(cex)),
                }
            })
            .map_err(|e| match e {
                TestError::Fail(reason, _) => reason.to_string(),
                other => other.to_string(),
            }),
        "DecodedLenEstimateDoesNotPanic" => runner
            .run(&overflow_len_strategy(), move |n| {
                c.fetch_add(1, Ordering::Relaxed);
                let cex = format!("({})", n);
                let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_decoded_len_estimate_does_not_panic(n)
                }));
                match out {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => Ok(()),
                    Ok(PropertyResult::Fail(_)) | Err(_) => Err(TestCaseError::fail(cex)),
                }
            })
            .map_err(|e| match e {
                TestError::Fail(reason, _) => reason.to_string(),
                other => other.to_string(),
            }),
        _ => {
            return (
                Err(format!("Unknown property for proptest: {property}")),
                Metrics::default(),
            );
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = counter.load(Ordering::Relaxed);
    (result, Metrics { inputs, elapsed_us })
}

// ============================================================================
// quickcheck adapter (fork with `etna` feature — fn-pointer API)
// ============================================================================

static QC_COUNTER: AtomicU64 = AtomicU64::new(0);

fn qc_binhex_alphabet_matches_spec(BinhexInput(v): BinhexInput) -> TestResult {
    QC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_binhex_alphabet_matches_spec(v) {
        PropertyResult::Pass => TestResult::passed(),
        PropertyResult::Discard => TestResult::discard(),
        PropertyResult::Fail(_) => TestResult::failed(),
    }
}

fn qc_decoded_len_estimate_does_not_panic(OverflowLen(n): OverflowLen) -> TestResult {
    QC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_decoded_len_estimate_does_not_panic(n) {
        PropertyResult::Pass => TestResult::passed(),
        PropertyResult::Discard => TestResult::discard(),
        PropertyResult::Fail(_) => TestResult::failed(),
    }
}

fn run_quickcheck_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_quickcheck_property);
    }
    QC_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let budget = cases_budget();
    let mut qc = QuickCheck::new()
        .tests(budget)
        .max_tests(budget.saturating_mul(4))
        .max_time(Duration::from_secs(86_400));
    let result = match property {
        "BinhexAlphabetMatchesSpec" => qc.quicktest(
            qc_binhex_alphabet_matches_spec as fn(BinhexInput) -> TestResult,
        ),
        "DecodedLenEstimateDoesNotPanic" => qc.quicktest(
            qc_decoded_len_estimate_does_not_panic as fn(OverflowLen) -> TestResult,
        ),
        _ => {
            return (
                Err(format!("Unknown property for quickcheck: {property}")),
                Metrics::default(),
            );
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = QC_COUNTER.load(Ordering::Relaxed);
    let status = match result.status {
        ResultStatus::Finished => Ok(()),
        ResultStatus::Failed { arguments } => Err(format!("({})", arguments.join(" "))),
        ResultStatus::Aborted { err } => Err(format!("quickcheck aborted: {err:?}")),
        ResultStatus::TimedOut => Err("quickcheck timed out".to_string()),
        ResultStatus::GaveUp => Err(format!(
            "quickcheck gave up after {} tests",
            result.n_tests_passed
        )),
    };
    (status, Metrics { inputs, elapsed_us })
}

// ============================================================================
// crabcheck adapter (fn-pointer API)
// ============================================================================

static CC_COUNTER: AtomicU64 = AtomicU64::new(0);

fn cc_binhex_alphabet_matches_spec(BinhexInput(v): BinhexInput) -> Option<bool> {
    CC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_binhex_alphabet_matches_spec(v) {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

fn cc_decoded_len_estimate_does_not_panic(OverflowLen(n): OverflowLen) -> Option<bool> {
    CC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_decoded_len_estimate_does_not_panic(n) {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

fn run_crabcheck_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_crabcheck_property);
    }
    CC_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let cc_config = crabcheck_qc::Config {
        tests: cases_budget(),
    };
    let result = match property {
        "BinhexAlphabetMatchesSpec" => {
            crabcheck_qc::quickcheck_with_config(cc_config, cc_binhex_alphabet_matches_spec)
        }
        "DecodedLenEstimateDoesNotPanic" => crabcheck_qc::quickcheck_with_config(
            cc_config,
            cc_decoded_len_estimate_does_not_panic,
        ),
        _ => {
            return (
                Err(format!("Unknown property for crabcheck: {property}")),
                Metrics::default(),
            );
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = CC_COUNTER.load(Ordering::Relaxed);
    let status = match result.status {
        crabcheck_qc::ResultStatus::Finished => Ok(()),
        crabcheck_qc::ResultStatus::Failed { arguments } => {
            Err(format!("({})", arguments.join(" ")))
        }
        crabcheck_qc::ResultStatus::TimedOut => Err("crabcheck timed out".to_string()),
        crabcheck_qc::ResultStatus::GaveUp => Err(format!(
            "crabcheck gave up: passed={}, discarded={}",
            result.passed, result.discarded
        )),
        crabcheck_qc::ResultStatus::Aborted { error } => {
            Err(format!("crabcheck aborted: {error}"))
        }
    };
    (status, Metrics { inputs, elapsed_us })
}

// ============================================================================
// hegel adapter (real hegeltest 0.3.7 — panic-on-cex API)
// ============================================================================

static HG_COUNTER: AtomicU64 = AtomicU64::new(0);

fn hegel_settings() -> HegelSettings {
    HegelSettings::new()
        .test_cases(cases_budget())
        .suppress_health_check(HealthCheck::all())
}

fn hg_draw_byte_from(tc: &TestCase, pool: &[u8]) -> u8 {
    let idx = tc.draw(
        hgen::integers::<usize>()
            .min_value(0)
            .max_value(pool.len() - 1),
    );
    pool[idx]
}

fn hg_draw_binhex_input(tc: &TestCase) -> Vec<u8> {
    let len = tc.draw(hgen::integers::<usize>().min_value(3).max_value(32));
    (0..len).map(|_| hg_draw_byte_from(tc, BYTE_POOL)).collect()
}

fn hg_draw_overflow_len(tc: &TestCase) -> usize {
    let idx = tc.draw(
        hgen::integers::<usize>()
            .min_value(0)
            .max_value(LEN_POOL.len() - 1),
    );
    LEN_POOL[idx]
}

fn run_hegel_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_hegel_property);
    }
    HG_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let settings = hegel_settings();
    let run_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match property {
        "BinhexAlphabetMatchesSpec" => {
            Hegel::new(|tc: TestCase| {
                HG_COUNTER.fetch_add(1, Ordering::Relaxed);
                let v = hg_draw_binhex_input(&tc);
                let cex = format!("({:?})", v);
                let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_binhex_alphabet_matches_spec(v)
                }));
                match out {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => {}
                    Ok(PropertyResult::Fail(_)) | Err(_) => panic!("{}", cex),
                }
            })
            .settings(settings.clone())
            .run();
        }
        "DecodedLenEstimateDoesNotPanic" => {
            Hegel::new(|tc: TestCase| {
                HG_COUNTER.fetch_add(1, Ordering::Relaxed);
                let n = hg_draw_overflow_len(&tc);
                let cex = format!("({})", n);
                let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_decoded_len_estimate_does_not_panic(n)
                }));
                match out {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => {}
                    Ok(PropertyResult::Fail(_)) | Err(_) => panic!("{}", cex),
                }
            })
            .settings(settings.clone())
            .run();
        }
        _ => panic!("__unknown_property:{}", property),
    }));
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = HG_COUNTER.load(Ordering::Relaxed);
    let metrics = Metrics { inputs, elapsed_us };
    let status = match run_result {
        Ok(()) => Ok(()),
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "hegel panicked with non-string payload".to_string()
            };
            if let Some(rest) = msg.strip_prefix("__unknown_property:") {
                return (
                    Err(format!("Unknown property for hegel: {rest}")),
                    Metrics::default(),
                );
            }
            Err(msg
                .strip_prefix("Property test failed: ")
                .unwrap_or(&msg)
                .to_string())
        }
    };
    (status, metrics)
}

// ============================================================================
// dispatch + main
// ============================================================================

fn run(tool: &str, property: &str) -> Outcome {
    match tool {
        "etna" => run_etna_property(property),
        "proptest" => run_proptest_property(property),
        "quickcheck" => run_quickcheck_property(property),
        "crabcheck" => run_crabcheck_property(property),
        "hegel" => run_hegel_property(property),
        _ => (
            Err(format!("Unknown tool: {tool}")),
            Metrics::default(),
        ),
    }
}

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn emit_json(
    tool: &str,
    property: &str,
    status: &str,
    metrics: Metrics,
    counterexample: Option<&str>,
    error: Option<&str>,
) {
    let cex = counterexample.map_or("null".to_string(), json_str);
    let err = error.map_or("null".to_string(), json_str);
    println!(
        "{{\"status\":{},\"tests\":{},\"discards\":0,\"time\":{},\"counterexample\":{},\"error\":{},\"tool\":{},\"property\":{}}}",
        json_str(status),
        metrics.inputs,
        json_str(&format!("{}us", metrics.elapsed_us)),
        cex,
        err,
        json_str(tool),
        json_str(property),
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <tool> <property>", args[0]);
        eprintln!("Tools: etna | proptest | quickcheck | crabcheck | hegel");
        eprintln!(
            "Properties: BinhexAlphabetMatchesSpec | DecodedLenEstimateDoesNotPanic | All"
        );
        std::process::exit(2);
    }
    let (tool, property) = (args[1].as_str(), args[2].as_str());

    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(tool, property)));
    std::panic::set_hook(previous_hook);

    let (result, metrics) = match caught {
        Ok(outcome) => outcome,
        Err(payload) => {
            let msg = if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = payload.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "panic with non-string payload".to_string()
            };
            emit_json(tool, property, "aborted", Metrics::default(), None, Some(&msg));
            return;
        }
    };

    match result {
        Ok(()) => emit_json(tool, property, "passed", metrics, None, None),
        Err(e) => emit_json(tool, property, "failed", metrics, Some(&e), None),
    }
}
