# rust-base64 — Injected Bugs

Total mutations: 2 (from 546 commits scanned; 544 non-fix or terminally inexpressible — see below)

## Bug Index

| # | Name | Variant | File | Injection | Fix Commit |
|---|------|---------|------|-----------|------------|
| 1 | `binhex_alphabet` | `binhex_alphabet_838355e_1` | `patches/binhex_alphabet_838355e_1.patch` | `patch` | `838355e0ac5fb8237ec9b96be5edb011bff00275` |
| 2 | `decoded_len_overflow` | `decoded_len_overflow_fa47981_1` | `patches/decoded_len_overflow_fa47981_1.patch` | `patch` | `fa47981fd791467183e4c61112b848095aab21ac` |

## Property Mapping

| Variant | Property | Witness(es) |
|---------|----------|-------------|
| `binhex_alphabet_838355e_1` | `property_binhex_alphabet_matches_spec` | `witness_binhex_alphabet_matches_spec_case_hello_world`, `witness_binhex_alphabet_matches_spec_case_full_range` |
| `decoded_len_overflow_fa47981_1` | `property_decoded_len_estimate_does_not_panic` | `witness_decoded_len_estimate_does_not_panic_case_usize_max`, `witness_decoded_len_estimate_does_not_panic_case_near_max` |

## Framework Coverage

| Property | proptest | quickcheck | crabcheck | hegel |
|----------|---------:|-----------:|----------:|------:|
| `property_binhex_alphabet_matches_spec` | OK | OK | OK | OK |
| `property_decoded_len_estimate_does_not_panic` | OK | OK | OK | OK |

## Bug Details

### 1. binhex_alphabet (838355e_1)
- **Variant**: `binhex_alphabet_838355e_1`
- **Location**: `src/alphabet.rs`, the `pub const BIN_HEX` alphabet literal
- **Property**: `property_binhex_alphabet_matches_spec`
- **Witnesses**: `witness_binhex_alphabet_matches_spec_case_hello_world`, `witness_binhex_alphabet_matches_spec_case_full_range`
- **Fix commit**: `838355e0ac5fb8237ec9b96be5edb011bff00275` — "Correct BinHex 4.0 alphabet according to specifications"
- **Invariant violated**: The crate's `BIN_HEX` constant must match the [BinHex 4.0 spec alphabet](http://files.stairways.com/other/binhex-40-specs-info.txt) — specifically, the 64-character set containing `f` and omitting `7`. The property encodes the same input under both `alphabet::BIN_HEX` and a freshly-constructed `Alphabet::new(BINHEX_SPEC)` and demands byte-for-byte equality.
- **How the mutation triggers**: The buggy literal contains `3456789` and `abcdeh` where the spec has `345689` and `abcdefh` — effectively swapping `7` in place of `f`. Any input byte whose base64 lookup traverses one of the disagreeing table indices (7, 8, 9, 10, 37, 38, 39, 40) encodes to a different character under the crate alphabet than under the spec alphabet. The property sees the mismatched bytes and fails.

### 2. decoded_len_overflow (fa47981_1)
- **Variant**: `decoded_len_overflow_fa47981_1`
- **Location**: `src/engine/general_purpose/decode.rs`, `impl GeneralPurposeEstimate::new`
- **Property**: `property_decoded_len_estimate_does_not_panic`
- **Witnesses**: `witness_decoded_len_estimate_does_not_panic_case_usize_max`, `witness_decoded_len_estimate_does_not_panic_case_near_max`
- **Fix commit**: `fa47981fd791467183e4c61112b848095aab21ac` — "Switch to non-overflowing decoded length formula"
- **Invariant violated**: `base64::decoded_len_estimate` is a public function whose argument is `usize`. It must be total on its declared input domain — returning a value for every `usize` input, never panicking. The estimator's job is size-hinting for downstream `Vec::with_capacity`-style calls; a panic here is a crash on advertised public-API input.
- **How the mutation triggers**: The buggy constructor uses `encoded_len.checked_add(3).expect("Overflow when calculating decoded len estimate")`. When `encoded_len > usize::MAX - 3` (i.e., `usize::MAX`, `usize::MAX - 1`, `usize::MAX - 2`), `checked_add(3)` returns `None` and `.expect` panics. The property wraps the call in `std::panic::catch_unwind` and reports the panic as a failure.

## Skipped candidates

### Pre-engine-refactor commits — surface removed
The crate underwent a major API refactor from free functions + `Config` to the `Engine`/`GeneralPurpose`/`Alphabet` trait surface between 0.13.x and 0.20.x. All fix commits before that refactor target APIs that no longer exist at HEAD (e.g., `encode_config`, `decode_config_buf`, `Config::new`, `decode_config_slice`, `STANDARD` as a free-function knob rather than an engine instance). Those mutations are terminally inexpressible against the post-refactor codebase.

### Non-fix or duplicative fix commits
- `afd5dd29` ("Fix panic when decoding last symbol"): the crate ships deterministic unit tests covering this exact class of input in `src/engine/tests.rs`. A PBT invariant for the same regression would exercise the same code path the existing unit tests already guarantee.
- SIMD decode path fixes and the family of round-trip-regression fixes: the crate's own test harness already exercises `encode(decode(x)) == x` and `decode(encode(x)) == x` extensively with deterministic vectors. A PBT wrapper would add minimal signal on top of the existing tests.
- Merge commits, version bumps, CI config, dependency updates, and documentation-only changes: not bug fixes.
