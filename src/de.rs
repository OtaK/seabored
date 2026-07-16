use half::f16;
use parsio::Read;
use std::borrow::Cow;

use crate::{
    Value,
    error::SeaboredDeError,
    ib::{self, AdditionalInfoAction, InitialByte},
    io::ReadExt as _,
    mt::MajorType,
    types::{CborInteger, CborIntegerValue, CborSequence},
};

pub trait CborDeserialize<'a> {
    fn cbor_deserialize_from<R: Read<'a>>(reader: &mut R) -> Result<Self, SeaboredDeError>
    where
        Self: Sized + 'a;
}

impl<'a, T1, T2> CborDeserialize<'a> for (T1, T2)
where
    T1: CborDeserialize<'a>,
    T2: CborDeserialize<'a>,
{
    #[inline(always)]
    fn cbor_deserialize_from<R: Read<'a>>(reader: &mut R) -> Result<Self, SeaboredDeError>
    where
        Self: Sized + 'a,
    {
        T1::cbor_deserialize_from(reader)
            .and_then(|t1| T2::cbor_deserialize_from(reader).map(|t2| (t1, t2)))
    }
}

impl<'a> CborDeserialize<'a> for Value<'a> {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from<R: Read<'a>>(reader: &mut R) -> Result<Self, SeaboredDeError>
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
                    MajorType::Bytes | MajorType::String | MajorType::Array => {
                        let mut seq = CborSequence::new_indefinite(mt);
                        while reader.peek_byte()? != ib::consts::IB_BREAK {
                            seq.push(Value::cbor_deserialize_from(reader)?)
                        }
                        reader.advance(1)?; // Skip over the BREAK byte
                        Ok(Value::Sequence(seq))
                    }
                    MajorType::Map => {
                        let mut seq = CborSequence::new_indefinite(mt);
                        while reader.peek_byte()? != ib::consts::IB_BREAK {
                            seq.push(<(Value, Value)>::cbor_deserialize_from(reader)?)
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
                    Value::SimpleValue(value_or_len.try_into()?)
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
                let len = value_or_len.try_into()?;
                let mut seq = CborSequence::new_finite(MajorType::Array, len);
                for _ in 0..len {
                    seq.push(Value::cbor_deserialize_from(reader)?)
                }
                seq
            }),
            MajorType::Map => Value::Map({
                let len = value_or_len.try_into()?;
                let mut seq = CborSequence::new_finite(MajorType::Map, len);
                for _ in 0..len {
                    seq.push(<(Value, Value)>::cbor_deserialize_from(reader)?)
                }
                seq
            }),
            MajorType::Tagged => Value::Tagged((
                value_or_len,
                Box::new(Value::cbor_deserialize_from(reader)?),
            )),
        })
    }
}
