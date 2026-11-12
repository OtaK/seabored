use crate::{
    Value,
    error::SeaboredSerError,
    ib,
    mt::MajorType,
    profile::Profile,
    types::{CborBigInteger, CborInteger, CborIntegerValue, CborSequence, IB_LIMIT},
};

use parsio::Write;

pub trait CborSerialize {
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError>;

    /// Encode with an explicit serialization profile.
    ///
    /// Implementations that cannot honor a strict profile must override this and return
    /// [`SeaboredSerError::ProfileUnsupported`]; the default only supports
    /// [`Profile::General`].
    #[inline]
    fn cbor_serialize_to_with_profile<W: Write>(
        &self,
        writer: &mut W,
        profile: Profile,
    ) -> Result<usize, SeaboredSerError> {
        if profile.is_strict() {
            return Err(SeaboredSerError::ProfileUnsupported {
                profile,
                what: core::any::type_name::<Self>(),
            });
        }
        self.cbor_serialize_to(writer)
    }

    #[inline]
    fn cbor_serialize(&self) -> Result<Vec<u8>, SeaboredSerError> {
        let mut buf = vec![];
        self.cbor_serialize_to(&mut buf)?;
        Ok(buf)
    }

    #[inline]
    fn cbor_serialize_with_profile(&self, profile: Profile) -> Result<Vec<u8>, SeaboredSerError> {
        let mut buf = vec![];
        self.cbor_serialize_to_with_profile(&mut buf, profile)?;
        Ok(buf)
    }
}

/// Write a definite-length CBOR preamble for `mt`.
///
/// `len` is the container length for bytes/strings/arrays/maps, or the tag
/// number for `MajorType::Tagged`. Integer and simple-value major types are
/// rejected: they have no complex preamble.
#[cfg_attr(feature = "inline-nontrivial", inline)]
pub fn write_preamble<W: Write>(
    mt: MajorType,
    len: u64,
    writer: &mut W,
) -> Result<usize, SeaboredSerError> {
    match mt {
        MajorType::Bytes
        | MajorType::String
        | MajorType::Array
        | MajorType::Map
        | MajorType::Tagged => {
            CborIntegerValue::from(len).serialize_complex_mt_preamble(mt, writer)
        }
        _ => Err(SeaboredSerError::UnexpectedPreambleMajorType(mt)),
    }
}

/// Serialize `value` to `writer` with an explicit serialization profile.
///
/// Shorthand for [`CborSerialize::cbor_serialize_to_with_profile`].
#[cfg_attr(feature = "inline-nontrivial", inline)]
pub fn to_writer_with_profile<W: Write, T: CborSerialize + ?Sized>(
    writer: &mut W,
    value: &T,
    profile: Profile,
) -> Result<usize, SeaboredSerError> {
    value.cbor_serialize_to_with_profile(writer, profile)
}

/// Serialize `value` to a `Vec` with an explicit serialization profile.
///
/// Shorthand for [`CborSerialize::cbor_serialize_with_profile`].
#[cfg_attr(feature = "inline-nontrivial", inline)]
pub fn to_vec_with_profile<T: CborSerialize + ?Sized>(
    value: &T,
    profile: Profile,
) -> Result<Vec<u8>, SeaboredSerError> {
    value.cbor_serialize_with_profile(profile)
}

/// Serialize `value` with preferred-plus serialization (draft-09 §4.1).
#[inline]
pub fn to_vec_preferred_plus<T: CborSerialize + ?Sized>(
    value: &T,
) -> Result<Vec<u8>, SeaboredSerError> {
    to_vec_with_profile(value, Profile::PreferredPlus)
}

/// Serialize `value` with deterministic serialization (draft-09 §5.1).
#[inline]
pub fn to_vec_deterministic<T: CborSerialize + ?Sized>(
    value: &T,
) -> Result<Vec<u8>, SeaboredSerError> {
    to_vec_with_profile(value, Profile::Deterministic)
}

impl<T1, T2> CborSerialize for (T1, T2)
where
    T1: CborSerialize,
    T2: CborSerialize,
{
    #[inline(always)]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        Ok(self.0.cbor_serialize_to(writer)? + self.1.cbor_serialize_to(writer)?)
    }

    #[inline(always)]
    fn cbor_serialize_to_with_profile<W: Write>(
        &self,
        writer: &mut W,
        profile: Profile,
    ) -> Result<usize, SeaboredSerError> {
        Ok(self.0.cbor_serialize_to_with_profile(writer, profile)?
            + self.1.cbor_serialize_to_with_profile(writer, profile)?)
    }
}

