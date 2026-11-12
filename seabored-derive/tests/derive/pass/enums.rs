#![allow(dead_code)]

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
enum Basic {
    Unit,
    Newtype(u8),
    Tuple(u8, u16),
    Struct {
        a: u8,
        #[cbor(default)]
        b: u8,
    },
}

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
enum Keys {
    #[cbor(key = 1)]
    Unit,
    #[cbor(key = -2)]
    Value(u8),
}

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
enum Tags {
    #[cbor(tag = 7)]
    Newtype(u8),
    #[cbor(tag = 8)]
    Tuple(u8, u16),
    #[cbor(tag = 9)]
    Struct {
        a: u8,
    },
}

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(tag = 42)]
enum Tagged {
    Unit,
    Value(u8),
}

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
enum Generic<T> {
    Value(T),
}

fn main() {}
