#![allow(dead_code)]

#[derive(seabored::CborDeserialize)]
struct Bad {
    #[cbor(indefinite)]
    value: u8,
}

fn main() {}
