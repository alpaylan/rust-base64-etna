# rust-base64 — Injected Bugs

Total mutations: 2

## Bug Index

| # | Variant | Name | Location | Injection | Fix Commit |
|---|---------|------|----------|-----------|------------|
| 1 | `binhex_alphabet_838355e_1` | `binhex_alphabet` | `src/alphabet.rs` | `patch` | `838355e0ac5fb8237ec9b96be5edb011bff00275` |
| 2 | `decoded_len_overflow_fa47981_1` | `decoded_len_overflow` | `src/engine/general_purpose/decode.rs` | `patch` | `fa47981fd791467183e4c61112b848095aab21ac` |

## Property Mapping

| Variant | Property | Witness(es) |
|---------|----------|-------------|
| `binhex_alphabet_838355e_1` | `BinhexAlphabetMatchesSpec` | `witness_binhex_alphabet_matches_spec_case_hello_world`, `witness_binhex_alphabet_matches_spec_case_full_range` |
| `decoded_len_overflow_fa47981_1` | `DecodedLenEstimateDoesNotPanic` | `witness_decoded_len_estimate_does_not_panic_case_usize_max`, `witness_decoded_len_estimate_does_not_panic_case_near_max` |

## Framework Coverage

| Property | proptest | quickcheck | crabcheck | hegel |
|----------|---------:|-----------:|----------:|------:|
| `BinhexAlphabetMatchesSpec` | ✓ | ✓ | ✓ | ✓ |
| `DecodedLenEstimateDoesNotPanic` | ✓ | ✓ | ✓ | ✓ |

## Bug Details

### 1. binhex_alphabet

- **Variant**: `binhex_alphabet_838355e_1`
- **Location**: `src/alphabet.rs`
- **Property**: `BinhexAlphabetMatchesSpec`
- **Witness(es)**:
  - `witness_binhex_alphabet_matches_spec_case_hello_world`
  - `witness_binhex_alphabet_matches_spec_case_full_range`
- **Source**: [#271](https://github.com/marshallpierce/rust-base64/pull/271) — Correct BinHex 4.0 alphabet according to specifications
  > The `alphabet::BIN_HEX` literal misspelled the BinHex 4.0 alphabet — it included `7` and omitted `f` where the spec is the other way around. The fix replaces the literal with the correct 64-character set, so any encode/decode traversing indices 7–10 or 37–40 now matches the reference alphabet.
- **Fix commit**: `838355e0ac5fb8237ec9b96be5edb011bff00275` — Correct BinHex 4.0 alphabet according to specifications
- **Invariant violated**: The crate's `BIN_HEX` constant must match the [BinHex 4.0 spec alphabet](http://files.stairways.com/other/binhex-40-specs-info.txt) — specifically, the 64-character set containing `f` and omitting `7`. The property encodes the same input under both `alphabet::BIN_HEX` and a freshly-constructed `Alphabet::new(BINHEX_SPEC)` and demands byte-for-byte equality.
- **How the mutation triggers**: The buggy literal contains `3456789` and `abcdeh` where the spec has `345689` and `abcdefh` — effectively swapping `7` in place of `f`. Any input byte whose base64 lookup traverses one of the disagreeing table indices (7, 8, 9, 10, 37, 38, 39, 40) encodes to a different character under the crate alphabet than under the spec alphabet. The property sees the mismatched bytes and fails.

### 2. decoded_len_overflow

- **Variant**: `decoded_len_overflow_fa47981_1`
- **Location**: `src/engine/general_purpose/decode.rs`
- **Property**: `DecodedLenEstimateDoesNotPanic`
- **Witness(es)**:
  - `witness_decoded_len_estimate_does_not_panic_case_usize_max`
  - `witness_decoded_len_estimate_does_not_panic_case_near_max`
- **Source**: [#217](https://github.com/marshallpierce/rust-base64/pull/217) — Switch to non-overflowing decoded length formula
  > `GeneralPurposeEstimate::new` computed `encoded_len.checked_add(3).expect("Overflow when calculating decoded len estimate")`, panicking on any `encoded_len > usize::MAX - 3`. The fix switches to a non-overflowing formula so `base64::decoded_len_estimate` is total over its declared `usize` domain.
- **Fix commit**: `fa47981fd791467183e4c61112b848095aab21ac` — Switch to non-overflowing decoded length formula
- **Invariant violated**: `base64::decoded_len_estimate` is a public function whose argument is `usize`. It must be total on its declared input domain — returning a value for every `usize` input, never panicking. The estimator's job is size-hinting for downstream `Vec::with_capacity`-style calls; a panic here is a crash on advertised public-API input.
- **How the mutation triggers**: The buggy constructor uses `encoded_len.checked_add(3).expect("Overflow when calculating decoded len estimate")`. When `encoded_len > usize::MAX - 3` (i.e., `usize::MAX`, `usize::MAX - 1`, `usize::MAX - 2`), `checked_add(3)` returns `None` and `.expect` panics. The property wraps the call in `std::panic::catch_unwind` and reports the panic as a failure.

## Dropped Candidates

- `afd5dd29` (Fix panic when decoding last symbol) — Already covered by deterministic unit tests in `src/engine/tests.rs` on InvalidLastSymbol — a PBT wrapper would duplicate an existing unit test rather than exercise a distinct invariant.
