//! Profile-aware checking decode across native, derive, and serde surfaces.

use seabored::{
    DecodeOptions, Profile, Value,
    de::{from_bytes_with_options, from_bytes_with_profile},
    error::SeaboredDeError,
    ser::CborSerialize as _,
    types::CborMap,
};

const CHECKED: DecodeOptions = DecodeOptions::PREFERRED_PLUS_CHECKED;
const DETERMINISTIC: DecodeOptions = DecodeOptions::DETERMINISTIC_CHECKED;

fn non_preferred(result: Result<impl core::fmt::Debug, SeaboredDeError>) -> bool {
    matches!(result, Err(SeaboredDeError::NonPreferredEncoding { .. }))
}

#[test]
fn checking_rejects_non_shortest_integers_and_strings() {
    assert!(non_preferred(from_bytes_with_options::<u8>(
        &[0x18, 0x01],
        CHECKED
    )));
    assert!(from_bytes_with_options::<u8>(&[0x01], CHECKED).is_ok());

    // Definite text with a non-shortest length header.
    assert!(non_preferred(from_bytes_with_options::<String>(
        &[0x78, 0x01, 0x61],
        CHECKED
    )));
    // Indefinite text.
    assert!(non_preferred(from_bytes_with_options::<String>(
        &[0x7f, 0x61, 0x61, 0xff],
        CHECKED
    )));
    assert!(
        from_bytes_with_options::<String>(&[0x7f, 0x61, 0x61, 0xff], DecodeOptions::PREFERRED_PLUS)
            .is_ok()
    );
}

#[test]
fn checking_rejects_indefinite_and_non_shortest_containers() {
    assert!(non_preferred(from_bytes_with_options::<Vec<u8>>(
        &[0x9f, 0x01, 0xff],
        CHECKED
    )));
    assert!(non_preferred(from_bytes_with_options::<Vec<u8>>(
        &[0x98, 0x01, 0x01],
        CHECKED
    )));
    assert!(from_bytes_with_options::<Vec<u8>>(&[0x81, 0x01], CHECKED).is_ok());
}

#[test]
fn checking_applies_bignum_rules() {
    // Empty and leading-zero payloads, and values that fit major type 0.
    for bytes in [
        &[0xc2, 0x40][..],
        &[0xc2, 0x42, 0x00, 0x03],
        &[0xc2, 0x41, 0x03],
    ] {
        assert!(non_preferred(from_bytes_with_options::<u128>(
            bytes, CHECKED
        )));
    }

    // 2^64 needs the bignum form.
    let two_to_64 = hex_literal::hex!("c249010000000000000000");
    assert_eq!(
        from_bytes_with_options::<u128>(&two_to_64, CHECKED).unwrap(),
        1 << 64
    );

    // Tag 3 with a value that fits major type 1 is rejected.
    assert!(non_preferred(from_bytes_with_options::<i128>(
        &[0xc3, 0x41, 0x01],
        CHECKED
    )));
    // Non-checking preferred-plus still accepts empty payloads.
    assert_eq!(
        from_bytes_with_options::<u128>(&[0xc2, 0x40], DecodeOptions::PREFERRED_PLUS).unwrap(),
        0
    );
}

