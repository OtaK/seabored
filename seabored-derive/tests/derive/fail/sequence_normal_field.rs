#![allow(dead_code)]

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(sequence)]
struct Values(Vec<u8>);

#[derive(seabored::CborSerialize)]
struct Outer {
    values: Values,
}

fn main() {}
