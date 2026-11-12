use half::f16;
use parsio::{DepthAwareReader, DepthAwareReaderGuard, Read};
use std::borrow::Cow;

use crate::{
    Value,
    error::SeaboredDeError,
    ib::{self, AdditionalInfo, AdditionalInfoAction, InitialByte},
    io::ReadExt as _,
    mt::MajorType,
    profile::DecodeOptions,
    ser::CborSerialize as _,
    types::{CborBigInteger, CborInteger, CborIntegerValue, CborSequence, validate_float_encoding},
};

/// Reads a single CBOR item from a byte slice, up to
/// [`parsio::DepthAwareReader::DEFAULT_LIMIT`] nesting levels
#[inline]
pub fn from_bytes<'cbor, T: CborDeserialize<'cbor> + 'cbor>(
    bytes: &'cbor [u8],
) -> Result<T, SeaboredDeError> {
    from_bytes_with_limit(bytes, <DepthAwareReader<&'cbor [u8]>>::DEFAULT_LIMIT)
}

/// Reads a single CBOR item with explicit decoding options.
#[inline]
pub fn from_bytes_with_options<'cbor, T: CborDeserialize<'cbor> + 'cbor>(
    bytes: &'cbor [u8],
    options: DecodeOptions,
) -> Result<T, SeaboredDeError> {
    from_bytes_with_options_and_limit(
        bytes,
        options,
        <DepthAwareReader<&'cbor [u8]>>::DEFAULT_LIMIT,
    )
}

/// Reads a single CBOR item for `profile`; strict profiles check conformance.
#[inline]
pub fn from_bytes_with_profile<'cbor, T: CborDeserialize<'cbor> + 'cbor>(
    bytes: &'cbor [u8],
    profile: crate::profile::Profile,
) -> Result<T, SeaboredDeError> {
    from_bytes_with_options(
        bytes,
        DecodeOptions {
            profile,
            checking: profile.is_strict(),
        },
    )
}

/// Reads a single CBOR item and checks preferred-plus serialization (draft-09 §4.2).
#[inline]
pub fn from_bytes_preferred_plus<'cbor, T: CborDeserialize<'cbor> + 'cbor>(
    bytes: &'cbor [u8],
) -> Result<T, SeaboredDeError> {
    from_bytes_with_profile(bytes, crate::profile::Profile::PreferredPlus)
}

/// Reads a single CBOR item and checks deterministic serialization (draft-09 §5.2).
#[inline]
pub fn from_bytes_deterministic_checked<'cbor, T: CborDeserialize<'cbor> + 'cbor>(
    bytes: &'cbor [u8],
) -> Result<T, SeaboredDeError> {
    from_bytes_with_profile(bytes, crate::profile::Profile::Deterministic)
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

/// Reads a single CBOR item with explicit options and nesting limit.
#[inline]
pub fn from_bytes_with_options_and_limit<'cbor, T: CborDeserialize<'cbor> + 'cbor>(
    bytes: &'cbor [u8],
    options: DecodeOptions,
    limit: u16,
) -> Result<T, SeaboredDeError> {
    let mut reader = DepthAwareReader::from_reader_with_limit(bytes, limit);
    let mut guard = reader.enter()?;
    T::cbor_deserialize_from_with_profile(&mut guard, options)
}

pub trait CborDeserialize<'cbor> {
    fn cbor_deserialize_from_with_profile<R: Read<'cbor>>(
        reader: &mut DepthAwareReaderGuard<'_, R>,
        options: DecodeOptions,
    ) -> Result<Self, SeaboredDeError>
    where
        Self: Sized + 'cbor;

    /// General, non-checking decoding.
    #[inline]
    fn cbor_deserialize_from<R: Read<'cbor>>(
        reader: &mut DepthAwareReaderGuard<'_, R>,
    ) -> Result<Self, SeaboredDeError>
    where
        Self: Sized + 'cbor,
    {
        Self::cbor_deserialize_from_with_profile(reader, DecodeOptions::GENERAL)
    }
}

/// Skip one well-formed CBOR item without materialising it.
#[cfg_attr(feature = "inline-nontrivial", inline)]
pub fn skip_value<'de, R: Read<'de>>(
    reader: &mut DepthAwareReaderGuard<'_, R>,
) -> Result<(), SeaboredDeError> {
    skip_value_with_options(reader, DecodeOptions::GENERAL)
}

