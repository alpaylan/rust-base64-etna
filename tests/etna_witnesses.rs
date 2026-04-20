//! Deterministic witness tests for the ETNA workload.
//!
//! Each `witness_*_case_*` test calls one of the framework-neutral
//! `property_*` functions with frozen inputs. On the base HEAD every witness
//! passes. On a mutated variant (marauders-activated or checked out via
//! `etna/<variant>`), the corresponding witness fails.

use base64::etna::{
    property_binhex_alphabet_matches_spec, property_decoded_len_estimate_does_not_panic,
    PropertyResult,
};

fn assert_pass(label: &str, p: PropertyResult) {
    match p {
        PropertyResult::Pass => {}
        PropertyResult::Discard => panic!("witness {}: property returned Discard", label),
        PropertyResult::Fail(msg) => panic!("witness {}: property failed: {}", label, msg),
    }
}

// Exercises any index where the buggy BIN_HEX alphabet differs from the spec.
// "Hello, world!" encodes through positions that traverse the 7/f swap, so a
// buggy alphabet produces a different encoded byte at one of those positions.
#[test]
fn witness_binhex_alphabet_matches_spec_case_hello_world() {
    let input = b"Hello, world!".to_vec();
    assert_pass(
        "binhex_alphabet_matches_spec_case_hello_world",
        property_binhex_alphabet_matches_spec(input),
    );
}

// A larger byte sequence that exercises the full 0..=255 range on its way
// through the alphabet table, so we know the witness hits a disagreeing index.
#[test]
fn witness_binhex_alphabet_matches_spec_case_full_range() {
    let input: Vec<u8> = (0u8..=255u8).collect();
    assert_pass(
        "binhex_alphabet_matches_spec_case_full_range",
        property_binhex_alphabet_matches_spec(input),
    );
}

// The pre-fix `decoded_len_estimate` panicked via `.checked_add(3).expect(..)`
// for `encoded_len = usize::MAX`. The current non-overflowing formula returns
// a plain value.
#[test]
fn witness_decoded_len_estimate_does_not_panic_case_usize_max() {
    assert_pass(
        "decoded_len_estimate_does_not_panic_case_usize_max",
        property_decoded_len_estimate_does_not_panic(usize::MAX),
    );
}

// `usize::MAX - 2` still triggers the pre-fix `checked_add(3)` overflow.
#[test]
fn witness_decoded_len_estimate_does_not_panic_case_near_max() {
    assert_pass(
        "decoded_len_estimate_does_not_panic_case_near_max",
        property_decoded_len_estimate_does_not_panic(usize::MAX - 2),
    );
}
