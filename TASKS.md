# rust-base64 — ETNA Tasks

Total tasks: 8

## Task Index

| Task | Variant | Framework | Property | Witness |
|------|---------|-----------|----------|---------|
| 001 | `binhex_alphabet_838355e_1` | proptest | `BinhexAlphabetMatchesSpec` | `witness_binhex_alphabet_matches_spec_case_hello_world` |
| 002 | `binhex_alphabet_838355e_1` | quickcheck | `BinhexAlphabetMatchesSpec` | `witness_binhex_alphabet_matches_spec_case_hello_world` |
| 003 | `binhex_alphabet_838355e_1` | crabcheck | `BinhexAlphabetMatchesSpec` | `witness_binhex_alphabet_matches_spec_case_hello_world` |
| 004 | `binhex_alphabet_838355e_1` | hegel | `BinhexAlphabetMatchesSpec` | `witness_binhex_alphabet_matches_spec_case_hello_world` |
| 005 | `decoded_len_overflow_fa47981_1` | proptest | `DecodedLenEstimateDoesNotPanic` | `witness_decoded_len_estimate_does_not_panic_case_usize_max` |
| 006 | `decoded_len_overflow_fa47981_1` | quickcheck | `DecodedLenEstimateDoesNotPanic` | `witness_decoded_len_estimate_does_not_panic_case_usize_max` |
| 007 | `decoded_len_overflow_fa47981_1` | crabcheck | `DecodedLenEstimateDoesNotPanic` | `witness_decoded_len_estimate_does_not_panic_case_usize_max` |
| 008 | `decoded_len_overflow_fa47981_1` | hegel | `DecodedLenEstimateDoesNotPanic` | `witness_decoded_len_estimate_does_not_panic_case_usize_max` |

## Witness Catalog

- `witness_binhex_alphabet_matches_spec_case_hello_world` — base passes, variant fails
- `witness_binhex_alphabet_matches_spec_case_full_range` — base passes, variant fails
- `witness_decoded_len_estimate_does_not_panic_case_usize_max` — base passes, variant fails
- `witness_decoded_len_estimate_does_not_panic_case_near_max` — base passes, variant fails