/// Skip one CBOR item, applying the requested serialization checks.
#[cfg_attr(feature = "inline-nontrivial", inline)]
pub fn skip_value_with_options<'de, R: Read<'de>>(
    reader: &mut DepthAwareReaderGuard<'_, R>,
    options: DecodeOptions,
) -> Result<(), SeaboredDeError> {
    let ib = InitialByte::peek(reader)?;
    if ib.raw() == ib::consts::IB_BREAK {
        return Err(SeaboredDeError::NotWellFormed("unexpected BREAK"));
    }

    let (mt, ai) = ib.mt_ai();

    if ai.is_indefinite() {
        if options.is_checking() {
            let what = match mt {
                MajorType::Bytes => "byte string",
                MajorType::String => "text string",
                MajorType::Array => "array",
                MajorType::Map => "map",
                _ => "indefinite item",
            };
            return Err(SeaboredDeError::NonPreferredEncoding { what });
        }

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
                    skip_value_with_options(&mut nested, options)?;
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
                    skip_value_with_options(&mut nested, options)?;
                    skip_value_with_options(&mut nested, options)?;
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

    if mt != MajorType::SimpleValueOrFloat {
        ai.ensure_shortest(len, options, "argument")?;
    }

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
                skip_value_with_options(&mut nested, options)?;
            }
            Ok(())
        }
        MajorType::Map => {
            let mut nested = reader.enter()?;
            let mut last_key: Option<Vec<u8>> = None;
            for _ in 0..len {
                if options.is_checking()
                    && options.profile == crate::profile::Profile::Deterministic
                {
                    let key: Value<'_> =
                        Value::cbor_deserialize_from_with_profile(&mut nested, options)?;
                    let mut key_bytes = Vec::new();
                    key.cbor_serialize_to_with_profile(&mut key_bytes, options.profile)
                        .map_err(|_| SeaboredDeError::NonPreferredEncoding {
                            what: "map key order",
                        })?;
                    if let Some(previous) = &last_key
                        && key_bytes <= *previous
                    {
                        return Err(SeaboredDeError::NonPreferredEncoding {
                            what: "map key order",
                        });
                    }
                    last_key = Some(key_bytes);
                } else {
                    skip_value_with_options(&mut nested, options)?;
                }
                skip_value_with_options(&mut nested, options)?;
            }
            Ok(())
        }
        MajorType::Tagged => {
            let mut nested = reader.enter()?;
            if options.is_checking() && (len == 2 || len == 3) {
                skip_bignum_payload(&mut nested, options)
            } else {
                skip_value_with_options(&mut nested, options)
            }
        }
        MajorType::SimpleValueOrFloat => {
            // RFC 8949 §3.3: `f8` with an argument below 0x20 is not well-formed
            if ai.raw() == 24 && len < 0x20 {
                return Err(SeaboredDeError::NotWellFormed(
                    "two-byte simple value with an argument below 0x20",
                ));
            }
            validate_float_encoding(ib, len, options)
        }
    }
}

/// Validates and skips a tag 2/3 payload without materialising it.
///
/// Checking requires a definite byte string, more than 8 significant bytes, and no
/// leading zero (draft-09 §4.1 item 5).
fn skip_bignum_payload<'de, R: Read<'de>>(
    reader: &mut DepthAwareReaderGuard<'_, R>,
    options: DecodeOptions,
) -> Result<(), SeaboredDeError> {
    let ib = InitialByte::peek(reader)?;
    let (mt, ai) = ib.mt_ai();
    if mt != MajorType::Bytes || ai.is_indefinite() {
        return Err(SeaboredDeError::NonPreferredEncoding {
            what: "bignum payload",
        });
    }

    reader.advance(1)?;
    let len = ai.length(reader)?.ok_or(SeaboredDeError::NotWellFormed(
        "indefinite length reached in a bignum payload",
    ))?;
    ai.ensure_shortest(len, options, "bignum payload")?;
    let len: usize = len.try_into()?;
    if len <= 8 {
        return Err(SeaboredDeError::NonPreferredEncoding { what: "bignum" });
    }

    let bytes = reader.read_slice(len)?;
    if bytes[0] == 0 {
        return Err(SeaboredDeError::NonPreferredEncoding { what: "bignum" });
    }
    Ok(())
}

