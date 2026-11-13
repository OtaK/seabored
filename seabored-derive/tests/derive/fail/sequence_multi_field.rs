#![allow(dead_code)]

#[derive(seabored::CborSerialize)]
#[cbor(sequence)]
struct Bad(Vec<u8>, Vec<u8>);

fn main() {}
