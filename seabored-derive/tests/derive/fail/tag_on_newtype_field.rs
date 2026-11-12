#![allow(dead_code)]

#[derive(seabored::CborSerialize)]
struct Bad(#[cbor(tag = 1)] u8);

fn main() {}