#[test]
fn value_bignums_decode_to_normalized_forms() {
    // A payload that fits major type 0 becomes an integer even without checking.
    let small: Value<'_> =
        from_bytes_with_options(&hex_literal::hex!("c2420003"), DecodeOptions::GENERAL).unwrap();
    assert_eq!(small, Value::Integer(3u8.into()));

    // Empty payload means n = 0: tag 2 is 0, tag 3 is -1 (RFC 8949 §3.4.3; draft-09
    // §4.2 as clarified by cbor-wg PR #101). It is not zero for both tags.
    let zero: Value<'_> = from_bytes_with_options(&[0xc2, 0x40], DecodeOptions::GENERAL).unwrap();
    assert_eq!(zero, Value::Integer(0u8.into()));
    let minus_one: Value<'_> =
        from_bytes_with_options(&[0xc3, 0x40], DecodeOptions::GENERAL).unwrap();
    assert_eq!(minus_one, Value::Integer((-1i8).into()));
    assert_eq!(minus_one.cbor_serialize().unwrap(), [0x20]);
    for bytes in [&[0xc2, 0x40][..], &[0xc3, 0x40][..]] {
        assert!(non_preferred(from_bytes_with_options::<Value>(
            bytes, CHECKED
        )));
        assert!(from_bytes_with_options::<Value>(bytes, DecodeOptions::PREFERRED_PLUS).is_ok());
    }

    // 2^64 needs the bignum form.
    let two_to_64 = hex_literal::hex!("c249010000000000000000");
    let big: Value<'_> = from_bytes_with_options(&two_to_64, DecodeOptions::GENERAL).unwrap();
    let big_int = big.as_big_integer().expect("big integer");
    assert!(!big_int.is_negative());
    assert_eq!(
        big_int.magnitude(),
        &hex_literal::hex!("010000000000000000")
    );
    assert_eq!(
        big.cbor_serialize().unwrap(),
        two_to_64,
        "normalized bignums re-encode canonically"
    );

    // Tag 3 is negative, and leading zeroes are stripped without checking.
    let padded: Value<'_> = from_bytes_with_options(
        &hex_literal::hex!("c34a0001000000000000000000"),
        DecodeOptions::GENERAL,
    )
    .unwrap();
    let padded_int = padded.as_big_integer().expect("big integer");
    assert!(padded_int.is_negative());
    assert_eq!(
        padded_int.magnitude(),
        &hex_literal::hex!("010000000000000000")
    );

    // A non-byte payload stays a generic tag when not checking, and is rejected when checking.
    let not_bytes: Value<'_> =
        from_bytes_with_options(&[0xc2, 0x01], DecodeOptions::GENERAL).unwrap();
    assert!(matches!(not_bytes, Value::Tagged(_)));
    assert!(non_preferred(from_bytes_with_options::<Value>(
        &[0xc2, 0x01],
        CHECKED
    )));
}

#[test]
fn checking_typed_maps_enforces_key_order() {
    let sorted = hex_literal::hex!("a2016161026162");
    let unsorted = hex_literal::hex!("a2026162016161");

    let map: CborMap<u8, String> =
        from_bytes_with_options(&sorted, DETERMINISTIC).expect("sorted map");
    assert_eq!(map.len(), 2);
    assert!(non_preferred(
        from_bytes_with_options::<CborMap<u8, String>>(&unsorted, DETERMINISTIC)
    ));
    assert!(
        from_bytes_with_options::<CborMap<u8, String>>(&unsorted, DecodeOptions::PREFERRED_PLUS)
            .is_ok()
    );
}

#[test]
fn profile_entry_point_sets_checking_for_strict_profiles() {
    let bytes = [0x18, 0x01];
    assert!(non_preferred(from_bytes_with_profile::<u8>(
        &bytes,
        Profile::PreferredPlus
    )));
    assert_eq!(
        from_bytes_with_profile::<u8>(&bytes, Profile::General).unwrap(),
        1
    );
}

#[test]
fn value_deterministic_checking_rejects_unordered_maps() {
    let unsorted = hex_literal::hex!("a2016178006179");
    assert!(non_preferred(from_bytes_with_options::<Value>(
        &unsorted,
        DETERMINISTIC
    )));
    assert!(from_bytes_with_options::<Value>(&unsorted, DecodeOptions::DETERMINISTIC).is_ok());
}

#[cfg(feature = "derive")]
mod derive_decoding {
    use super::*;
    use seabored::{CborDeserialize, CborSerialize};

    #[derive(Debug, PartialEq, CborSerialize, CborDeserialize)]
    #[cbor(profile = "rfc")]
    struct RuntimeChecked {
        a: u8,
    }

    #[test]
    fn runtime_options_override_the_rfc_attribute() {
        let bytes = hex_literal::hex!("a161611801");
        assert!(from_bytes_with_options::<RuntimeChecked>(&bytes, DecodeOptions::GENERAL).is_ok());
        assert!(non_preferred(from_bytes_with_options::<RuntimeChecked>(
            &bytes, CHECKED
        )));
    }

    #[derive(Debug, PartialEq, CborSerialize, CborDeserialize)]
    #[cbor(profile = "rfc")]
    struct RuntimeSorted {
        #[cbor(key = "z")]
        z: u8,
        #[cbor(key = "a")]
        a: u8,
    }

