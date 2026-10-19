use std::borrow::Cow;

use seabored::de::from_bytes;
use seabored::error::SeaboredDeError;
use seabored::ser::CborSerialize as _;

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
struct NamedMap {
    a: u8,
    #[cbor(key = 1)]
    b: u16,
    #[cbor(skip, default = 9)]
    skipped: u8,
    #[cbor(default)]
    missing: u8,
}

#[test]
fn named_map_round_trip() {
    let value = NamedMap {
        a: 1,
        b: 2,
        skipped: 4,
        missing: 3,
    };
    let encoded = value.cbor_serialize().unwrap();
    assert_eq!(encoded, hex_literal::hex!("a36161010102676d697373696e6703"));

    let decoded: NamedMap = from_bytes(&hex_literal::hex!("a26161010102")).unwrap();
    assert_eq!(
        decoded,
        NamedMap {
            a: 1,
            b: 2,
            skipped: 9,
            missing: 0
        }
    );

    assert!(matches!(
        from_bytes::<NamedMap>(&hex_literal::hex!("a36161010102616103")),
        Err(SeaboredDeError::DuplicateField { name: "a" })
    ));
    assert!(matches!(
        from_bytes::<NamedMap>(&hex_literal::hex!("a1616101")),
        Err(SeaboredDeError::MissingField { name: "b" })
    ));

    let with_unknown: NamedMap = from_bytes(&hex_literal::hex!("a36161010102617a05")).unwrap();
    assert_eq!(with_unknown.a, 1);
    assert_eq!(with_unknown.b, 2);
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(tag = 42)]
struct Tagged {
    #[cbor(tag = 5)]
    x: u32,
    #[cbor(simple_value, tag = 6)]
    s: u8,
}

#[test]
fn container_field_and_simple_tags() {
    let value = Tagged { x: 1, s: 1 };
    assert_eq!(
        value.cbor_serialize().unwrap(),
        hex_literal::hex!("d82aa26178c5016173c6e1")
    );
    assert_eq!(
        from_bytes::<Tagged>(&hex_literal::hex!("d82aa26178c5016173c6e1")).unwrap(),
        value
    );
    assert!(matches!(
        from_bytes::<Tagged>(&hex_literal::hex!("d82ba26178c5016173c6e1")),
        Err(SeaboredDeError::WrongTag {
            actual: 43,
            expected: 42
        })
    ));
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(array)]
struct ArrayStruct {
    a: u8,
    b: u16,
}

#[test]
fn named_array_form() {
    let value = ArrayStruct { a: 1, b: 2 };
    assert_eq!(value.cbor_serialize().unwrap(), hex_literal::hex!("820102"));
    assert_eq!(
        from_bytes::<ArrayStruct>(&hex_literal::hex!("820102")).unwrap(),
        value
    );
    assert!(matches!(
        from_bytes::<ArrayStruct>(&hex_literal::hex!("8101")),
        Err(SeaboredDeError::WrongLen {
            expected: 2,
            got: 1
        })
    ));
    assert!(matches!(
        from_bytes::<ArrayStruct>(&hex_literal::hex!("83010203")),
        Err(SeaboredDeError::WrongLen {
            expected: 2,
            got: 3
        })
    ));
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
struct Tuple(u8, u16);

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
struct Newtype(u32);

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(tag = 1)]
struct TaggedNewtype(u32);

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(tag = 1)]
struct TaggedUnit;

#[test]
fn tuple_newtype_and_unit_forms() {
    assert_eq!(
        Tuple(1, 2).cbor_serialize().unwrap(),
        hex_literal::hex!("820102")
    );
    assert_eq!(
        from_bytes::<Tuple>(&hex_literal::hex!("820102")).unwrap(),
        Tuple(1, 2)
    );

    assert_eq!(
        Newtype(1).cbor_serialize().unwrap(),
        hex_literal::hex!("01")
    );
    assert_eq!(
        from_bytes::<Newtype>(&hex_literal::hex!("01")).unwrap(),
        Newtype(1)
    );

    assert_eq!(
        TaggedNewtype(1).cbor_serialize().unwrap(),
        hex_literal::hex!("c101")
    );
    assert_eq!(
        from_bytes::<TaggedNewtype>(&hex_literal::hex!("c101")).unwrap(),
        TaggedNewtype(1)
    );

    assert_eq!(
        TaggedUnit.cbor_serialize().unwrap(),
        hex_literal::hex!("c1f6")
    );
    assert_eq!(
        from_bytes::<TaggedUnit>(&hex_literal::hex!("c1f6")).unwrap(),
        TaggedUnit
    );
}

mod custom {
    use seabored::de::CborDeserialize as _;
    use seabored::error::{SeaboredDeError, SeaboredSerError};
    use seabored::reexports::parsio::{DepthAwareReaderGuard, Read, Write};
    use seabored::ser::CborSerialize as _;

    pub fn cbor_serialize_to<W: Write>(
        value: &u32,
        writer: &mut W,
    ) -> Result<usize, SeaboredSerError> {
        value.cbor_serialize_to(writer)
    }

