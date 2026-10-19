use color_eyre::eyre::Result;
use half::f16;
use pretty_assertions::{assert_eq, assert_ne};
use seabored::{Value, mt::MajorType, ser::CborSerialize as _, types::CborSequence};

macro_rules! assert_value_eq_bytes {
    ($value:expr, $hexstring:literal) => {
        let bytes = hex_literal::hex!($hexstring);
        assert_eq!(
            $value,
            seabored::de::from_bytes::<Value>(&bytes).unwrap(),
            "deser: value != parse(bytes)"
        );
        assert_eq!(
            $value.cbor_serialize().unwrap(),
            bytes,
            "ser: value.ser() != bytes"
        );
    };
}

/// Appendix A's indefinite containers, requested explicitly: `FromIterator` builds definite.
fn indefinite_array<'a, I: IntoIterator<Item = Value<'a>>>(items: I) -> Value<'a> {
    let mut seq = CborSequence::new_indefinite(MajorType::Array);
    seq.extend(items);
    Value::Sequence(seq)
}

fn indefinite_map<'a, I: IntoIterator<Item = (Value<'a>, Value<'a>)>>(items: I) -> Value<'a> {
    let mut seq = CborSequence::new_indefinite(MajorType::Map);
    seq.extend(items);
    Value::Map(seq)
}

