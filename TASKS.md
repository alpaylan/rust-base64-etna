# rust-base64 — ETNA Tasks

Total tasks: 8

ETNA tasks are **mutation/property/witness triplets**. Each row below is one runnable task. The `<PropertyKey>` token in the command column uses the PascalCase key recognised by `src/bin/etna.rs`; passing `All` runs every property for the named framework in a single invocation.

## Property keys

| Property | PropertyKey |
|----------|-------------|
| `property_binhex_alphabet_matches_spec` | `BinhexAlphabetMatchesSpec` |
| `property_decoded_len_estimate_does_not_panic` | `DecodedLenEstimateDoesNotPanic` |

## Task Index

| Task | Variant | Framework | Property | Witness | Command |
|------|---------|-----------|----------|---------|---------|
| 001 | `binhex_alphabet_838355e_1` | proptest | `property_binhex_alphabet_matches_spec` | `witness_binhex_alphabet_matches_spec_case_hello_world` | `cargo run --release --bin etna -- proptest BinhexAlphabetMatchesSpec` |
| 002 | `binhex_alphabet_838355e_1` | quickcheck | `property_binhex_alphabet_matches_spec` | `witness_binhex_alphabet_matches_spec_case_hello_world` | `cargo run --release --bin etna -- quickcheck BinhexAlphabetMatchesSpec` |
| 003 | `binhex_alphabet_838355e_1` | crabcheck | `property_binhex_alphabet_matches_spec` | `witness_binhex_alphabet_matches_spec_case_hello_world` | `cargo run --release --bin etna -- crabcheck BinhexAlphabetMatchesSpec` |
| 004 | `binhex_alphabet_838355e_1` | hegel | `property_binhex_alphabet_matches_spec` | `witness_binhex_alphabet_matches_spec_case_hello_world` | `cargo run --release --bin etna -- hegel BinhexAlphabetMatchesSpec` |
| 005 | `decoded_len_overflow_fa47981_1` | proptest | `property_decoded_len_estimate_does_not_panic` | `witness_decoded_len_estimate_does_not_panic_case_usize_max` | `cargo run --release --bin etna -- proptest DecodedLenEstimateDoesNotPanic` |
| 006 | `decoded_len_overflow_fa47981_1` | quickcheck | `property_decoded_len_estimate_does_not_panic` | `witness_decoded_len_estimate_does_not_panic_case_usize_max` | `cargo run --release --bin etna -- quickcheck DecodedLenEstimateDoesNotPanic` |
| 007 | `decoded_len_overflow_fa47981_1` | crabcheck | `property_decoded_len_estimate_does_not_panic` | `witness_decoded_len_estimate_does_not_panic_case_usize_max` | `cargo run --release --bin etna -- crabcheck DecodedLenEstimateDoesNotPanic` |
| 008 | `decoded_len_overflow_fa47981_1` | hegel | `property_decoded_len_estimate_does_not_panic` | `witness_decoded_len_estimate_does_not_panic_case_usize_max` | `cargo run --release --bin etna -- hegel DecodedLenEstimateDoesNotPanic` |

## Witness catalog

Each witness is a deterministic concrete test. Base build: passes. Variant-active build: fails. Witnesses live in `tests/etna_witnesses.rs`.

| Witness | Property | Detects | Input shape |
|---------|----------|---------|-------------|
| `witness_binhex_alphabet_matches_spec_case_hello_world` | `property_binhex_alphabet_matches_spec` | `binhex_alphabet_838355e_1` | `b"Hello, world!"` — encodes through positions that traverse the buggy 7/f swap; a wrong alphabet produces a different output byte and the property compares unequal |
| `witness_binhex_alphabet_matches_spec_case_full_range` | `property_binhex_alphabet_matches_spec` | `binhex_alphabet_838355e_1` | `0u8..=255u8` — saturating the byte range guarantees the encoder walks every alphabet index, so any single-character disagreement between crate and spec alphabets is caught |
| `witness_decoded_len_estimate_does_not_panic_case_usize_max` | `property_decoded_len_estimate_does_not_panic` | `decoded_len_overflow_fa47981_1` | `usize::MAX` — the pre-fix `checked_add(3).expect(..)` panics because `checked_add(3)` returns `None`, and the `catch_unwind` wrapper reports the panic as a property failure |
| `witness_decoded_len_estimate_does_not_panic_case_near_max` | `property_decoded_len_estimate_does_not_panic` | `decoded_len_overflow_fa47981_1` | `usize::MAX - 2` — the boundary just inside the overflow window: `usize::MAX - 2 + 3 > usize::MAX`, still trips the buggy expect; non-overflowing formula returns a valid estimate |
