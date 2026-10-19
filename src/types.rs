mod float;
mod integer;
#[cfg(feature = "hazmat")]
mod raw;
mod seq;

use std::borrow::Cow;

pub use float::CborFloat;
pub use integer::CborInteger;
pub(crate) use integer::{CborIntegerValue, IB_LIMIT};
use parsio::{DepthAwareReaderGuard, Read as _};
#[cfg(feature = "hazmat")]
pub use raw::RawValue;
pub use seq::CborSequence;

use crate::{de::CborDeserialize, ib::InitialByte, ser::CborSerialize};

impl CborSerialize for bool {
    #[inline(always)]
    fn cbor_serialize_to<W: parsio::Write>(
        &self,
        buf: &mut W,
    ) -> Result<usize, crate::error::SeaboredSerError> {
        buf.write(&[crate::ib::consts::IB_FALSE | *self as u8])
            .map_err(Into::into)
    }
}

impl<'de> CborDeserialize<'de> for bool {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from<R: parsio::Read<'de>>(
        reader: &mut DepthAwareReaderGuard<'_, 'de, R>,
    ) -> Result<Self, crate::error::SeaboredDeError>
    where
        Self: Sized + 'de,
    {
        let ib = InitialByte::cbor_deserialize_from(reader)?;
        Ok(match ib.0 {
            crate::ib::consts::IB_TRUE => true,
            crate::ib::consts::IB_FALSE => false,
            _ => {
                return Err(crate::error::SeaboredDeError::IncorrectInitialByteSet {
                    actual: ib.0,
                    expected: &[crate::ib::consts::IB_FALSE, crate::ib::consts::IB_TRUE],
                });
            }
        })
    }
}

impl CborSerialize for &str {
    #[inline(always)]
    fn cbor_serialize_to<W: parsio::Write>(
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
    fn cbor_deserialize_from<R: parsio::Read<'de>>(
        reader: &mut DepthAwareReaderGuard<'_, 'de, R>,
    ) -> Result<Self, crate::error::SeaboredDeError> {
        let ib = InitialByte::peek(reader)?;
        let (mt, ai) = ib.mt_ai();
        if mt != crate::mt::MajorType::String {
            return Err(crate::error::SeaboredDeError::IncorrectMajorType {
                actual: mt,
                expected: &[crate::mt::MajorType::String],
            });
        }

        reader.advance(1)?;

        Ok(match read_string_body(reader, mt, ai)? {
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
    fn cbor_serialize_to<W: parsio::Write>(
        &self,
        writer: &mut W,
    ) -> Result<usize, crate::error::SeaboredSerError> {
        CborIntegerValue::serialize_inline_bytes(self, crate::mt::MajorType::Bytes, writer)
    }
}

impl<'de> CborDeserialize<'de> for Cow<'de, [u8]> {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from<R: parsio::Read<'de>>(
        reader: &mut DepthAwareReaderGuard<'_, 'de, R>,
    ) -> Result<Self, crate::error::SeaboredDeError> {
        let ib = InitialByte::peek(reader)?;
        let (mt, ai) = ib.mt_ai();
        if mt != crate::mt::MajorType::Bytes {
            return Err(crate::error::SeaboredDeError::IncorrectMajorType {
                actual: mt,
                expected: &[crate::mt::MajorType::Bytes],
            });
        }

        reader.advance(1)?;

        read_string_body(reader, mt, ai)
    }
}

/// Byte/text string body: one item when definite (lent where possible), concatenated chunks when
/// indefinite (RFC 8949 §3.2.3). `mt`/`ai` come from the already-consumed initial byte.
#[cfg_attr(feature = "inline-nontrivial", inline)]
fn read_string_body<'de, R: parsio::Read<'de>>(
    reader: &mut DepthAwareReaderGuard<'_, 'de, R>,
    mt: crate::mt::MajorType,
    ai: crate::ib::AdditionalInfo,
) -> Result<Cow<'de, [u8]>, crate::error::SeaboredDeError> {
    match ai.find_subsequent_len(reader) {
        Ok(len) => Ok(reader.read_slice(len.try_into()?)?),
        Err(crate::error::SeaboredDeError::IndefiniteLen) => {
            read_indefinite_string_body(reader, mt)
        }
        Err(e) => Err(e),
    }
}

/// Cold, out of line: inlining the chunk loop into every string path is bloat.
#[cold]
#[inline(never)]
fn read_indefinite_string_body<'de, R: parsio::Read<'de>>(
    reader: &mut DepthAwareReaderGuard<'_, 'de, R>,
    mt: crate::mt::MajorType,
) -> Result<Cow<'de, [u8]>, crate::error::SeaboredDeError> {
    let mut chunks: Option<Vec<u8>> = None;
    // One level deeper for the container body, like every other indefinite-length item
    let mut reader = reader.enter()?;

    loop {
        let ib = InitialByte::peek(&mut reader)?;
        if ib.0 == crate::ib::consts::IB_BREAK {
            reader.advance(1)?;
            return Ok(Cow::Owned(chunks.unwrap_or_default()));
        }

        // Appendix F kind 3 subkind 3, rule shared with the native path
        ib.expect_string_chunk(mt)?;

        reader.advance(1)?;
        let len: usize = ib.ai().find_subsequent_len(&mut reader)?.try_into()?;
        let chunk = reader.read_slice(len)?;

        // A chunk is text itself: multi-byte characters may not split
        if mt == crate::mt::MajorType::String {
            simdutf8::basic::from_utf8(&chunk)?;
        }

        chunks
            .get_or_insert_with(Vec::new)
            .extend_from_slice(&chunk);
    }
}
