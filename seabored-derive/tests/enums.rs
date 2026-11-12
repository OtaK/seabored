use std::borrow::Cow;

use seabored::de::from_bytes;
use seabored::error::SeaboredDeError;
use seabored::ser::CborSerialize as _;

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
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

#[test]
fn enum_variant_shapes() {
    assert_eq!(
        Basic::Unit.cbor_serialize().unwrap(),
        hex_literal::hex!("64556e6974")
    );
    assert_eq!(
        Basic::Newtype(1).cbor_serialize().unwrap(),
        hex_literal::hex!("a1674e65777479706501")
    );
    assert_eq!(
        Basic::Tuple(1, 2).cbor_serialize().unwrap(),
        hex_literal::hex!("a1655475706c65820102")
    );
    assert_eq!(
        Basic::Struct { a: 1, b: 2 }.cbor_serialize().unwrap(),
        hex_literal::hex!("a166537472756374a2616101616202")
    );

    assert_eq!(
        from_bytes::<Basic>(&hex_literal::hex!("64556e6974")).unwrap(),
        Basic::Unit
    );
    assert_eq!(
        from_bytes::<Basic>(&hex_literal::hex!("a1674e65777479706501")).unwrap(),
        Basic::Newtype(1)
    );
    assert_eq!(
        from_bytes::<Basic>(&hex_literal::hex!("a1655475706c65820102")).unwrap(),
        Basic::Tuple(1, 2)
    );
    assert_eq!(
        from_bytes::<Basic>(&hex_literal::hex!("a166537472756374a1616101")).unwrap(),
        Basic::Struct { a: 1, b: 0 }
    );
}

