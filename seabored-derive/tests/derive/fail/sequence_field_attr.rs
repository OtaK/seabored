#![allow(dead_code)]

#[derive(seabored::CborSerialize)]
#[cbor(sequence)]
struct Bad(#[cbor(tag = 1)] Vec<u8>);

fn main() {}
