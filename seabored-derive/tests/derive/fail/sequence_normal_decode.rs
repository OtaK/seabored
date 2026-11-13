#![allow(dead_code)]

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(sequence)]
struct Values(Vec<u8>);

fn main() {
    let _: Result<Values, _> = seabored::de::from_bytes(&[]);
}
