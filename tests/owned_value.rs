//! `Value` lends what the input can lend, so it can never be `DeserializeOwned`.
//! `OwnedValue` is the escape hatch for decoders that cannot lend their bytes.
use seabored::{OwnedValue, Value};
use serde::de::DeserializeOwned;

fn assert_deserialize_owned<T: DeserializeOwned>() {}

#[test]
fn owned_value_is_deserialize_owned() {
    // The compile-time half of the finding: `Value` cannot satisfy this bound, `OwnedValue` does
    assert_deserialize_owned::<OwnedValue>();

    let bytes = [0x82, 0x01, 0x61, 0x61];
    let via_serde: OwnedValue = seabored::serde::from_slice(&bytes).unwrap();
    assert_eq!(
        via_serde.0,
        seabored::de::from_bytes::<Value>(&bytes).unwrap()
    );
    // and it still serialises to the same bytes
    assert_eq!(seabored::serde::to_vec(&via_serde).unwrap(), bytes);
}

#[test]
fn foreign_decoders_can_use_it() {
    // A `&[u8]` reader cannot lend, so the target must be `DeserializeOwned`;
    // `ciborium::from_reader::<Value, _>` does not compile (`Value<'_>` is lifetime-bound).
    let bytes = [0xa1, 0x61, 0x61, 0x01]; // {"a": 1}
    let ours = seabored::de::from_bytes::<Value>(&bytes)
        .unwrap()
        .into_owned();

    let via_ciborium: OwnedValue = ciborium::from_reader(&bytes[..]).unwrap();
    assert_eq!(via_ciborium.0, ours);

    let via_cbor4ii: OwnedValue = cbor4ii::serde::from_slice(&bytes).unwrap();
    assert_eq!(via_cbor4ii.0, ours);
}

#[test]
fn into_owned_keeps_container_shape() {
    // 9f 40 ff: indefinite-length byte string, one empty chunk; bf ff: indefinite-length map
    let bytes = [0x82, 0x5f, 0x40, 0xff, 0xbf, 0xff];
    let borrowed = seabored::de::from_bytes::<Value>(&bytes).unwrap();
    let owned = borrowed.clone().into_owned();
    assert_eq!(owned, borrowed);
    assert_eq!(
        seabored::ser::CborSerialize::cbor_serialize(&owned).unwrap(),
        bytes
    );
}

/// The reason `visit_map` was fixed: a serde-decoded `Value::Map` said `mt: Array`, so
/// re-serialising wrote `81 a1` for input `a1 a1` — silent in release, `debug_assert` in debug.
#[test]
fn serde_decoded_map_keeps_its_major_type() {
    let bytes = [0xa1, 0x61, 0x61, 0x01]; // {"a": 1}
    let value: Value = seabored::serde::from_slice(&bytes).unwrap();
    assert_eq!(value.mt(), seabored::mt::MajorType::Map);
    assert_eq!(
        seabored::ser::CborSerialize::cbor_serialize(&value).unwrap(),
        bytes
    );
    // the same bytes through the native decoder, for comparison
    assert_eq!(value, seabored::de::from_bytes::<Value>(&bytes).unwrap());
}
