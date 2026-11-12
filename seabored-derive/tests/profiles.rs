use seabored::de::from_bytes;
use seabored::error::{SeaboredDeError, SeaboredSerError};
use seabored::ser::CborSerialize as _;
use seabored::{Profile, Value};

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(profile = "deterministic")]
struct Sorted {
    #[cbor(key = "z")]
    z: u8,
    #[cbor(key = "a")]
    a: u8,
    #[cbor(key = 1)]
    one: u8,
    #[cbor(key = -1)]
    neg: u8,
}

#[test]
fn deterministic_encoding_sorts_by_encoded_key_bytes() {
    let value = Sorted {
        z: 4,
        a: 3,
        one: 1,
        neg: 2,
    };
    assert_eq!(
        value.cbor_serialize().unwrap(),
        hex_literal::hex!("a401012002616103617a04")
    );
    assert_eq!(
        from_bytes::<Sorted>(&hex_literal::hex!("a401012002616103617a04")).unwrap(),
        value
    );
}

#[test]
fn deterministic_rejects_order_unknowns_and_indefinite_maps() {
    assert!(matches!(
        from_bytes::<Sorted>(&hex_literal::hex!("a420020101616103617a04")),
        Err(SeaboredDeError::NonPreferredEncoding {
            what: "map key order"
        })
    ));
    assert!(matches!(
        from_bytes::<Sorted>(&hex_literal::hex!(
            "a501012002616103617a04617805"
        )),
        Err(SeaboredDeError::UnexpectedKey { key }) if key == "x"
    ));
    assert!(matches!(
        from_bytes::<Sorted>(&hex_literal::hex!("bf01012002616103617a04ff")),
        Err(SeaboredDeError::NonPreferredEncoding { what: "map header" })
    ));
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(profile = "deterministic")]
struct DeterministicDefault {
    a: u8,
    #[cbor(default)]
    b: u8,
}

#[test]
fn deterministic_does_not_fill_defaults() {
    assert!(matches!(
        from_bytes::<DeterministicDefault>(&hex_literal::hex!("a1616101")),
        Err(SeaboredDeError::MissingField { name: "b" })
    ));
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(profile = "pref-plus")]
struct PrefPlus {
    a: u8,
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(profile = "pref-plus")]
struct PrefPlusInt {
    #[cbor(key = 1)]
    a: u8,
}

#[test]
fn pref_plus_rejects_non_preferred_headers_and_keys() {
    assert_eq!(
        from_bytes::<PrefPlus>(&hex_literal::hex!("a1616101")).unwrap(),
        PrefPlus { a: 1 }
    );
    assert!(matches!(
        from_bytes::<PrefPlus>(&hex_literal::hex!("b801616101")),
        Err(SeaboredDeError::NonPreferredEncoding { what: "map header" })
    ));
    assert!(matches!(
        from_bytes::<PrefPlus>(&hex_literal::hex!("a178016101")),
        Err(SeaboredDeError::NonPreferredEncoding { what: "map key" })
    ));
    assert!(matches!(
        from_bytes::<PrefPlusInt>(&hex_literal::hex!("a1180101")),
        Err(SeaboredDeError::NonPreferredEncoding { what: "map key" })
    ));
}

#[test]
fn pref_plus_rejects_indefinite_maps_and_checks_skipped_values() {
    assert!(matches!(
        from_bytes::<PrefPlus>(&hex_literal::hex!("bf616101ff")),
        Err(SeaboredDeError::NonPreferredEncoding { what: "map header" })
    ));
    assert!(matches!(
        from_bytes::<PrefPlus>(&hex_literal::hex!("a261610161781801")),
        Err(SeaboredDeError::NonPreferredEncoding { what: "argument" })
    ));
}

#[test]
fn pref_plus_checks_bignums_in_skipped_fields() {
    let mut wire = hex_literal::hex!("a26161016162").to_vec();
    for payload in [
        &hex_literal::hex!("c240")[..],     // empty
        &hex_literal::hex!("c2420003")[..], // leading zero
        &hex_literal::hex!("c24103")[..],   // fits major type 0
        &hex_literal::hex!("c34101")[..],   // tag 3 fits major type 1
        &hex_literal::hex!("c201")[..],     // payload is not a byte string
        &hex_literal::hex!("81c24103")[..], // nested in a skipped array
    ] {
        let mut bytes = wire.clone();
        bytes.extend_from_slice(payload);
        assert!(
            matches!(
                from_bytes::<PrefPlus>(&bytes),
                Err(SeaboredDeError::NonPreferredEncoding { .. })
            ),
            "accepted {payload:02x?}"
        );
    }
    wire.extend_from_slice(&hex_literal::hex!("c249010000000000000000"));
    assert_eq!(from_bytes::<PrefPlus>(&wire).unwrap(), PrefPlus { a: 1 });
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(tag = 5, profile = "pref-plus")]
struct PrefPlusTagged {
    a: u8,
}

#[test]
fn pref_plus_checks_tag_widths() {
    assert_eq!(
        from_bytes::<PrefPlusTagged>(&hex_literal::hex!("c5a1616101")).unwrap(),
        PrefPlusTagged { a: 1 }
    );
    assert!(matches!(
        from_bytes::<PrefPlusTagged>(&hex_literal::hex!("d805a1616101")),
        Err(SeaboredDeError::NonPreferredEncoding { what: "tag" })
    ));
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(array, profile = "deterministic")]
struct DeterministicArray {
    a: u8,
    b: u8,
}

