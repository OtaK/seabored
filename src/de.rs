use half::f16;
use parsio::{DepthAwareReader, DepthAwareReaderGuard, Read};
use std::borrow::Cow;

use crate::{
    Value,
    error::SeaboredDeError,
    ib::{self, AdditionalInfoAction, InitialByte},
    io::ReadExt as _,
    mt::MajorType,
    types::{CborInteger, CborIntegerValue, CborSequence},
};

/// Reads a single CBOR item from a byte slice, up to
/// [`parsio::DepthAwareReader::DEFAULT_LIMIT`] nesting levels
#[inline]
pub fn from_bytes<'cbor, T: CborDeserialize<'cbor> + 'cbor>(
    bytes: &'cbor [u8],
) -> Result<T, SeaboredDeError> {
    from_bytes_with_limit(bytes, <DepthAwareReader<'cbor, &'cbor [u8]>>::DEFAULT_LIMIT)
}

/// Reads a single CBOR item from a byte slice, allowing at most `limit` levels of nesting.
#[inline]
pub fn from_bytes_with_limit<'cbor, T: CborDeserialize<'cbor> + 'cbor>(
    bytes: &'cbor [u8],
    limit: u16,
) -> Result<T, SeaboredDeError> {
    let mut reader = DepthAwareReader::from_reader_with_limit(bytes, limit);
    let mut guard = reader.enter()?;
    T::cbor_deserialize_from(&mut guard)
}

pub trait CborDeserialize<'cbor> {
    fn cbor_deserialize_from<R: Read<'cbor>>(
        reader: &mut DepthAwareReaderGuard<'_, 'cbor, R>,
    ) -> Result<Self, SeaboredDeError>
    where
        Self: Sized + 'cbor;
}

/// Skip one well-formed CBOR item without materialising it.
///
/// Validates well-formedness (initial-byte widths, string chunks and UTF-8,
/// container boundaries, tag payloads). It does **not** enforce preferred
/// encodings: non-shortest arguments are accepted here, so a strict profile
/// must check the headers it interprets itself.
#[cfg_attr(feature = "inline-nontrivial", inline)]
pub fn skip_value<'de, R: Read<'de>>(
    reader: &mut DepthAwareReaderGuard<'_, 'de, R>,
) -> Result<(), SeaboredDeError> {
    let ib = InitialByte::peek(reader)?;
    if ib.raw() == ib::consts::IB_BREAK {
        return Err(SeaboredDeError::NotWellFormed("unexpected BREAK"));
    }

    let (mt, ai) = ib.mt_ai();

    if ai.is_indefinite() {
        match mt {
            MajorType::Bytes | MajorType::String => {
                reader.advance(1)?;
                let mut nested = reader.enter()?;
                loop {
                    let chunk_ib = InitialByte::peek(&mut nested)?;
                    if chunk_ib.raw() == ib::consts::IB_BREAK {
                        nested.advance(1)?;
                        break;
                    }
                    chunk_ib.expect_string_chunk(mt)?;
                    nested.advance(1)?;
                    let len = chunk_ib.ai().length(&mut nested)?.ok_or(
                        SeaboredDeError::NotWellFormed(
                            "indefinite-length chunk inside an indefinite-length string",
                        ),
                    )?;
                    let chunk = nested.read_slice(len.try_into()?)?;
                    if mt == MajorType::String {
                        simdutf8::basic::from_utf8(&chunk)?;
                    }
                }
            }
            MajorType::Array => {
                reader.advance(1)?;
                let mut nested = reader.enter()?;
                loop {
                    if InitialByte::peek(&mut nested)?.raw() == ib::consts::IB_BREAK {
                        nested.advance(1)?;
                        break;
                    }
                    skip_value(&mut nested)?;
                }
            }
            MajorType::Map => {
                reader.advance(1)?;
                let mut nested = reader.enter()?;
                loop {
                    if InitialByte::peek(&mut nested)?.raw() == ib::consts::IB_BREAK {
                        nested.advance(1)?;
                        break;
                    }
                    skip_value(&mut nested)?;
                    skip_value(&mut nested)?;
                }
            }
            _ => {
                return Err(SeaboredDeError::NotWellFormed(
                    "indefinite length on a major type that has no indefinite form",
                ));
            }
        }
        return Ok(());
    }

    reader.advance(1)?;
    let len = ai.length(reader)?.ok_or(SeaboredDeError::NotWellFormed(
        "indefinite length reached after the indefinite branch",
    ))?;

    match mt {
        MajorType::Uint | MajorType::NegativeUint => Ok(()),
        MajorType::Bytes | MajorType::String => {
            let bytes = reader.read_slice(len.try_into()?)?;
            if mt == MajorType::String {
                simdutf8::basic::from_utf8(&bytes)?;
            }
            Ok(())
        }
        MajorType::Array => {
            let mut nested = reader.enter()?;
            for _ in 0..len {
                skip_value(&mut nested)?;
            }
            Ok(())
        }
        MajorType::Map => {
            let mut nested = reader.enter()?;
            for _ in 0..len {
                skip_value(&mut nested)?;
                skip_value(&mut nested)?;
            }
            Ok(())
        }
        MajorType::Tagged => {
            let mut nested = reader.enter()?;
            skip_value(&mut nested)
        }
        MajorType::SimpleValueOrFloat => {
            // RFC 8949 §3.3: `f8` with an argument below 0x20 is not well-formed
            if ai.raw() == 24 && len < 0x20 {
                return Err(SeaboredDeError::NotWellFormed(
                    "two-byte simple value with an argument below 0x20",
                ));
            }
            Ok(())
        }
    }
}