impl CborSerialize for Value<'_> {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        self.cbor_serialize_to_with_profile(writer, Profile::General)
    }

    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to_with_profile<W: Write>(
        &self,
        writer: &mut W,
        profile: Profile,
    ) -> Result<usize, SeaboredSerError> {
        match self {
            Value::Integer(cbor_integer) => {
                cbor_integer.cbor_serialize_to_with_profile(writer, profile)
            }
            Value::BigInteger(big) => serialize_value_bignum(big, writer, profile),
            Value::Float(cbor_float) => cbor_float.cbor_serialize_to_with_profile(writer, profile),
            Value::Bytes(cow) => {
                CborIntegerValue::serialize_inline_bytes(cow, MajorType::Bytes, writer)
            }
            Value::String(cow) => {
                CborIntegerValue::serialize_inline_bytes(cow.as_bytes(), MajorType::String, writer)
            }
            Value::Sequence(seq) => serialize_value_sequence(seq, writer, profile),
            Value::Map(seq) => serialize_value_map(seq, writer, profile),
            Value::Tagged((tag, value)) => serialize_value_tagged(*tag, value, writer, profile),
            Value::SimpleValue(value) => serialize_simple_value(*value, writer),
            Value::Bool(true) => Ok(writer.write(&[ib::consts::IB_TRUE])?),
            Value::Bool(false) => Ok(writer.write(&[ib::consts::IB_FALSE])?),
            Value::Null => Ok(writer.write(&[ib::consts::IB_NULL])?),
            Value::Undefined => Ok(writer.write(&[ib::consts::IB_UNDEFINED])?),
        }
    }
}

#[cfg_attr(feature = "inline-nontrivial", inline)]
fn serialize_simple_value<W: Write>(value: u8, writer: &mut W) -> Result<usize, SeaboredSerError> {
    // 20..=23 mean false/true/null/undefined and 24..=31 are reserved (RFC 8949 §3.3)
    if (20..32).contains(&value) {
        return Err(SeaboredSerError::ReservedSimpleValue(value));
    }
    if value <= IB_LIMIT {
        Ok(writer.write(&[ib::consts::IB_SIMPLE_VALUE | value])?)
    } else {
        Ok(writer.write(&[ib::consts::IB_SIMPLE_VALUE_NEXT_BYTE, value])?)
    }
}

#[cfg_attr(feature = "inline-nontrivial", inline)]
fn serialize_value_sequence<W: Write>(
    seq: &CborSequence<Value<'_>>,
    writer: &mut W,
    profile: Profile,
) -> Result<usize, SeaboredSerError> {
    if !profile.is_strict() {
        return seq.cbor_serialize_to(writer);
    }

    match seq.mt {
        MajorType::Array => {
            let mut written = CborIntegerValue::from(seq.len())
                .serialize_complex_mt_preamble(MajorType::Array, writer)?;
            for value in seq.iter() {
                written += value.cbor_serialize_to_with_profile(writer, profile)?;
            }
            Ok(written)
        }
        MajorType::Bytes | MajorType::String => {
            let mut bytes = Vec::new();
            for chunk in seq.iter() {
                match (seq.mt, chunk) {
                    (MajorType::Bytes, Value::Bytes(chunk)) => {
                        bytes.extend_from_slice(chunk.as_ref());
                    }
                    (MajorType::String, Value::String(chunk)) => {
                        bytes.extend_from_slice(chunk.as_bytes());
                    }
                    _ => {
                        return Err(SeaboredSerError::ProfileUnsupported {
                            profile,
                            what: "string chunk in a Value sequence",
                        });
                    }
                }
            }
            CborIntegerValue::serialize_inline_bytes(&bytes, seq.mt, writer)
        }
        _ => Err(SeaboredSerError::ProfileUnsupported {
            profile,
            what: "Value::Sequence major type",
        }),
    }
}