    #[test]
    fn runtime_deterministic_options_check_map_order() {
        let unsorted = hex_literal::hex!("a2617a02616101");
        assert!(
            from_bytes_with_options::<RuntimeSorted>(&unsorted, DecodeOptions::GENERAL).is_ok()
        );
        assert!(non_preferred(from_bytes_with_options::<RuntimeSorted>(
            &unsorted,
            DETERMINISTIC
        )));

        let sorted = hex_literal::hex!("a2616101617a02");
        assert_eq!(
            from_bytes_with_options::<RuntimeSorted>(&sorted, DETERMINISTIC).unwrap(),
            RuntimeSorted { z: 2, a: 1 }
        );
    }
}

#[cfg(feature = "serde")]
mod serde_decoding {
    use super::*;

    #[test]
    fn slicing_api_checks_the_whole_item() {
        let non_shortest = hex_literal::hex!("a161611801");
        assert!(seabored::serde::from_slice_with_options::<Value>(&non_shortest, CHECKED).is_err());
        assert!(
            seabored::serde::from_slice_with_options::<Value>(
                &non_shortest,
                DecodeOptions::GENERAL
            )
            .is_ok()
        );

        let unsorted = hex_literal::hex!("a2016178006179");
        assert!(
            seabored::serde::from_slice_with_options::<Value>(&unsorted, DETERMINISTIC).is_err()
        );
        assert!(
            seabored::serde::from_slice_with_options::<Value>(
                &unsorted,
                DecodeOptions::PREFERRED_PLUS
            )
            .is_ok()
        );
    }

    #[test]
    fn streaming_deserializer_checks_bignums() {
        use seabored::reexports::parsio::DepthAwareReader;
        use seabored::serde::Deserializer;
        use serde::Deserialize as _;

        for bytes in [
            &hex_literal::hex!("c240")[..],
            &hex_literal::hex!("c2420003")[..],
            &hex_literal::hex!("c24103")[..],
            &hex_literal::hex!("c301")[..],
        ] {
            let mut reader = DepthAwareReader::from_reader(bytes);
            let mut deserializer = Deserializer::new_with_options(&mut reader, CHECKED).unwrap();
            assert!(
                Value::deserialize(&mut deserializer).is_err(),
                "accepted {bytes:02x?}"
            );
        }

        let big = hex_literal::hex!("c249010000000000000000");
        let mut reader = DepthAwareReader::from_reader(&big[..]);
        let mut deserializer = Deserializer::new_with_options(&mut reader, CHECKED).unwrap();
        let value = Value::deserialize(&mut deserializer).unwrap();
        assert_eq!(
            value.as_big_integer().unwrap().magnitude(),
            &hex_literal::hex!("010000000000000000")
        );

        let small = hex_literal::hex!("c24103");
        let mut reader = DepthAwareReader::from_reader(&small[..]);
        let mut deserializer =
            Deserializer::new_with_options(&mut reader, DecodeOptions::GENERAL).unwrap();
        assert_eq!(
            Value::deserialize(&mut deserializer).unwrap(),
            Value::Integer(3u8.into())
        );

        #[derive(serde::Deserialize)]
        struct Known {
            #[allow(dead_code)]
            known: u8,
        }

        let bytes = hex_literal::hex!("a2656b6e6f776e01666269676e756dc24103");
        let mut reader = DepthAwareReader::from_reader(&bytes[..]);
        let mut deserializer = Deserializer::new_with_options(&mut reader, CHECKED).unwrap();
        assert!(
            Known::deserialize(&mut deserializer).is_err(),
            "skipped bignum was accepted"
        );
    }

    #[test]
    fn reader_checking_is_explicitly_unsupported() {
        let bytes = [0x01u8];
        assert!(matches!(
            seabored::serde::from_reader_with_options::<Value, _>(&bytes[..], CHECKED),
            Err(SeaboredDeError::ProfileUnsupported { .. })
        ));
        assert!(
            seabored::serde::from_reader_with_options::<Value, _>(
                &bytes[..],
                DecodeOptions::GENERAL
            )
            .is_ok()
        );
    }
}