    pub fn cbor_deserialize_from<'de, R: Read<'de>>(
        reader: &mut DepthAwareReaderGuard<'_, 'de, R>,
    ) -> Result<u32, SeaboredDeError> {
        u32::cbor_deserialize_from(reader)
    }
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
struct With {
    #[cbor(with = custom)]
    value: u32,
}

#[test]
fn with_module_round_trip() {
    let value = With { value: 1 };
    assert_eq!(
        value.cbor_serialize().unwrap(),
        hex_literal::hex!("a16576616c756501")
    );
    assert_eq!(
        from_bytes::<With>(&hex_literal::hex!("a16576616c756501")).unwrap(),
        value
    );
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
struct Indefinite {
    #[cbor(indefinite)]
    items: Vec<u8>,
}

#[test]
fn indefinite_collection_is_encode_only() {
    let value = Indefinite { items: vec![1, 2] };
    assert_eq!(
        value.cbor_serialize().unwrap(),
        hex_literal::hex!("a1656974656d739f0102ff")
    );
    assert_eq!(
        from_bytes::<Indefinite>(&hex_literal::hex!("a1656974656d739f0102ff")).unwrap(),
        value
    );
    assert_eq!(
        from_bytes::<Indefinite>(&hex_literal::hex!("a1656974656d73820102")).unwrap(),
        value
    );
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
struct Generic<T> {
    value: T,
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
struct Borrowed<'a> {
    value: Cow<'a, str>,
}

#[test]
fn generics_and_borrowed_fields() {
    let value = Generic { value: 1u32 };
    assert_eq!(
        value.cbor_serialize().unwrap(),
        hex_literal::hex!("a16576616c756501")
    );
    assert_eq!(
        from_bytes::<Generic<u32>>(&hex_literal::hex!("a16576616c756501")).unwrap(),
        value
    );

    let borrowed: Borrowed<'_> = from_bytes(&hex_literal::hex!("a16576616c75656161")).unwrap();
    assert!(matches!(borrowed.value, Cow::Borrowed("a")));
    assert_eq!(
        Borrowed {
            value: Cow::Borrowed("a")
        }
        .cbor_serialize()
        .unwrap(),
        hex_literal::hex!("a16576616c75656161")
    );
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
struct DefaultExpr {
    #[cbor(default = 5)]
    value: u8,
    #[cbor(skip, default = 7)]
    skipped: u8,
}

#[test]
fn default_expressions_and_skip() {
    let decoded: DefaultExpr = from_bytes(&hex_literal::hex!("a0")).unwrap();
    assert_eq!(
        decoded,
        DefaultExpr {
            value: 5,
            skipped: 7
        }
    );

    let decoded: DefaultExpr = from_bytes(&hex_literal::hex!("a16576616c756501")).unwrap();
    assert_eq!(
        decoded,
        DefaultExpr {
            value: 1,
            skipped: 7
        }
    );

    assert_eq!(
        DefaultExpr {
            value: 1,
            skipped: 9
        }
        .cbor_serialize()
        .unwrap(),
        hex_literal::hex!("a16576616c756501")
    );
}

#[test]
fn indefinite_and_definite_containers_decode() {
    #[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
    struct MapIndefinite {
        a: u8,
        b: u8,
    }

    let decoded: MapIndefinite = from_bytes(&hex_literal::hex!("bf616101616202ff")).unwrap();
    assert_eq!(decoded, MapIndefinite { a: 1, b: 2 });

    let decoded: Tuple = from_bytes(&hex_literal::hex!("9f0102ff")).unwrap();
    assert_eq!(decoded, Tuple(1, 2));

    assert!(matches!(
        from_bytes::<Tuple>(&hex_literal::hex!("9f010203ff")),
        Err(SeaboredDeError::TooManyElements { expected: 2 })
    ));
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
struct SkipDefault {
    #[cbor(skip, default = 9)]
    skipped: u8,
}

#[test]
fn skipped_field_requires_no_traits() {
    let decoded: SkipDefault = from_bytes(&hex_literal::hex!("a0")).unwrap();
    assert_eq!(decoded.skipped, 9);
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
struct TaggedIndefinite {
    #[cbor(tag = 5, indefinite)]
    items: Vec<u8>,
}

#[test]
fn tag_and_indefinite_compose() {
    let value = TaggedIndefinite { items: vec![1, 2] };
    assert_eq!(
        value.cbor_serialize().unwrap(),
        hex_literal::hex!("a1656974656d73c59f0102ff")
    );
    assert_eq!(
        from_bytes::<TaggedIndefinite>(&hex_literal::hex!("a1656974656d73c59f0102ff")).unwrap(),
        value
    );
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
struct ExplicitKeys {
    #[cbor(key = "z")]
    text: u8,
    #[cbor(key = -1)]
    negative: u8,
}

#[test]
fn explicit_text_and_negative_integer_keys() {
    let value = ExplicitKeys {
        text: 1,
        negative: 2,
    };
    assert_eq!(
        value.cbor_serialize().unwrap(),
        hex_literal::hex!("a2617a012002")
    );
    assert_eq!(
        from_bytes::<ExplicitKeys>(&hex_literal::hex!("a2617a012002")).unwrap(),
        value
    );
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(array)]
struct ArrayFields {
    #[cbor(tag = 1)]
    a: u8,
    #[cbor(simple_value)]
    b: u8,
    #[cbor(indefinite)]
    c: Vec<u8>,
}

#[test]
fn field_attributes_in_array_form() {
    let value = ArrayFields {
        a: 1,
        b: 2,
        c: vec![3],
    };
    assert_eq!(
        value.cbor_serialize().unwrap(),
        hex_literal::hex!("83c101e29f03ff")
    );
    assert_eq!(
        from_bytes::<ArrayFields>(&hex_literal::hex!("83c101e29f03ff")).unwrap(),
        value
    );
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
struct IndefiniteSet {
    #[cbor(indefinite)]
    items: std::collections::BTreeSet<u8>,
}

#[test]
fn indefinite_set_round_trip() {
    let value = IndefiniteSet {
        items: [1, 2].into_iter().collect(),
    };
    assert_eq!(
        value.cbor_serialize().unwrap(),
        hex_literal::hex!("a1656974656d739f0102ff")
    );
    assert_eq!(
        from_bytes::<IndefiniteSet>(&hex_literal::hex!("a1656974656d739f0102ff")).unwrap(),
        value
    );
}
