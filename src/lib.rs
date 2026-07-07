#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

#[cfg(feature = "facet")]
compile_error!("The `facet` feature isn't ready for prime-time yet");

use crate::mt::MajorType;

#[cfg(any(feature = "serde", feature = "facet"))]
pub mod adapters;
pub mod de;
pub mod ib;
pub mod io;
pub mod mt;
pub mod ser;
pub mod types;

pub mod error;

// Compat to prevent breaking changes
#[cfg(feature = "serde")]
pub use crate::adapters::serde;

// TODO: Homebrew a derive for CBOR specifics (like p+ or dcbor profiles, tags, simple values etc)
#[cfg(feature = "derive")]
mod derive {}

pub mod reexports {
    pub use half;
}

#[derive(Debug, Clone)]
pub(crate) enum SyntacticValue<'a> {
    Value(Value<'a>),
    Break,
}

impl<'a> From<Value<'a>> for SyntacticValue<'a> {
    fn from(value: Value<'a>) -> Self {
        SyntacticValue::Value(value)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum Value<'a> {
    Integer(types::CborInteger),
    Float(types::CborFloat),
    Bytes(std::borrow::Cow<'a, [u8]>),
    String(std::borrow::Cow<'a, str>),
    Sequence(types::CborSequence<Value<'a>>),
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
    pub(crate) fn mt(&self) -> MajorType {
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

/// Builds an indefinite length Array
impl<'a> FromIterator<Value<'a>> for Value<'a> {
    #[inline(always)]
    fn from_iter<T: IntoIterator<Item = Value<'a>>>(iter: T) -> Self {
        let mut seq = types::CborSequence::new_indefinite(MajorType::Array);
        seq.extend(iter);
        Self::Sequence(seq)
    }
}

/// Builds an indefinite length Map
impl<'a> FromIterator<(Value<'a>, Value<'a>)> for Value<'a> {
    #[inline(always)]
    fn from_iter<T: IntoIterator<Item = (Value<'a>, Value<'a>)>>(iter: T) -> Self {
        let mut seq = types::CborSequence::new_indefinite(MajorType::Map);
        seq.extend(iter);
        Self::Map(seq)
    }
}

#[cfg(feature = "serde")]
impl ::serde::Serialize for Value<'_> {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        match self {
            Value::Integer(cbor_integer) => {
                if cbor_integer.negative {
                    serializer.serialize_i64(cbor_integer.try_into().unwrap())
                } else {
                    serializer.serialize_u64(cbor_integer.into())
                }
            }
            Value::Float(cbor_float) => serializer.serialize_f64(**cbor_float),
            Value::Bytes(cow) => serializer.serialize_bytes(cow),
            Value::String(cow) => serializer.serialize_str(cow),
            Value::Sequence(cbor_sequence) => {
                use ::serde::ser::SerializeSeq;

                let mut seq = serializer.serialize_seq(Some(cbor_sequence.len()))?;
                for value in cbor_sequence.iter() {
                    seq.serialize_element(value)?;
                }
                seq.end()
            }
            Value::Map(cbor_sequence) => {
                use ::serde::ser::SerializeMap as _;

                let mut seqmap = serializer.serialize_map(Some(cbor_sequence.len()))?;
                for (k, v) in cbor_sequence.iter() {
                    seqmap.serialize_entry(k, v)?;
                }

                seqmap.end()
            }
            Value::Tagged((tag, value)) => serializer.serialize_newtype_struct(
                crate::adapters::DYN_TAGGED_TYP_NAME,
                &crate::adapters::DynamicTaggedValue {
                    tag: *tag,
                    value: std::borrow::Cow::Borrowed(value.as_ref()),
                },
            ),
            Value::SimpleValue(v) => serializer.serialize_newtype_struct(
                crate::adapters::SimpleValue::TYP_NAME,
                &crate::adapters::SimpleValue(*v),
            ),
            Value::Bool(v) => serializer.serialize_bool(*v),
            Value::Null => serializer.serialize_none(),
            Value::Undefined => serializer.serialize_unit(),
        }
    }
}

#[cfg(feature = "serde")]
impl<'a, 'de: 'a> ::serde::Deserialize<'de> for Value<'a> {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        #[derive(Default)]
        struct ValueVisitor<'a>(std::marker::PhantomData<&'a ()>);

        impl<'a, 'de: 'a> ::serde::de::Visitor<'de> for ValueVisitor<'a> {
            type Value = Value<'de>;

            #[inline(always)]
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "A CBOR Value")
            }

            #[inline(always)]
            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: ::serde::de::Error,
            {
                Ok(Value::Bool(v))
            }

            #[inline(always)]
            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: ::serde::de::Error,
            {
                Ok(Value::Integer(v.into()))
            }

            #[inline(always)]
            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: ::serde::de::Error,
            {
                Ok(Value::Integer(v.into()))
            }

            #[inline(always)]
            fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
            where
                E: ::serde::de::Error,
            {
                Ok(Value::Float(v.into()))
            }

            #[inline(always)]
            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: ::serde::de::Error,
            {
                Ok(Value::Float(v.into()))
            }

            #[inline(always)]
            fn visit_char<E>(self, v: char) -> Result<Self::Value, E>
            where
                E: ::serde::de::Error,
            {
                self.visit_str(v.encode_utf8(&mut [0u8; 4]))
            }

            #[inline(always)]
            fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
            where
                E: ::serde::de::Error,
            {
                Ok(Value::String(std::borrow::Cow::Borrowed(v)))
            }

            #[inline(always)]
            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: ::serde::de::Error,
            {
                Ok(Value::String(std::borrow::Cow::Owned(v)))
            }

            #[inline(always)]
            fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
            where
                E: ::serde::de::Error,
            {
                Ok(Value::Bytes(std::borrow::Cow::Borrowed(v)))
            }

            #[inline(always)]
            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
            where
                E: ::serde::de::Error,
            {
                Ok(Value::Bytes(v.into()))
            }

            #[inline(always)]
            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: ::serde::de::Error,
            {
                Ok(Value::Null)
            }

            #[inline(always)]
            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                deserializer.deserialize_any(self)
            }

            #[inline(always)]
            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: ::serde::de::Error,
            {
                Ok(Value::Undefined)
            }

            #[inline(always)]
            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                use ::serde::Deserialize as _;
                let dtv = crate::adapters::DynamicTaggedValue::deserialize(deserializer)?;
                Ok(Value::Tagged((dtv.tag, Box::new(dtv.value.into_owned()))))
            }

            #[inline(always)]
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: ::serde::de::SeqAccess<'de>,
            {
                let mut arr = Vec::with_capacity(seq.size_hint().unwrap_or(4).min(256));

                while let Some(elem) = seq.next_element()? {
                    arr.push(elem);
                }

                Ok(Value::Sequence(arr.into()))
            }

            #[inline(always)]
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: ::serde::de::MapAccess<'de>,
            {
                let mut vmap = Vec::with_capacity(map.size_hint().unwrap_or(4).min(256));

                while let Some((k, v)) = map.next_entry()? {
                    vmap.push((k, v));
                }

                Ok(Value::Map(vmap.into()))
            }
        }

        deserializer.deserialize_any(ValueVisitor::default())
    }
}
