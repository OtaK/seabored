use std::borrow::Cow;
use std::collections::{BTreeSet, HashSet};

use seabored::Value;
use seabored::{
    de::{self, skip_value},
    error::{SeaboredDeError, SeaboredSerError},
    ib::{AdditionalInfo, InitialByte, consts},
    mt::MajorType,
    reexports::parsio::{self, Read as _},
    ser::{CborSerialize as _, write_preamble},
    types::CborSimpleValue,
};

fn ai_and_rest(bytes: &[u8]) -> (AdditionalInfo, &[u8]) {
    let mut reader = bytes;
    let ib = InitialByte::peek(&mut reader).unwrap();
    reader.advance(1).unwrap();
    (ib.ai(), reader)
}

#[test]
fn additional_info_length_and_shortest_form() {
    let (ai, mut rest) = ai_and_rest(&[0x00]);
    assert_eq!(ai.raw(), 0);
    assert!(!ai.is_indefinite());
    assert_eq!(ai.length(&mut rest).unwrap(), Some(0));
    assert!(ai.is_shortest(0));
    assert!(!ai.is_shortest(1));

    let (ai, mut rest) = ai_and_rest(&[0xa3]);
    assert_eq!(ai.raw(), 3);
    assert_eq!(ai.length(&mut rest).unwrap(), Some(3));
    assert!(ai.is_shortest(3));

    let (ai, mut rest) = ai_and_rest(&[0x18, 0x18]);
    assert_eq!(ai.length(&mut rest).unwrap(), Some(24));
    assert!(ai.is_shortest(24));
    assert!(!ai.is_shortest(23));

    let (ai, mut rest) = ai_and_rest(&[0x19, 0x01, 0x00]);
    assert_eq!(ai.length(&mut rest).unwrap(), Some(256));
    assert!(ai.is_shortest(256));
    assert!(!ai.is_shortest(255));

    let (ai, mut rest) = ai_and_rest(&[0x1a, 0x00, 0x01, 0x00, 0x00]);
    assert_eq!(ai.length(&mut rest).unwrap(), Some(65_536));
    assert!(ai.is_shortest(65_536));
    assert!(!ai.is_shortest(65_535));

    let (ai, mut rest) = ai_and_rest(&[0x1b, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00]);
    assert_eq!(ai.length(&mut rest).unwrap(), Some(1 << 32));
    assert!(ai.is_shortest(1 << 32));
    assert!(!ai.is_shortest((1 << 32) - 1));

    let (ai, mut rest) = ai_and_rest(&[0x1f]);
    assert!(ai.is_indefinite());
    assert_eq!(ai.length(&mut rest).unwrap(), None);
    assert!(!ai.is_shortest(0));

    let (ai, mut rest) = ai_and_rest(&[0x1c]);
    assert!(matches!(
        ai.length(&mut rest),
        Err(SeaboredDeError::IllegalAdditionalInfo(28))
    ));
}

#[test]
fn preamble_writer_is_shortest_and_validates_major_type() {
    let mut out = Vec::new();
    assert_eq!(write_preamble(MajorType::Map, 3, &mut out).unwrap(), 1);
    assert_eq!(out, hex_literal::hex!("a3"));

    out.clear();
    assert_eq!(write_preamble(MajorType::Array, 24, &mut out).unwrap(), 2);
    assert_eq!(out, hex_literal::hex!("9818"));

    out.clear();
    assert_eq!(write_preamble(MajorType::String, 24, &mut out).unwrap(), 2);
    assert_eq!(out, hex_literal::hex!("7818"));

    out.clear();
    assert_eq!(write_preamble(MajorType::Tagged, 24, &mut out).unwrap(), 2);
    assert_eq!(out, hex_literal::hex!("d818"));

    assert!(matches!(
        write_preamble(MajorType::Uint, 1, &mut out),
        Err(SeaboredSerError::UnexpectedPreambleMajorType(
            MajorType::Uint
        ))
    ));
    assert!(matches!(
        write_preamble(MajorType::SimpleValueOrFloat, 1, &mut out),
        Err(SeaboredSerError::UnexpectedPreambleMajorType(
            MajorType::SimpleValueOrFloat
        ))
    ));
}

