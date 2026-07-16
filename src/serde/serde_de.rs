use crate::{
    de::CborDeserialize,
    error::SeaboredDeError,
    ib::{self, InitialByte},
    mt::MajorType,
    serde::TAG,
    serde::{DYN_TAGGED_TYP_NAME, Deserializer, SimpleValue, parse_tag_from_typ},
};

use parsio::Read;

impl ::serde_core::de::Error for SeaboredDeError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self::Serde(msg.to_string())
    }
}

impl<'de, R: Read<'de>> ::serde_core::Deserializer<'de> for &mut Deserializer<'de, R> {
    type Error = SeaboredDeError;

    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        let ib = InitialByte::peek(&mut self.reader)?;
        match ib.mt() {
            MajorType::Uint => self.deserialize_u64(visitor),
            MajorType::NegativeUint => self.deserialize_i64(visitor),
            MajorType::Bytes => self.deserialize_bytes(visitor),
            MajorType::String => self.deserialize_str(visitor),
            MajorType::Array => self.deserialize_seq(visitor),
            MajorType::Map => self.deserialize_map(visitor),
            MajorType::Tagged => self.deserialize_newtype_struct(DYN_TAGGED_TYP_NAME, visitor),
            MajorType::SimpleValueOrFloat => match ib.0 {
                ib::consts::IB_TRUE | ib::consts::IB_FALSE => self.deserialize_bool(visitor),
                ib::consts::IB_NULL => self.deserialize_option(visitor),
                ib::consts::IB_UNDEFINED => self.deserialize_unit(visitor),
                ib::consts::IB_FLOAT_16 | ib::consts::IB_FLOAT_32 => self.deserialize_f32(visitor),
                ib::consts::IB_FLOAT_64 => self.deserialize_f64(visitor),
                _ => self.deserialize_newtype_struct(SimpleValue::TYP_NAME, visitor),
            },
        }
    }

    #[inline(always)]
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        visitor.visit_bool(bool::cbor_deserialize_from(&mut self.reader)?)
    }

    #[inline(always)]
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        visitor.visit_i8(i8::cbor_deserialize_from(&mut self.reader)?)
    }

    #[inline(always)]
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        visitor.visit_i16(i16::cbor_deserialize_from(&mut self.reader)?)
    }

    #[inline(always)]
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        visitor.visit_i32(i32::cbor_deserialize_from(&mut self.reader)?)
    }

    #[inline(always)]
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        visitor.visit_i64(i64::cbor_deserialize_from(&mut self.reader)?)
    }

    #[inline(always)]
    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        visitor.visit_i128(i128::cbor_deserialize_from(&mut self.reader)?)
    }

    #[inline(always)]
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        visitor.visit_u8(u8::cbor_deserialize_from(&mut self.reader)?)
    }

    #[inline(always)]
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        visitor.visit_u16(u16::cbor_deserialize_from(&mut self.reader)?)
    }

    #[inline(always)]
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        visitor.visit_u32(u32::cbor_deserialize_from(&mut self.reader)?)
    }

    #[inline(always)]
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        visitor.visit_u64(u64::cbor_deserialize_from(&mut self.reader)?)
    }

    #[inline(always)]
    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        visitor.visit_u128(u128::cbor_deserialize_from(&mut self.reader)?)
    }

    #[inline(always)]
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        visitor.visit_f32(f32::cbor_deserialize_from(&mut self.reader)?)
    }

    #[inline(always)]
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        visitor.visit_f64(f64::cbor_deserialize_from(&mut self.reader)?)
    }

    #[inline(always)]
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        match std::borrow::Cow::<str>::cbor_deserialize_from(&mut self.reader)? {
            std::borrow::Cow::Borrowed(s) => visitor.visit_borrowed_str(s),
            std::borrow::Cow::Owned(s) => visitor.visit_string(s),
        }
    }

    #[inline(always)]
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    #[inline(always)]
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        match std::borrow::Cow::<[u8]>::cbor_deserialize_from(&mut self.reader)? {
            std::borrow::Cow::Borrowed(buf_ref) => visitor.visit_borrowed_bytes(buf_ref),
            std::borrow::Cow::Owned(buf) => visitor.visit_byte_buf(buf),
        }
    }

    #[inline(always)]
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    #[inline(always)]
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    #[inline(always)]
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        let ib = InitialByte::peek(&mut self.reader)?;
        if ib.0 != ib::consts::IB_NULL {
            visitor.visit_some(self)
        } else {
            self.reader.advance(1)?;
            visitor.visit_none()
        }
    }

    #[inline(always)]
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        let ib = InitialByte::peek(&mut self.reader)?;
        if ib.0 != ib::consts::IB_UNDEFINED {
            return Err(SeaboredDeError::IncorrectMajorType {
                actual: ib.mt(),
                expected: &[MajorType::SimpleValueOrFloat],
            });
        }

        self.reader.advance(1)?;

        visitor.visit_unit()
    }

    #[inline(always)]
    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        // Dyn taagged value accessor (eg: Value::Tagged contained in a struct)
        if name == DYN_TAGGED_TYP_NAME {
            return visitor
                .visit_newtype_struct(&mut DynamicTaggedValueAccessor { parent: &mut *self });
        }

        // SimpleValue special handling
        if name == SimpleValue::TYP_NAME {
            return visitor.visit_newtype_struct(&mut SimpleValueAccessor { parent: &mut *self });
        }

        // Special TaggedValue struct handling
        // We're going to read the provisional tag from the type name
        // Then we'll deserialize and check it against the wire data
        if let Some(expected_tag) = parse_tag_from_typ(name) {
            // Consume tag and check it
            let ib = InitialByte::cbor_deserialize_from(&mut self.reader)?;
            let (mt, ai) = ib.mt_ai();
            if mt != MajorType::Tagged {
                return Err(SeaboredDeError::IncorrectMajorType {
                    actual: mt,
                    expected: &[MajorType::Tagged],
                });
            }

            let tag: u64 = ai.find_subsequent_len(&mut self.reader)?.into();
            if tag != expected_tag {
                return Err(SeaboredDeError::WrongTag {
                    actual: tag,
                    expected: expected_tag,
                });
            }
        }

        visitor.visit_newtype_struct(self)
    }

    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        let ib = InitialByte::peek(&mut self.reader)?;
        let (mt, ai) = ib.mt_ai();
        if mt != MajorType::Array {
            return Err(SeaboredDeError::IncorrectMajorType {
                actual: mt,
                expected: &[MajorType::Array],
            });
        }

        self.reader.advance(1)?;

        let len = match ai.find_subsequent_len(&mut self.reader) {
            Ok(len) => Some(len.try_into()?),
            Err(SeaboredDeError::IndefiniteLen) => None,
            Err(e) => return Err(e),
        };

        visitor.visit_seq(SeqAccessor { parent: self, len })
    }

    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        let ib = InitialByte::peek(&mut self.reader)?;
        let (mt, ai) = ib.mt_ai();
        if mt != MajorType::Array {
            return Err(SeaboredDeError::IncorrectMajorType {
                actual: mt,
                expected: &[MajorType::Array],
            });
        }

        self.reader.advance(1)?;

        // Consume any subsequent length bytes so the reader is positioned at the first element.
        let len = match ai.find_subsequent_len(&mut self.reader) {
            Ok(_) => Some(len),
            Err(SeaboredDeError::IndefiniteLen) => None,
            Err(e) => return Err(e),
        };
        // if deser_len != len {
        // FIXME: Verify if that's needed or not?
        // return Err(SeaboredDeError::Io(std::io::Error::new(
        //     std::io::ErrorKind::UnexpectedEof,
        //     format!("Expected {len} elements, CBOR presents {deser_len} elements"),
        // )));
        // }

        visitor.visit_seq(SeqAccessor { parent: self, len })
    }

    #[inline(always)]
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        let ib = InitialByte::peek(&mut self.reader)?;
        let (mt, ai) = ib.mt_ai();
        if mt != MajorType::Map {
            return Err(SeaboredDeError::IncorrectMajorType {
                actual: mt,
                expected: &[MajorType::Map],
            });
        }

        self.reader.advance(1)?;

        let len = match ai.find_subsequent_len(&mut self.reader) {
            Ok(len) => Some(len.try_into()?),
            Err(SeaboredDeError::IndefiniteLen) => None,
            Err(e) => return Err(e),
        };
        visitor.visit_map(SeqAccessor { parent: self, len })
    }

    #[inline(always)]
    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    #[inline(always)]
    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        let ib = InitialByte::peek(&mut self.reader)?;
        if ib.0 == ib::consts::IB_SMALL_MAP | 1 {
            // Skip the map itself basially, otherwise let everything error out
            self.reader.advance(1)?;
        }
        visitor.visit_enum(self)
    }

    #[inline(always)]
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    #[inline(always)]
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        use ::serde_core::Deserialize as _;
        let _ = crate::Value::deserialize(self)?;
        visitor.visit_unit()
    }

    #[inline(always)]
    fn is_human_readable(&self) -> bool {
        false
    }
}

