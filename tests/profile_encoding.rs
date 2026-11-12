//! Profile-aware encoding across native, `Value`, and serde surfaces.

use seabored::{
    Profile, Value,
    de::from_bytes,
    error::SeaboredSerError,
    mt::MajorType,
    ser::CborSerialize as _,
    types::{CborMap, CborSequence},
};

#[test]
fn value_strict_encoding_forces_definite_containers() {
    let array: Value<'_> = from_bytes(&hex_literal::hex!("9f0102ff")).unwrap();
    assert_eq!(
        array
            .cbor_serialize_with_profile(Profile::PreferredPlus)
            .unwrap(),
        hex_literal::hex!("820102")
    );

    let text: Value<'_> = from_bytes(&hex_literal::hex!("7f61616162ff")).unwrap();
    assert_eq!(
        text.cbor_serialize_with_profile(Profile::PreferredPlus)
            .unwrap(),
        hex_literal::hex!("626162")
    );
}

#[test]
fn value_deterministic_encoding_sorts_maps_recursively() {
    let map: Value<'_> = from_bytes(&hex_literal::hex!("a2016178006179")).unwrap();
    assert_eq!(
        map.cbor_serialize_with_profile(Profile::PreferredPlus)
            .unwrap(),
        hex_literal::hex!("a2016178006179")
    );
    assert_eq!(
        map.cbor_serialize_with_profile(Profile::Deterministic)
            .unwrap(),
        hex_literal::hex!("a2006179016178")
    );

    let nested: Value<'_> = from_bytes(&hex_literal::hex!("81a2016178006179")).unwrap();
    assert_eq!(
        nested
            .cbor_serialize_with_profile(Profile::Deterministic)
            .unwrap(),
        hex_literal::hex!("81a2006179016178")
    );
}

#[test]
fn value_preferred_plus_normalizes_bignums() {
    for (input, expected) in [
        (
            &hex_literal::hex!("c2420003")[..],
            &hex_literal::hex!("03")[..],
        ),
        (
            &hex_literal::hex!("c3420018")[..],
            &hex_literal::hex!("3818")[..],
        ),
        (&hex_literal::hex!("c240")[..], &hex_literal::hex!("00")[..]),
        (&hex_literal::hex!("c340")[..], &hex_literal::hex!("20")[..]),
        (
            &hex_literal::hex!("c25f420003ff")[..],
            &hex_literal::hex!("03")[..],
        ),
    ] {
        let value: Value<'_> = from_bytes(input).unwrap();
        assert_eq!(
            value
                .cbor_serialize_with_profile(Profile::PreferredPlus)
                .unwrap(),
            expected,
            "{input:02x?}"
        );
    }
}

#[test]
fn value_biginteger_encodes_as_a_canonical_tag() {
    use seabored::types::CborBigInteger;

    // Leading zeroes are dropped; 9 significant bytes need the tag even under strict profiles.
    let big = CborBigInteger::new(false, vec![0, 1, 0, 0, 0, 0, 0, 0, 0, 0]);
    let value = Value::BigInteger(big);
    let expected = hex_literal::hex!("c249010000000000000000");
    assert_eq!(value.cbor_serialize().unwrap(), expected);
    assert_eq!(
        value
            .cbor_serialize_with_profile(Profile::PreferredPlus)
            .unwrap(),
        expected
    );
    assert_eq!(
        value
            .cbor_serialize_with_profile(Profile::Deterministic)
            .unwrap(),
        expected
    );

    // A bignum that fits major type 1 stays a tag under general and is rejected when strict.
    let small = Value::BigInteger(CborBigInteger::new(true, vec![1]));
    assert_eq!(small.cbor_serialize().unwrap(), hex_literal::hex!("c34101"));
    assert!(matches!(
        small.cbor_serialize_with_profile(Profile::PreferredPlus),
        Err(SeaboredSerError::ProfileUnsupported { .. })
    ));

    // Empty payload is n = 0: tag 2 is 0 and tag 3 is -1 (RFC 8949 §3.4.3; draft-09
    // §4.2 as clarified by cbor-wg PR #101).
    let zero = Value::BigInteger(CborBigInteger::new(false, Vec::new()));
    assert_eq!(zero.cbor_serialize().unwrap(), hex_literal::hex!("c240"));
    let minus_one = Value::BigInteger(CborBigInteger::new(true, Vec::new()));
    assert_eq!(
        minus_one.cbor_serialize().unwrap(),
        hex_literal::hex!("c340")
    );
    assert!(!minus_one.as_big_integer().unwrap().is_zero());
    for value in [&zero, &minus_one] {
        assert!(matches!(
            value.cbor_serialize_with_profile(Profile::PreferredPlus),
            Err(SeaboredSerError::ProfileUnsupported { .. })
        ));
    }

    // A normalized bignum decodes to an integer, so the round trip canonicalizes.
    let decoded: Value<'_> = from_bytes(&expected).unwrap();
    assert_eq!(
        decoded,
        Value::BigInteger(CborBigInteger::new(false, vec![1, 0, 0, 0, 0, 0, 0, 0, 0]))
    );
    assert_eq!(
        decoded
            .cbor_serialize_with_profile(Profile::Deterministic)
            .unwrap(),
        expected
    );
}