/// https://www.rfc-editor.org/rfc/rfc8949.html#name-examples-of-encoded-cbor-da
/// Implementation and test of "well-formed" items
#[wasm_bindgen_test::wasm_bindgen_test(unsupported = test)]
fn appendix_a() -> Result<()> {
    assert_value_eq_bytes!(Value::Integer(0_u8.into()), "00");
    assert_value_eq_bytes!(Value::Integer(1_u8.into()), "01");
    assert_value_eq_bytes!(Value::Integer(10_u8.into()), "0a");
    assert_value_eq_bytes!(Value::Integer(23_u8.into()), "17");
    assert_value_eq_bytes!(Value::Integer(24_u8.into()), "1818");
    assert_value_eq_bytes!(Value::Integer(25_u8.into()), "1819");
    assert_value_eq_bytes!(Value::Integer(100_u8.into()), "1864");
    assert_value_eq_bytes!(Value::Integer(1000_u16.into()), "1903e8");
    assert_value_eq_bytes!(Value::Integer(1000000_u32.into()), "1a000f4240");
    assert_value_eq_bytes!(
        Value::Integer(1000000000000_u64.into()),
        "1b000000e8d4a51000"
    );
    assert_value_eq_bytes!(Value::Integer(u64::MAX.into()), "1bffffffffffffffff");
    //     18446744073709551616 	hex_literal::hex!("0xc249010000000000000000")
    assert_value_eq_bytes!(
        Value::Integer((-(u64::MAX as i128)).try_into().unwrap()),
        "3bffffffffffffffff"
    );
    //     -18446744073709551617 	hex_literal::hex!("0xc349010000000000000000")
    assert_value_eq_bytes!(Value::Integer((-1_i8).into()), "20");
    assert_value_eq_bytes!(Value::Integer((-10_i8).into()), "29");
    assert_value_eq_bytes!(Value::Integer((-100_i8).into()), "3863");
    assert_value_eq_bytes!(Value::Integer((-1000_i16).into()), "3903e7");
    assert_value_eq_bytes!(Value::Float(f16::ZERO.into()), "f90000");
    assert_value_eq_bytes!(Value::Float(f16::NEG_ZERO.into()), "f98000");
    assert_value_eq_bytes!(Value::Float(f16::ONE.into()), "f93c00");
    assert_value_eq_bytes!(Value::Float(1.1f64.into()), "fb3ff199999999999a");
    assert_value_eq_bytes!(Value::Float(f16::from_f32(1.5).into()), "f93e00");
    assert_value_eq_bytes!(Value::Float(f16::from_f32(65504.0).into()), "f97bff");
    assert_value_eq_bytes!(Value::Float(100000.0_f32.into()), "fa47c35000");
    assert_value_eq_bytes!(Value::Float(3.402_823_5e38_f32.into()), "fa7f7fffff");
    assert_value_eq_bytes!(Value::Float(1.0e+300_f64.into()), "fb7e37e43c8800759c");
    assert_value_eq_bytes!(Value::Float(f16::from_f32(5.960_464_5e-8).into()), "f90001");
    assert_value_eq_bytes!(
        Value::Float(f16::from_f32(0.000_061_035_156).into()),
        "f90400"
    );
    assert_value_eq_bytes!(Value::Float(f16::from_f32(-4.).into()), "f9c400");
    assert_value_eq_bytes!(Value::Float((-4.1_f64).into()), "fbc010666666666666");
    assert_value_eq_bytes!(Value::Float(f16::INFINITY.into()), "f97c00");

    // NaN should never be equal to itself
    {
        let bytes = hex_literal::hex!("f97e00");
        let nan_value = seabored::de::from_bytes::<Value>(&bytes).unwrap();
        assert_ne!(Value::Float(f16::NAN.into()), nan_value);
        let Value::Float(float) = &nan_value else {
            panic!("f16-sized NaN isn't a f16");
        };
        assert!(float.is_nan(), "f16-sized NaN isn't NaN");
        assert_eq!(nan_value.cbor_serialize().unwrap(), bytes);
    }

    assert_value_eq_bytes!(Value::Float(f16::NEG_INFINITY.into()), "f9fc00");

    // Preferred-plus profile
    // assert_value_eq_bytes!(Value::Float(f32::INFINITY.into()), "fa7f800000");
    assert_value_eq_bytes!(Value::Float(f32::INFINITY.into()), "f97c00");

    {
        let bytes = hex_literal::hex!("fa7fc00000");
        let nan_value = seabored::de::from_bytes::<Value>(&bytes).unwrap();
        assert_ne!(Value::Float(f32::NAN.into()), nan_value);
        let Value::Float(float) = &nan_value else {
            panic!("f32-sized NaN isn't a f32");
        };
        assert!(float.is_nan(), "f32-sized NaN isn't NaN");
        // p+ profile
        // assert_eq!(nan_value.cbor_serialize().unwrap(), bytes);
        assert_eq!(
            nan_value.cbor_serialize().unwrap(),
            hex_literal::hex!("f97e00")
        );
    }

    // p+ profile
    // assert_value_eq_bytes!(Value::Float(f32::NEG_INFINITY.into()), "faff800000");
    assert_value_eq_bytes!(Value::Float(f32::NEG_INFINITY.into()), "f9fc00");
    // p+ profile
    // assert_value_eq_bytes!(Value::Float(f64::INFINITY.into()), "fb7ff0000000000000");
    assert_value_eq_bytes!(Value::Float(f64::INFINITY.into()), "f97c00");

    {
        let bytes = hex_literal::hex!("fb7ff8000000000000");
        let nan_value = seabored::de::from_bytes::<Value>(&bytes).unwrap();
        assert_ne!(Value::Float(f64::NAN.into()), nan_value);
        let Value::Float(float) = &nan_value else {
            panic!("f64-sized NaN isn't a f64");
        };
        assert!(float.is_nan(), "f64-sized NaN isn't NaN");
        // p+ profile
        // assert_eq!(nan_value.cbor_serialize().unwrap(), bytes);
        assert_eq!(
            nan_value.cbor_serialize().unwrap(),
            hex_literal::hex!("f97e00")
        );
    }

    // p+ profile
    // assert_value_eq_bytes!(Value::Float(f64::NEG_INFINITY.into()), "fbfff0000000000000");
    assert_value_eq_bytes!(Value::Float(f64::NEG_INFINITY.into()), "f9fc00");

    assert_value_eq_bytes!(Value::Bool(false), "f4");
    assert_value_eq_bytes!(Value::Bool(true), "f5");
    assert_value_eq_bytes!(Value::Null, "f6");
    assert_value_eq_bytes!(Value::Undefined, "f7");
    assert_value_eq_bytes!(Value::SimpleValue(16), "f0");
    assert_value_eq_bytes!(Value::SimpleValue(255), "f8ff");

    // TODO: Once "standard" tags are interpreted correctly, utilize a datetime type
    // 0("2013-03-21T20:04:00Z") 	0xc074323031332d30332d32315432303a30343a30305a
    assert_value_eq_bytes!(
        Value::Tagged((
            0u8.into(),
            Box::new(Value::String("2013-03-21T20:04:00Z".into()))
        )),
        "c074323031332d30332d32315432303a30343a30305a"
    );

    assert_value_eq_bytes!(
        Value::Tagged((1u8.into(), Box::new(Value::Integer(1363896240_u32.into())))),
        "c11a514b67b0"
    );

    assert_value_eq_bytes!(
        Value::Tagged((1u8.into(), Box::new(Value::Float(1363896240.5_f64.into())))),
        "c1fb41d452d9ec200000"
    );

    assert_value_eq_bytes!(
        Value::Tagged((
            23u8.into(),
            Box::new(Value::Bytes((&hex_literal::hex!("01020304")).into()))
        )),
        "d74401020304"
    );

    assert_value_eq_bytes!(
        Value::Tagged((
            24u8.into(),
            Box::new(Value::Bytes((&hex_literal::hex!("6449455446")).into()))
        )),
        "d818456449455446"
    );

    assert_value_eq_bytes!(
        Value::Tagged((
            32u8.into(),
            Box::new(Value::String("http://www.example.com".into()))
        )),
        "d82076687474703a2f2f7777772e6578616d706c652e636f6d"
    );

    assert_value_eq_bytes!(Value::Bytes(b"".into()), "40");
    assert_value_eq_bytes!(
        Value::Bytes((&hex_literal::hex!("01020304")).into()),
        "4401020304"
    );
    assert_value_eq_bytes!(Value::String("".into()), "60");
    assert_value_eq_bytes!(Value::String("a".into()), "6161");
    assert_value_eq_bytes!(Value::String("IETF".into()), "6449455446");
    assert_value_eq_bytes!(Value::String("\"\\".into()), "62225c");
    assert_value_eq_bytes!(Value::String("\u{00fc}".into()), "62c3bc");
    assert_value_eq_bytes!(Value::String("\u{6c34}".into()), "63e6b0b4");
    assert_value_eq_bytes!(Value::String("𐅑".into()), "64f0908591");
    assert_value_eq_bytes!(Value::Sequence(Default::default()), "80");
    assert_value_eq_bytes!(
        Value::Sequence(
            (1u8..=3)
                .map(|v| Value::Integer(v.into()))
                .collect::<Vec<_>>()
                .into()
        ),
        "83010203"
    );

    assert_value_eq_bytes!(
        Value::Sequence(
            (vec![
                Value::Integer(1u8.into()),
                Value::Sequence(
                    (vec![Value::Integer(2u8.into()), Value::Integer(3u8.into())]).into()
                ),
                Value::Sequence(
                    (vec![Value::Integer(4u8.into()), Value::Integer(5u8.into())]).into()
                )
            ])
            .into()
        ),
        "8301820203820405"
    );

    assert_value_eq_bytes!(
        Value::Sequence(
            (1u8..=25)
                .map(|v| Value::Integer(v.into()))
                .collect::<Vec<_>>()
                .into()
        ),
        "98190102030405060708090a0b0c0d0e0f101112131415161718181819"
    );

    assert_value_eq_bytes!(
        Value::Map(CborSequence::default().with_mt(MajorType::Map)),
        "a0"
    );

    assert_value_eq_bytes!(
        Value::Map(
            CborSequence::from(
                [(1u8, 2u8), (3u8, 4u8)]
                    .into_iter()
                    .map(|(k, v)| (Value::Integer(k.into()), Value::Integer(v.into())))
                    .collect::<Vec<_>>()
            )
            .with_mt(MajorType::Map)
        ),
        "a201020304"
    );

    assert_value_eq_bytes!(
        Value::Map(
            CborSequence::from(vec![
                (Value::String("a".into()), Value::Integer(1u8.into())),
                (
                    Value::String("b".into()),
                    Value::Sequence(
                        vec![Value::Integer(2u8.into()), Value::Integer(3u8.into())].into()
                    )
                )
            ])
            .with_mt(MajorType::Map)
        ),
        "a26161016162820203"
    );

    assert_value_eq_bytes!(
        Value::Sequence(
            vec![
                Value::String("a".into()),
                Value::Map(
                    CborSequence::from(vec![(
                        Value::String("b".into()),
                        Value::String("c".into())
                    )])
                    .with_mt(MajorType::Map)
                )
            ]
            .into()
        ),
        "826161a161626163"
    );

    assert_value_eq_bytes!(
        Value::Map(
            CborSequence::from(
                ["a", "b", "c", "d", "e"]
                    .into_iter()
                    .map(|k| (
                        Value::String(k.into()),
                        Value::String(k.to_uppercase().into())
                    ))
                    .collect::<Vec<_>>()
            )
            .with_mt(MajorType::Map)
        ),
        "a56161614161626142616361436164614461656145"
    );

    assert_value_eq_bytes!(
        {
            let mut seq = CborSequence::new_indefinite(MajorType::Bytes);
            seq.extend([
                Value::Bytes((&hex_literal::hex!("0102")).into()),
                Value::Bytes((&hex_literal::hex!("030405")).into()),
            ]);
            Value::Sequence(seq)
        },
        "5f42010243030405ff"
    );

    assert_value_eq_bytes!(
        {
            let mut seq = CborSequence::new_indefinite(MajorType::String);
            seq.extend([Value::String("strea".into()), Value::String("ming".into())]);
            Value::Sequence(seq)
        },
        "7f657374726561646d696e67ff"
    );

    assert_value_eq_bytes!(indefinite_array(Vec::<Value>::new()), "9fff");

    assert_value_eq_bytes!(
        indefinite_array([
            Value::Integer(1u8.into()),
            Value::Sequence((vec![Value::Integer(2u8.into()), Value::Integer(3u8.into())]).into()),
            indefinite_array([Value::Integer(4u8.into()), Value::Integer(5u8.into())])
        ]),
        "9f018202039f0405ffff"
    );

    assert_value_eq_bytes!(
        indefinite_array([
            Value::Integer(1u8.into()),
            Value::Sequence((vec![Value::Integer(2u8.into()), Value::Integer(3u8.into())]).into()),
            Value::Sequence((vec![Value::Integer(4u8.into()), Value::Integer(5u8.into())]).into())
        ]),
        "9f01820203820405ff"
    );

    assert_value_eq_bytes!(
        Value::Sequence(
            (vec![
                Value::Integer(1u8.into()),
                Value::Sequence(
                    (vec![Value::Integer(2u8.into()), Value::Integer(3u8.into())]).into()
                ),
                indefinite_array([Value::Integer(4u8.into()), Value::Integer(5u8.into())])
            ])
            .into()
        ),
        "83018202039f0405ff"
    );

    assert_value_eq_bytes!(
        Value::Sequence(
            vec![
                Value::Integer(1u8.into()),
                indefinite_array(vec![Value::Integer(2u8.into()), Value::Integer(3u8.into())]),
                Value::Sequence(CborSequence::from(vec![
                    Value::Integer(4u8.into()),
                    Value::Integer(5u8.into())
                ]))
            ]
            .into()
        ),
        "83019f0203ff820405"
    );

    assert_value_eq_bytes!(
        indefinite_array((1u8..=25).map(|v| Value::Integer(v.into()))),
        "9f0102030405060708090a0b0c0d0e0f101112131415161718181819ff"
    );

    assert_value_eq_bytes!(
        indefinite_map([
            (Value::String("a".into()), Value::Integer(1u8.into())),
            (
                Value::String("b".into()),
                indefinite_array([Value::Integer(2u8.into()), Value::Integer(3u8.into())])
            )
        ]),
        "bf61610161629f0203ffff"
    );

    assert_value_eq_bytes!(
        Value::Sequence(
            vec![
                Value::String("a".into()),
                indefinite_map([(Value::String("b".into()), Value::String("c".into()))])
            ]
            .into()
        ),
        "826161bf61626163ff"
    );

    assert_value_eq_bytes!(
        indefinite_map([
            (Value::String("Fun".into()), Value::Bool(true)),
            (Value::String("Amt".into()), Value::Integer((-2i8).into()))
        ]),
        "bf6346756ef563416d7421ff"
    );

    Ok(())
}

