#![allow(dead_code)]

use std::marker::PhantomData;

struct NoContainer<T>(PhantomData<T>);

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(sequence)]
struct Bad(NoContainer<u8>);

fn main() {}
