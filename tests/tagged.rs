//! `Tagged<'a, TAG, V>` and `Value::Tagged` share one path: the const generic is read back from
//! the type name, the tag travels through a thread-local, `DYN_TAGGED_TYP_NAME` is the sentinel.
//! Nothing else reached tags except through `Value`, so this was uncovered.
use seabored::serde::Tagged;

const MY_TAG: u64 = 1;

#[test]
fn typed_tag_wrapper_round_trips() {
    let bytes = [0xc1, 0x18, 0x2a]; // tag 1 around 42
    let expected = Tagged::<'_, { MY_TAG }, u8>::from(42u8);
    assert_eq!(seabored::serde::to_vec(&expected).unwrap(), bytes);

    let decoded: Tagged<'_, { MY_TAG }, u8> = seabored::serde::from_slice(&bytes).unwrap();
    assert_eq!(decoded, expected);
}

#[test]
fn dynamic_tag_agrees_with_the_typed_one() {
    let bytes = [0xc1, 0x18, 0x2a];
    let dynamic: seabored::Value = seabored::serde::from_slice(&bytes).unwrap();
    assert_eq!(seabored::serde::to_vec(&dynamic).unwrap(), bytes);
    assert_eq!(
        dynamic,
        seabored::de::from_bytes::<seabored::Value>(&bytes).unwrap()
    );
}

#[test]
fn unparsable_tagged_names_error_instead_of_panicking() {
    use serde::{Deserialize, Serialize};

    const MALFORMED: &str = "seabored::serde::Tagged<'_, nope, u8>";

    struct ForgedSer;
    impl Serialize for ForgedSer {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_newtype_struct(MALFORMED, &1u8)
        }
    }
    assert!(seabored::serde::to_vec(&ForgedSer).is_err());

    struct ForgedDe;
    struct AnyVisitor;
    impl<'de> serde::de::Visitor<'de> for AnyVisitor {
        type Value = ForgedDe;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("anything")
        }
        fn visit_u8<E: serde::de::Error>(self, _: u8) -> Result<Self::Value, E> {
            Ok(ForgedDe)
        }
    }
    impl<'de> Deserialize<'de> for ForgedDe {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            d.deserialize_newtype_struct(MALFORMED, AnyVisitor)
        }
    }
    assert!(seabored::serde::from_slice::<ForgedDe>(&[0x01]).is_err());

    // A foreign newtype name still goes through untouched (no tag written or expected)
    struct ForeignSer;
    impl Serialize for ForeignSer {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_newtype_struct("other::Thing", &42u8)
        }
    }
    assert_eq!(seabored::serde::to_vec(&ForeignSer).unwrap(), [0x18, 0x2a]);
}
