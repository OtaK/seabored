use crate::{
    error::SeaboredSerError,
    ib,
    mt::MajorType,
    ser::CborSerialize,
    serde::{DYN_TAGGED_TYP_NAME, Serializer, SimpleValue, TypeNameTag, parse_tag_from_typ},
    types::CborIntegerValue,
};

use ::serde_core::ser::Impossible;
use parsio::Write;

impl ::serde_core::ser::Error for SeaboredSerError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self::Serde(msg.to_string())
    }
}

impl<'a, W: Write> ::serde_core::Serializer for &'a mut Serializer<W> {
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
        v.cbor_serialize_to_with_profile(&mut self.writer, self.profile)
    }

    #[inline(always)]
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to_with_profile(&mut self.writer, self.profile)
    }

    #[inline(always)]
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to_with_profile(&mut self.writer, self.profile)
    }

    #[inline(always)]
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to_with_profile(&mut self.writer, self.profile)
    }

    #[inline(always)]
    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to_with_profile(&mut self.writer, self.profile)
    }

    #[inline(always)]
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to_with_profile(&mut self.writer, self.profile)
    }

    #[inline(always)]
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to_with_profile(&mut self.writer, self.profile)
    }

    #[inline(always)]
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to_with_profile(&mut self.writer, self.profile)
    }

    #[inline(always)]
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to_with_profile(&mut self.writer, self.profile)
    }

    #[inline(always)]
    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to_with_profile(&mut self.writer, self.profile)
    }

    #[inline(always)]
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to_with_profile(&mut self.writer, self.profile)
    }

    #[inline(always)]
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to_with_profile(&mut self.writer, self.profile)
    }

    #[inline(always)]
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut tmp_buf = [0u8; 4];
        let str = v.encode_utf8(&mut tmp_buf);
        (&*str).cbor_serialize_to_with_profile(&mut self.writer, self.profile)
    }

    #[inline(always)]
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to_with_profile(&mut self.writer, self.profile)
    }

    #[inline(always)]
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        v.cbor_serialize_to_with_profile(&mut self.writer, self.profile)
    }

    #[inline(always)]
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.writer
            .write(&[ib::consts::IB_NULL])
            .map_err(Into::into)
    }

    #[inline(always)]
    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ::serde_core::Serialize,
    {
        value.serialize(self)
    }

    #[inline(always)]
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.writer
            .write(&[ib::consts::IB_UNDEFINED])
            .map_err(Into::into)
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
        T: ?Sized + ::serde_core::Serialize,
    {
        if name == crate::serde::CHUNKED_STRING_TYP_NAME {
            let (mt, is_indefinite) = crate::serde::SER_STRING.take().ok_or_else(|| {
                SeaboredSerError::Serde(
                    "serialized the `ChunkedString` sentinel without a major type".into(),
                )
            })?;

            if self.profile.is_strict() || !is_indefinite {
                let header = match mt {
                    MajorType::Bytes => ib::consts::IB_BYTE_STRING_SEQUENCE_START,
                    MajorType::String => ib::consts::IB_UTF_8_STRING_SEQUENCE_START,
                    _ => {
                        return Err(SeaboredSerError::Serde(
                            "chunked string sentinel with a non-string major type".into(),
                        ));
                    }
                };
                let mut payload = vec![header];
                {
                    let mut serializer =
                        Serializer::with_profile(&mut payload, crate::profile::Profile::General);
                    serializer.chunked_string = Some(mt);
                    value.serialize(&mut serializer)?;
                }
                let payload: crate::Value<'_> = crate::de::from_bytes(&payload).map_err(|_| {
                    SeaboredSerError::Serde("chunked string payload is not one CBOR item".into())
                })?;
                let profile = if self.profile.is_strict() {
                    self.profile
                } else {
                    crate::profile::Profile::PreferredPlus
                };
                return payload.cbor_serialize_to_with_profile(&mut self.writer, profile);
            }

            let header = match mt {
                MajorType::Bytes => ib::consts::IB_BYTE_STRING_SEQUENCE_START,
                MajorType::String => ib::consts::IB_UTF_8_STRING_SEQUENCE_START,
                _ => {
                    return Err(SeaboredSerError::Serde(
                        "chunked string sentinel with a non-string major type".into(),
                    ));
                }
            };
            let written = self.writer.write(&[header])?;
            self.chunked_string = Some(mt);
            let body = value.serialize(&mut *self);
            self.chunked_string = None;
            return Ok(written + body?);
        }

        if name == SimpleValue::TYP_NAME {
            // Not interpreted: the payload travels as a value (see `serialize_simple_value`)
            return value.serialize(&mut VerbatimSerialiser(&mut self.writer));
        }

        #[cfg(feature = "hazmat")]
        if name == crate::types::RawValue::TYP_NAME {
            if self.profile.is_strict() {
                let mut buf = Vec::new();
                value.serialize(&mut VerbatimSerialiser(&mut buf))?;
                let raw = crate::types::RawValue::from_slice_unchecked(&buf);
                return raw.cbor_serialize_to_with_profile(&mut self.writer, self.profile);
            }
            // General: the payload is the bytes themselves, written verbatim without framing
            return value.serialize(&mut VerbatimSerialiser(&mut self.writer));
        }

        if name == DYN_TAGGED_TYP_NAME {
            // Tag from `SER_TAG`; the value serializes normally
            let tag = crate::serde::SER_TAG.take().ok_or_else(|| {
                SeaboredSerError::Serde(
                    "serialized the `DynamicTaggedValue` sentinel without a tag".into(),
                )
            })?;

            // Strict tag 2/3 follows native `Value` encoding: strip leading zeroes and
            // emit major type 0/1 when the magnitude fits. Buffer the payload (only for
            // bignum tags, only under a strict profile) so it can be inspected.
            if (tag == 2 || tag == 3) && self.profile.is_strict() {
                let mut payload = Vec::new();
                {
                    let mut serializer =
                        Serializer::with_profile(&mut payload, crate::profile::Profile::General);
                    value.serialize(&mut serializer)?;
                }
                let payload: crate::Value<'_> = crate::de::from_bytes(&payload).map_err(|_| {
                    SeaboredSerError::ProfileUnsupported {
                        profile: self.profile,
                        what: "bignum payload",
                    }
                })?;
                let bytes = crate::ser::value_bytes(&payload, self.profile)?;
                return crate::ser::serialize_bignum_tag(
                    tag,
                    &bytes,
                    &mut self.writer,
                    self.profile,
                );
            }

            // Write tag
            let written = CborIntegerValue::from(tag)
                .serialize_complex_mt_preamble(MajorType::Tagged, &mut self.writer)?;

            // And value
            return Ok(written + value.serialize(self)?);
        }

        // Tagged: tag first, then the value
        let written = match parse_tag_from_typ(name) {
            TypeNameTag::NotOurs => 0,
            TypeNameTag::Unparsable => {
                return Err(SeaboredSerError::Serde(
                    "unparsable `Tagged` type name: its tag cannot be recovered".into(),
                ));
            }
            // Serialize tag value first
            TypeNameTag::Tag(tag) => CborIntegerValue::from(tag)
                .serialize_complex_mt_preamble(MajorType::Tagged, &mut self.writer)?,
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
        T: ?Sized + ::serde_core::Serialize,
    {
        // Map len 1
        let mut written = CborIntegerValue::from(1u8)
            .serialize_complex_mt_preamble(MajorType::Map, &mut self.writer)?;
        // key
        written += variant.cbor_serialize_to_with_profile(&mut self.writer, self.profile)?;
        // value
        Ok(written + value.serialize(self)?)
    }

    #[inline(always)]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        if let Some(mt) = self.chunked_string.take() {
            return Ok(SequenceSerializer {
                mode: ContainerMode::ChunkedString { mt },
                written: 0,
                parent: self,
            });
        }

        let strict = self.profile.is_strict();
        let written = if let Some(len) = len {
            CborIntegerValue::from(len)
                .serialize_complex_mt_preamble(MajorType::Array, &mut self.writer)?
        } else if strict {
            0
        } else {
            self.writer.write(&[ib::consts::IB_ARRAY_SEQUENCE_START])?
        };
        let mode = if len.is_none() && strict {
            ContainerMode::BufferedSequence {
                bytes: Vec::new(),
                count: 0,
            }
        } else {
            ContainerMode::Streaming {
                sized: len.is_some(),
            }
        };

        Ok(SequenceSerializer {
            mode,
            written,
            parent: self,
        })
    }

    #[inline(always)]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        let written = CborIntegerValue::from(len)
            .serialize_complex_mt_preamble(MajorType::Array, &mut self.writer)?;

        Ok(SequenceSerializer {
            mode: ContainerMode::Streaming { sized: true },
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
        written += variant.cbor_serialize_to_with_profile(&mut self.writer, self.profile)?;
        // start of tuple list
        written += CborIntegerValue::from(len)
            .serialize_complex_mt_preamble(MajorType::Array, &mut self.writer)?;
        Ok(SequenceSerializer {
            mode: ContainerMode::Streaming { sized: true },
            written,
            parent: self,
        })
    }

    #[inline(always)]
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        let strict = self.profile.is_strict();
        let deterministic = self.profile == crate::profile::Profile::Deterministic;
        let buffer = deterministic || (len.is_none() && strict);
        let written = if buffer {
            0
        } else if let Some(len) = len {
            CborIntegerValue::from(len)
                .serialize_complex_mt_preamble(MajorType::Map, &mut self.writer)?
        } else {
            self.writer.write(&[ib::consts::IB_MAP_SEQUENCE_START])?
        };
        let mode = if buffer {
            ContainerMode::BufferedMap {
                entries: Vec::new(),
                pending_key: None,
                count: 0,
            }
        } else {
            ContainerMode::Streaming {
                sized: len.is_some(),
            }
        };

        Ok(SequenceSerializer {
            mode,
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
        if self.profile == crate::profile::Profile::Deterministic {
            return Ok(SequenceSerializer {
                mode: ContainerMode::BufferedMap {
                    entries: Vec::new(),
                    pending_key: None,
                    count: 0,
                },
                written: 0,
                parent: self,
            });
        }

        let written = CborIntegerValue::from(len)
            .serialize_complex_mt_preamble(MajorType::Map, &mut self.writer)?;

        Ok(SequenceSerializer {
            mode: ContainerMode::Streaming { sized: true },
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
        written += variant.cbor_serialize_to_with_profile(&mut self.writer, self.profile)?;

        if self.profile == crate::profile::Profile::Deterministic {
            return Ok(SequenceSerializer {
                mode: ContainerMode::BufferedMap {
                    entries: Vec::new(),
                    pending_key: None,
                    count: 0,
                },
                written,
                parent: self,
            });
        }

        // start of map with field => value
        written += CborIntegerValue::from(len)
            .serialize_complex_mt_preamble(MajorType::Map, &mut self.writer)?;

        Ok(SequenceSerializer {
            mode: ContainerMode::Streaming { sized: true },
            written,
            parent: self,
        })
    }

    #[inline(always)]
    fn is_human_readable(&self) -> bool {
        false
    }
}

enum ContainerMode {
    Streaming {
        sized: bool,
    },
    BufferedSequence {
        bytes: Vec<u8>,
        count: usize,
    },
    BufferedMap {
        entries: Vec<(Vec<u8>, Vec<u8>)>,
        pending_key: Option<Vec<u8>>,
        count: usize,
    },
    ChunkedString {
        mt: MajorType,
    },
}

pub struct SequenceSerializer<'a, W: Write> {
    mode: ContainerMode,
    written: usize,
    parent: &'a mut Serializer<W>,
}

impl<W: Write> SequenceSerializer<'_, W> {
    fn finish(mut self) -> Result<usize, SeaboredSerError> {
        match &mut self.mode {
            ContainerMode::Streaming { sized } => {
                if !*sized {
                    self.written += self.parent.writer.write(&[ib::consts::IB_BREAK])?;
                }
            }
            ContainerMode::BufferedSequence { bytes, count } => {
                let mut written = CborIntegerValue::from(*count)
                    .serialize_complex_mt_preamble(MajorType::Array, &mut self.parent.writer)?;
                written += self.parent.writer.write(bytes.as_slice())?;
                self.written += written;
            }
            ContainerMode::ChunkedString { .. } => {
                self.written += self.parent.writer.write(&[ib::consts::IB_BREAK])?;
            }
            ContainerMode::BufferedMap {
                entries,
                pending_key,
                count: _,
            } => {
                if pending_key.is_some() {
                    return Err(SeaboredSerError::Serde(
                        "map key was serialized without a value".into(),
                    ));
                }
                if self.parent.profile == crate::profile::Profile::Deterministic {
                    entries.sort_by(|left, right| left.0.cmp(&right.0));
                }

                let mut written = CborIntegerValue::from(entries.len())
                    .serialize_complex_mt_preamble(MajorType::Map, &mut self.parent.writer)?;
                for (key, value) in entries {
                    written += self.parent.writer.write(key.as_slice())?;
                    written += self.parent.writer.write(value.as_slice())?;
                }
                self.written += written;
            }
        }
        Ok(self.written)
    }
}

impl<W: Write> ::serde_core::ser::SerializeSeq for SequenceSerializer<'_, W> {
    type Ok = usize;
    type Error = SeaboredSerError;

    #[inline(always)]
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ::serde_core::Serialize,
    {
        match &mut self.mode {
            ContainerMode::Streaming { .. } => {
                self.written += value.serialize(&mut *self.parent)?;
                Ok(())
            }
            ContainerMode::BufferedSequence { bytes, count } => {
                let mut serializer = Serializer::with_profile(bytes, self.parent.profile);
                value.serialize(&mut serializer)?;
                *count += 1;
                Ok(())
            }
            ContainerMode::ChunkedString { mt } => {
                let mut buf = Vec::new();
                {
                    let mut serializer =
                        Serializer::with_profile(&mut buf, crate::profile::Profile::General);
                    value.serialize(&mut serializer)?;
                }
                let first = *buf.first().ok_or_else(|| {
                    SeaboredSerError::Serde("empty chunk in a chunked string".into())
                })?;
                if (first >> 5) != (*mt as u8) || (first & 0x1f) == 0x1f {
                    return Err(SeaboredSerError::Serde(
                        "chunk is not a definite string of the container's major type".into(),
                    ));
                }
                self.written += self.parent.writer.write(&buf)?;
                Ok(())
            }
            ContainerMode::BufferedMap { .. } => Err(SeaboredSerError::Serde(
                "sequence element in a buffered map".into(),
            )),
        }
    }

    #[inline(always)]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish()
    }
}

