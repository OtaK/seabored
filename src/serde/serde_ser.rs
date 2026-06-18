use serde::Serialize;

use crate::{
    error::SeaboredSerError,
    ib,
    io::Write,
    mt::MajorType,
    ser::CborSerialize,
    serde::{DYN_TAGGED_TYP_NAME, DynamicTaggedValue, SimpleValue, parse_tag_from_typ},
    types::CborIntegerValue,
};

impl serde::ser::Error for SeaboredSerError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self::Serde(msg.to_string())
    }
}

pub struct Serializer<W: Write> {
    pub(crate) writer: W,
}

impl<'a, W: Write> serde::Serializer for &'a mut Serializer<W> {
    type Ok = usize;
    type Error = SeaboredSerError;

    type SerializeSeq = SequenceSerializer<'a, W>;
    type SerializeTuple = SequenceSerializer<'a, W>;
    type SerializeTupleStruct = SequenceSerializer<'a, W>;
    type SerializeTupleVariant = SequenceSerializer<'a, W>;
    type SerializeMap = SequenceSerializer<'a, W>;
    type SerializeStruct = SequenceSerializer<'a, W>;
    type SerializeStructVariant = SequenceSerializer<'a, W>;

    #[inline(always)]
    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(self.writer.write(&[ib::consts::IB_FALSE | v as u8])?)
    }

    #[inline(always)]
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to(&mut self.writer)
    }

    #[inline(always)]
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to(&mut self.writer)
    }

    #[inline(always)]
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to(&mut self.writer)
    }

    #[inline(always)]
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to(&mut self.writer)
    }

    #[inline(always)]
    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to(&mut self.writer)
    }

    #[inline(always)]
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to(&mut self.writer)
    }

    #[inline(always)]
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to(&mut self.writer)
    }

    #[inline(always)]
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to(&mut self.writer)
    }

    #[inline(always)]
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to(&mut self.writer)
    }

    #[inline(always)]
    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to(&mut self.writer)
    }

    #[inline(always)]
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to(&mut self.writer)
    }

    #[inline(always)]
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to(&mut self.writer)
    }

    #[inline(always)]
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut tmp_buf = [0u8; 4];
        let str = v.encode_utf8(&mut tmp_buf);
        (&*str).cbor_serialize_to(&mut self.writer)
    }

    #[inline(always)]
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to(&mut self.writer)
    }

    #[inline(always)]
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to(&mut self.writer)
    }

    #[inline(always)]
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.writer.write(&[ib::consts::IB_NULL])
    }

    #[inline(always)]
    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        value.serialize(self)
    }

    #[inline(always)]
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.writer.write(&[ib::consts::IB_UNDEFINED])
    }

    #[inline(always)]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    #[inline(always)]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn serialize_newtype_struct<T>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        if name == SimpleValue::TYP_NAME {
            // SAFETY: Because of the newtype struct name match, and the fact
            // that [`SimpleValue`] is `#[repr(transparent)]` over its single [`u8`] member,
            // we can ascertain that extracting a u8 from a pointer to [`SimpleValue`] is
            // always safe
            //
            // Yeah yeah I know it looks ugly and shady but blame serde type erasure for this
            let sv: u8 = unsafe { std::ptr::read(value as *const _ as *const u8) };

            return if sv <= crate::types::IB_LIMIT {
                self.writer.write(&[ib::consts::IB_SIMPLE_VALUE | sv])
            } else {
                self.writer
                    .write(&[ib::consts::IB_SIMPLE_VALUE_NEXT_BYTE, sv])
            };
        }

        #[cfg(feature = "hazmat")]
        if name == crate::types::RawValue::TYP_NAME {
            // SAFETY: The newtype-struct name sentinel guarantees that T == RawValue.
            // RawValue is #[repr(transparent)] over Vec<u8>, so the pointer cast is sound.
            let raw: &crate::types::RawValue =
                unsafe { std::mem::transmute(value as *const T as *const crate::types::RawValue) };
            return self.writer.write(raw.as_bytes());
        }

        if name == DYN_TAGGED_TYP_NAME {
            // SAFETY: If we get this type name, then treat the value as a DynamicTaggedValue ref
            let dyn_tagged: &DynamicTaggedValue =
                unsafe { std::mem::transmute(value as *const T as *const DynamicTaggedValue) };

            // Write tag
            let written = dyn_tagged
                .tag
                .serialize_complex_mt_preamble(MajorType::Tagged, &mut self.writer)?;

            // And value
            return Ok(written + dyn_tagged.value.serialize(self)?);
        }

        // Special TaggedValue struct handling
        // We're going to output the tag before outputting the value
        let written = if let Some(tag) = parse_tag_from_typ(name) {
            // Serialize tag value first
            CborIntegerValue::from(tag)
                .serialize_complex_mt_preamble(MajorType::Tagged, &mut self.writer)?
        } else {
            0
        };

        Ok(written + value.serialize(self)?)
    }

    #[inline(always)]
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        // Map len 1
        let mut written = CborIntegerValue::from(1u8)
            .serialize_complex_mt_preamble(MajorType::Map, &mut self.writer)?;
        // key
        written += variant.cbor_serialize_to(&mut self.writer)?;
        // value
        Ok(written + value.serialize(self)?)
    }

    #[inline(always)]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        let sized = len.is_some();
        let written = if let Some(len) = len {
            CborIntegerValue::from(len)
                .serialize_complex_mt_preamble(MajorType::Array, &mut self.writer)?
        } else {
            self.writer.write(&[ib::consts::IB_ARRAY_SEQUENCE_START])?
        };

        Ok(SequenceSerializer {
            sized,
            written,
            parent: self,
        })
    }

    #[inline(always)]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        let written = CborIntegerValue::from(len)
            .serialize_complex_mt_preamble(MajorType::Array, &mut self.writer)?;

        Ok(SequenceSerializer {
            sized: true,
            written,
            parent: self,
        })
    }

    #[inline(always)]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_tuple(len)
    }

    #[inline(always)]
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        // Map len 1
        let mut written = CborIntegerValue::from(1u8)
            .serialize_complex_mt_preamble(MajorType::Map, &mut self.writer)?;
        // key
        written += variant.cbor_serialize_to(&mut self.writer)?;
        // start of tuple list
        written += CborIntegerValue::from(len)
            .serialize_complex_mt_preamble(MajorType::Array, &mut self.writer)?;
        Ok(SequenceSerializer {
            sized: true,
            written,
            parent: self,
        })
    }

    #[inline(always)]
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        let sized = len.is_some();
        let written = if let Some(len) = len {
            CborIntegerValue::from(len)
                .serialize_complex_mt_preamble(MajorType::Map, &mut self.writer)?
        } else {
            self.writer.write(&[ib::consts::IB_MAP_SEQUENCE_START])?
        };

        Ok(SequenceSerializer {
            sized,
            written,
            parent: self,
        })
    }

    #[inline(always)]
    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        let written = CborIntegerValue::from(len)
            .serialize_complex_mt_preamble(MajorType::Map, &mut self.writer)?;

        Ok(SequenceSerializer {
            sized: true,
            written,
            parent: self,
        })
    }

    #[inline(always)]
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        // Map len 1
        let mut written = CborIntegerValue::from(1u8)
            .serialize_complex_mt_preamble(MajorType::Map, &mut self.writer)?;
        // key
        written += variant.cbor_serialize_to(&mut self.writer)?;
        // start of map with field => value
        written += CborIntegerValue::from(len)
            .serialize_complex_mt_preamble(MajorType::Map, &mut self.writer)?;

        Ok(SequenceSerializer {
            sized: true,
            written,
            parent: self,
        })
    }

    #[inline(always)]
    fn is_human_readable(&self) -> bool {
        false
    }
}