#[test]
fn deterministic_rejects_indefinite_arrays() {
    assert!(matches!(
        from_bytes::<DeterministicArray>(&hex_literal::hex!("9f0102ff")),
        Err(SeaboredDeError::NonPreferredEncoding {
            what: "array header"
        })
    ));
    assert_eq!(
        from_bytes::<DeterministicArray>(&hex_literal::hex!("820102")).unwrap(),
        DeterministicArray { a: 1, b: 2 }
    );
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(profile = "deterministic")]
enum DeterministicEnum {
    Value {
        #[cbor(key = "z")]
        z: u8,
        #[cbor(key = "a")]
        a: u8,
    },
}

#[test]
fn deterministic_struct_variant_sorts_fields() {
    let value = DeterministicEnum::Value { z: 2, a: 1 };
    let encoded = value.cbor_serialize().unwrap();
    assert_eq!(encoded, hex_literal::hex!("a16556616c7565a2616101617a02"));
    assert_eq!(from_bytes::<DeterministicEnum>(&encoded).unwrap(), value);
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(profile = "pref-plus")]
enum PrefPlusEnum {
    #[cbor(key = 1)]
    Unit,
    #[cbor(key = 2)]
    Value(u8),
}

#[test]
fn pref_plus_checks_enum_headers_and_keys() {
    assert_eq!(
        from_bytes::<PrefPlusEnum>(&hex_literal::hex!("01")).unwrap(),
        PrefPlusEnum::Unit
    );
    assert_eq!(
        from_bytes::<PrefPlusEnum>(&hex_literal::hex!("a10201")).unwrap(),
        PrefPlusEnum::Value(1)
    );
    assert!(matches!(
        from_bytes::<PrefPlusEnum>(&hex_literal::hex!("1801")),
        Err(SeaboredDeError::NonPreferredEncoding {
            what: "variant key"
        })
    ));
    assert!(matches!(
        from_bytes::<PrefPlusEnum>(&hex_literal::hex!("a1180201")),
        Err(SeaboredDeError::NonPreferredEncoding {
            what: "variant key"
        })
    ));
    assert!(matches!(
        from_bytes::<PrefPlusEnum>(&hex_literal::hex!("b8010201")),
        Err(SeaboredDeError::NonPreferredEncoding { what: "map header" })
    ));
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(profile = "rfc")]
struct RuntimeProfile {
    #[cbor(key = "z")]
    z: u8,
    #[cbor(key = "a")]
    a: u8,
}

#[test]
fn runtime_profile_overrides_the_attribute() {
    let value = RuntimeProfile { z: 2, a: 1 };

    assert_eq!(
        value.cbor_serialize().unwrap(),
        hex_literal::hex!("a2617a02616101")
    );
    assert_eq!(
        value
            .cbor_serialize_with_profile(Profile::Deterministic)
            .unwrap(),
        hex_literal::hex!("a2616101617a02")
    );
}

#[derive(Debug, PartialEq, seabored::CborSerialize)]
#[cbor(profile = "rfc")]
struct RuntimeIndefinite {
    #[cbor(indefinite)]
    items: Vec<u8>,
}

#[test]
fn runtime_strict_profile_rejects_an_indefinite_field() {
    let value = RuntimeIndefinite { items: vec![1, 2] };
    assert!(matches!(
        value.cbor_serialize_with_profile(Profile::PreferredPlus),
        Err(SeaboredSerError::IndefiniteForbidden { .. })
    ));
}

#[derive(Debug, PartialEq, seabored::CborSerialize)]
#[cbor(profile = "rfc")]
struct RuntimeInner {
    #[cbor(key = "z")]
    z: u8,
    #[cbor(key = "a")]
    a: u8,
}

#[derive(Debug, PartialEq, seabored::CborSerialize)]
#[cbor(profile = "rfc")]
struct RuntimeOuter {
    inner: RuntimeInner,
}

#[test]
fn runtime_profile_reaches_nested_derived_values() {
    let value = RuntimeOuter {
        inner: RuntimeInner { z: 2, a: 1 },
    };

    assert_eq!(
        value.cbor_serialize().unwrap(),
        hex_literal::hex!("a165696e6e6572a2617a02616101")
    );
    assert_eq!(
        value
            .cbor_serialize_with_profile(Profile::Deterministic)
            .unwrap(),
        hex_literal::hex!("a165696e6e6572a2616101617a02")
    );
}

mod profiled_with {
    use seabored::{
        Profile, Value, error::SeaboredSerError, reexports::parsio::Write, ser::CborSerialize as _,
    };

    pub fn cbor_serialize_to_with_profile<W: Write>(
        value: &Value<'_>,
        writer: &mut W,
        profile: Profile,
    ) -> Result<usize, SeaboredSerError> {
        value.cbor_serialize_to_with_profile(writer, profile)
    }
}

#[derive(Debug, PartialEq, seabored::CborSerialize)]
#[cbor(profile = "deterministic")]
struct ProfiledWith<'a> {
    #[cbor(with = profiled_with)]
    map: Value<'a>,
}

#[test]
fn with_module_receives_the_parent_profile() {
    let map: Value<'_> = from_bytes(&hex_literal::hex!("a2016178006179")).unwrap();
    let value = ProfiledWith { map };

    assert_eq!(
        value.cbor_serialize().unwrap(),
        hex_literal::hex!("a1636d6170a2006179016178")
    );
    assert_eq!(
        value
            .cbor_serialize_with_profile(Profile::PreferredPlus)
            .unwrap(),
        hex_literal::hex!("a1636d6170a2016178006179")
    );
}