fn skip(bytes: &[u8]) -> Result<(), SeaboredDeError> {
    let mut reader = parsio::DepthAwareReader::from_reader(bytes);
    let mut guard = reader.enter()?;
    skip_value(&mut guard)
}

#[test]
fn skip_accepts_well_formed_items() {
    for bytes in [
        &hex_literal::hex!("00")[..],
        &hex_literal::hex!("f6")[..],
        &hex_literal::hex!("e0")[..],
        &hex_literal::hex!("f820")[..],
        &hex_literal::hex!("9f0102ff")[..],
        &hex_literal::hex!("bf01020304ff")[..],
        &hex_literal::hex!("1801")[..],
        &hex_literal::hex!("a1180101")[..],
        &hex_literal::hex!("5f5800ff")[..],
        &hex_literal::hex!("5f40ff")[..],
        &hex_literal::hex!("5f4161ff")[..],
        &hex_literal::hex!("7f60ff")[..],
        &hex_literal::hex!("7f6161ff")[..],
        &hex_literal::hex!("bf0102ff")[..],
        &hex_literal::hex!("7f62c3a9ff")[..],
        &hex_literal::hex!("c001")[..],
        &hex_literal::hex!("c0c101")[..],
    ] {
        skip(bytes).unwrap_or_else(|e| panic!("{bytes:02x?}: {e}"));
    }
}

#[test]
fn skip_rejects_non_well_formed_items() {
    for bytes in [
        &hex_literal::hex!("ff")[..],
        &hex_literal::hex!("1f")[..],
        &hex_literal::hex!("df01")[..],
        &hex_literal::hex!("1c")[..],
        &hex_literal::hex!("f800")[..],
        &hex_literal::hex!("f81f")[..],
        &hex_literal::hex!("7f7fff")[..],
        &hex_literal::hex!("5f7fffff")[..],
        &hex_literal::hex!("7f61c361a9ff")[..],
        &hex_literal::hex!("9f01")[..],
        &hex_literal::hex!("bf01ff")[..],
        &hex_literal::hex!("c0")[..],
        &hex_literal::hex!("61ff")[..],
    ] {
        assert!(skip(bytes).is_err(), "accepted {bytes:02x?}");
    }
}

#[test]
fn simple_values_encode_and_decode() {
    for (value, bytes) in [
        (0, &hex_literal::hex!("e0")[..]),
        (19, &hex_literal::hex!("f3")[..]),
        (32, &hex_literal::hex!("f820")[..]),
        (255, &hex_literal::hex!("f8ff")[..]),
    ] {
        let encoded = CborSimpleValue(value).cbor_serialize().unwrap();
        assert_eq!(encoded, bytes);
        assert_eq!(
            de::from_bytes::<CborSimpleValue>(bytes).unwrap(),
            CborSimpleValue(value)
        );
    }

    for reserved in 20..=31 {
        assert!(matches!(
            CborSimpleValue(reserved).cbor_serialize(),
            Err(SeaboredSerError::ReservedSimpleValue(v)) if v == reserved
        ));
    }

    assert!(matches!(
        de::from_bytes::<CborSimpleValue>(&hex_literal::hex!("f4")),
        Err(SeaboredDeError::ReservedSimpleValue(20))
    ));
    assert!(matches!(
        de::from_bytes::<CborSimpleValue>(&hex_literal::hex!("f81f")),
        Err(SeaboredDeError::NotWellFormed(_))
    ));
    assert!(matches!(
        de::from_bytes::<CborSimpleValue>(&hex_literal::hex!("f90000")),
        Err(SeaboredDeError::IncorrectInitialByteSet { .. })
    ));
}