impl<'a, T1, T2> CborDeserialize<'a> for (T1, T2)
where
    T1: CborDeserialize<'a>,
    T2: CborDeserialize<'a>,
{
    #[inline(always)]
    fn cbor_deserialize_from_with_profile<R: Read<'a>>(
        reader: &mut DepthAwareReaderGuard<'_, R>,
        options: DecodeOptions,
    ) -> Result<Self, SeaboredDeError>
    where
        Self: Sized + 'a,
    {
        T1::cbor_deserialize_from_with_profile(reader, options).and_then(|t1| {
            T2::cbor_deserialize_from_with_profile(reader, options).map(|t2| (t1, t2))
        })
    }
}

impl<'a> CborDeserialize<'a> for Value<'a> {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from_with_profile<R: Read<'a>>(
        reader: &mut DepthAwareReaderGuard<'_, R>,
        options: DecodeOptions,
    ) -> Result<Self, SeaboredDeError>
    where
        Self: Sized + 'a,
    {
        let ib = InitialByte::cbor_deserialize_from(reader)?;
        let (mt, ai) = ib.mt_ai();

        let value_or_len = match ai.action()? {
            AdditionalInfoAction::DoNothing => CborIntegerValue::from(ai.raw()),
            AdditionalInfoAction::Uint8 => CborIntegerValue::from(reader.read_byte()?),
            AdditionalInfoAction::Uint16 => CborIntegerValue::from(reader.read_be_u16()?),
            AdditionalInfoAction::Uint32 => CborIntegerValue::from(reader.read_be_u32()?),
            AdditionalInfoAction::Uint64 => CborIntegerValue::from(reader.read_be_u64()?),
            AdditionalInfoAction::IndefiniteLenSeq => {
                return decode_value_indefinite(reader, mt, options);
            }
        };

        match mt {
            MajorType::Uint | MajorType::NegativeUint => {
                decode_value_integer(mt, ai, value_or_len, options)
            }
            MajorType::SimpleValueOrFloat => decode_value_simple(ib, value_or_len, options),
            MajorType::Bytes => decode_value_bytes(reader, ai, value_or_len, options),
            MajorType::String => decode_value_string(reader, ai, value_or_len, options),
            MajorType::Array => decode_value_array(reader, ai, value_or_len, options),
            MajorType::Map => decode_value_map(reader, ai, value_or_len, options),
            MajorType::Tagged => decode_value_tagged(reader, ai, value_or_len, options),
        }
    }
}

#[inline(never)]
fn decode_value_indefinite<'a, R: Read<'a>>(
    reader: &mut DepthAwareReaderGuard<'_, R>,
    mt: MajorType,
    options: DecodeOptions,
) -> Result<Value<'a>, SeaboredDeError> {
    if options.is_checking() {
        let what = match mt {
            MajorType::Bytes => "byte string",
            MajorType::String => "text string",
            MajorType::Array => "array",
            MajorType::Map => "map",
            _ => "indefinite item",
        };
        return Err(SeaboredDeError::NonPreferredEncoding { what });
    }

    match mt {
        MajorType::Bytes | MajorType::String => {
            let mut seq = CborSequence::new_indefinite(mt);
            let mut reader = reader.enter()?;
            while reader.peek_byte()? != ib::consts::IB_BREAK {
                InitialByte::peek(&mut reader)?.expect_string_chunk(mt)?;
                seq.push(Value::cbor_deserialize_from_with_profile(
                    &mut reader,
                    options,
                )?)
            }
            reader.advance(1)?;
            Ok(Value::Sequence(seq))
        }
        MajorType::Array => {
            let mut seq = CborSequence::new_indefinite(mt);
            let mut reader = reader.enter()?;
            while reader.peek_byte()? != ib::consts::IB_BREAK {
                seq.push(Value::cbor_deserialize_from_with_profile(
                    &mut reader,
                    options,
                )?)
            }
            reader.advance(1)?;
            Ok(Value::Sequence(seq))
        }
        MajorType::Map => {
            let mut seq = CborSequence::new_indefinite(mt);
            let mut reader = reader.enter()?;
            while reader.peek_byte()? != ib::consts::IB_BREAK {
                seq.push(<(Value, Value)>::cbor_deserialize_from_with_profile(
                    &mut reader,
                    options,
                )?)
            }
            reader.advance(1)?;
            Ok(Value::Map(seq))
        }
        _ => Err(SeaboredDeError::IncorrectMajorType {
            actual: mt,
            expected: &[
                MajorType::Bytes,
                MajorType::String,
                MajorType::Array,
                MajorType::Map,
            ],
        }),
    }
}

