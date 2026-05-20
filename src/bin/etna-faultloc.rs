use std::fmt;

use base64::etna::{
    property_binhex_alphabet_matches_spec,
    property_decoded_len_estimate_does_not_panic, PropertyResult,
};
use crabcheck::profiling::quickcheck;
use crabcheck::quickcheck::{Arbitrary, Mutate};
use rand_etna::Rng;

#[derive(Clone)]
struct Bytes(Vec<u8>);
impl fmt::Debug for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.0.fmt(f) }
}

#[derive(Clone, Copy)]
struct Usize(usize);
impl fmt::Debug for Usize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.0) }
}

impl<R: Rng> Arbitrary<R> for Bytes {
    fn generate(rng: &mut R, _n: usize) -> Self {
        let len = rng.random_range(0usize..=64);
        Bytes((0..len).map(|_| rng.random()).collect())
    }
}

// Mirror existing crabcheck adapter: heavily skew toward values near
// usize::MAX where the decoded-len overflow bug actually fires.
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
            },
            1 if out.len() < 64 => out.push(rng.random()),
            _ if !out.is_empty() => { out.pop(); },
            _ => {},
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

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 3 { return; }
    let result = match (args[1].as_str(), args[2].as_str()) {
        ("crabcheck", "BinhexAlphabetMatchesSpec") => {
            quickcheck(|Bytes(v)| to_opt(property_binhex_alphabet_matches_spec(v)))
        },
        ("crabcheck", "DecodedLenEstimateDoesNotPanic") => {
            quickcheck(|Usize(n)| to_opt(property_decoded_len_estimate_does_not_panic(n)))
        },
        (a, b) => panic!("Unknown: {a} {b}"),
    };
    println!("Result: {:?}", result);
}
