//! `Value`'s two serialization paths must agree on the edges: extreme integers and NaN.

use seabored::{Value, ser::CborSerialize};

#[test]
fn negative_integers_below_i64_min_serialize_through_serde() {
    // -2^64: a legal major-type-1 item that only i128 carries. This used to panic in
    // `Value`'s serde `Serialize` (`try_into::<i64>().unwrap()`).
    let bytes = [0x3b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
    let value = seabored::de::from_bytes::<Value>(&bytes).unwrap();
    assert_eq!(value.cbor_serialize().unwrap(), bytes);

    let through_serde = seabored::serde::to_vec(&value).unwrap();
    assert_eq!(through_serde, bytes);
    assert_eq!(
        seabored::serde::from_slice::<Value>(&through_serde).unwrap(),
        value
    );
}

#[test]
fn each_128_bit_width_uses_the_shortest_form() {
    use seabored::ser::CborSerialize;

    // -2^64 fits major type 1; only what major type 1 cannot hold becomes a tag 3 bignum
    assert_eq!(
        (-(1i128 << 64)).cbor_serialize().unwrap(),
        [0x3b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
    );
    assert_eq!(i128::MIN.cbor_serialize().unwrap()[..2], [0xc3, 0x50]);

    // u128: major type 0 up to u64::MAX, tag 2 above
    assert_eq!(
        (u64::MAX as u128).cbor_serialize().unwrap(),
        [0x1b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
    );
    assert_eq!((1u128 << 64).cbor_serialize().unwrap()[..2], [0xc2, 0x49]);
}

#[test]
fn nan_is_canonicalized_whatever_its_sign() {
    // Preferred-plus permits exactly one NaN: `f9 7e 00`
    for nan in [
        &[0xfa, 0x7f, 0xc0, 0x00, 0x00][..], // positive f32 NaN
        &[0xfb, 0xff, 0xf8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00][..], // negative f64 NaN
        &[0xfb, 0x7f, 0xf8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01][..], // f64 NaN payload
    ] {
        let value = seabored::de::from_bytes::<Value>(nan).unwrap();
        assert_eq!(
            value.cbor_serialize().unwrap(),
            [0xf9, 0x7e, 0x00],
            "{nan:02x?}"
        );
        assert_eq!(
            seabored::serde::to_vec(&value).unwrap(),
            [0xf9, 0x7e, 0x00],
            "{nan:02x?}"
        );
    }
}
