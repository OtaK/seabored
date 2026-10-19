#![allow(dead_code)]

#[derive(seabored::CborSerialize)]
#[cbor(profile = "deterministic")]
struct Bad {
    #[cbor(indefinite)]
    value: Vec<u8>,
}

fn main() {}