impl<'de, R: Read<'de>> ::serde_core::de::EnumAccess<'de> for &mut Deserializer<'de, R> {
    type Error = SeaboredDeError;

    type Variant = Self;

    #[inline(always)]
    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: ::serde_core::de::DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(&mut *self)?;
        Ok((variant, self))
    }
}

impl<'de, R: Read<'de>> ::serde_core::de::VariantAccess<'de> for &mut Deserializer<'de, R> {
    type Error = SeaboredDeError;

    #[inline(always)]
    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline(always)]
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: ::serde_core::de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self)
    }

    #[inline(always)]
    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        use ::serde_core::de::Deserializer as _;
        self.deserialize_tuple(len, visitor)
    }

    #[inline(always)]
    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        use ::serde_core::de::Deserializer as _;
        self.deserialize_map(visitor)
    }
}

pub(crate) struct DynamicTaggedValueAccessor<'a, 'de, R: Read<'de>> {
    parent: &'a mut Deserializer<'de, R>,
}

impl<'a, 'de, R: Read<'de>> ::serde_core::Deserializer<'de>
    for &mut DynamicTaggedValueAccessor<'a, 'de, R>
{
    type Error = SeaboredDeError;

    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        // Consume tag and check it
        let ib = InitialByte::cbor_deserialize_from(&mut self.parent.reader)?;
        let (mt, ai) = ib.mt_ai();
        if mt != MajorType::Tagged {
            return Err(SeaboredDeError::IncorrectMajorType {
                actual: mt,
                expected: &[MajorType::Tagged],
            });
        }

        // Cache the tag in the structure, we'll go fetch it on the other side
        TAG.set(Some(
            ai.find_subsequent_len(&mut self.parent.reader)?.into(),
        ));

        visitor.visit_newtype_struct(&mut *self.parent)
    }

    ::serde_core::forward_to_deserialize_any! {
        i8 i16 i32 i64 i128
        u8 u16 u32 u64 u128
        bool f32 f64
        char str string
        bytes byte_buf
        seq map
        struct tuple tuple_struct
        identifier ignored_any
        option unit unit_struct newtype_struct enum
    }
}

