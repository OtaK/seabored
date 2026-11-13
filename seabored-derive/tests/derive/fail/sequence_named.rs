#![allow(dead_code)]

#[derive(seabored::CborSerialize)]
#[cbor(sequence)]
struct Bad {
    values: Vec<u8>,
}

fn main() {}