impl<W: Write> ::serde_core::ser::SerializeTuple for SequenceSerializer<'_, W> {
    type Ok = usize;
    type Error = SeaboredSerError;

    #[inline(always)]
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ::serde_core::Serialize,
    {
        self.written += value.serialize(&mut *self.parent)?;
        Ok(())
    }

    #[inline(always)]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish()
    }
}

impl<W: Write> ::serde_core::ser::SerializeTupleStruct for SequenceSerializer<'_, W> {
    type Ok = usize;
    type Error = SeaboredSerError;

    #[inline(always)]
    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ::serde_core::Serialize,
    {
        self.written += value.serialize(&mut *self.parent)?;
        Ok(())
    }

    #[inline(always)]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish()
    }
}

impl<W: Write> ::serde_core::ser::SerializeTupleVariant for SequenceSerializer<'_, W> {
    type Ok = usize;
    type Error = SeaboredSerError;

    #[inline(always)]
    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ::serde_core::Serialize,
    {
        self.written += value.serialize(&mut *self.parent)?;
        Ok(())
    }

    #[inline(always)]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish()
    }
}

impl<W: Write> ::serde_core::ser::SerializeMap for SequenceSerializer<'_, W> {
    type Ok = usize;
    type Error = SeaboredSerError;

    #[inline(always)]
    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ::serde_core::Serialize,
    {
        match &mut self.mode {
            ContainerMode::Streaming { .. } => {
                self.written += key.serialize(&mut *self.parent)?;
                Ok(())
            }
            ContainerMode::BufferedMap {
                pending_key,
                entries: _,
                count: _,
            } => {
                if pending_key.is_some() {
                    return Err(SeaboredSerError::Serde(
                        "map key was serialized twice".into(),
                    ));
                }
                let mut key_bytes = Vec::new();
                let mut serializer = Serializer::with_profile(&mut key_bytes, self.parent.profile);
                key.serialize(&mut serializer)?;
                *pending_key = Some(key_bytes);
                Ok(())
            }
            ContainerMode::ChunkedString { .. } => Err(SeaboredSerError::Serde(
                "map key in a chunked string".into(),
            )),
            ContainerMode::BufferedSequence { .. } => Err(SeaboredSerError::Serde(
                "map key in a buffered sequence".into(),
            )),
        }
    }

    #[inline(always)]
    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ::serde_core::Serialize,
    {
        match &mut self.mode {
            ContainerMode::Streaming { .. } => {
                self.written += value.serialize(&mut *self.parent)?;
                Ok(())
            }
            ContainerMode::BufferedMap {
                entries,
                pending_key,
                count,
            } => {
                let key = pending_key.take().ok_or_else(|| {
                    SeaboredSerError::Serde("map value was serialized without a key".into())
                })?;
                let mut value_bytes = Vec::new();
                let mut serializer =
                    Serializer::with_profile(&mut value_bytes, self.parent.profile);
                value.serialize(&mut serializer)?;
                entries.push((key, value_bytes));
                *count += 1;
                Ok(())
            }
            ContainerMode::ChunkedString { .. } => Err(SeaboredSerError::Serde(
                "map value in a chunked string".into(),
            )),
            ContainerMode::BufferedSequence { .. } => Err(SeaboredSerError::Serde(
                "map value in a buffered sequence".into(),
            )),
        }
    }

    #[inline(always)]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish()
    }
}

