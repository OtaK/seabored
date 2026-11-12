use crate::{
    Value,
    error::SeaboredSerError,
    ib,
    mt::MajorType,
    types::{CborIntegerValue, IB_LIMIT},
};

use parsio::Write;

pub trait CborSerialize {
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError>;
    #[inline(always)]
    fn cbor_serialize(&self) -> Result<Vec<u8>, SeaboredSerError> {
        let mut buf = vec![];
        self.cbor_serialize_to(&mut buf)?;
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

impl<T1, T2> CborSerialize for (T1, T2)
where
    T1: CborSerialize,
    T2: CborSerialize,
{
    #[inline(always)]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        Ok(self.0.cbor_serialize_to(writer)? + self.1.cbor_serialize_to(writer)?)
    }
}

impl CborSerialize for Value<'_> {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        match self {
            Value::Integer(cbor_integer) => cbor_integer.cbor_serialize_to(writer),
            Value::Float(cbor_float) => cbor_float.cbor_serialize_to(writer),
            Value::Bytes(cow) => {
                CborIntegerValue::serialize_inline_bytes(cow, MajorType::Bytes, writer)
            }
            Value::String(cow) => {
                CborIntegerValue::serialize_inline_bytes(cow.as_bytes(), MajorType::String, writer)
            }
            Value::Sequence(seq) => seq.cbor_serialize_to(writer),
            Value::Map(seq) => {
                debug_assert_eq!(seq.mt, MajorType::Map);
                seq.cbor_serialize_to(writer)
            }
            Value::Tagged((tag, value)) => Ok(tag
                .serialize_complex_mt_preamble(MajorType::Tagged, writer)?
                + value.cbor_serialize_to(writer)?),
            Value::SimpleValue(value) => {
                // 20..=23 mean false/true/null/undefined and 24..=31 are reserved (RFC 8949 §3.3)
                if (20..32).contains(value) {
                    return Err(SeaboredSerError::ReservedSimpleValue(*value));
                }
                if value <= &IB_LIMIT {
                    Ok(writer.write(&[ib::consts::IB_SIMPLE_VALUE | value])?)
                } else {
                    Ok(writer.write(&[ib::consts::IB_SIMPLE_VALUE_NEXT_BYTE, *value])?)
                }
            }
            Value::Bool(true) => Ok(writer.write(&[ib::consts::IB_TRUE])?),
            Value::Bool(false) => Ok(writer.write(&[ib::consts::IB_FALSE])?),
            Value::Null => Ok(writer.write(&[ib::consts::IB_NULL])?),
            Value::Undefined => Ok(writer.write(&[ib::consts::IB_UNDEFINED])?),
        }
    }
}