pub struct SequenceSerializer<'a, W: Write> {
    sized: bool,
    written: usize,
    parent: &'a mut Serializer<W>,
}

impl<W: Write> serde::ser::SerializeSeq for SequenceSerializer<'_, W> {
    type Ok = usize;
    type Error = SeaboredSerError;

    #[inline(always)]
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.written += value.serialize(&mut *self.parent)?;
        Ok(())
    }

    #[inline(always)]
    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        if !self.sized {
            self.written += self.parent.writer.write(&[ib::consts::IB_BREAK])?;
        }
        Ok(self.written)
    }
}

impl<W: Write> serde::ser::SerializeTuple for SequenceSerializer<'_, W> {
    type Ok = usize;
    type Error = SeaboredSerError;

    #[inline(always)]
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.written += value.serialize(&mut *self.parent)?;
        Ok(())
    }

    #[inline(always)]
    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        if !self.sized {
            self.written += self.parent.writer.write(&[ib::consts::IB_BREAK])?;
        }
        Ok(self.written)
    }
}

impl<W: Write> serde::ser::SerializeTupleStruct for SequenceSerializer<'_, W> {
    type Ok = usize;
    type Error = SeaboredSerError;

    #[inline(always)]
    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.written += value.serialize(&mut *self.parent)?;
        Ok(())
    }

    #[inline(always)]
    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        if !self.sized {
            self.written += self.parent.writer.write(&[ib::consts::IB_BREAK])?;
        }
        Ok(self.written)
    }
}

impl<W: Write> serde::ser::SerializeTupleVariant for SequenceSerializer<'_, W> {
    type Ok = usize;
    type Error = SeaboredSerError;

    #[inline(always)]
    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.written += value.serialize(&mut *self.parent)?;
        Ok(())
    }

    #[inline(always)]
    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        if !self.sized {
            self.written += self.parent.writer.write(&[ib::consts::IB_BREAK])?;
        }
        Ok(self.written)
    }
}

impl<W: Write> serde::ser::SerializeMap for SequenceSerializer<'_, W> {
    type Ok = usize;
    type Error = SeaboredSerError;

    #[inline(always)]
    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.written += key.serialize(&mut *self.parent)?;
        Ok(())
    }

    #[inline(always)]
    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.written += value.serialize(&mut *self.parent)?;
        Ok(())
    }

    #[inline(always)]
    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        if !self.sized {
            self.written += self.parent.writer.write(&[ib::consts::IB_BREAK])?;
        }
        Ok(self.written)
    }
}

impl<W: Write> serde::ser::SerializeStruct for SequenceSerializer<'_, W> {
    type Ok = usize;
    type Error = SeaboredSerError;

    #[inline(always)]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.written += key.cbor_serialize_to(&mut self.parent.writer)?;
        self.written += value.serialize(&mut *self.parent)?;
        Ok(())
    }

    #[inline(always)]
    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        if !self.sized {
            self.written += self.parent.writer.write(&[ib::consts::IB_BREAK])?;
        }
        Ok(self.written)
    }
}

impl<W: Write> serde::ser::SerializeStructVariant for SequenceSerializer<'_, W> {
    type Ok = usize;
    type Error = SeaboredSerError;

    #[inline(always)]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.written += key.cbor_serialize_to(&mut self.parent.writer)?;
        self.written += value.serialize(&mut *self.parent)?;
        Ok(())
    }

    #[inline(always)]
    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        if !self.sized {
            self.written += self.parent.writer.write(&[ib::consts::IB_BREAK])?;
        }
        Ok(self.written)
    }
}
