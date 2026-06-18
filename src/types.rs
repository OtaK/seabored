mod float;
mod integer;
#[cfg(feature = "hazmat")]
mod raw;
mod seq;

use std::borrow::Cow;

pub use float::CborFloat;
pub use integer::CborInteger;
pub(crate) use integer::{CborIntegerValue, IB_LIMIT};
#[cfg(feature = "hazmat")]
pub use raw::RawValue;
pub use seq::CborSequence;

use crate::{de::CborDeserialize, ib::InitialByte, ser::CborSerialize};

impl CborSerialize for bool {
    #[inline(always)]
    fn cbor_serialize_to<W: crate::io::Write>(
        &self,
        buf: &mut W,
    ) -> Result<usize, crate::error::SeaboredSerError> {
        buf.write(&[crate::ib::consts::IB_FALSE | *self as u8])
    }
}

impl<'de> CborDeserialize<'de> for bool {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from<R: crate::io::Read<'de>>(
        reader: &mut R,
    ) -> Result<Self, crate::error::SeaboredDeError<'de>>
    where
        Self: Sized + 'de,
    {
        let ib = InitialByte::cbor_deserialize_from(reader)?;
        Ok(match ib.0 {
            crate::ib::consts::IB_TRUE => true,
            crate::ib::consts::IB_FALSE => false,
            _ => {
                return Err(crate::error::SeaboredDeError::IncorrectMajorType {
                    actual: ib.mt(),
                    expected: &[crate::mt::MajorType::SimpleValueOrFloat],
                });
            }
        })
    }
}

impl CborSerialize for &str {
    #[inline(always)]
    fn cbor_serialize_to<W: crate::io::Write>(
        &self,
        writer: &mut W,
    ) -> Result<usize, crate::error::SeaboredSerError> {
        CborIntegerValue::serialize_inline_bytes(
            self.as_bytes(),
            crate::mt::MajorType::String,
            writer,
        )
    }
}

impl<'de> CborDeserialize<'de> for Cow<'de, str> {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from<R: crate::io::Read<'de>>(
        reader: &mut R,
    ) -> Result<Self, crate::error::SeaboredDeError<'de>> {
        let ib = InitialByte::peek(reader)?;
        let (mt, ai) = ib.mt_ai();
        if mt != crate::mt::MajorType::String {
            return Err(crate::error::SeaboredDeError::IncorrectMajorType {
                actual: mt,
                expected: &[crate::mt::MajorType::String],
            });
        }

        reader.advance(1)?;

        let len = ai.find_subsequent_len(reader)?;
        Ok(match reader.read_slice(len.try_into()?)? {
            Cow::Borrowed(s) => Cow::Borrowed(simdutf8::basic::from_utf8(s)?),
            Cow::Owned(s) => Cow::Owned({
                let _ = simdutf8::basic::from_utf8(&s)?;
                // SAFETY: Checked right above
                unsafe { String::from_utf8_unchecked(s) }
            }),
        })
    }
}

impl CborSerialize for &[u8] {
    #[inline(always)]
    fn cbor_serialize_to<W: crate::io::Write>(
        &self,
        writer: &mut W,
    ) -> Result<usize, crate::error::SeaboredSerError> {
        CborIntegerValue::serialize_inline_bytes(self, crate::mt::MajorType::Bytes, writer)
    }
}

impl<'de> CborDeserialize<'de> for Cow<'de, [u8]> {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from<R: crate::io::Read<'de>>(
        reader: &mut R,
    ) -> Result<Self, crate::error::SeaboredDeError<'de>> {
        let ib = InitialByte::peek(reader)?;
        let (mt, ai) = ib.mt_ai();
        if mt != crate::mt::MajorType::Bytes {
            return Err(crate::error::SeaboredDeError::IncorrectMajorType {
                actual: mt,
                expected: &[crate::mt::MajorType::Bytes],
            });
        }

        reader.advance(1)?;

        let len = ai.find_subsequent_len(reader)?;
        reader.read_slice(len.try_into()?)
    }
}
