#![allow(dead_code)]

#[derive(seabored::CborSerialize)]
enum Bad {
    #[cbor(tag = 1)]
    Unit,
}

fn main() {}
