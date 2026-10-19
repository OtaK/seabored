mod float;
mod integer;
#[cfg(feature = "hazmat")]
mod raw;
mod seq;
mod simple;

use std::borrow::Cow;
use std::collections::{BTreeSet, HashSet};
use std::hash::Hash;

pub use float::CborFloat;
pub use integer::CborInteger;
pub(crate) use integer::{CborIntegerValue, IB_LIMIT};
use parsio::{DepthAwareReaderGuard, Read, Write};
#[cfg(feature = "hazmat")]
pub use raw::RawValue;
pub use seq::CborSequence;
pub use simple::CborSimpleValue;

use crate::{
    de::CborDeserialize,
    error::{SeaboredDeError, SeaboredSerError},
    ib::{self, InitialByte},
    mt::MajorType,
    ser::CborSerialize,
};

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

impl CborSerialize for String {
    #[inline]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        CborSerialize::cbor_serialize_to(&self.as_str(), writer)
    }
}

impl<'de> CborDeserialize<'de> for String {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from<R: Read<'de>>(
        reader: &mut DepthAwareReaderGuard<'_, 'de, R>,
    ) -> Result<Self, SeaboredDeError> {
        <Cow<'de, str> as CborDeserialize<'de>>::cbor_deserialize_from(reader).map(Cow::into_owned)
    }
}

impl CborSerialize for Cow<'_, str> {
    #[inline]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        CborSerialize::cbor_serialize_to(&self.as_ref(), writer)
    }
}

impl CborSerialize for Cow<'_, [u8]> {
    #[inline]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        CborSerialize::cbor_serialize_to(&self.as_ref(), writer)
    }
}

impl<T: CborSerialize> CborSerialize for Vec<T> {
    #[inline]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        let mut written = CborIntegerValue::from(self.len())
            .serialize_complex_mt_preamble(MajorType::Array, writer)?;
        for item in self {
            written += item.cbor_serialize_to(writer)?;
        }
        Ok(written)
    }
}

impl<'de, T: CborDeserialize<'de> + 'de> CborDeserialize<'de> for Vec<T> {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from<R: Read<'de>>(
        reader: &mut DepthAwareReaderGuard<'_, 'de, R>,
    ) -> Result<Self, SeaboredDeError> {
        let ib = InitialByte::cbor_deserialize_from(reader)?;
        let (mt, ai) = ib.mt_ai();
        if mt != MajorType::Array {
            return Err(SeaboredDeError::IncorrectMajorType {
                actual: mt,
                expected: &[MajorType::Array],
            });
        }

        match ai.length(reader)? {
            Some(len) => {
                let len: usize = len.try_into()?;
                // The wire length is not a reason to allocate: one item needs at
                // least one byte, so the remaining input bounds the items that exist.
                // Streaming readers report no remaining length; cap those at 256.
                let cap = len.min(reader.remaining().unwrap_or(usize::MAX)).min(256);
                let mut out = Vec::with_capacity(cap);
                let mut nested = reader.enter()?;
                for _ in 0..len {
                    out.push(T::cbor_deserialize_from(&mut nested)?);
                }
                Ok(out)
            }
            None => {
                let mut out = Vec::new();
                let mut nested = reader.enter()?;
                loop {
                    if InitialByte::peek(&mut nested)?.raw() == ib::consts::IB_BREAK {
                        nested.advance(1)?;
                        break;
                    }
                    out.push(T::cbor_deserialize_from(&mut nested)?);
                }
                Ok(out)
            }
        }
    }
}

impl<T: CborSerialize + Ord> CborSerialize for BTreeSet<T> {
    #[inline]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        let mut written = CborIntegerValue::from(self.len())
            .serialize_complex_mt_preamble(MajorType::Array, writer)?;
        for item in self {
            written += item.cbor_serialize_to(writer)?;
        }
        Ok(written)
    }
}