impl<W: Write> ::serde_core::ser::SerializeStruct for SequenceSerializer<'_, W> {
    type Ok = usize;
    type Error = SeaboredSerError;

    #[inline(always)]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ::serde_core::Serialize,
    {
        match &mut self.mode {
            ContainerMode::Streaming { .. } => {
                self.written += key
                    .cbor_serialize_to_with_profile(&mut self.parent.writer, self.parent.profile)?;
                self.written += value.serialize(&mut *self.parent)?;
                Ok(())
            }
            ContainerMode::BufferedMap { entries, .. } => {
                let mut key_bytes = Vec::new();
                let mut key_serializer =
                    Serializer::with_profile(&mut key_bytes, self.parent.profile);
                key.cbor_serialize_to_with_profile(
                    &mut key_serializer.writer,
                    self.parent.profile,
                )?;

                let mut value_bytes = Vec::new();
                let mut value_serializer =
                    Serializer::with_profile(&mut value_bytes, self.parent.profile);
                value.serialize(&mut value_serializer)?;
                entries.push((key_bytes, value_bytes));
                Ok(())
            }
            ContainerMode::ChunkedString { .. } => Err(SeaboredSerError::Serde(
                "struct field in a chunked string".into(),
            )),
            ContainerMode::BufferedSequence { bytes, count } => {
                let mut serializer = Serializer::with_profile(bytes, self.parent.profile);
                value.serialize(&mut serializer)?;
                *count += 1;
                Ok(())
            }
        }
    }

    #[inline(always)]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish()
    }
}

