#![allow(dead_code)]

use std::collections::HashSet;

#[derive(seabored::CborSerialize)]
#[cbor(sequence, unordered, profile = "deterministic")]
struct Bad(HashSet<u8>);

fn main() {}