/// `FromIterator` builds definite-length; indefinite only on explicit request.
#[wasm_bindgen_test::wasm_bindgen_test(unsupported = test)]
fn from_iter_is_definite_length() -> Result<()> {
    assert_value_eq_bytes!(
        vec![Value::Integer(1u8.into()), Value::Integer(2u8.into())]
            .into_iter()
            .collect::<Value>(),
        "820102"
    );
    assert_value_eq_bytes!(
        [(Value::String("a".into()), Value::Integer(1u8.into()))]
            .into_iter()
            .collect::<Value>(),
        "a1616101"
    );
    // The explicit ask still produces the indefinite form, for both container kinds
    assert_value_eq_bytes!(
        indefinite_array([Value::Integer(1u8.into()), Value::Integer(2u8.into())]),
        "9f0102ff"
    );
    assert_value_eq_bytes!(
        indefinite_map([(Value::String("a".into()), Value::Integer(1u8.into()))]),
        "bf616101ff"
    );

    // A decoded indefinite container can be re-encoded in the preferred form by clearing the flag,
    // which is what the documentation on `CborSequence::is_indefinite` says to do
    let decoded = seabored::de::from_bytes::<Value>(&hex_literal::hex!("9f0102ff"))?;
    let Value::Sequence(mut seq) = decoded else {
        unreachable!("expected an indefinite-length array");
    };
    assert!(seq.is_indefinite);
    seq.is_indefinite = false;
    assert_eq!(
        Value::Sequence(seq).cbor_serialize()?,
        hex_literal::hex!("820102")
    );

    Ok(())
}

