#![allow(dead_code)]

#[derive(seabored::CborSerialize)]
#[cbor(map, array)]
struct Empty;

fn main() {}