#[test]
fn strict_profiles_reject_an_indefinite_sequence() {
    let mut sequence = CborSequence::new_indefinite(MajorType::Array);
    sequence.extend([1u8, 2u8]);

    assert_eq!(
        sequence.cbor_serialize().unwrap(),
        hex_literal::hex!("9f0102ff")
    );
    assert!(matches!(
        sequence.cbor_serialize_with_profile(Profile::PreferredPlus),
        Err(SeaboredSerError::IndefiniteForbidden { .. })
    ));
}

#[test]
fn typed_map_deterministic_sorts_entries_by_key_bytes() {
    let map: CborMap<u8, &str> = [(2, "b"), (1, "a")].into_iter().collect();

    assert_eq!(
        map.cbor_serialize_with_profile(Profile::PreferredPlus)
            .unwrap(),
        hex_literal::hex!("a2026162016161")
    );
    assert_eq!(
        map.cbor_serialize_with_profile(Profile::Deterministic)
            .unwrap(),
        hex_literal::hex!("a2016161026162")
    );
}

#[test]
fn typed_map_round_trips_and_preserves_indefinite_general_form() {
    let bytes = hex_literal::hex!("bf026162016161ff");
    let map: CborMap<u8, String> = from_bytes(&bytes).unwrap();

    assert!(map.is_indefinite);
    assert_eq!(map.cbor_serialize().unwrap(), bytes);
    assert!(matches!(
        map.cbor_serialize_with_profile(Profile::PreferredPlus),
        Err(SeaboredSerError::IndefiniteForbidden { .. })
    ));
}

#[cfg(feature = "hazmat")]
#[test]
fn raw_value_strict_encoding_validates_the_bytes() {
    use seabored::types::RawValue;

    let canonical = hex_literal::hex!("820102");
    let raw = RawValue::from_slice_unchecked(&canonical);
    assert_eq!(
        raw.cbor_serialize_with_profile(Profile::PreferredPlus)
            .unwrap(),
        canonical
    );

    let indefinite = hex_literal::hex!("9f0102ff");
    let raw = RawValue::from_slice_unchecked(&indefinite);
    assert!(matches!(
        raw.cbor_serialize_with_profile(Profile::PreferredPlus),
        Err(SeaboredSerError::ProfileUnsupported { .. })
    ));

    let unsorted = hex_literal::hex!("a2016178006179");
    let raw = RawValue::from_slice_unchecked(&unsorted);
    assert!(
        raw.cbor_serialize_with_profile(Profile::PreferredPlus)
            .is_ok()
    );
    assert!(matches!(
        raw.cbor_serialize_with_profile(Profile::Deterministic),
        Err(SeaboredSerError::ProfileUnsupported { .. })
    ));
}

#[test]
fn legacy_impls_do_not_silently_claim_strict_profiles() {
    struct Legacy;

    impl seabored::ser::CborSerialize for Legacy {
        fn cbor_serialize_to<W: seabored::reexports::parsio::Write>(
            &self,
            writer: &mut W,
        ) -> Result<usize, SeaboredSerError> {
            Ok(seabored::reexports::parsio::Write::write(writer, &[0x00])?)
        }
    }

    assert_eq!(Legacy.cbor_serialize().unwrap(), hex_literal::hex!("00"));
    assert!(matches!(
        Legacy.cbor_serialize_with_profile(Profile::PreferredPlus),
        Err(SeaboredSerError::ProfileUnsupported { .. })
    ));
}

#[cfg(feature = "serde")]
mod serde_encoding {
    use super::*;
    use serde::{
        Serialize, Serializer,
        ser::{SerializeMap as _, SerializeSeq as _},
    };

    struct UnsizedSequence;