#[test]
fn enum_errors() {
    assert!(matches!(
        from_bytes::<Basic>(&hex_literal::hex!("617a")),
        Err(SeaboredDeError::UnexpectedKey { .. })
    ));
    assert!(matches!(
        from_bytes::<Basic>(&hex_literal::hex!("a1617a01")),
        Err(SeaboredDeError::UnexpectedKey { .. })
    ));
    assert!(matches!(
        from_bytes::<Basic>(&hex_literal::hex!("a2674e65777479706501617a01")),
        Err(SeaboredDeError::WrongLen {
            expected: 1,
            got: 2
        })
    ));
    assert!(matches!(
        from_bytes::<Basic>(&hex_literal::hex!("a164556e697401")),
        Err(SeaboredDeError::IncorrectMajorType { .. })
    ));
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
enum IntEnum {
    #[cbor(key = 1)]
    Unit,
    #[cbor(key = -2)]
    Newtype(u8),
}

#[test]
fn integer_variant_keys() {
    assert_eq!(
        IntEnum::Unit.cbor_serialize().unwrap(),
        hex_literal::hex!("01")
    );
    assert_eq!(
        IntEnum::Newtype(1).cbor_serialize().unwrap(),
        hex_literal::hex!("a12101")
    );
    assert_eq!(
        from_bytes::<IntEnum>(&hex_literal::hex!("01")).unwrap(),
        IntEnum::Unit
    );
    assert_eq!(
        from_bytes::<IntEnum>(&hex_literal::hex!("a12101")).unwrap(),
        IntEnum::Newtype(1)
    );
    assert!(matches!(
        from_bytes::<IntEnum>(&hex_literal::hex!("02")),
        Err(SeaboredDeError::UnexpectedKey { key }) if key == "2"
    ));
    assert!(matches!(
        from_bytes::<IntEnum>(&hex_literal::hex!("a10201")),
        Err(SeaboredDeError::UnexpectedKey { key }) if key == "2"
    ));
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
enum VariantTags {
    #[cbor(tag = 7)]
    Newtype(u8),
    #[cbor(tag = 8)]
    Tuple(u8, u16),
    #[cbor(tag = 9)]
    Struct { a: u8 },
}

#[test]
fn variant_tags_wrap_the_payload() {
    assert_eq!(
        VariantTags::Newtype(1).cbor_serialize().unwrap(),
        hex_literal::hex!("a1674e657774797065c701")
    );
    assert_eq!(
        VariantTags::Tuple(1, 2).cbor_serialize().unwrap(),
        hex_literal::hex!("a1655475706c65c8820102")
    );
    assert_eq!(
        VariantTags::Struct { a: 1 }.cbor_serialize().unwrap(),
        hex_literal::hex!("a166537472756374c9a1616101")
    );
    assert_eq!(
        from_bytes::<VariantTags>(&hex_literal::hex!("a1674e657774797065c701")).unwrap(),
        VariantTags::Newtype(1)
    );
    assert!(matches!(
        from_bytes::<VariantTags>(&hex_literal::hex!("a1674e657774797065c601")),
        Err(SeaboredDeError::WrongTag {
            actual: 6,
            expected: 7
        })
    ));
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(tag = 42)]
enum TaggedEnum {
    Unit,
    Newtype(u8),
}

#[test]
fn enum_container_tag_wraps_the_whole_encoding() {
    assert_eq!(
        TaggedEnum::Unit.cbor_serialize().unwrap(),
        hex_literal::hex!("d82a64556e6974")
    );
    assert_eq!(
        TaggedEnum::Newtype(1).cbor_serialize().unwrap(),
        hex_literal::hex!("d82aa1674e65777479706501")
    );
    assert_eq!(
        from_bytes::<TaggedEnum>(&hex_literal::hex!("d82a64556e6974")).unwrap(),
        TaggedEnum::Unit
    );
    assert_eq!(
        from_bytes::<TaggedEnum>(&hex_literal::hex!("d82aa1674e65777479706501")).unwrap(),
        TaggedEnum::Newtype(1)
    );
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
enum Rich {
    #[cbor(key = 1, tag = 10)]
    Struct {
        #[cbor(key = "a", tag = 5)]
        a: u8,
        #[cbor(simple_value)]
        s: u8,
        #[cbor(default = 3)]
        d: u8,
        #[cbor(skip, default = 4)]
        skipped: u8,
        #[cbor(indefinite)]
        items: Vec<u8>,
    },
}

#[test]
fn struct_variant_field_attributes() {
    let value = Rich::Struct {
        a: 1,
        s: 1,
        d: 2,
        skipped: 4,
        items: vec![3],
    };
    assert_eq!(
        value.cbor_serialize().unwrap(),
        hex_literal::hex!("a101caa46161c5016173e1616402656974656d739f03ff")
    );
    assert_eq!(
        from_bytes::<Rich>(&hex_literal::hex!(
            "a101caa46161c5016173e1616402656974656d739f03ff"
        ))
        .unwrap(),
        value
    );
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
enum Generic<T> {
    Value(T),
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
enum Borrowed<'a> {
    Value(Cow<'a, str>),
}

#[test]
fn enum_generics_and_borrowed_payloads() {
    assert_eq!(
        Generic::Value(1u32).cbor_serialize().unwrap(),
        hex_literal::hex!("a16556616c756501")
    );
    assert_eq!(
        from_bytes::<Generic<u32>>(&hex_literal::hex!("a16556616c756501")).unwrap(),
        Generic::Value(1)
    );

    let value: Borrowed<'_> = from_bytes(&hex_literal::hex!("a16556616c75656161")).unwrap();
    assert!(matches!(value, Borrowed::Value(Cow::Borrowed("a"))));
}

#[test]
fn indefinite_variant_map_and_variant_tag_decoding() {
    assert_eq!(
        from_bytes::<Basic>(&hex_literal::hex!("bf674e65777479706501ff")).unwrap(),
        Basic::Newtype(1)
    );
    assert_eq!(
        from_bytes::<VariantTags>(&hex_literal::hex!("a1655475706c65c8820102")).unwrap(),
        VariantTags::Tuple(1, 2)
    );
    assert_eq!(
        from_bytes::<VariantTags>(&hex_literal::hex!("a166537472756374c9a1616101")).unwrap(),
        VariantTags::Struct { a: 1 }
    );

    assert!(matches!(
        from_bytes::<Basic>(&hex_literal::hex!(
            "bf674e65777479706501674e65777479706502ff"
        )),
        Err(SeaboredDeError::TooManyElements { expected: 1 })
    ));
}
