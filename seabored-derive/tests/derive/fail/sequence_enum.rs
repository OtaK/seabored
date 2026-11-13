#![allow(dead_code)]

#[derive(seabored::CborSerialize)]
#[cbor(sequence)]
enum Bad {
    Values(Vec<u8>),
}

fn main() {}
