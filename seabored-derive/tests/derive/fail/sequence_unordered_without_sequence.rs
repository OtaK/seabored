#![allow(dead_code)]

#[derive(seabored::CborSerialize)]
#[cbor(unordered)]
struct Bad(Vec<u8>);

fn main() {}
