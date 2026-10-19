//! `Value` accepts strings and bytes from decoders that cannot lend input.
//!
//! serde's default `visit_str`/`visit_bytes` reject them, so any format building a string in a
//! scratch buffer failed. `visit_char` forwards to `visit_str`, so it failed too.

use seabored::Value;
use serde::Deserialize as _;
use serde::de::value::{
    BorrowedStrDeserializer, BytesDeserializer, CharDeserializer, Error, StrDeserializer,
};

#[test]
fn value_takes_ephemeral_strings_and_bytes() {
    assert_eq!(
        Value::deserialize(StrDeserializer::<Error>::new("hi")).unwrap(),
        Value::String("hi".into())
    );
    assert_eq!(
        Value::deserialize(CharDeserializer::<Error>::new('x')).unwrap(),
        Value::String("x".into())
    );
    assert_eq!(
        Value::deserialize(BytesDeserializer::<Error>::new(&[1u8, 2, 3][..])).unwrap(),
        Value::Bytes(vec![1u8, 2, 3].into())
    );
}

#[test]
fn borrowed_input_is_still_zero_copy() {
    assert!(matches!(
        Value::deserialize(BorrowedStrDeserializer::<Error>::new("hi")).unwrap(),
        Value::String(std::borrow::Cow::Borrowed(_))
    ));
}