impl<W: Write> ::serde_core::ser::SerializeStructVariant for SequenceSerializer<'_, W> {
    type Ok = usize;
    type Error = SeaboredSerError;

    #[inline(always)]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ::serde_core::Serialize,
    {
        match &mut self.mode {
            ContainerMode::Streaming { .. } => {
                self.written += key
                    .cbor_serialize_to_with_profile(&mut self.parent.writer, self.parent.profile)?;
                self.written += value.serialize(&mut *self.parent)?;
                Ok(())
            }
            ContainerMode::BufferedMap { entries, .. } => {
                let mut key_bytes = Vec::new();
                let mut key_serializer =
                    Serializer::with_profile(&mut key_bytes, self.parent.profile);
                key.cbor_serialize_to_with_profile(
                    &mut key_serializer.writer,
                    self.parent.profile,
                )?;

                let mut value_bytes = Vec::new();
                let mut value_serializer =
                    Serializer::with_profile(&mut value_bytes, self.parent.profile);
                value.serialize(&mut value_serializer)?;
                entries.push((key_bytes, value_bytes));
                Ok(())
            }
            ContainerMode::ChunkedString { .. } => Err(SeaboredSerError::Serde(
                "struct variant field in a chunked string".into(),
            )),
            ContainerMode::BufferedSequence { bytes, count } => {
                let mut serializer = Serializer::with_profile(bytes, self.parent.profile);
                value.serialize(&mut serializer)?;
                *count += 1;
                Ok(())
            }
        }
    }

    #[inline(always)]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish()
    }
}