#[test]
fn strings_and_cows_round_trip() {
    assert_eq!("a".cbor_serialize().unwrap(), hex_literal::hex!("6161"));
    assert_eq!(
        "a".to_owned().cbor_serialize().unwrap(),
        hex_literal::hex!("6161")
    );
    assert_eq!(
        Cow::Borrowed("a").cbor_serialize().unwrap(),
        hex_literal::hex!("6161")
    );
    assert_eq!(
        Cow::Borrowed(&b"a"[..]).cbor_serialize().unwrap(),
        hex_literal::hex!("4161")
    );

    assert_eq!(
        de::from_bytes::<String>(&hex_literal::hex!("6161")).unwrap(),
        "a"
    );
    assert_eq!(
        de::from_bytes::<String>(&hex_literal::hex!("7f6161ff")).unwrap(),
        "a"
    );
    let borrowed: Cow<'_, str> = de::from_bytes(&hex_literal::hex!("6161")).unwrap();
    assert!(matches!(borrowed, Cow::Borrowed("a")));
    let bytes: Cow<'_, [u8]> = de::from_bytes(&hex_literal::hex!("4161")).unwrap();
    assert!(matches!(bytes, Cow::Borrowed(b"a")));
}

#[test]
fn vec_is_an_array_even_for_u8() {
    assert_eq!(
        vec![1u8, 2].cbor_serialize().unwrap(),
        hex_literal::hex!("820102")
    );
    assert_eq!(
        vec![1u32, 2].cbor_serialize().unwrap(),
        hex_literal::hex!("820102")
    );

    assert_eq!(
        de::from_bytes::<Vec<u8>>(&hex_literal::hex!("820102")).unwrap(),
        vec![1, 2]
    );
    assert_eq!(
        de::from_bytes::<Vec<u32>>(&hex_literal::hex!("9f0102ff")).unwrap(),
        vec![1, 2]
    );
}

#[test]
fn sets_round_trip_and_collapse_duplicates() {
    let btree: BTreeSet<u32> = [2, 1].into_iter().collect();
    assert_eq!(btree.cbor_serialize().unwrap(), hex_literal::hex!("820102"));
    assert_eq!(
        de::from_bytes::<BTreeSet<u32>>(&hex_literal::hex!("83010102")).unwrap(),
        [1, 2].into_iter().collect()
    );
    assert_eq!(
        de::from_bytes::<BTreeSet<u32>>(&hex_literal::hex!("9f0201ff")).unwrap(),
        [1, 2].into_iter().collect()
    );

    let hash: HashSet<u32> = [1, 2].into_iter().collect();
    let encoded = hash.cbor_serialize().unwrap();
    assert_eq!(encoded.len(), 3);
    assert_eq!(encoded[0], 0x82);
    assert_eq!(de::from_bytes::<HashSet<u32>>(&encoded).unwrap(), hash);
    assert_eq!(
        de::from_bytes::<HashSet<u32>>(&hex_literal::hex!("83010102")).unwrap(),
        hash
    );
}

#[test]
fn option_and_box_round_trip() {
    assert_eq!(
        Option::<u32>::None.cbor_serialize().unwrap(),
        hex_literal::hex!("f6")
    );
    assert_eq!(
        Some(1u32).cbor_serialize().unwrap(),
        hex_literal::hex!("01")
    );
    assert_eq!(
        de::from_bytes::<Option<u32>>(&hex_literal::hex!("f6")).unwrap(),
        None
    );
    assert_eq!(
        de::from_bytes::<Option<u32>>(&hex_literal::hex!("01")).unwrap(),
        Some(1)
    );
    assert!(matches!(
        de::from_bytes::<Option<Value>>(&hex_literal::hex!("f7")),
        Err(SeaboredDeError::UnsupportedSimpleValue(0xf7))
    ));

    assert_eq!(
        Box::new(1u32).cbor_serialize().unwrap(),
        hex_literal::hex!("01")
    );
    assert_eq!(
        *de::from_bytes::<Box<u32>>(&hex_literal::hex!("01")).unwrap(),
        1
    );
}

#[test]
fn skip_respects_the_depth_limit() {
    let mut tags = vec![0xc0u8; 10];
    tags.push(0x00);
    let mut reader = parsio::DepthAwareReader::from_reader_with_limit(&tags[..], 4);
    let mut guard = reader.enter().unwrap();
    assert!(skip_value(&mut guard).is_err());
}

#[test]
fn initial_byte_raw_is_public() {
    let ib = InitialByte::peek(&mut &hex_literal::hex!("ff")[..]).unwrap();
    assert_eq!(ib.raw(), consts::IB_BREAK);
}