#[inline(never)]
fn decode_value_integer<'a>(
    mt: MajorType,
    ai: AdditionalInfo,
    value_or_len: CborIntegerValue,
    options: DecodeOptions,
) -> Result<Value<'a>, SeaboredDeError> {
    ai.ensure_shortest(u64::from(value_or_len), options, "integer")?;
    Ok(Value::Integer(CborInteger {
        value: value_or_len,
        negative: mt == MajorType::NegativeUint,
    }))
}

#[inline(never)]
fn decode_value_simple<'a>(
    ib: InitialByte,
    value_or_len: CborIntegerValue,
    options: DecodeOptions,
) -> Result<Value<'a>, SeaboredDeError> {
    match ib.0 {
        ib::consts::IB_FALSE => Ok(Value::Bool(false)),
        ib::consts::IB_TRUE => Ok(Value::Bool(true)),
        ib::consts::IB_NULL => Ok(Value::Null),
        ib::consts::IB_UNDEFINED => Ok(Value::Undefined),
        ib::consts::IB_SIMPLE_VALUE_NEXT_BYTE => {
            let value: u8 = value_or_len.try_into()?;
            if value < 0x20 {
                return Err(SeaboredDeError::NotWellFormed(
                    "two-byte simple value with an argument below 0x20",
                ));
            }
            Ok(Value::SimpleValue(value))
        }
        ib::consts::IB_FLOAT_16 => {
            let bits: u16 = value_or_len.try_into()?;
            validate_float_encoding(ib, bits as u64, options)?;
            Ok(Value::Float(f16::from_bits(bits).into()))
        }
        ib::consts::IB_FLOAT_32 => {
            let bits: u32 = value_or_len.try_into()?;
            validate_float_encoding(ib, bits as u64, options)?;
            Ok(Value::Float(f32::from_bits(bits).into()))
        }
        ib::consts::IB_FLOAT_64 => {
            let bits: u64 = value_or_len.into();
            validate_float_encoding(ib, bits, options)?;
            Ok(Value::Float(f64::from_bits(bits).into()))
        }
        ib::consts::IB_SIMPLE_VALUE..ib::consts::IB_FALSE => {
            Ok(Value::SimpleValue(ib.0 - ib::consts::IB_SIMPLE_VALUE))
        }
        _ => Err(SeaboredDeError::UnsupportedSimpleValue(ib.0)),
    }
}

#[inline(never)]
fn decode_value_bytes<'a, R: Read<'a>>(
    reader: &mut DepthAwareReaderGuard<'_, R>,
    ai: AdditionalInfo,
    value_or_len: CborIntegerValue,
    options: DecodeOptions,
) -> Result<Value<'a>, SeaboredDeError> {
    ai.ensure_shortest(u64::from(value_or_len), options, "byte string length")?;
    Ok(Value::Bytes(reader.read_slice(value_or_len.try_into()?)?))
}

#[inline(never)]
fn decode_value_string<'a, R: Read<'a>>(
    reader: &mut DepthAwareReaderGuard<'_, R>,
    ai: AdditionalInfo,
    value_or_len: CborIntegerValue,
    options: DecodeOptions,
) -> Result<Value<'a>, SeaboredDeError> {
    ai.ensure_shortest(u64::from(value_or_len), options, "text string length")?;
    Ok(Value::String(
        match reader.read_slice(value_or_len.try_into()?)? {
            Cow::Borrowed(s) => Cow::Borrowed(simdutf8::basic::from_utf8(s)?),
            Cow::Owned(s) => Cow::Owned({
                simdutf8::basic::from_utf8(&s)?;
                // SAFETY: Checked above
                unsafe { String::from_utf8_unchecked(s) }
            }),
        },
    ))
}

#[inline(never)]
fn decode_value_tagged<'a, R: Read<'a>>(
    reader: &mut DepthAwareReaderGuard<'_, R>,
    ai: AdditionalInfo,
    value_or_len: CborIntegerValue,
    options: DecodeOptions,
) -> Result<Value<'a>, SeaboredDeError> {
    let tag: u64 = value_or_len.into();
    ai.ensure_shortest(tag, options, "tag")?;
    let payload = Value::cbor_deserialize_from_with_profile(&mut reader.enter()?, options)?;
    if tag == 2 || tag == 3 {
        return bignum_or_tagged(value_or_len, payload, options);
    }
    Ok(Value::Tagged((value_or_len, Box::new(payload))))
}

