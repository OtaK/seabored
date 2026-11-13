#![allow(dead_code)]

use std::collections::HashMap;

#[derive(seabored::CborSerialize)]
#[cbor(sequence)]
struct Bad(HashMap<u8, u8>);

fn main() {}
