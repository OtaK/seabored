#![allow(dead_code)]

#[derive(seabored::CborDeserialize)]
enum Bad {
    #[cbor(key = "x")]
    A,
    #[cbor(key = "x")]
    B,
}

fn main() {}
