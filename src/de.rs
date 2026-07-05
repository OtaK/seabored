use half::f16;
use parsio::Read;
use std::borrow::Cow;
use winnow::{Parser, error::ParserError as _};

use crate::{
    SyntacticValue, Value,
    error::SeaboredDeError,
    ib::{self, AdditionalInfoAction, InitialByte},
    io::ReadExt as _,
    mt::MajorType,
    types::{CborFloat, CborInteger, CborIntegerValue, CborSequence},
};

pub trait CborDeserialize<'a> {
    fn cbor_deserialize_from<R: Read<'a>>(reader: &mut R) -> Result<Self, SeaboredDeError<'a>>
    where
        Self: Sized + 'a;
}

impl<'a, T1, T2> CborDeserialize<'a> for (T1, T2)
where
    T1: CborDeserialize<'a>,
    T2: CborDeserialize<'a>,
{
    #[inline(always)]
    fn cbor_deserialize_from<R: Read<'a>>(reader: &mut R) -> Result<Self, SeaboredDeError<'a>>
    where
        Self: Sized + 'a,
    {
        T1::cbor_deserialize_from(reader)
            .and_then(|t1| T2::cbor_deserialize_from(reader).map(|t2| (t1, t2)))
    }
}

impl<'a> CborDeserialize<'a> for Value<'a> {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from<R: Read<'a>>(reader: &mut R) -> Result<Self, SeaboredDeError<'a>>
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

// TODO: Restore the winnow implementation by implementing the Stream trait manually over `io::Read`
// It's a bit faster, see
// log/seabored/value-de   time:   [5.5742 ms 5.6268 ms 5.6815 ms]
//                         thrpt:  [236.31 MiB/s 238.61 MiB/s 240.86 MiB/s]
//              change:
//                         time:   [+46.050% +47.723% +49.469%] (p = 0.00 < 0.05)
//                         thrpt:  [-33.097% -32.306% -31.530%]
//        Performance has regressed.

type Stream<'i> = &'i [u8];
type StreamErr<'i> = winnow::error::ErrMode<winnow::error::ContextError<Stream<'i>>>;

#[inline(always)]
fn read_ib<'i>(input: &mut Stream<'i>) -> Result<InitialByte, SeaboredDeError<'i>> {
    winnow::binary::be_u8::<_, StreamErr<'i>>
        .map(InitialByte)
        .parse_next(input)
        .map_err(Into::into)
}

#[inline(always)]
fn cut_err<'i, T>(input: &mut Stream<'i>) -> Result<T, SeaboredDeError<'i>> {
    Err(winnow::error::ErrMode::Cut(winnow::error::ContextError::from_input(input)).into())
}

#[inline(always)]
fn take_bytes<'i>(
    input: &mut Stream<'i>,
    len: usize,
) -> winnow::ModalResult<SyntacticValue<'i>, winnow::error::ContextError<&'i [u8]>> {
    winnow::token::take(len)
        .map(|bytes: &[u8]| Value::Bytes(Cow::Borrowed(bytes)).into())
        .parse_next(input)
}

#[inline(always)]
fn take_string<'i>(
    input: &mut Stream<'i>,
    len: usize,
) -> winnow::ModalResult<SyntacticValue<'i>, winnow::error::ContextError<&'i [u8]>> {
    winnow::token::take(len)
        .try_map::<_, _, simdutf8::basic::Utf8Error>(|bytes: &[u8]| {
            Ok(Value::String(Cow::Borrowed(simdutf8::basic::from_utf8(bytes)?)).into())
        })
        .parse_next(input)
}

#[inline(always)]
fn take_array<'i>(
    input: &mut Stream<'i>,
    len: usize,
) -> Result<SyntacticValue<'i>, SeaboredDeError<'i>> {
    let mut seq = CborSequence::new_finite(MajorType::Array, len);
    for _ in 0..len {
        let SyntacticValue::Value(value) = parse_value_inner(input)? else {
            return cut_err(input);
        };
        seq.push(value);
    }
    Ok(Value::Sequence(seq).into())
}

#[inline(always)]
fn take_map<'i>(
    input: &mut Stream<'i>,
    len: usize,
) -> Result<SyntacticValue<'i>, SeaboredDeError<'i>> {
    let mut seq = CborSequence::new_finite(MajorType::Map, len);
    for _ in 0..len {
        let (SyntacticValue::Value(key), SyntacticValue::Value(value)) =
            parse_value_inner(input)
                .and_then(|key| parse_value_inner(input).map(|value| (key, value)))?
        else {
            return cut_err(input);
        };

        seq.push((key, value));
    }

    Ok(Value::Map(seq).into())
}

#[inline(always)]
fn take_sequence<'i>(
    input: &mut Stream<'i>,
    mt: MajorType,
) -> Result<SyntacticValue<'i>, SeaboredDeError<'i>> {
    let mut seq = CborSequence::new_indefinite(mt);

    loop {
        let value = parse_value_inner(input)?;

        match value {
            SyntacticValue::Value(value) => {
                if mt != MajorType::Array && mt != value.mt() {
                    return cut_err(input);
                }

                seq.push(value);
            }
            SyntacticValue::Break => {
                break;
            }
        }
    }

    Ok(Value::Sequence(seq).into())
}

