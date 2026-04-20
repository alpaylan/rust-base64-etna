//! ETNA framework-neutral property functions for rust-base64.
//!
//! Each `property_<name>` is a pure function taking concrete, owned inputs and
//! returning `PropertyResult`. Framework adapters in `src/bin/etna.rs` and
//! witness tests in `tests/etna_witnesses.rs` all call these functions
//! directly — invariants are never re-implemented inside an adapter.

#![allow(missing_docs)]

use crate::alphabet::{self, Alphabet};
use crate::engine::{self, general_purpose::GeneralPurpose, DecodePaddingMode, Engine as _};

#[derive(Debug)]
pub enum PropertyResult {
    Pass,
    Fail(String),
    Discard,
}

/// The BinHex 4.0 alphabet as specified by
/// <http://files.stairways.com/other/binhex-40-specs-info.txt>.
///
/// Encoded here directly from the spec so the property function has a
/// framework-independent oracle to compare against. Note the presence of `f`
/// and the absence of `7`.
const BINHEX_SPEC: &str =
    "!\"#$%&'()*+,-012345689@ABCDEFGHIJKLMNPQRSTUVXYZ[`abcdefhijklmpqr";

fn binhex_config() -> engine::GeneralPurposeConfig {
    engine::GeneralPurposeConfig::new()
        .with_decode_allow_trailing_bits(true)
        .with_encode_padding(false)
        .with_decode_padding_mode(DecodePaddingMode::RequireNone)
}

/// Invariant: the crate's `BIN_HEX` alphabet must match the BinHex 4.0 spec
/// alphabet (`BINHEX_SPEC`). We check this behaviorally by encoding the same
/// input under both the crate alphabet and a fresh `Alphabet::new(BINHEX_SPEC)`
/// and demanding that both produce identical output.
///
/// Bug this catches:
/// - `binhex_alphabet_838355e_1`: the pre-fix constant used
///   `...3456789...abcdeh...` (swaps `7` in for `f`). Encoding any input that
///   touches an index where the two alphabets disagree (indexes 7, 8, 9, 10,
///   37, 38, 39, 40) produces a different byte at that position and the check
///   fails.
pub fn property_binhex_alphabet_matches_spec(input: Vec<u8>) -> PropertyResult {
    if input.len() > 512 {
        return PropertyResult::Discard;
    }
    let cfg = binhex_config();
    let crate_engine = GeneralPurpose::new(&alphabet::BIN_HEX, cfg);
    let spec_alphabet = match Alphabet::new(BINHEX_SPEC) {
        Ok(a) => a,
        Err(e) => return PropertyResult::Fail(format!("spec alphabet rejected: {:?}", e)),
    };
    let spec_engine = GeneralPurpose::new(&spec_alphabet, cfg);

    let crate_encoded = crate_engine.encode(&input);
    let spec_encoded = spec_engine.encode(&input);
    if crate_encoded != spec_encoded {
        return PropertyResult::Fail(format!(
            "BIN_HEX alphabet disagrees with spec: crate={:?} spec={:?}",
            crate_encoded, spec_encoded
        ));
    }

    // Round-trip sanity — a correct alphabet must also round-trip.
    let decoded = match crate_engine.decode(crate_encoded.as_bytes()) {
        Ok(v) => v,
        Err(e) => {
            return PropertyResult::Fail(format!("round-trip decode failed: {}", e));
        }
    };
    if decoded != input {
        return PropertyResult::Fail(format!(
            "round-trip mismatch: expected {:?}, got {:?}",
            input, decoded
        ));
    }
    PropertyResult::Pass
}

/// Invariant: `base64::decoded_len_estimate(encoded_len)` must be a total
/// function — it must return a value for every `usize` input without panicking.
///
/// Bug this catches:
/// - `decoded_len_overflow_fa47981_1`: the pre-fix `GeneralPurposeEstimate::new`
///   used `encoded_len.checked_add(3).expect("Overflow ...")`, which panics for
///   any `encoded_len > usize::MAX - 3`. The public `decoded_len_estimate`
///   delegates to that internal method, so the panic propagates all the way
///   out — and a crate that panics on public-API input it advertises as
///   `usize` is broken.
pub fn property_decoded_len_estimate_does_not_panic(encoded_len: usize) -> PropertyResult {
    match std::panic::catch_unwind(|| crate::decoded_len_estimate(encoded_len)) {
        Ok(_) => PropertyResult::Pass,
        Err(p) => {
            let msg = if let Some(s) = p.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = p.downcast_ref::<&str>() {
                (*s).to_string()
            } else {
                "panic with non-string payload".to_string()
            };
            PropertyResult::Fail(format!(
                "decoded_len_estimate({}) panicked: {}",
                encoded_len, msg
            ))
        }
    }
}