struct SimpleValueAccessor<'a, 'de, R: Read<'de>> {
    parent: &'a mut Deserializer<'de, R>,
}

impl<'a, 'de, R: Read<'de>> ::serde_core::Deserializer<'de>
    for &mut SimpleValueAccessor<'a, 'de, R>
{
    type Error = SeaboredDeError;

    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::Visitor<'de>,
    {
        let ib = InitialByte::peek(&mut self.parent.reader)?;
        let mt = ib.mt();
        if mt != MajorType::SimpleValueOrFloat {
            return Err(SeaboredDeError::IncorrectMajorType {
                actual: mt,
                expected: &[MajorType::SimpleValueOrFloat],
            });
        }

        self.parent.reader.advance(1)?;

        let u8_value = match ib.0 {
            value @ ib::consts::IB_SIMPLE_VALUE..ib::consts::IB_FALSE => {
                value - ib::consts::IB_SIMPLE_VALUE
            }
            ib::consts::IB_SIMPLE_VALUE_NEXT_BYTE => self.parent.reader.read_byte()?,
            _ => return Err(SeaboredDeError::ReservedSimpleValue(ib.0)),
        };

        visitor.visit_u8(u8_value)
    }

    ::serde_core::forward_to_deserialize_any! {
        i8 i16 i32 i64 i128
        u8 u16 u32 u64 u128
        bool f32 f64
        char str string
        bytes byte_buf
        seq map
        struct tuple tuple_struct
        identifier ignored_any
        option unit unit_struct newtype_struct enum
    }
}

struct SeqAccessor<'a, 'de, R: Read<'de>> {
    parent: &'a mut Deserializer<'de, R>,
    len: Option<usize>,
}

impl<'a, 'de, R: Read<'de>> SeqAccessor<'a, 'de, R> {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn deser_from_seed<S: ::serde_core::de::DeserializeSeed<'de>>(
        &mut self,
        seed: S,
    ) -> Result<Option<S::Value>, SeaboredDeError> {
        Ok(if let Some(len) = &mut self.len {
            if *len > 0 {
                let value = seed.deserialize(&mut *self.parent)?;
                *len -= 1;
                Some(value)
            } else {
                None
            }
        } else {
            let ib = InitialByte::peek(&mut self.parent.reader)?;

            if ib.0 != ib::consts::IB_BREAK {
                Some(seed.deserialize(&mut *self.parent)?)
            } else {
                self.parent.reader.advance(1)?;
                None
            }
        })
    }
}

impl<'a, 'de, R: Read<'de>> ::serde_core::de::SeqAccess<'de> for SeqAccessor<'a, 'de, R> {
    type Error = SeaboredDeError;

    #[inline(always)]
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: ::serde_core::de::DeserializeSeed<'de>,
    {
        self.deser_from_seed(seed)
    }

    #[inline(always)]
    fn size_hint(&self) -> Option<usize> {
        self.len
    }
}

impl<'a, 'de, R: Read<'de>> ::serde_core::de::MapAccess<'de> for SeqAccessor<'a, 'de, R> {
    type Error = SeaboredDeError;

    #[inline(always)]
    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: ::serde_core::de::DeserializeSeed<'de>,
    {
        self.deser_from_seed(seed)
    }

    #[inline(always)]
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: ::serde_core::de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.parent)
    }

    #[inline(always)]
    fn size_hint(&self) -> Option<usize> {
        self.len
    }
}

#[cfg(test)]
mod tests {
    #[derive(Debug, serde::Deserialize)]
    struct IndefiniteSeqMapStruct<'a> {
        test: std::borrow::Cow<'a, str>,
    }

    #[wasm_bindgen_test::wasm_bindgen_test(unsupported = test)]
    fn indefinite_seq_map_struct() {
        let v = crate::serde::from_slice::<IndefiniteSeqMapStruct>(
            &mut &hex_literal::hex!(
                "BF6474657374781F7468697274792D6F6E657468697274792D6F6E657468697274792D6F6E6531FF"
            )[..],
        )
        .unwrap();

        pretty_assertions::assert_eq!(v.test, "thirty-onethirty-onethirty-one1");
    }
}