fn unhex(hex: &str) -> Vec<u8> {
    assert_eq!(hex.len() % 2, 0, "odd-length hex string {hex}");
    (0..hex.len() / 2)
        .map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

/// RFC 8949 §3: *"A decoder MUST NOT return a decoded data item when it encounters input that is
/// not a well-formed encoded CBOR data item."* Both entry points must reject these.
fn assert_not_well_formed(hex: &str) {
    let bytes = unhex(hex);
    assert!(
        seabored::de::from_bytes::<Value>(&bytes).is_err(),
        "accepted not-well-formed {hex}"
    );
    #[cfg(feature = "serde")]
    assert!(
        seabored::serde::from_slice::<Value>(&bytes).is_err(),
        "accepted not-well-formed {hex} through serde"
    );
}

/// RFC's well-formedness checklist:
/// https://www.rfc-editor.org/rfc/rfc8949.html#name-well-formedness-errors-and-
#[wasm_bindgen_test::wasm_bindgen_test(unsupported = test)]
fn appendix_f() -> Result<()> {
    // F.1 kind 1: too much data, or a break code outside an indefinite-length item
    for hex in ["ff", "81ff", "a1ff", "bf00ff"] {
        assert_not_well_formed(hex);
    }

    // F.1 kind 2: truncations
    for hex in [
        "18",
        "1901",
        "1a0102",
        "1b010203040506",
        "38",
        "5801",
        "7801",
        "81",
        "a1",
        "9f",
        "bf",
        "c0",
    ] {
        assert_not_well_formed(hex);
    }

    // F.1 kind 3, subkind 1: reserved additional information
    for hex in [
        "1c", "1d", "1e", "3c", "3d", "3e", "5c", "5d", "5e", "7c", "7d", "7e", "9c", "9d", "9e",
        "bc", "bd", "be", "dc", "dd", "de", "fc", "fd", "fe",
    ] {
        assert_not_well_formed(hex);
    }

    // F.1 kind 3, subkind 2: `f8` followed by a byte below 0x20 — the value fits the initial
    // byte (or is not a simple value at all), so the two-byte form is not well-formed
    for hex in ["f800", "f801", "f818", "f81f"] {
        assert_not_well_formed(hex);
    }

    // F.1 kind 3, subkind 3: chunks that are not definite-length strings of the same major
    // type, and indefinite-length strings nested as chunks
    for hex in [
        "5f00ff",
        "5f21ff",
        "5f6100ff",
        "5f80ff",
        "5fa0ff",
        "5fc000ff",
        "5fe0ff",
        "7f4100ff",
        "5f5f4100ffff",
        "7f7f6100ffff",
    ] {
        assert_not_well_formed(hex);
    }

    // F.1 kind 3, subkind 4: additional-info 31 on a major type that cannot have one
    for hex in ["1f", "3f", "df"] {
        assert_not_well_formed(hex);
    }

    // The well-formed neighbours of those vectors decode on both entry points
    for hex in [
        "e0",       // simple value 0, shortest form
        "f3",       // simple value 19, shortest form
        "f820",     // simple value 32, the first value that needs the two-byte form
        "f8ff",     // simple value 255
        "f4",       // false
        "f7",       // undefined
        "5fff",     // empty indefinite-length byte string
        "7fff",     // empty indefinite-length text string
        "5f4100ff", // indefinite-length byte string, one definite chunk
        "7f6100ff", // indefinite-length text string, one definite chunk
        "9f00ff",   // indefinite-length array
        "bf0000ff", // indefinite-length map, one pair
    ] {
        let bytes = unhex(hex);
        assert!(
            seabored::de::from_bytes::<Value>(&bytes).is_ok(),
            "rejected well-formed {hex}"
        );
        #[cfg(feature = "serde")]
        assert!(
            seabored::serde::from_slice::<Value>(&bytes).is_ok(),
            "rejected well-formed {hex} through serde"
        );
    }

    // Simple values the crate can represent round-trip through the encoder
    for simple in [16u8, 19, 32, 255] {
        let value = Value::SimpleValue(simple);
        let encoded = value.cbor_serialize()?;
        assert_eq!(seabored::de::from_bytes::<Value>(&encoded)?, value);
    }
    assert_eq!(
        Value::SimpleValue(19).cbor_serialize()?,
        hex_literal::hex!("f3")
    );
    assert_eq!(
        Value::SimpleValue(32).cbor_serialize()?,
        hex_literal::hex!("f820")
    );
    assert_eq!(
        Value::Bool(false).cbor_serialize()?,
        hex_literal::hex!("f4")
    );

    // Encoder side of subkind 2, both entry points: 20..=23 would alias
    // false/true/null/undefined and 24..=31 are reserved, so neither may be written
    for simple in [20u8, 21, 22, 23, 24, 31] {
        assert!(
            Value::SimpleValue(simple).cbor_serialize().is_err(),
            "encoded reserved simple value {simple}"
        );
        #[cfg(feature = "serde")]
        assert!(
            seabored::serde::to_vec(&seabored::serde::SimpleValue(simple)).is_err(),
            "encoded reserved simple value {simple} through serde"
        );
    }
    #[cfg(feature = "serde")]
    for simple in [16u8, 19, 32, 255] {
        let encoded = seabored::serde::to_vec(&seabored::serde::SimpleValue(simple))?;
        assert_eq!(
            seabored::de::from_bytes::<Value>(&encoded)?,
            Value::SimpleValue(simple)
        );
    }

    Ok(())
}