#[inline(always)]
fn take_sequence_map<'i>(
    input: &mut Stream<'i>,
) -> Result<SyntacticValue<'i>, SeaboredDeError<'i>> {
    let mut map = CborSequence::new_indefinite(MajorType::Map);

    loop {
        let key = parse_value_inner(input)?;

        match key {
            SyntacticValue::Value(key) => {
                let value = parse_value_inner(input)?;

                match value {
                    SyntacticValue::Value(value) => {
                        map.push((key, value));
                    }
                    SyntacticValue::Break => {
                        return cut_err(input);
                    }
                }
            }
            SyntacticValue::Break => {
                break;
            }
        }
    }

    Ok(Value::Map(map).into())
}

#[inline(always)]
pub fn parse_value<'i>(input: &mut Stream<'i>) -> Result<Value<'i>, SeaboredDeError<'i>> {
    let syn_value = parse_value_inner(input)?;

    // Check if we have leftovers in the buffer
    if !input.is_empty() {
        return Err(
            winnow::error::ErrMode::Cut(winnow::error::ContextError::from_input(input)).into(),
        );
    }

    let SyntacticValue::Value(value) = syn_value else {
        return Err(SeaboredDeError::Incomplete(winnow::error::Needed::Unknown));
    };

    Ok(value)
}

#[inline(always)]
pub fn parse_value_streaming<'i>(input: &mut Stream<'i>) -> Result<Value<'i>, SeaboredDeError<'i>> {
    let SyntacticValue::Value(value) = parse_value_inner(input)? else {
        return Err(SeaboredDeError::Incomplete(winnow::error::Needed::Unknown));
    };

    Ok(value)
}

#[inline(always)]
fn parse_value_inner<'i>(
    input: &mut Stream<'i>,
) -> Result<SyntacticValue<'i>, SeaboredDeError<'i>> {
    let ib = read_ib(input)?;

    if ib.0 == ib::consts::IB_BREAK {
        return Ok(SyntacticValue::Break);
    }

    let (mt, ai) = ib.mt_ai();

    let ai_action = ai
        .action()
        .map_err(|_| cut_err::<AdditionalInfoAction>(input).unwrap_err())?;

    let value_or_len = match ai_action {
        AdditionalInfoAction::DoNothing => CborIntegerValue::from(ai.0),
        AdditionalInfoAction::Uint8 => winnow::binary::be_u8::<_, StreamErr<'i>>
            .map(CborIntegerValue::from)
            .parse_next(input)?,
        AdditionalInfoAction::Uint16 => winnow::binary::be_u16::<_, StreamErr<'i>>
            .map(CborIntegerValue::from)
            .parse_next(input)?,
        AdditionalInfoAction::Uint32 => winnow::binary::be_u32::<_, StreamErr<'i>>
            .map(CborIntegerValue::from)
            .parse_next(input)?,
        AdditionalInfoAction::Uint64 => winnow::binary::be_u64::<_, StreamErr<'i>>
            .map(CborIntegerValue::from)
            .parse_next(input)?,
        AdditionalInfoAction::IndefiniteLenSeq => {
            return match mt {
                MajorType::String | MajorType::Bytes | MajorType::Array => {
                    Ok(take_sequence(input, mt)?)
                }
                MajorType::Map => Ok(take_sequence_map(input)?),
                _ => cut_err(input),
            };
        }
    };

    Ok(match mt {
        MajorType::Uint | MajorType::NegativeUint => Value::Integer(CborInteger {
            value: value_or_len,
            negative: mt == MajorType::NegativeUint,
        }),
        MajorType::SimpleValueOrFloat => match ib.0 {
            value @ ib::consts::IB_SIMPLE_VALUE..ib::consts::IB_FALSE => {
                Value::SimpleValue(value - ib::consts::IB_SIMPLE_VALUE)
            }
            ib::consts::IB_FALSE => Value::Bool(false),
            ib::consts::IB_TRUE => Value::Bool(true),
            ib::consts::IB_NULL => Value::Null,
            ib::consts::IB_UNDEFINED => Value::Undefined,
            ib::consts::IB_SIMPLE_VALUE_NEXT_BYTE => Value::SimpleValue(value_or_len.try_into()?),
            ib::consts::IB_FLOAT_16 => {
                Value::Float(CborFloat::from(f16::from_bits(value_or_len.try_into()?)))
            }
            ib::consts::IB_FLOAT_32 => {
                Value::Float(CborFloat::from(f32::from_bits(value_or_len.try_into()?)))
            }
            ib::consts::IB_FLOAT_64 => {
                Value::Float(CborFloat::from(f64::from_bits(value_or_len.into())))
            }
            _ => return cut_err(input),
        },
        MajorType::Bytes => {
            let SyntacticValue::Value(v) = take_bytes(input, value_or_len.try_into()?)? else {
                return cut_err(input);
            };
            v
        }
        MajorType::String => {
            let SyntacticValue::Value(v) = take_string(input, value_or_len.try_into()?)? else {
                return cut_err(input);
            };
            v
        }
        MajorType::Array => {
            let SyntacticValue::Value(v) = take_array(input, value_or_len.try_into()?)? else {
                return cut_err(input);
            };
            v
        }
        MajorType::Map => {
            let SyntacticValue::Value(v) = take_map(input, value_or_len.try_into()?)? else {
                return cut_err(input);
            };
            v
        }
        MajorType::Tagged => {
            let SyntacticValue::Value(v) = parse_value_inner(input)? else {
                return cut_err(input);
            };
            Value::Tagged((value_or_len, Box::new(v)))
        }
    }
    .into())
}
