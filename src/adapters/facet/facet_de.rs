use std::borrow::Cow;

use facet_reflect::Partial;
use parsio::Read;

use crate::{
    de::CborDeserialize,
    error::{FacetError, SeaboredDeError},
    ib::{self, InitialByte},
    mt::MajorType,
};

#[inline(always)]
pub fn from_slice<'de, T: facet::Facet<'de> + 'de>(
    bytes: &'de [u8],
) -> Result<T, SeaboredDeError<'de>> {
    from_reader(&mut &bytes[..])
}

#[inline(always)]
pub fn from_reader<'de, R: Read<'de>, T: facet::Facet<'de> + 'de>(
    reader: &mut R,
) -> Result<T, SeaboredDeError<'de>> {
    Ok(
        facet_deser(Partial::alloc::<T>().map_err(FacetError::from)?, reader)?
            .build()
            .map_err(FacetError::from)?
            .materialize()
            .map_err(FacetError::from)?,
    )
}

// Freely inspired by Amos's facet-cbor that has since disappeared from the facet repo
// Minus the slop code, that is.
// Doesn't use facet-format because the design is absolutely horrid and I ain't dealing
// with that crap.
fn facet_deser<'facet, R: Read<'facet>>(
    partial: Partial<'facet, true>,
    reader: &mut R,
) -> Result<Partial<'facet, true>, SeaboredDeError<'facet>> {
    let shape = partial.shape();
    if shape.is_transparent() {
        return Ok(
            facet_deser(partial.begin_inner().map_err(FacetError::from)?, reader)?
                .end()
                .map_err(FacetError::from)?,
        );
    }

    if shape.has_default_attr() {
        return Ok(partial.set_default().map_err(FacetError::from)?);
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
                partial.set(()).map_err(FacetError::from)?
            }
            facet::ScalarType::Bool => partial
                .set(bool::cbor_deserialize_from(reader)?)
                .map_err(FacetError::from)?,
            facet::ScalarType::Char => partial
                .set(
                    (&*Cow::<str>::cbor_deserialize_from(reader)?)
                        .chars()
                        .next()
                        .ok_or_else(|| {
                            SeaboredDeError::Incomplete(winnow::error::Needed::Size(
                                1.try_into().unwrap(),
                            ))
                        })?,
                )
                .map_err(FacetError::from)?,
            facet::ScalarType::Str => partial
                .set(Cow::<str>::cbor_deserialize_from(reader)?)
                .map_err(FacetError::from)?,
            facet::ScalarType::String => partial
                .set(Cow::<str>::cbor_deserialize_from(reader)?.into_owned())
                .map_err(FacetError::from)?,
            facet::ScalarType::CowStr => partial
                .set(Cow::<str>::cbor_deserialize_from(reader)?)
                .map_err(FacetError::from)?,
            facet::ScalarType::F32 => partial
                .set(f32::cbor_deserialize_from(reader)?)
                .map_err(FacetError::from)?,
            facet::ScalarType::F64 => partial
                .set(f64::cbor_deserialize_from(reader)?)
                .map_err(FacetError::from)?,
            facet::ScalarType::U8 => partial
                .set(u8::cbor_deserialize_from(reader)?)
                .map_err(FacetError::from)?,
            facet::ScalarType::U16 => partial
                .set(u16::cbor_deserialize_from(reader)?)
                .map_err(FacetError::from)?,
            facet::ScalarType::U32 => partial
                .set(u32::cbor_deserialize_from(reader)?)
                .map_err(FacetError::from)?,
            facet::ScalarType::U64 => partial
                .set(u64::cbor_deserialize_from(reader)?)
                .map_err(FacetError::from)?,
            facet::ScalarType::U128 => partial
                .set(u128::cbor_deserialize_from(reader)?)
                .map_err(FacetError::from)?,
            facet::ScalarType::USize => partial
                .set(cfg_select! {
                    target_pointer_width = "32" => {
                        u32::cbor_deserialize_from(reader)? as usize
                    }
                    target_pointer_width = "64" => {
                        u64::cbor_deserialize_from(reader)? as usize
                    }
                })
                .map_err(FacetError::from)?,
            facet::ScalarType::I8 => partial
                .set(i8::cbor_deserialize_from(reader)?)
                .map_err(FacetError::from)?,
            facet::ScalarType::I16 => partial
                .set(i16::cbor_deserialize_from(reader)?)
                .map_err(FacetError::from)?,
            facet::ScalarType::I32 => partial
                .set(i32::cbor_deserialize_from(reader)?)
                .map_err(FacetError::from)?,
            facet::ScalarType::I64 => partial
                .set(i64::cbor_deserialize_from(reader)?)
                .map_err(FacetError::from)?,
            facet::ScalarType::I128 => partial
                .set(i128::cbor_deserialize_from(reader)?)
                .map_err(FacetError::from)?,
            facet::ScalarType::ISize => partial
                .set(cfg_select! {
                    target_pointer_width = "32" => {
                        i32::cbor_deserialize_from(reader)? as isize
                    }
                    target_pointer_width = "64" => {
                        i64::cbor_deserialize_from(reader)? as isize
                    }
                })
                .map_err(FacetError::from)?,
            // facet::ScalarType::ConstTypeId => todo!(), // what is this
            _ => return Err(FacetError::UnsupportedFacetScalar(st).into()),
        });
    }

    match shape.def {
        facet::Def::Option(_) => {
            let ib = InitialByte::peek(reader)?;
            return Ok(if *ib == ib::consts::IB_NULL {
                reader.advance(1)?;
                partial
            } else {
                facet_deser(partial.begin_some().map_err(FacetError::from)?, reader)?
                    .end()
                    .map_err(FacetError::from)?
            });
        }
        facet::Def::List(facet::ListDef { t, .. })
        | facet::Def::Slice(facet::SliceDef { t, .. })
        | facet::Def::Array(facet::ArrayDef { t, .. })
            if t.is_type::<u8>() =>
        {
            return Ok(partial
                .set(Cow::<[u8]>::cbor_deserialize_from(reader)?)
                .map_err(FacetError::from)?);
        }
        facet::Def::List(_) => {
            let ib = InitialByte::cbor_deserialize_from(reader)?;
            let (mt, ai) = ib.mt_ai();

            if mt != MajorType::Array {
                return Err(SeaboredDeError::IncorrectMajorType {
                    actual: mt,
                    expected: &[MajorType::Array],
                });
            }

            let mut sized_list = None;

            let mut partial = match ai.find_subsequent_len(reader) {
                Ok(len) => {
                    let len = len.try_into()?;
                    sized_list.replace(len);
                    partial
                        .init_list_with_capacity(len)
                        .map_err(FacetError::from)?
                }
                Err(SeaboredDeError::IndefiniteLen) => {
                    partial.init_list().map_err(FacetError::from)?
                }
                Err(e) => return Err(e),
            };

            if let Some(len) = sized_list {
                for _ in 0..len {
                    partial =
                        facet_deser(partial.begin_list_item().map_err(FacetError::from)?, reader)?
                            .end()
                            .map_err(FacetError::from)?;
                }
            } else {
                while *InitialByte::peek(reader)? != ib::consts::IB_BREAK {
                    partial =
                        facet_deser(partial.begin_list_item().map_err(FacetError::from)?, reader)?
                            .end()
                            .map_err(FacetError::from)?;
                }
            }

            return Ok(partial);
        }
        facet::Def::Set(_) => {
            let ib = InitialByte::cbor_deserialize_from(reader)?;
            let (mt, ai) = ib.mt_ai();

            if mt != MajorType::Array {
                return Err(SeaboredDeError::IncorrectMajorType {
                    actual: mt,
                    expected: &[MajorType::Array],
                });
            }

            let mut sized_list: Option<usize> = None;

            match ai.find_subsequent_len(reader) {
                Ok(len) => {
                    sized_list.replace(len.try_into()?);
                }
                Err(SeaboredDeError::IndefiniteLen) => {}
                Err(e) => return Err(e),
            };

            let mut partial = partial.init_set().map_err(FacetError::from)?;

            if let Some(len) = sized_list {
                for _ in 0..len {
                    partial =
                        facet_deser(partial.begin_set_item().map_err(FacetError::from)?, reader)?
                            .end()
                            .map_err(FacetError::from)?;
                }
            } else {
                while *InitialByte::peek(reader)? != ib::consts::IB_BREAK {
                    partial =
                        facet_deser(partial.begin_set_item().map_err(FacetError::from)?, reader)?
                            .end()
                            .map_err(FacetError::from)?;
                }
            }

            return Ok(partial);
        }
        facet::Def::Map(_) => {
            let ib = InitialByte::cbor_deserialize_from(reader)?;
            let (mt, ai) = ib.mt_ai();

            if mt != MajorType::Map {
                return Err(SeaboredDeError::IncorrectMajorType {
                    actual: mt,
                    expected: &[MajorType::Map],
                });
            }

            let mut sized_list = None;

            let mut partial = match ai.find_subsequent_len(reader) {
                Ok(len) => {
                    let len = len.try_into()?;
                    sized_list.replace(len);
                    partial
                        .init_list_with_capacity(len)
                        .map_err(FacetError::from)?
                }
                Err(SeaboredDeError::IndefiniteLen) => {
                    partial.init_list().map_err(FacetError::from)?
                }
                Err(e) => return Err(e),
            };

            if let Some(len) = sized_list {
                for _ in 0..len {
                    partial = facet_deser(partial.begin_key().map_err(FacetError::from)?, reader)?
                        .end()
                        .map_err(FacetError::from)?;
                    partial =
                        facet_deser(partial.begin_value().map_err(FacetError::from)?, reader)?
                            .end()
                            .map_err(FacetError::from)?;
                }
            } else {
                while *InitialByte::peek(reader)? != ib::consts::IB_BREAK {
                    partial = facet_deser(partial.begin_key().map_err(FacetError::from)?, reader)?
                        .end()
                        .map_err(FacetError::from)?;
                    partial =
                        facet_deser(partial.begin_value().map_err(FacetError::from)?, reader)?
                            .end()
                            .map_err(FacetError::from)?;
                }
            }

            return Ok(partial);
        }
        facet::Def::Array(array_def) => {
            let ib = InitialByte::cbor_deserialize_from(reader)?;
            let (mt, ai) = ib.mt_ai();

            if mt != MajorType::Array {
                return Err(SeaboredDeError::IncorrectMajorType {
                    actual: mt,
                    expected: &[MajorType::Array],
                });
            }

            let len: usize = array_def.n.min(ai.find_subsequent_len(reader)?.try_into()?);
            let mut partial = partial;
            for i in 0..len {
                partial = facet_deser(
                    partial.begin_nth_field(i).map_err(FacetError::from)?,
                    reader,
                )?
                .end()
                .map_err(FacetError::from)?;
            }

            return Ok(partial);
        }
        facet::Def::Pointer(_) => {
            let ib = InitialByte::peek(reader)?;
            return Ok(if *ib == ib::consts::IB_NULL {
                reader.advance(1)?;
                partial
            } else {
                facet_deser(partial.begin_smart_ptr().map_err(FacetError::from)?, reader)?
                    .end()
                    .map_err(FacetError::from)?
            });
        }
        _ => {}
    }

    match shape.ty {
        facet::Type::User(facet::UserType::Struct(struct_type)) => match struct_type.kind {
            facet::StructKind::Struct => {
                let ib = InitialByte::cbor_deserialize_from(reader)?;
                let (mt, ai) = ib.mt_ai();

                if mt != MajorType::Map {
                    return Err(SeaboredDeError::IncorrectMajorType {
                        actual: mt,
                        expected: &[MajorType::Map],
                    });
                }

                let len: usize = ai.find_subsequent_len(reader)?.try_into()?;

                let mut partial = partial;
                for _ in 0..len {
                    let key = Cow::<str>::cbor_deserialize_from(reader)?;
                    if partial.field_index(&key).is_some() {
                        partial = facet_deser(
                            partial.begin_field(&key).map_err(FacetError::from)?,
                            reader,
                        )?
                        .end()
                        .map_err(FacetError::from)?;
                    } else {
                        // Skip
                        let _ = crate::Value::cbor_deserialize_from(reader)?;
                    }
                }

                Ok(partial)
            }
            facet::StructKind::TupleStruct | facet::StructKind::Tuple => {
                let ib = InitialByte::cbor_deserialize_from(reader)?;
                let (mt, ai) = ib.mt_ai();

                if mt != MajorType::Array {
                    return Err(SeaboredDeError::IncorrectMajorType {
                        actual: mt,
                        expected: &[MajorType::Array],
                    });
                }

                let len: usize = struct_type
                    .fields
                    .len()
                    .min(ai.find_subsequent_len(reader)?.try_into()?);

                let mut partial = partial;
                for i in 0..len {
                    partial = facet_deser(
                        partial.begin_nth_field(i).map_err(FacetError::from)?,
                        reader,
                    )?
                    .end()
                    .map_err(FacetError::from)?;
                }

                return Ok(partial);
            }
            facet::StructKind::Unit => {
                let ib = InitialByte::peek(reader)?;
                if *ib != ib::consts::IB_UNDEFINED {
                    return Err(SeaboredDeError::IncorrectInitialByte {
                        actual: *ib,
                        expected: ib::consts::IB_UNDEFINED,
                    });
                }
                Ok(partial)
            }
        },
        facet::Type::User(facet::UserType::Enum(_)) => {
            let ib = InitialByte::cbor_deserialize_from(reader)?;
            let (mt, ai) = ib.mt_ai();

            if mt != MajorType::Map {
                return Err(SeaboredDeError::IncorrectMajorType {
                    actual: mt,
                    expected: &[MajorType::Map],
                });
            }

            let map_len: usize = ai.find_subsequent_len(reader)?.try_into()?;
            debug_assert_eq!(
                map_len, 1,
                "Enums repr'd as CBOR must be 1-len maps (variant => variant contents)"
            );

            let variant_str = Cow::<str>::cbor_deserialize_from(reader)?;
            let (_, variant) = partial
                .find_variant(&variant_str)
                .ok_or_else(|| FacetError::UnknownFacetEnumVariant(variant_str.to_string()))?;
            let variant_kind = variant.data.kind;
            let fc = variant.data.fields.len();
            let mut partial = partial
                .select_variant_named(&variant_str)
                .map_err(FacetError::from)?;

            match variant_kind {
                facet::StructKind::Struct => {
                    let ib = InitialByte::cbor_deserialize_from(reader)?;
                    let (mt, ai) = ib.mt_ai();

                    if mt != MajorType::Map {
                        return Err(SeaboredDeError::IncorrectMajorType {
                            actual: mt,
                            expected: &[MajorType::Map],
                        });
                    }

                    let len: usize = ai.find_subsequent_len(reader)?.try_into()?;

                    for _ in 0..len {
                        let key = Cow::<str>::cbor_deserialize_from(reader)?;
                        if partial.field_index(&key).is_some() {
                            partial = facet_deser(
                                partial.begin_field(&key).map_err(FacetError::from)?,
                                reader,
                            )?
                            .end()
                            .map_err(FacetError::from)?;
                        } else {
                            // Skip
                            let _ = crate::Value::cbor_deserialize_from(reader)?;
                        }
                    }
                }
                facet::StructKind::TupleStruct => {
                    if fc == 1 {
                        partial = facet_deser(
                            partial.begin_nth_field(0).map_err(FacetError::from)?,
                            reader,
                        )?
                        .end()
                        .map_err(FacetError::from)?;
                    } else {
                        let ib = InitialByte::cbor_deserialize_from(reader)?;
                        let (mt, ai) = ib.mt_ai();
                        if mt != MajorType::Array {
                            return Err(SeaboredDeError::IncorrectMajorType {
                                actual: mt,
                                expected: &[MajorType::Array],
                            });
                        }

                        let len: usize = ai.find_subsequent_len(reader)?.try_into()?;
                        if len != fc {
                            return Err(SeaboredDeError::WrongLen {
                                expected: fc,
                                got: len,
                            });
                        }

                        for i in 0..fc {
                            partial = facet_deser(
                                partial.begin_nth_field(i).map_err(FacetError::from)?,
                                reader,
                            )?
                            .end()
                            .map_err(FacetError::from)?;
                        }
                    }
                }
                facet::StructKind::Tuple => {
                    let ib = InitialByte::cbor_deserialize_from(reader)?;
                    let (mt, ai) = ib.mt_ai();
                    if mt != MajorType::Array {
                        return Err(SeaboredDeError::IncorrectMajorType {
                            actual: mt,
                            expected: &[MajorType::Array],
                        });
                    }

                    let len: usize = ai.find_subsequent_len(reader)?.try_into()?;
                    if len != fc {
                        return Err(SeaboredDeError::WrongLen {
                            expected: fc,
                            got: len,
                        });
                    }

                    for i in 0..fc {
                        partial = facet_deser(
                            partial.begin_nth_field(i).map_err(FacetError::from)?,
                            reader,
                        )?
                        .end()
                        .map_err(FacetError::from)?;
                    }
                }
                facet::StructKind::Unit => {
                    let ib = InitialByte::cbor_deserialize_from(reader)?;
                    if *ib != ib::consts::IB_UNDEFINED {
                        return Err(SeaboredDeError::IncorrectInitialByte {
                            actual: *ib,
                            expected: ib::consts::IB_UNDEFINED,
                        });
                    }
                }
            }
            Ok(partial)
        }
        _ => return Err(FacetError::UnsupportedFacetType(shape.ty).into()),
    }
}
