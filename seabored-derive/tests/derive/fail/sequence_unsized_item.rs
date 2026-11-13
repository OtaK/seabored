#![allow(dead_code)]

#[derive(seabored::CborSerialize)]
#[cbor(sequence)]
struct Bad(Box<[u8]>);

fn main() {}
