#![allow(dead_code)]

#[derive(seabored::CborSerialize)]
struct Tuple(#[cbor(key = "a")] u8);

fn main() {}