/// Interprets a tag 2/3 payload. A byte string becomes [`Value::Integer`] when it fits major
/// types 0/1 and [`Value::BigInteger`] otherwise; anything else stays tagged unless checking.
pub(crate) fn bignum_or_tagged<'a>(
    tag: CborIntegerValue,
    payload: Value<'a>,
    options: DecodeOptions,
) -> Result<Value<'a>, SeaboredDeError> {
    let bytes: Cow<'a, [u8]> = match payload {
        Value::Bytes(bytes) => bytes,
        Value::Sequence(seq) if seq.mt == MajorType::Bytes => {
            if !seq.iter().all(|chunk| matches!(chunk, Value::Bytes(_))) {
                return bignum_unsupported(tag, Value::Sequence(seq), options);
            }
            let mut bytes = Vec::new();
            for chunk in seq {
                if let Value::Bytes(chunk) = chunk {
                    bytes.extend_from_slice(chunk.as_ref());
                }
            }
            Cow::Owned(bytes)
        }
        other => return bignum_unsupported(tag, other, options),
    };

    if options.is_checking() && (bytes.is_empty() || bytes[0] == 0 || bytes.len() <= 8) {
        return Err(SeaboredDeError::NonPreferredEncoding { what: "bignum" });
    }

    // RFC 8949 §3.4.3: tag 2 is `n`, tag 3 is `-1 - n`. An empty payload is `n = 0`,
    // so tag 2 is 0 and tag 3 is -1 (not "zero" for both). draft-09 §4.2 now states
    // this explicitly; the published -09 text predates cbor-wg PR #101, which fixed
    // the sentence and Appendix G.2.
    let negative = u64::from(tag) == 3;
    if bytes.len() <= 8 {
        let mut argument = 0u64;
        for &byte in bytes.iter() {
            argument = (argument << 8) | u64::from(byte);
        }
        Ok(Value::Integer(CborInteger {
            value: argument.into(),
            negative,
        }))
    } else {
        Ok(Value::BigInteger(CborBigInteger::new(negative, bytes)))
    }
}

fn bignum_unsupported<'a>(
    tag: CborIntegerValue,
    payload: Value<'a>,
    options: DecodeOptions,
) -> Result<Value<'a>, SeaboredDeError> {
    if options.is_checking() {
        return Err(SeaboredDeError::NonPreferredEncoding {
            what: "bignum payload",
        });
    }
    Ok(Value::Tagged((tag, Box::new(payload))))
}

#[inline(never)]
fn decode_value_array<'a, R: Read<'a>>(
    reader: &mut DepthAwareReaderGuard<'_, R>,
    ai: AdditionalInfo,
    value_or_len: CborIntegerValue,
    options: DecodeOptions,
) -> Result<Value<'a>, SeaboredDeError> {
    ai.ensure_shortest(u64::from(value_or_len), options, "array length")?;
    let len: usize = value_or_len.try_into()?;
    let mut seq =
        CborSequence::new_finite(MajorType::Array, len.min(reader.remaining().unwrap_or(len)));
    let mut reader = reader.enter()?;
    for _ in 0..len {
        seq.push(Value::cbor_deserialize_from_with_profile(
            &mut reader,
            options,
        )?)
    }
    Ok(Value::Sequence(seq))
}

fn decode_value_map<'a, R: Read<'a>>(
    reader: &mut DepthAwareReaderGuard<'_, R>,
    ai: AdditionalInfo,
    value_or_len: CborIntegerValue,
    options: DecodeOptions,
) -> Result<Value<'a>, SeaboredDeError> {
    ai.ensure_shortest(u64::from(value_or_len), options, "map length")?;
    let len: usize = value_or_len.try_into()?;
    let mut seq = CborSequence::new_finite(
        MajorType::Map,
        len.min(reader.remaining().unwrap_or(len) / 2),
    );
    let mut reader = reader.enter()?;
    let mut last_key: Option<Vec<u8>> = None;
    for _ in 0..len {
        let key = Value::cbor_deserialize_from_with_profile(&mut reader, options)?;
        if options.is_checking() && options.profile == crate::profile::Profile::Deterministic {
            let mut key_bytes = Vec::new();
            key.cbor_serialize_to_with_profile(&mut key_bytes, options.profile)
                .map_err(|_| SeaboredDeError::NonPreferredEncoding {
                    what: "map key order",
                })?;
            if let Some(previous) = &last_key
                && key_bytes <= *previous
            {
                return Err(SeaboredDeError::NonPreferredEncoding {
                    what: "map key order",
                });
            }
            last_key = Some(key_bytes);
        }
        let value = Value::cbor_deserialize_from_with_profile(&mut reader, options)?;
        seq.push((key, value));
    }
    Ok(Value::Map(seq))
}
