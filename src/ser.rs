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
