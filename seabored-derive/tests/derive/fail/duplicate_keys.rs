#![allow(dead_code)]

#[derive(seabored::CborDeserialize)]
struct Bad {
    #[cbor(key = "x")]
    a: u8,
    #[cbor(key = "x")]
    b: u8,
}

fn main() {}
