#![allow(dead_code)]

use std::marker::PhantomData;

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
struct NamedMap {
    a: u8,
    #[cbor(key = 1)]
    b: u16,
    #[cbor(default)]
    c: u8,
    #[cbor(skip, default = 3)]
    skipped: u8,
    #[cbor(tag = 5)]
    tagged: u8,
    #[cbor(simple_value)]
    simple: u8,
}

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(array)]
struct NamedArray {
    a: u8,
    b: u16,
}

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
struct Tuple(u8, u16);

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(tag = 1)]
struct Newtype(u32);

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(tag = 1)]
struct Unit;

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
struct Generic<T> {
    value: T,
}

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
struct Skipped<T> {
    #[cbor(skip)]
    marker: PhantomData<T>,
}

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
struct Indefinite {
    #[cbor(indefinite)]
    items: Vec<u8>,
}

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(profile = "pref-plus")]
struct Preferred {
    value: u32,
}

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(profile = "deterministic")]
struct Deterministic {
    #[cbor(key = "b")]
    b: u8,
    #[cbor(key = "a")]
    a: u8,
}

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(profile = "deterministic")]
enum DeterministicEnum {
    Value {
        #[cbor(key = "b")]
        b: u8,
        #[cbor(key = "a")]
        a: u8,
    },
}

mod custom {
    use seabored::de::CborDeserialize as _;
    use seabored::error::{SeaboredDeError, SeaboredSerError};
    use seabored::reexports::parsio::{DepthAwareReaderGuard, Read, Write};
    use seabored::ser::CborSerialize as _;
    use seabored::{DecodeOptions, Profile};

    pub fn cbor_serialize_to_with_profile<W: Write>(
        value: &u32,
        writer: &mut W,
        profile: Profile,
    ) -> Result<usize, SeaboredSerError> {
        value.cbor_serialize_to_with_profile(writer, profile)
    }

    pub fn cbor_deserialize_from_with_profile<'de, R: Read<'de>>(
        reader: &mut DepthAwareReaderGuard<'_, R>,
        options: DecodeOptions,
    ) -> Result<u32, SeaboredDeError> {
        u32::cbor_deserialize_from_with_profile(reader, options)
    }
}

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
struct With {
    #[cbor(with = custom)]
    value: u32,
}

fn main() {}
