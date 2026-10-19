#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

use crate::mt::MajorType;

pub mod de;
pub mod ib;
pub mod io;
pub mod mt;
pub mod ser;
#[cfg(feature = "serde")]
pub mod serde;
pub mod types;

pub mod error;

// TODO: Homebrew a derive for CBOR specifics (like p+ or dcbor profiles, tags, simple values etc)
#[cfg(feature = "derive")]
mod derive {}

pub mod reexports {
    pub use half;
}

/// A decoded CBOR item. Strings and bytes borrow when the input can lend, so `Value` is never
/// `DeserializeOwned` (`'de: 'a` in the serde impl); [`OwnedValue`] is for readers that cannot lend.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Value<'a> {
    Integer(types::CborInteger),
    Float(types::CborFloat),
    Bytes(std::borrow::Cow<'a, [u8]>),
    String(std::borrow::Cow<'a, str>),
    /// An array. [`FromIterator`] builds definite-length; a decoded indefinite one keeps its form
    /// (clear [`CborSequence::is_indefinite`](types::CborSequence::is_indefinite) to re-encode definite).
    Sequence(types::CborSequence<Value<'a>>),
    /// A map, with the same caveat as [`Value::Sequence`].
    Map(types::CborSequence<(Value<'a>, Value<'a>)>),
    Tagged((types::CborIntegerValue, Box<Value<'a>>)),
    SimpleValue(u8),
    Bool(bool),
    Null,
    #[default]
    Undefined,
}

impl Value<'_> {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    pub fn mt(&self) -> MajorType {
        match self {
            Value::Integer(cbor_int) => MajorType::from(cbor_int),
            Value::Bytes(_) => MajorType::Bytes,
            Value::String(_) => MajorType::String,
            Value::Sequence(seq) => seq.mt,
            Value::Map(_) => MajorType::Map,
            Value::Tagged(_) => MajorType::Tagged,
            _ => MajorType::SimpleValueOrFloat,
        }
    }

    pub fn as_integer(&self) -> Option<&types::CborInteger> {
        match self {
            Self::Integer(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<&types::CborFloat> {
        match self {
            Self::Float(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_simple_value(&self) -> Option<u8> {
        match self {
            Self::SimpleValue(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_sequence(&self) -> Option<&types::CborSequence<Self>> {
        match self {
            Self::Sequence(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&types::CborSequence<(Self, Self)>> {
        match self {
            Self::Map(v) => Some(v),
            _ => None,
        }
    }
}

/// Builds a definite-length array (profile `draft-ietf-cbor-serialization-06` §4.1 item 2).
/// Indefinite: `Value::Sequence(CborSequence::new_indefinite(MajorType::Array))`.
impl<'a> FromIterator<Value<'a>> for Value<'a> {
    #[inline(always)]
    fn from_iter<T: IntoIterator<Item = Value<'a>>>(iter: T) -> Self {
        let iter = iter.into_iter();
        // The hint is capacity only: the header is written from the final item count
        let mut seq = types::CborSequence::new_finite(MajorType::Array, iter.size_hint().0);
        seq.extend(iter);
        Self::Sequence(seq)
    }
}

/// Builds a definite-length map, like the array impl above.
impl<'a> FromIterator<(Value<'a>, Value<'a>)> for Value<'a> {
    #[inline(always)]
    fn from_iter<T: IntoIterator<Item = (Value<'a>, Value<'a>)>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let mut seq = types::CborSequence::new_finite(MajorType::Map, iter.size_hint().0);
        seq.extend(iter);
        Self::Map(seq)
    }
}

#[cfg(feature = "serde")]
impl ::serde_core::Serialize for Value<'_> {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde_core::Serializer,
    {
        match self {
            Value::Integer(cbor_integer) => {
                if cbor_integer.negative {
                    match i64::try_from(cbor_integer) {
                        Ok(value) => serializer.serialize_i64(value),
                        // Below i64::MIN only i128 can carry it: major type 1 holds down to -2^64
                        Err(_) => serializer.serialize_i128(-(u64::from(cbor_integer) as i128) - 1),
                    }
                } else {
                    serializer.serialize_u64(cbor_integer.into())
                }
            }
            Value::Float(cbor_float) => serializer.serialize_f64(**cbor_float),
            Value::Bytes(cow) => serializer.serialize_bytes(cow),
            Value::String(cow) => serializer.serialize_str(cow),
            Value::Sequence(cbor_sequence) => {
                use ::serde_core::ser::SerializeSeq;

                let mut seq = serializer.serialize_seq(Some(cbor_sequence.len()))?;
                for value in cbor_sequence.iter() {
                    seq.serialize_element(value)?;
                }
                seq.end()
            }
            Value::Map(cbor_sequence) => {
                use ::serde_core::ser::SerializeMap as _;

                let mut seqmap = serializer.serialize_map(Some(cbor_sequence.len()))?;
                for (k, v) in cbor_sequence.iter() {
                    seqmap.serialize_entry(k, v)?;
                }

                seqmap.end()
            }
            Value::Tagged((tag, value)) => {
                crate::serde::serialize_tagged(serializer, (*tag).into(), value.as_ref())
            }
            Value::SimpleValue(v) => crate::serde::serialize_simple_value(serializer, *v),
            Value::Bool(v) => serializer.serialize_bool(*v),
            Value::Null => serializer.serialize_none(),
            Value::Undefined => serializer.serialize_unit(),
        }
    }
}

#[cfg(feature = "serde")]
impl<'a, 'de: 'a> ::serde_core::Deserialize<'de> for Value<'a> {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde_core::Deserializer<'de>,
    {
        #[derive(Default)]
        struct ValueVisitor<'a>(std::marker::PhantomData<&'a ()>);

        impl<'a, 'de: 'a> ::serde_core::de::Visitor<'de> for ValueVisitor<'a> {
            type Value = Value<'de>;

            #[inline(always)]
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "A CBOR Value")
            }

            #[inline(always)]
            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: ::serde_core::de::Error,
            {
                Ok(Value::Bool(v))
            }

            #[inline(always)]
            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: ::serde_core::de::Error,
            {
                Ok(Value::Integer(v.into()))
            }

            #[inline(always)]
            fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
            where
                E: ::serde_core::de::Error,
            {
                if let Ok(value) = i64::try_from(v) {
                    return self.visit_i64(value);
                }
                // Only CBOR's major type 1 range can be represented: -2^64 <= v < -1
                match u64::try_from(-(v + 1)) {
                    Ok(magnitude) => Ok(Value::Integer(types::CborInteger {
                        value: magnitude.into(),
                        negative: true,
                    })),
                    Err(_) => Err(E::custom("integer below CBOR's major type 1 range")),
                }
            }

            #[inline(always)]
            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: ::serde_core::de::Error,
            {
                Ok(Value::Integer(v.into()))
            }

            #[inline(always)]
            fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
            where
                E: ::serde_core::de::Error,
            {
                Ok(Value::Float(v.into()))
            }

            #[inline(always)]
            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: ::serde_core::de::Error,
            {
                Ok(Value::Float(v.into()))
            }

            #[inline(always)]
            fn visit_char<E>(self, v: char) -> Result<Self::Value, E>
            where
                E: ::serde_core::de::Error,
            {
                self.visit_str(v.encode_utf8(&mut [0u8; 4]))
            }

            #[inline(always)]
            fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
            where
                E: ::serde_core::de::Error,
            {
                Ok(Value::String(std::borrow::Cow::Borrowed(v)))
            }

            #[inline(always)]
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: ::serde_core::de::Error,
            {
                Ok(Value::String(std::borrow::Cow::Owned(v.to_owned())))
            }

            #[inline(always)]
            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: ::serde_core::de::Error,
            {
                Ok(Value::String(std::borrow::Cow::Owned(v)))
            }

            #[inline(always)]
            fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
            where
                E: ::serde_core::de::Error,
            {
                Ok(Value::Bytes(std::borrow::Cow::Borrowed(v)))
            }

            #[inline(always)]
            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: ::serde_core::de::Error,
            {
                Ok(Value::Bytes(std::borrow::Cow::Owned(v.to_vec())))
            }

            #[inline(always)]
            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
            where
                E: ::serde_core::de::Error,
            {
                Ok(Value::Bytes(v.into()))
            }

            #[inline(always)]
            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: ::serde_core::de::Error,
            {
                Ok(Value::Null)
            }

            #[inline(always)]
            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: ::serde_core::Deserializer<'de>,
            {
                deserializer.deserialize_any(self)
            }

            #[inline(always)]
            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: ::serde_core::de::Error,
            {
                Ok(Value::Undefined)
            }

            #[inline(always)]
            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: ::serde_core::Deserializer<'de>,
            {
                /// The name is hidden, so the payload decides: `visit_u8` is a simple value,
                /// `visit_newtype_struct` a tag the accessor already put aside.
                struct SentinelVisitor;

                impl<'de> ::serde_core::de::Visitor<'de> for SentinelVisitor {
                    type Value = Value<'de>;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        write!(formatter, "a CBOR simple value or a tagged CBOR Value")
                    }

                    #[inline(always)]
                    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
                    where
                        E: ::serde_core::de::Error,
                    {
                        Ok(Value::SimpleValue(v))
                    }

                    #[inline(always)]
                    fn visit_newtype_struct<D>(
                        self,
                        deserializer: D,
                    ) -> Result<Self::Value, D::Error>
                    where
                        D: ::serde_core::Deserializer<'de>,
                    {
                        // The tag came through the thread-local; missing means a forged sentinel
                        let tag = crate::serde::TAG.get().ok_or_else(|| {
                            ::serde_core::de::Error::custom("tagged CBOR Value without a tag")
                        })?;

                        Ok(Value::Tagged((
                            tag.into(),
                            Box::new(<Value<'de> as ::serde_core::Deserialize<'de>>::deserialize(
                                deserializer,
                            )?),
                        )))
                    }
                }

                deserializer.deserialize_any(SentinelVisitor)
            }

            #[inline(always)]
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: ::serde_core::de::SeqAccess<'de>,
            {
                // `0` when unknown: `Vec`'s first push allocates 4, so an empty container costs
                // nothing (RFC 8949 §10; same idea as the hint).
                let mut arr = Vec::with_capacity(seq.size_hint().unwrap_or(0).min(256));

                while let Some(elem) = seq.next_element()? {
                    arr.push(elem);
                }

                Ok(Value::Sequence(arr.into()))
            }

            #[inline(always)]
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: ::serde_core::de::MapAccess<'de>,
            {
                // `Vec::into()` would say `Array`; a map's sequence carries the map major type
                let mut vmap = types::CborSequence::new_finite(
                    crate::mt::MajorType::Map,
                    map.size_hint().unwrap_or(0).min(256),
                );

                while let Some((k, v)) = map.next_entry()? {
                    vmap.push((k, v));
                }

                Ok(Value::Map(vmap))
            }
        }

        deserializer.deserialize_any(ValueVisitor::default())
    }
}

impl<'a> Value<'a> {
    /// Drops the borrow. Shape survives (major type, definiteness), so re-serialising gives the
    /// same bytes.
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    pub fn into_owned(self) -> Value<'static> {
        match self {
            Value::Integer(value) => Value::Integer(value),
            Value::Float(value) => Value::Float(value),
            Value::Bytes(bytes) => Value::Bytes(std::borrow::Cow::Owned(bytes.into_owned())),
            Value::String(string) => Value::String(std::borrow::Cow::Owned(string.into_owned())),
            Value::Sequence(seq) => Value::Sequence(map_sequence(seq, Value::into_owned)),
            Value::Map(map) => {
                Value::Map(map_sequence(map, |(k, v)| (k.into_owned(), v.into_owned())))
            }
            Value::Tagged((tag, value)) => Value::Tagged((tag, Box::new(value.into_owned()))),
            Value::SimpleValue(value) => Value::SimpleValue(value),
            Value::Bool(value) => Value::Bool(value),
            Value::Null => Value::Null,
            Value::Undefined => Value::Undefined,
        }
    }
}

/// Converts the elements of a sequence, keeping its major type and its definiteness
fn map_sequence<T: crate::ser::CborSerialize, U: crate::ser::CborSerialize>(
    seq: types::CborSequence<T>,
    mut convert: impl FnMut(T) -> U,
) -> types::CborSequence<U> {
    let mut converted = if seq.is_indefinite {
        types::CborSequence::new_indefinite(seq.mt)
    } else {
        types::CborSequence::new_finite(seq.mt, seq.len())
    };
    converted.extend(seq.into_iter().map(&mut convert));
    converted
}

/// A [`Value`] that owns its contents, for decoders that cannot lend bytes (`ciborium`, `cbor4ii`,
/// anything streaming); `Value` itself is never `DeserializeOwned`.
///
/// ```
/// use seabored::{OwnedValue, Value};
///
/// let value: Value<'static> = seabored::de::from_bytes(&[0x61, 0x61]).unwrap();
/// let owned = OwnedValue::from(value);
/// assert_eq!(*owned, Value::String("a".into()));
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OwnedValue(pub Value<'static>);

impl std::ops::Deref for OwnedValue {
    type Target = Value<'static>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Value<'static>> for OwnedValue {
    #[inline(always)]
    fn from(value: Value<'static>) -> Self {
        Self(value)
    }
}

impl From<OwnedValue> for Value<'static> {
    #[inline(always)]
    fn from(value: OwnedValue) -> Self {
        value.0
    }
}

#[cfg(feature = "serde")]
impl ::serde_core::Serialize for OwnedValue {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde_core::Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> ::serde_core::Deserialize<'de> for OwnedValue {
    /// Deserializes a [`Value`] then owns it — this is how the impl can be `for<'de>` where
    /// `Value`'s cannot: the borrow is dropped inside the impl, not demanded of every caller.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde_core::Deserializer<'de>,
    {
        Ok(Self(
            <Value<'de> as ::serde_core::Deserialize<'de>>::deserialize(deserializer)?.into_owned(),
        ))
    }
}