    impl Serialize for UnsizedSequence {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut sequence = serializer.serialize_seq(None)?;
            sequence.serialize_element(&1u8)?;
            sequence.serialize_element(&2u8)?;
            sequence.end()
        }
    }

    struct UnsizedMap;

    impl Serialize for UnsizedMap {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut map = serializer.serialize_map(None)?;
            map.serialize_entry(&2u8, "b")?;
            map.serialize_entry(&1u8, "a")?;
            map.end()
        }
    }

    #[derive(Serialize)]
    struct OutOfOrder {
        z: u8,
        a: u8,
    }

    #[test]
    fn strict_unsized_sequences_become_definite() {
        assert_eq!(
            seabored::serde::to_vec_with_profile(&UnsizedSequence, Profile::PreferredPlus).unwrap(),
            hex_literal::hex!("820102")
        );
        assert_eq!(
            seabored::serde::to_vec_with_profile(&UnsizedSequence, Profile::Deterministic).unwrap(),
            hex_literal::hex!("820102")
        );
    }

    #[test]
    fn strict_unsized_maps_become_definite_and_deterministic() {
        assert_eq!(
            seabored::serde::to_vec_with_profile(&UnsizedMap, Profile::PreferredPlus).unwrap(),
            hex_literal::hex!("a2026162016161")
        );
        assert_eq!(
            seabored::serde::to_vec_with_profile(&UnsizedMap, Profile::Deterministic).unwrap(),
            hex_literal::hex!("a2016161026162")
        );
    }

    #[test]
    fn deterministic_serde_sorts_struct_fields() {
        let value = OutOfOrder { z: 2, a: 1 };
        assert_eq!(
            seabored::serde::to_vec_with_profile(&value, Profile::PreferredPlus).unwrap(),
            hex_literal::hex!("a2617a02616101")
        );
        assert_eq!(
            seabored::serde::to_vec_with_profile(&value, Profile::Deterministic).unwrap(),
            hex_literal::hex!("a2616101617a02")
        );
    }

    #[test]
    fn serde_preserves_indefinite_string_chunks_under_general() {
        let bytes: Value<'_> = from_bytes(&hex_literal::hex!("5f4201024103ff")).unwrap();
        assert_eq!(
            seabored::serde::to_vec(&bytes).unwrap(),
            hex_literal::hex!("5f4201024103ff")
        );
        assert_eq!(
            seabored::serde::to_vec_with_profile(&bytes, Profile::PreferredPlus).unwrap(),
            hex_literal::hex!("43010203")
        );

        let text: Value<'_> = from_bytes(&hex_literal::hex!("7f61616162ff")).unwrap();
        assert_eq!(
            seabored::serde::to_vec(&text).unwrap(),
            hex_literal::hex!("7f61616162ff")
        );
        assert_eq!(
            seabored::serde::to_vec_with_profile(&text, Profile::Deterministic).unwrap(),
            hex_literal::hex!("626162")
        );

        // A finite chunk sequence is concatenated even under general serialization.
        let mut finite = CborSequence::new_finite(MajorType::Bytes, 2);
        finite.extend([Value::Bytes(vec![1].into()), Value::Bytes(vec![2].into())]);
        let finite = Value::Sequence(finite);
        assert_eq!(
            seabored::serde::to_vec(&finite).unwrap(),
            hex_literal::hex!("420102")
        );
    }

    #[test]
    fn strict_serde_value_bignums_are_canonical() {
        use seabored::types::CborBigInteger;

        let small = Value::BigInteger(CborBigInteger::new(false, vec![1]));
        assert_eq!(
            seabored::serde::to_vec_with_profile(&small, Profile::PreferredPlus).unwrap(),
            [0x01]
        );

        let tagged = Value::Tagged((2u8.into(), Box::new(Value::Bytes(vec![1].into()))));
        assert_eq!(
            seabored::serde::to_vec_with_profile(&tagged, Profile::Deterministic).unwrap(),
            [0x01]
        );

        let negative = Value::Tagged((3u8.into(), Box::new(Value::Bytes(vec![1].into()))));
        assert_eq!(
            seabored::serde::to_vec_with_profile(&negative, Profile::PreferredPlus).unwrap(),
            [0x21]
        );

        let big = Value::BigInteger(CborBigInteger::new(false, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]));
        assert_eq!(
            seabored::serde::to_vec_with_profile(&big, Profile::PreferredPlus).unwrap(),
            hex_literal::hex!("c249010203040506070809")
        );

        let not_bytes = Value::Tagged((2u8.into(), Box::new(Value::Integer(1u8.into()))));
        assert!(matches!(
            seabored::serde::to_vec_with_profile(&not_bytes, Profile::PreferredPlus),
            Err(SeaboredSerError::ProfileUnsupported { .. })
        ));
    }

    #[test]
    fn serde_value_bignums_round_trip() {
        let bytes = hex_literal::hex!("c249010000000000000000");
        let value: Value<'_> = seabored::serde::from_slice(&bytes).unwrap();
        assert!(value.as_big_integer().is_some());
        assert_eq!(seabored::serde::to_vec(&value).unwrap(), bytes);
    }

    #[test]
    fn deterministic_serde_sorts_hash_maps() {
        let mut map = std::collections::HashMap::new();
        map.insert(2u8, "b");
        map.insert(1u8, "a");
        assert_eq!(
            seabored::serde::to_vec_with_profile(&map, Profile::Deterministic).unwrap(),
            hex_literal::hex!("a2016161026162")
        );
    }
}
