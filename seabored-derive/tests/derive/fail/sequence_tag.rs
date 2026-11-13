#![allow(dead_code)]

#[derive(seabored::CborSerialize)]
#[cbor(sequence, tag = 1)]
struct Bad(Vec<u8>);

fn main() {}