/// Rejects every call a sentinel payload is not allowed to make.
macro_rules! verbatim_payload_only {
    ($( fn $name:ident $(<$g:ident>)? ( $( $arg:ident : $ty:ty ),* $(,)? ) -> $ret:ty ; )*) => {
        $(
            #[inline]
            #[allow(unused_variables)]
            fn $name $(<$g: ?Sized + ::serde_core::Serialize>)? (self $(, $arg: $ty)*)
                -> Result<$ret, Self::Error>
            {
                Err(SeaboredSerError::Serde(
                    "expected a simple value or raw bytes payload".into(),
                ))
            }
        )*
    };
}

/// Verbatim payloads: a simple-value byte, or raw bytes (no framing). Everything else errors.
struct VerbatimSerialiser<'w, W: Write>(&'w mut W);

impl<W: Write> ::serde_core::Serializer for &mut VerbatimSerialiser<'_, W> {
    type Ok = usize;
    type Error = SeaboredSerError;

    type SerializeSeq = Impossible<usize, SeaboredSerError>;
    type SerializeTuple = Impossible<usize, SeaboredSerError>;
    type SerializeTupleStruct = Impossible<usize, SeaboredSerError>;
    type SerializeTupleVariant = Impossible<usize, SeaboredSerError>;
    type SerializeMap = Impossible<usize, SeaboredSerError>;
    type SerializeStruct = Impossible<usize, SeaboredSerError>;
    type SerializeStructVariant = Impossible<usize, SeaboredSerError>;

    /// The simple value sentinel's payload
    #[inline]
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        // 20..=23 are false/true/null/undefined and 24..=31 are reserved (RFC 8949 §3.3)
        if (20..32).contains(&v) {
            return Err(SeaboredSerError::ReservedSimpleValue(v));
        }
        Ok(if v <= crate::types::IB_LIMIT {
            self.0.write(&[ib::consts::IB_SIMPLE_VALUE | v])?
        } else {
            self.0.write(&[ib::consts::IB_SIMPLE_VALUE_NEXT_BYTE, v])?
        })
    }

    /// The `RawValue` sentinel's payload, written with no framing
    #[inline]
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(self.0.write(v)?)
    }

    verbatim_payload_only!(
        fn serialize_bool(v: bool) -> Self::Ok;
        fn serialize_i8(v: i8) -> Self::Ok;
        fn serialize_i16(v: i16) -> Self::Ok;
        fn serialize_i32(v: i32) -> Self::Ok;
        fn serialize_i64(v: i64) -> Self::Ok;
        fn serialize_u16(v: u16) -> Self::Ok;
        fn serialize_u32(v: u32) -> Self::Ok;
        fn serialize_u64(v: u64) -> Self::Ok;
        fn serialize_f32(v: f32) -> Self::Ok;
        fn serialize_f64(v: f64) -> Self::Ok;
        fn serialize_char(v: char) -> Self::Ok;
        fn serialize_str(v: &str) -> Self::Ok;
        fn serialize_none() -> Self::Ok;
        fn serialize_unit() -> Self::Ok;
        fn serialize_unit_struct(name: &'static str) -> Self::Ok;
        fn serialize_unit_variant(
            name: &'static str,
            index: u32,
            variant: &'static str,
        ) -> Self::Ok;
        fn serialize_seq(len: Option<usize>) -> Self::SerializeSeq;
        fn serialize_tuple(len: usize) -> Self::SerializeTuple;
        fn serialize_tuple_struct(name: &'static str, len: usize) -> Self::SerializeTupleStruct;
        fn serialize_map(len: Option<usize>) -> Self::SerializeMap;
        fn serialize_struct(name: &'static str, len: usize) -> Self::SerializeStruct;
        fn serialize_struct_variant(
            name: &'static str,
            index: u32,
            variant: &'static str,
            len: usize,
        ) -> Self::SerializeStructVariant;
        fn serialize_some<T>(value: &T) -> Self::Ok;
        fn serialize_newtype_struct<T>(name: &'static str, value: &T) -> Self::Ok;
        fn serialize_newtype_variant<T>(
            name: &'static str,
            index: u32,
            variant: &'static str,
            value: &T,
        ) -> Self::Ok;
        fn serialize_tuple_variant(
            name: &'static str,
            variant_index: u32,
            variant: &'static str,
            len: usize,
        ) -> Self::SerializeTupleVariant;
    );
}
