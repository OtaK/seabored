//! Simple values round-trip through serde as they always did natively.
//!
//! They used to encode but not decode: the `SimpleValue` sentinel's `visit_u8` was handed to a
//! visitor that assumed every newtype struct was a tag.
use seabored::Value;

#[test]
fn bare_simple_values_round_trip_through_serde() {
    for (bytes, simple) in [
        (vec![0xf0u8], 16u8),
        (vec![0xf3], 19),
        (vec![0xf8, 0x20], 32),
        (vec![0xf8, 0x3b], 59),
        (vec![0xf8, 0xff], 255),
    ] {
        let value: Value = seabored::serde::from_slice(&bytes).unwrap();
        assert_eq!(value, Value::SimpleValue(simple), "serde: {bytes:02x?}");
        assert_eq!(
            seabored::serde::to_vec(&value).unwrap(),
            bytes,
            "re-serialise {bytes:02x?}"
        );
        // the native path agreed all along
        assert_eq!(
            value,
            seabored::de::from_bytes::<Value>(&bytes).unwrap(),
            "native: {bytes:02x?}"
        );
    }
}

#[test]
fn the_other_sentinel_still_decodes() {
    // c1 01: tag 1 wrapping 1, the newtype-struct sentinel that shares this path
    let bytes = [0xc1, 0x01];
    let value: Value = seabored::serde::from_slice(&bytes).unwrap();
    assert_eq!(value, seabored::de::from_bytes::<Value>(&bytes).unwrap());
    assert_eq!(seabored::serde::to_vec(&value).unwrap(), bytes);

    // and a tag around something that needs the whole decoder: c1 82 01 02
    let bytes = [0xc1, 0x82, 0x01, 0x02];
    let value: Value = seabored::serde::from_slice(&bytes).unwrap();
    assert_eq!(value, seabored::de::from_bytes::<Value>(&bytes).unwrap());
}

#[test]
fn the_typed_wrapper_still_works() {
    // `SimpleValue` has its own visitor and never went through the broken path
    let bytes = [0xf8, 0x3b];
    let value: seabored::serde::SimpleValue = seabored::serde::from_slice(&bytes).unwrap();
    assert_eq!(value.0, 59);
    assert_eq!(seabored::serde::to_vec(&value).unwrap(), bytes);
}

#[test]
fn the_boolean_forms_are_unaffected() {
    // 20..=23 are false/true/null/undefined, and they route before the sentinel is reached
    for bytes in [vec![0xf4u8], vec![0xf5], vec![0xf6], vec![0xf7]] {
        let value: Value = seabored::serde::from_slice(&bytes).unwrap();
        assert_eq!(value, seabored::de::from_bytes::<Value>(&bytes).unwrap());
        assert_eq!(seabored::serde::to_vec(&value).unwrap(), bytes);
    }
}
