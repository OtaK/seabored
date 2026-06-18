use std::borrow::Cow;

use crate::{error::SeaboredSerError, io::Write, ser::CborSerialize};

/// A raw, pre-encoded CBOR value backed by a byte buffer
///
/// The bytes are assumed to be valid CBOR
///
/// When serialized via [`CborSerialize`], the bytes are written verbatim into
/// the output stream without any additional framing
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RawValue<'a>(Cow<'a, [u8]>);

impl<'a> RawValue<'a> {
    #[cfg(feature = "serde")]
    pub(crate) const TYP_NAME: &'static str = "seabored::types::RawValue";

    /// Creates a `RawValue` from owned raw bytes without verifying that they are valid CBOR
    #[inline(always)]
    pub fn from_bytes_unchecked(bytes: Vec<u8>) -> Self {
        Self(Cow::Owned(bytes))
    }

    /// Creates a `RawValue` from raw borrowed bytes without verifying that they are valid CBOR
    #[inline(always)]
    pub fn from_slice_unchecked(bytes: &'a [u8]) -> Self {
        Self(Cow::Borrowed(bytes))
    }

    /// Serializes `value` into CBOR bytes and stores them inside a `RawValue`
    #[cfg(feature = "serde")]
    #[inline]
    pub fn from_serialize<T: ::serde::Serialize>(value: &T) -> Result<Self, SeaboredSerError> {
        crate::serde::to_vec(value).map(Self::from_bytes_unchecked)
    }

    /// Returns a reference to the raw CBOR bytes
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Consumes this `RawValue` and returns the underlying bytes
    #[inline(always)]
    pub fn into_inner(self) -> Cow<'a, [u8]> {
        self.0
    }

    /// Parses the raw CBOR bytes into a value of type `T`
    #[cfg(feature = "serde")]
    #[inline]
    pub fn parse<'de, T: ::serde::Deserialize<'a>>(
        &'de self,
    ) -> Result<T, crate::error::SeaboredDeError<'de>>
    where
        'de: 'a,
    {
        crate::serde::from_slice(&self.0)
    }
}

impl CborSerialize for RawValue<'_> {
    /// Writes the raw CBOR bytes verbatim into `writer`
    #[inline(always)]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        writer.write(&self.0)
    }
}

impl AsRef<[u8]> for RawValue<'_> {
    #[inline(always)]
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Serializes the raw CBOR bytes verbatim into the CBOR stream.
///
/// The seabored CBOR serializer recognises the newtype-struct sentinel name and
/// writes the bytes directly without any additional framing. Other serializers
/// will see this as an opaque newtype wrapping `[u8]` and serialize accordingly
#[cfg(feature = "serde")]
impl ::serde::Serialize for RawValue<'_> {
    #[inline(always)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        serializer.serialize_newtype_struct(Self::TYP_NAME, self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Value, types::RawValue};
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;

    #[test]
    fn raw_value_hashmap_roundtrip() {
        let mut map: HashMap<&'static str, RawValue> = HashMap::new();

        map.insert("int_key", RawValue::from_serialize(&42u64).unwrap());
        map.insert("neg_key", RawValue::from_serialize(&-7i64).unwrap());
        map.insert("bool_key", RawValue::from_serialize(&true).unwrap());
        map.insert("str_key", RawValue::from_serialize(&"hello CBOR").unwrap());
        map.insert(
            "bytes_key",
            RawValue::from_serialize(&serde_bytes::ByteBuf::from(vec![0xDE, 0xAD, 0xBE, 0xEF]))
                .unwrap(),
        );
        map.insert(
            "null_key",
            RawValue::from_serialize(&Option::<u8>::None).unwrap(),
        );
        map.insert("arr_key", RawValue::from_serialize(&[1u32, 2, 3]).unwrap());

        let bytes = crate::serde::to_vec(&map).unwrap();
        let decoded: Value<'_> = crate::serde::from_slice(&bytes).unwrap();
        let Value::Map(seq) = decoded else {
            panic!("Expected map");
        };
        let seq = seq.to_vec();

        let map: HashMap<_, _> = seq
            .into_iter()
            .map(|(k, v)| match k {
                Value::String(k) => (k.into_owned(), v),
                _ => panic!("Expected string keys got {k:?}"),
            })
            .collect();

        assert_eq!(map["int_key"], Value::Integer(42u64.into()),);
        assert_eq!(map["neg_key"], Value::Integer((-7i64).into()),);
        assert_eq!(map["bool_key"], Value::Bool(true));
        assert_eq!(map["str_key"], Value::String("hello CBOR".into()),);
        assert_eq!(
            map["bytes_key"],
            Value::Bytes(vec![0xDE, 0xAD, 0xBE, 0xEF].into()),
        );
        assert_eq!(map["null_key"], Value::Null);

        let expected_arr = Value::Sequence(
            vec![
                Value::Integer(1u8.into()),
                Value::Integer(2u8.into()),
                Value::Integer(3u8.into()),
            ]
            .into(),
        );
        assert_eq!(map["arr_key"], expected_arr);
    }

    #[test]
    fn raw_value_ref_from_serialize_roundtrip() {
        let raw = RawValue::from_serialize(&99u64).unwrap();

        let parsed: u64 = raw.parse().unwrap();
        assert_eq!(parsed, 99u64);

        let out = raw.cbor_serialize().unwrap();
        assert_eq!(out, raw.as_bytes());
    }

    #[test]
    fn raw_value_unchecked_passthrough() {
        let raw = RawValue::from_bytes_unchecked(vec![0x01]);
        assert_eq!(raw.cbor_serialize().unwrap(), &[0x01]);

        let bytes = [0x01u8];
        let raw_ref = RawValue::from_slice_unchecked(&bytes);
        assert_eq!(raw_ref.cbor_serialize().unwrap(), &[0x01]);
    }
}