impl<'a, T1, T2> CborDeserialize<'a> for (T1, T2)
where
    T1: CborDeserialize<'a>,
    T2: CborDeserialize<'a>,
{
    #[inline(always)]
    fn cbor_deserialize_from<R: Read<'a>>(
        reader: &mut DepthAwareReaderGuard<'_, 'a, R>,
    ) -> Result<Self, SeaboredDeError>
    where
        Self: Sized + 'a,
    {
        T1::cbor_deserialize_from(reader)
            .and_then(|t1| T2::cbor_deserialize_from(reader).map(|t2| (t1, t2)))
    }
}

impl<'a> CborDeserialize<'a> for Value<'a> {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from<R: Read<'a>>(
        reader: &mut DepthAwareReaderGuard<'_, 'a, R>,
    ) -> Result<Self, SeaboredDeError>
    where
        Self: Sized + 'a,
    {
        let ib = InitialByte::cbor_deserialize_from(reader)?;
        let (mt, ai) = ib.mt_ai();

        let value_or_len = match ai.action()? {
            AdditionalInfoAction::DoNothing => CborIntegerValue::from(ai.0),
            AdditionalInfoAction::Uint8 => CborIntegerValue::from(reader.read_byte()?),
            AdditionalInfoAction::Uint16 => CborIntegerValue::from(reader.read_be_u16()?),
            AdditionalInfoAction::Uint32 => CborIntegerValue::from(reader.read_be_u32()?),
            AdditionalInfoAction::Uint64 => CborIntegerValue::from(reader.read_be_u64()?),
            AdditionalInfoAction::IndefiniteLenSeq => {
                return match mt {
                    MajorType::Bytes | MajorType::String => {
                        let mut seq = CborSequence::new_indefinite(mt);
                        let mut reader = reader.enter()?;
                        while reader.peek_byte()? != ib::consts::IB_BREAK {
                            // Appendix F kind 3 subkind 3, rule shared with the serde path
                            InitialByte::peek(&mut reader)?.expect_string_chunk(mt)?;
                            seq.push(Value::cbor_deserialize_from(&mut reader)?)
                        }
                        reader.advance(1)?; // Skip over the BREAK byte
                        Ok(Value::Sequence(seq))
                    }
                    MajorType::Array => {
                        let mut seq = CborSequence::new_indefinite(mt);
                        let mut reader = reader.enter()?;
                        while reader.peek_byte()? != ib::consts::IB_BREAK {
                            seq.push(Value::cbor_deserialize_from(&mut reader)?)
                        }
                        reader.advance(1)?; // Skip over the BREAK byte
                        Ok(Value::Sequence(seq))
                    }
                    MajorType::Map => {
                        let mut seq = CborSequence::new_indefinite(mt);
                        let mut reader = reader.enter()?;
                        while reader.peek_byte()? != ib::consts::IB_BREAK {
                            seq.push(<(Value, Value)>::cbor_deserialize_from(&mut reader)?)
                        }
                        reader.advance(1)?; // Skip over the BREAK byte
                        Ok(Value::Map(seq))
                    }
                    _ => {
                        return Err(SeaboredDeError::IncorrectMajorType {
                            actual: mt,
                            expected: &[
                                MajorType::Bytes,
                                MajorType::String,
                                MajorType::Array,
                                MajorType::Map,
                            ],
                        });
                    }
                };
            }
        };

        Ok(match mt {
            MajorType::Uint | MajorType::NegativeUint => Value::Integer(CborInteger {
                value: value_or_len,
                negative: mt == MajorType::NegativeUint,
            }),
            MajorType::SimpleValueOrFloat => match ib.0 {
                ib::consts::IB_FALSE => Value::Bool(false),
                ib::consts::IB_TRUE => Value::Bool(true),
                ib::consts::IB_NULL => Value::Null,
                ib::consts::IB_UNDEFINED => Value::Undefined,
                ib::consts::IB_SIMPLE_VALUE_NEXT_BYTE => {
                    // RFC 8949 §3.3: `f8` with an argument below 0x20 is not well-formed
                    // (Appendix F kind 3 subkind 2), so the decoder must not accept it
                    let value: u8 = value_or_len.try_into()?;
                    if value < 0x20 {
                        return Err(SeaboredDeError::NotWellFormed(
                            "two-byte simple value with an argument below 0x20",
                        ));
                    }
                    Value::SimpleValue(value)
                }
                ib::consts::IB_FLOAT_16 => {
                    Value::Float(f16::from_bits(value_or_len.try_into()?).into())
                }
                ib::consts::IB_FLOAT_32 => {
                    Value::Float(f32::from_bits(value_or_len.try_into()?).into())
                }
                ib::consts::IB_FLOAT_64 => Value::Float(f64::from_bits(value_or_len.into()).into()),
                ib::consts::IB_SIMPLE_VALUE..ib::consts::IB_FALSE => {
                    Value::SimpleValue(ib.0 - ib::consts::IB_SIMPLE_VALUE)
                }
                _ => return Err(SeaboredDeError::UnsupportedSimpleValue(ib.0)),
            },
            MajorType::Bytes => Value::Bytes(reader.read_slice(value_or_len.try_into()?)?),
            MajorType::String => {
                Value::String(match reader.read_slice(value_or_len.try_into()?)? {
                    Cow::Borrowed(s) => Cow::Borrowed(simdutf8::basic::from_utf8(s)?),
                    Cow::Owned(s) => Cow::Owned({
                        simdutf8::basic::from_utf8(&s)?;
                        // SAFETY: Checked above
                        unsafe { String::from_utf8_unchecked(s) }
                    }),
                })
            }
            MajorType::Array => Value::Sequence({
                let len: usize = value_or_len.try_into()?;
                // Wire length is not a reason to allocate: one item needs >= 1 byte, so the
                // remaining input bounds the pre-allocation (RFC 8949 §10).
                let mut seq = CborSequence::new_finite(
                    MajorType::Array,
                    len.min(reader.remaining().unwrap_or(len)),
                );
                let mut reader = reader.enter()?;
                for _ in 0..len {
                    seq.push(Value::cbor_deserialize_from(&mut reader)?)
                }
                seq
            }),
            MajorType::Map => Value::Map({
                let len: usize = value_or_len.try_into()?;
                // As arrays; a map entry costs at least two items
                let mut seq = CborSequence::new_finite(
                    MajorType::Map,
                    len.min(reader.remaining().unwrap_or(len) / 2),
                );
                let mut reader = reader.enter()?;
                for _ in 0..len {
                    seq.push(<(Value, Value)>::cbor_deserialize_from(&mut reader)?)
                }
                seq
            }),
            MajorType::Tagged => Value::Tagged((
                value_or_len,
                Box::new(Value::cbor_deserialize_from(&mut reader.enter()?)?),
            )),
        })
    }
}
