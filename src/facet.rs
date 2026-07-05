use std::borrow::Cow;

use facet_reflect::Partial;
use parsio::Read;

use crate::{
    de::CborDeserialize,
    error::SeaboredDeError,
    ib::{self, InitialByte},
};

// Freely inspired by Amos's facet-cbor that has since disappeared from the facet repo
// Minus the slop code, that is.
fn facet_deser<'facet, R: Read<'facet>>(
    partial: Partial<'facet, true>,
    reader: &mut R,
) -> Result<Partial<'facet, true>, SeaboredDeError<'facet>> {
    let shape = partial.shape();
    if shape.is_transparent() {
        return Ok(facet_deser(partial.begin_inner()?, reader)?.end()?);
    }

    if let Some(st) = shape.scalar_type() {
        return Ok(match st {
            facet::ScalarType::Unit => {
                let ib = InitialByte::peek(reader)?;
                if *ib != ib::consts::IB_UNDEFINED {
                    return Err(SeaboredDeError::IncorrectInitialByte {
                        actual: *ib,
                        expected: ib::consts::IB_UNDEFINED,
                    });
                }
                partial.set(())?
            }
            facet::ScalarType::Bool => partial.set(bool::cbor_deserialize_from(reader)?)?,
            facet::ScalarType::Char => partial.set(
                (&*Cow::<str>::cbor_deserialize_from(reader)?)
                    .chars()
                    .next()
                    .ok_or_else(|| {
                        SeaboredDeError::Incomplete(winnow::error::Needed::Size(
                            1.try_into().unwrap(),
                        ))
                    })?,
            )?,
            facet::ScalarType::Str => partial.set(Cow::<str>::cbor_deserialize_from(reader)?)?,
            facet::ScalarType::String => {
                partial.set(Cow::<str>::cbor_deserialize_from(reader)?.into_owned())?
            }
            facet::ScalarType::CowStr => partial.set(Cow::<str>::cbor_deserialize_from(reader)?)?,
            facet::ScalarType::F32 => partial.set(f32::cbor_deserialize_from(reader)?)?,
            facet::ScalarType::F64 => partial.set(f64::cbor_deserialize_from(reader)?)?,
            facet::ScalarType::U8 => partial.set(u8::cbor_deserialize_from(reader)?)?,
            facet::ScalarType::U16 => partial.set(u16::cbor_deserialize_from(reader)?)?,
            facet::ScalarType::U32 => partial.set(u32::cbor_deserialize_from(reader)?)?,
            facet::ScalarType::U64 => partial.set(u64::cbor_deserialize_from(reader)?)?,
            facet::ScalarType::U128 => partial.set(u128::cbor_deserialize_from(reader)?)?,
            facet::ScalarType::USize => partial.set(cfg_select! {
                target_pointer_width = "32" => {
                    u32::cbor_deserialize_from(reader)? as usize
                }
                target_pointer_width = "64" => {
                    u64::cbor_deserialize_from(reader)? as usize
                }
            })?,
            facet::ScalarType::I8 => partial.set(i8::cbor_deserialize_from(reader)?)?,
            facet::ScalarType::I16 => partial.set(i16::cbor_deserialize_from(reader)?)?,
            facet::ScalarType::I32 => partial.set(i32::cbor_deserialize_from(reader)?)?,
            facet::ScalarType::I64 => partial.set(i64::cbor_deserialize_from(reader)?)?,
            facet::ScalarType::I128 => partial.set(i128::cbor_deserialize_from(reader)?)?,
            facet::ScalarType::ISize => partial.set(cfg_select! {
                target_pointer_width = "32" => {
                    i32::cbor_deserialize_from(reader)? as isize
                }
                target_pointer_width = "64" => {
                    i64::cbor_deserialize_from(reader)? as isize
                }
            })?,
            // facet::ScalarType::ConstTypeId => todo!(), // what is this
            _ => return Err(SeaboredDeError::UnsupportedFacetScalar(st)),
        });
    }

    match shape.def {
        facet::Def::Option(_) => {
            let ib = InitialByte::peek(reader)?;
            return Ok(if *ib == ib::consts::IB_NULL {
                reader.advance(1)?;
                partial
            } else {
                facet_deser(partial.begin_some()?, reader)?.end()?
            });
        }
        facet::Def::List(list_def) => todo!(),
        facet::Def::Map(map_def) => todo!(),
        facet::Def::Set(set_def) => todo!(),
        facet::Def::Array(array_def) => todo!(),
        facet::Def::NdArray(nd_array_def) => todo!(),
        facet::Def::Slice(slice_def) => todo!(),
        facet::Def::Result(result_def) => todo!(),
        facet::Def::Pointer(pointer_def) => todo!(),
        facet::Def::DynamicValue(dynamic_value_def) => todo!(),
        _ => {}
    }

    todo!()
}