#[cfg_attr(feature = "inline-nontrivial", inline)]
fn serialize_value_map<W: Write>(
    seq: &CborSequence<(Value<'_>, Value<'_>)>,
    writer: &mut W,
    profile: Profile,
) -> Result<usize, SeaboredSerError> {
    if !profile.is_strict() {
        return seq.cbor_serialize_to(writer);
    }
    if seq.mt != MajorType::Map {
        return Err(SeaboredSerError::ProfileUnsupported {
            profile,
            what: "Value::Map major type",
        });
    }

    if profile == Profile::PreferredPlus {
        let mut written = CborIntegerValue::from(seq.len())
            .serialize_complex_mt_preamble(MajorType::Map, writer)?;
        for (key, value) in seq.iter() {
            written += key.cbor_serialize_to_with_profile(writer, profile)?;
            written += value.cbor_serialize_to_with_profile(writer, profile)?;
        }
        return Ok(written);
    }

    let mut entries = Vec::with_capacity(seq.len());
    for (key, value) in seq.iter() {
        let mut key_bytes = Vec::new();
        key.cbor_serialize_to_with_profile(&mut key_bytes, profile)?;
        entries.push((key_bytes, value));
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0));

    let mut written = CborIntegerValue::from(entries.len())
        .serialize_complex_mt_preamble(MajorType::Map, writer)?;
    for (key_bytes, value) in entries {
        written += writer.write(&key_bytes)?;
        written += value.cbor_serialize_to_with_profile(writer, profile)?;
    }
    Ok(written)
}

/// Writes a normalized bignum as tag 2/3. Strict profiles reject one that fits major types 0/1,
/// which would have to be an integer instead.
#[cfg_attr(feature = "inline-nontrivial", inline)]
fn serialize_value_bignum<W: Write>(
    big: &CborBigInteger<'_>,
    writer: &mut W,
    profile: Profile,
) -> Result<usize, SeaboredSerError> {
    let magnitude = big.magnitude();
    if profile.is_strict() && magnitude.len() <= 8 {
        return Err(SeaboredSerError::ProfileUnsupported {
            profile,
            what: "bignum that fits major type 0/1",
        });
    }

    let tag = CborIntegerValue::from(if big.is_negative() { 3u8 } else { 2u8 });
    let mut written = tag.serialize_complex_mt_preamble(MajorType::Tagged, writer)?;
    written += CborIntegerValue::serialize_inline_bytes(magnitude, MajorType::Bytes, writer)?;
    Ok(written)
}

#[cfg_attr(feature = "inline-nontrivial", inline)]
fn serialize_value_tagged<W: Write>(
    tag: CborIntegerValue,
    value: &Value<'_>,
    writer: &mut W,
    profile: Profile,
) -> Result<usize, SeaboredSerError> {
    let tag_number: u64 = tag.into();

    if profile.is_strict() && (tag_number == 2 || tag_number == 3) {
        let bytes = value_bytes(value, profile)?;
        return serialize_bignum_tag(tag_number, &bytes, writer, profile);
    }

    let mut written = tag.serialize_complex_mt_preamble(MajorType::Tagged, writer)?;
    written += value.cbor_serialize_to_with_profile(writer, profile)?;
    Ok(written)
}

/// Writes a bignum as tag 2/3, stripping leading zeroes and emitting a major type 0/1
/// integer when the magnitude fits. Strict profiles call this only for tag 2/3 payloads.
pub(crate) fn serialize_bignum_tag<W: Write>(
    tag: u64,
    bytes: &[u8],
    writer: &mut W,
    profile: Profile,
) -> Result<usize, SeaboredSerError> {
    debug_assert!(profile.is_strict());
    let significant = match bytes.iter().position(|&byte| byte != 0) {
        Some(first) => &bytes[first..],
        None => &[],
    };

    if significant.len() <= 8 {
        let mut argument = 0u64;
        for &byte in significant {
            argument = (argument << 8) | u64::from(byte);
        }
        let integer = CborInteger {
            value: CborIntegerValue::from(argument),
            negative: tag == 3,
        };
        return integer.cbor_serialize_to(writer);
    }

    let tag = CborIntegerValue::from(tag);
    let mut written = tag.serialize_complex_mt_preamble(MajorType::Tagged, writer)?;
    written += CborIntegerValue::serialize_inline_bytes(significant, MajorType::Bytes, writer)?;
    Ok(written)
}

pub(crate) fn value_bytes(
    value: &Value<'_>,
    profile: Profile,
) -> Result<Vec<u8>, SeaboredSerError> {
    match value {
        Value::Bytes(bytes) => Ok(bytes.to_vec()),
        Value::Sequence(seq) if seq.mt == MajorType::Bytes => {
            let mut bytes = Vec::new();
            for chunk in seq.iter() {
                match chunk {
                    Value::Bytes(chunk) => bytes.extend_from_slice(chunk.as_ref()),
                    _ => {
                        return Err(SeaboredSerError::ProfileUnsupported {
                            profile,
                            what: "bignum payload chunk",
                        });
                    }
                }
            }
            Ok(bytes)
        }
        _ => Err(SeaboredSerError::ProfileUnsupported {
            profile,
            what: "bignum payload",
        }),
    }
}