impl<'de, T: CborDeserialize<'de> + Ord + 'de> CborDeserialize<'de> for BTreeSet<T> {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from<R: Read<'de>>(
        reader: &mut DepthAwareReaderGuard<'_, 'de, R>,
    ) -> Result<Self, SeaboredDeError> {
        let ib = InitialByte::cbor_deserialize_from(reader)?;
        let (mt, ai) = ib.mt_ai();
        if mt != MajorType::Array {
            return Err(SeaboredDeError::IncorrectMajorType {
                actual: mt,
                expected: &[MajorType::Array],
            });
        }

        let mut out = BTreeSet::new();
        match ai.length(reader)? {
            Some(len) => {
                let mut nested = reader.enter()?;
                for _ in 0..len {
                    out.insert(T::cbor_deserialize_from(&mut nested)?);
                }
            }
            None => {
                let mut nested = reader.enter()?;
                loop {
                    if InitialByte::peek(&mut nested)?.raw() == ib::consts::IB_BREAK {
                        nested.advance(1)?;
                        break;
                    }
                    out.insert(T::cbor_deserialize_from(&mut nested)?);
                }
            }
        }
        Ok(out)
    }
}

impl<T: CborSerialize + Eq + Hash> CborSerialize for HashSet<T> {
    #[inline]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        let mut written = CborIntegerValue::from(self.len())
            .serialize_complex_mt_preamble(MajorType::Array, writer)?;
        for item in self {
            written += item.cbor_serialize_to(writer)?;
        }
        Ok(written)
    }
}

impl<'de, T: CborDeserialize<'de> + Eq + Hash + 'de> CborDeserialize<'de> for HashSet<T> {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from<R: Read<'de>>(
        reader: &mut DepthAwareReaderGuard<'_, 'de, R>,
    ) -> Result<Self, SeaboredDeError> {
        let ib = InitialByte::cbor_deserialize_from(reader)?;
        let (mt, ai) = ib.mt_ai();
        if mt != MajorType::Array {
            return Err(SeaboredDeError::IncorrectMajorType {
                actual: mt,
                expected: &[MajorType::Array],
            });
        }

        let mut out = HashSet::new();
        match ai.length(reader)? {
            Some(len) => {
                let mut nested = reader.enter()?;
                for _ in 0..len {
                    out.insert(T::cbor_deserialize_from(&mut nested)?);
                }
            }
            None => {
                let mut nested = reader.enter()?;
                loop {
                    if InitialByte::peek(&mut nested)?.raw() == ib::consts::IB_BREAK {
                        nested.advance(1)?;
                        break;
                    }
                    out.insert(T::cbor_deserialize_from(&mut nested)?);
                }
            }
        }
        Ok(out)
    }
}

impl<T: CborSerialize> CborSerialize for Option<T> {
    #[inline]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        match self {
            None => Ok(writer.write(&[ib::consts::IB_NULL])?),
            Some(value) => value.cbor_serialize_to(writer),
        }
    }
}

impl<'de, T: CborDeserialize<'de> + 'de> CborDeserialize<'de> for Option<T> {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from<R: Read<'de>>(
        reader: &mut DepthAwareReaderGuard<'_, 'de, R>,
    ) -> Result<Self, SeaboredDeError> {
        match reader.peek_byte()? {
            ib::consts::IB_NULL => {
                reader.advance(1)?;
                Ok(None)
            }
            ib::consts::IB_UNDEFINED => Err(SeaboredDeError::UnsupportedSimpleValue(
                ib::consts::IB_UNDEFINED,
            )),
            _ => T::cbor_deserialize_from(reader).map(Some),
        }
    }
}

impl<T: CborSerialize> CborSerialize for Box<T> {
    #[inline]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        CborSerialize::cbor_serialize_to(self.as_ref(), writer)
    }
}

impl<'de, T: CborDeserialize<'de> + 'de> CborDeserialize<'de> for Box<T> {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from<R: Read<'de>>(
        reader: &mut DepthAwareReaderGuard<'_, 'de, R>,
    ) -> Result<Self, SeaboredDeError> {
        T::cbor_deserialize_from(reader).map(Box::new)
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
