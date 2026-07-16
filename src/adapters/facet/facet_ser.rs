use facet_reflect::{HasFields, Peek};
use parsio::Write;

use crate::{
    error::{FacetError, SeaboredSerError},
    ib,
    mt::MajorType,
    ser::CborSerialize,
    types::CborIntegerValue,
};

#[inline(always)]
pub fn to_writer<'facet, W: Write, T: facet::Facet<'facet>>(
    writer: &mut W,
    value: &T,
) -> Result<usize, crate::error::SeaboredSerError> {
    facet_ser(Peek::new(value), writer)
}

#[inline(always)]
/// Serialize a data structure to a Vec
pub fn to_vec<'facet, T: facet::Facet<'facet>>(
    value: &T,
) -> Result<Vec<u8>, crate::error::SeaboredSerError> {
    let mut buf = vec![];
    let written = to_writer(&mut buf, value)?;
    debug_assert_eq!(written, buf.len());
    Ok(buf)
}

fn facet_ser<W: Write>(peek: Peek<'_, '_>, writer: &mut W) -> Result<usize, SeaboredSerError> {
    let peek = peek.innermost_peek();

    if let Some(scalar_ty) = peek.scalar_type() {
        return Ok(match scalar_ty {
            facet::ScalarType::Unit => writer.write(&[ib::consts::IB_UNDEFINED])?,
            facet::ScalarType::Bool => {
                writer
                    .write(&[ib::consts::IB_FALSE
                        | (*peek.get::<bool>().map_err(FacetError::from)?) as u8])?
            }
            facet::ScalarType::Char => {
                let mut buf = [0u8; 4];
                peek.get::<char>()
                    .map_err(FacetError::from)?
                    .encode_utf8(&mut buf);
                let str = unsafe { str::from_utf8_unchecked(&buf) };
                str.cbor_serialize_to(writer)?
            }
            facet::ScalarType::Str | facet::ScalarType::String | facet::ScalarType::CowStr => peek
                .as_str()
                .ok_or_else(|| {
                    use facet::Facet as _;
                    FacetError::from(facet_reflect::ReflectError::new(
                        facet_reflect::ReflectErrorKind::WrongShape {
                            expected: peek.shape(),
                            actual: str::SHAPE,
                        },
                        facet_path::Path::new(peek.shape()),
                    ))
                })?
                .cbor_serialize_to(writer)?,
            facet::ScalarType::F32 => peek
                .get::<f32>()
                .map_err(FacetError::from)?
                .cbor_serialize_to(writer)?,
            facet::ScalarType::F64 => peek
                .get::<f64>()
                .map_err(FacetError::from)?
                .cbor_serialize_to(writer)?,
            facet::ScalarType::U8 => peek
                .get::<u8>()
                .map_err(FacetError::from)?
                .cbor_serialize_to(writer)?,
            facet::ScalarType::U16 => peek
                .get::<u16>()
                .map_err(FacetError::from)?
                .cbor_serialize_to(writer)?,
            facet::ScalarType::U32 => peek
                .get::<u32>()
                .map_err(FacetError::from)?
                .cbor_serialize_to(writer)?,
            facet::ScalarType::U64 => peek
                .get::<u64>()
                .map_err(FacetError::from)?
                .cbor_serialize_to(writer)?,
            facet::ScalarType::U128 => peek
                .get::<u128>()
                .map_err(FacetError::from)?
                .cbor_serialize_to(writer)?,
            facet::ScalarType::USize => cfg_select! {
                target_pointer_width = "32" => {
                    (*peek.get::<usize>().map_err(FacetError::from)? as u32).cbor_serialize_to(writer)?
                }
                target_pointer_width = "64" => {
                    (*peek.get::<usize>().map_err(FacetError::from)? as u64).cbor_serialize_to(writer)?
                }
            },
            facet::ScalarType::I8 => peek
                .get::<i8>()
                .map_err(FacetError::from)?
                .cbor_serialize_to(writer)?,
            facet::ScalarType::I16 => peek
                .get::<i16>()
                .map_err(FacetError::from)?
                .cbor_serialize_to(writer)?,
            facet::ScalarType::I32 => peek
                .get::<i32>()
                .map_err(FacetError::from)?
                .cbor_serialize_to(writer)?,
            facet::ScalarType::I64 => peek
                .get::<i64>()
                .map_err(FacetError::from)?
                .cbor_serialize_to(writer)?,
            facet::ScalarType::I128 => peek
                .get::<i128>()
                .map_err(FacetError::from)?
                .cbor_serialize_to(writer)?,
            facet::ScalarType::ISize => cfg_select! {
                target_pointer_width = "32" => {
                    (*peek.get::<isize>().map_err(FacetError::from)? as i32).cbor_serialize_to(writer)?
                }
                target_pointer_width = "64" => {
                    (*peek.get::<isize>().map_err(FacetError::from)? as i64).cbor_serialize_to(writer)?
                }
            },
            // facet::ScalarType::ConstTypeId => todo!(), // What is this again
            _ => return Err(FacetError::UnsupportedFacetScalar(scalar_ty).into()),
        });
    }

    let shape = peek.shape();

    match shape.def {
        facet::Def::Option(_) => {
            return if let Some(value) = peek.into_option().map_err(FacetError::from)?.value() {
                facet_ser(value, writer)
            } else {
                Ok(writer.write(&[ib::consts::IB_NULL])?)
            };
        }
        facet::Def::List(facet::ListDef { t, .. })
        | facet::Def::Slice(facet::SliceDef { t, .. })
            if t.is_type::<u8>() =>
        {
            return if let Some(bytes) = peek.as_bytes() {
                bytes.cbor_serialize_to(writer)
            } else {
                match peek.get::<[u8]>() {
                    Ok(slice) => slice.cbor_serialize_to(writer),
                    Err(_) => {
                        let vec = peek.get::<Vec<u8>>().map_err(FacetError::from)?;
                        vec.as_slice().cbor_serialize_to(writer)
                    }
                }
            };
        }
        facet::Def::List(_) => {
            let list = peek.into_list().map_err(FacetError::from)?;
            let len = list.len();
            let mut written = CborIntegerValue::from(len)
                .serialize_complex_mt_preamble(MajorType::Array, writer)?;
            for item in list.iter() {
                written += facet_ser(item, writer)?;
            }
            return Ok(written);
        }
        facet::Def::Array(_) | facet::Def::Slice(_) => {
            let list = peek.into_list_like().map_err(FacetError::from)?;
            let len = list.len();
            let mut written = CborIntegerValue::from(len)
                .serialize_complex_mt_preamble(MajorType::Array, writer)?;
            for item in list.iter() {
                written += facet_ser(item, writer)?;
            }
            return Ok(written);
        }
        facet::Def::Map(_) => {
            let map = peek.into_map().map_err(FacetError::from)?;
            let len = map.len();
            let mut written = CborIntegerValue::from(len)
                .serialize_complex_mt_preamble(MajorType::Map, writer)?;
            for (k, v) in map.iter() {
                written += facet_ser(k, writer)?;
                written += facet_ser(v, writer)?;
            }
            return Ok(written);
        }
        facet::Def::Set(_) => {
            let list = peek.into_set().map_err(FacetError::from)?;
            let len = list.len();
            let mut written = CborIntegerValue::from(len)
                .serialize_complex_mt_preamble(MajorType::Array, writer)?;
            for item in list.iter() {
                written += facet_ser(item, writer)?;
            }
            return Ok(written);
        }
        facet::Def::Pointer(_) => {
            let ptr = peek.into_pointer().map_err(FacetError::from)?;
            return Ok(if let Some(inner) = ptr.borrow_inner() {
                facet_ser(inner, writer)?
            } else {
                writer.write(&[ib::consts::IB_NULL])?
            });
        }
        _ => {}
    }

    Ok(match shape.ty {
        facet::Type::User(facet::UserType::Struct(struct_type)) => match struct_type.kind {
            facet::StructKind::Struct => {
                let ps = peek.into_struct().map_err(FacetError::from)?;

                let len = ps.fields_for_serialize().count();
                let mut written = CborIntegerValue::from(len)
                    .serialize_complex_mt_preamble(MajorType::Map, writer)?;

                for (field_item, field) in ps.fields_for_serialize() {
                    written += field_item.effective_name().cbor_serialize_to(writer)?;
                    written += facet_ser(field, writer)?;
                }

                written
            }
            facet::StructKind::TupleStruct | facet::StructKind::Tuple => {
                let ps = peek.into_struct().map_err(FacetError::from)?;

                let len = ps.fields_for_serialize().count();

                let mut written = CborIntegerValue::from(len)
                    .serialize_complex_mt_preamble(MajorType::Array, writer)?;

                for (_, field) in ps.fields_for_serialize() {
                    written += facet_ser(field, writer)?;
                }

                written
            }
            facet::StructKind::Unit => writer.write(&[ib::consts::IB_UNDEFINED])?,
        },
        facet::Type::User(facet::UserType::Enum(_)) => {
            let pe = peek.into_enum().map_err(FacetError::from)?;
            let variant = pe.active_variant().map_err(FacetError::from)?;
            let variant_str = variant.rename.unwrap_or(variant.name);

            let mut written = CborIntegerValue::from(1u8)
                .serialize_complex_mt_preamble(MajorType::Map, writer)?;
            // Key
            written += variant_str.cbor_serialize_to(writer)?;
            // Value
            match variant.data.kind {
                facet::StructKind::Unit => {
                    written += writer.write(&[ib::consts::IB_UNDEFINED])?;
                }
                facet::StructKind::Struct => {
                    let len = variant.data.fields.len();
                    let mut effective_len = 0usize;
                    for i in 0..len {
                        let field = pe
                            .field(i)
                            .map_err(FacetError::from)?
                            .ok_or_else(|| FacetError::MissingField(i))?;
                        if unsafe { variant.data.fields[i].should_skip_serializing(field.data()) } {
                            continue;
                        }

                        effective_len += 1;
                    }

                    written += CborIntegerValue::from(effective_len)
                        .serialize_complex_mt_preamble(MajorType::Map, writer)?;

                    for i in 0..len {
                        let field = pe
                            .field(i)
                            .map_err(FacetError::from)?
                            .ok_or_else(|| FacetError::MissingField(i))?;

                        if unsafe { variant.data.fields[i].should_skip_serializing(field.data()) } {
                            continue;
                        }

                        // key
                        written += variant.data.fields[i].name.cbor_serialize_to(writer)?;
                        // value
                        written += facet_ser(field, writer)?;
                    }
                }
                facet::StructKind::TupleStruct => {
                    let len = variant.data.fields.len();
                    if len == 1 {
                        let field = pe
                            .field(0)
                            .map_err(FacetError::from)?
                            .ok_or_else(|| FacetError::MissingField(0))?;
                        written += facet_ser(field, writer)?;
                    } else {
                        written += CborIntegerValue::from(len)
                            .serialize_complex_mt_preamble(MajorType::Array, writer)?;
                        for i in 0..len {
                            let field = pe
                                .field(i)
                                .map_err(FacetError::from)?
                                .ok_or_else(|| FacetError::MissingField(i))?;
                            written += facet_ser(field, writer)?;
                        }
                    }
                }
                facet::StructKind::Tuple => {
                    let len = variant.data.fields.len();
                    written += CborIntegerValue::from(len)
                        .serialize_complex_mt_preamble(MajorType::Array, writer)?;
                    for i in 0..len {
                        let field = pe
                            .field(i)
                            .map_err(FacetError::from)?
                            .ok_or_else(|| FacetError::MissingField(i))?;
                        written += facet_ser(field, writer)?;
                    }
                }
            }

            written
        }
        _ => return Err(FacetError::UnsupportedFacetType(shape.ty).into()),
    })
}
