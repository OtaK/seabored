use half::f16;

use crate::{
    de::CborDeserialize,
    error::{SeaboredDeError, SeaboredSerError},
    ib::{self, InitialByte},
    io::ReadExt as _,
    ser::CborSerialize,
};

use parsio::{Read, Write};

#[derive(Debug, Clone, PartialEq, PartialOrd)]
// #[cfg_attr(
//     feature = "serde",
//     derive(serde::Serialize, serde::Deserialize),
//     serde(transparent)
// )]
#[repr(transparent)]
pub struct CborFloat(f64);

impl From<f16> for CborFloat {
    #[inline(always)]
    fn from(value: f16) -> Self {
        Self(value.to_f64())
    }
}

impl From<f32> for CborFloat {
    #[inline(always)]
    fn from(value: f32) -> Self {
        Self(value as f64)
    }
}

impl From<f64> for CborFloat {
    #[inline(always)]
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl From<CborFloat> for f64 {
    #[inline(always)]
    fn from(cf: CborFloat) -> f64 {
        cf.0
    }
}

impl std::ops::Deref for CborFloat {
    type Target = f64;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for CborFloat {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl CborFloat {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    pub fn try_cast_f32<'a>(&self) -> Result<f32, SeaboredDeError<'a>> {
        let float_f32 = self.0 as f32;
        if float_f32 as f64 == self.0 {
            Ok(float_f32)
        } else {
            Err(SeaboredDeError::FloatPrecisionLoss)
        }
    }

    #[cfg_attr(feature = "inline-nontrivial", inline)]
    pub fn try_cast_f16<'a>(&self) -> Result<f16, SeaboredDeError<'a>> {
        let float_f16 = f16::from_f64(self.0);
        if float_f16.to_f64() == self.0 {
            Ok(float_f16)
        } else {
            Err(SeaboredDeError::FloatPrecisionLoss)
        }
    }
}

impl CborSerialize for CborFloat {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        // We try to cast the inner value to the smallest representation possible by using lossless casts.
        // We use the smallest value type that succeeds in the lossless cast
        let maybe_f16 = Some(f16::from_f64(self.0)).filter(|f16_value| {
            self.0.is_nan() // Special case: in preferred-plus serialization, NaN are *always* encoded as f16
            || f16_value.to_f64() == self.0
        });

        if let Some(f16_value) = maybe_f16 {
            f16_value.cbor_serialize_to(writer)
        } else if let Ok(f32_value) = self.try_cast_f32() {
            f32_value.cbor_serialize_to(writer)
        } else {
            self.0.cbor_serialize_to(writer)
        }
    }
}

impl<'a> CborDeserialize<'a> for CborFloat {
    #[inline]
    fn cbor_deserialize_from<R: Read<'a>>(
        reader: &mut R,
    ) -> Result<Self, crate::error::SeaboredDeError<'a>> {
        Ok(Self(f64::cbor_deserialize_from(reader)?))
    }
}

impl<'a> CborDeserialize<'a> for f16 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from<R: Read<'a>>(reader: &mut R) -> Result<Self, SeaboredDeError<'a>>
    where
        Self: Sized + 'a,
    {
        let ib = InitialByte::cbor_deserialize_from(reader)?;

        match ib.0 {
            ib::consts::IB_FLOAT_16 => Ok(f16::from_bits(reader.read_be_u16()?)),
            _ => Err(SeaboredDeError::IncorrectInitialByte {
                actual: ib.0,
                expected: ib::consts::IB_FLOAT_16,
            }),
        }
    }
}

impl<'a> CborDeserialize<'a> for f32 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from<R: Read<'a>>(reader: &mut R) -> Result<Self, SeaboredDeError<'a>>
    where
        Self: Sized + 'a,
    {
        let ib = InitialByte::cbor_deserialize_from(reader)?;

        match ib.0 {
            ib::consts::IB_FLOAT_16 => Ok(f16::from_bits(reader.read_be_u16()?).to_f32()),
            ib::consts::IB_FLOAT_32 => Ok(f32::from_bits(reader.read_be_u32()?)),
            _ => Err(SeaboredDeError::IncorrectInitialByte {
                actual: ib.0,
                expected: ib::consts::IB_FLOAT_32,
            }),
        }
    }
}

impl<'a> CborDeserialize<'a> for f64 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from<R: Read<'a>>(reader: &mut R) -> Result<Self, SeaboredDeError<'a>>
    where
        Self: Sized + 'a,
    {
        let ib = InitialByte::cbor_deserialize_from(reader)?;

        match ib.0 {
            ib::consts::IB_FLOAT_16 => Ok(f16::from_bits(reader.read_be_u16()?).to_f64()),
            ib::consts::IB_FLOAT_32 => Ok(f32::from_bits(reader.read_be_u32()?) as f64),
            ib::consts::IB_FLOAT_64 => Ok(f64::from_bits(reader.read_be_u64()?)),
            _ => Err(SeaboredDeError::IncorrectInitialByte {
                actual: ib.0,
                expected: ib::consts::IB_FLOAT_32,
            }),
        }
    }
}

impl CborSerialize for f16 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        let mut buf = [ib::consts::IB_FLOAT_16, 0, 0];
        buf[1..].copy_from_slice(&self.to_be_bytes());
        writer.write(&buf).map_err(Into::into)
    }
}

impl CborSerialize for f32 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        // Special case: in preferred-plus serialization, NaN are *always* encoded as f16
        if self.is_nan() {
            return f16::NAN.cbor_serialize_to(writer);
        }

        let maybe_f16_value =
            Some(f16::from_f32(*self)).filter(|f16_value| &f16_value.to_f32() == self);

        if let Some(f16_value) = maybe_f16_value {
            return f16_value.cbor_serialize_to(writer);
        }

        let mut buf = [ib::consts::IB_FLOAT_32, 0, 0, 0, 0];
        buf[1..].copy_from_slice(&self.to_be_bytes());
        writer.write(&buf).map_err(Into::into)
    }
}

impl CborSerialize for f64 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        // Special case: in preferred-plus serialization, NaN are *always* encoded as f16
        if self.is_nan() {
            return f16::NAN.cbor_serialize_to(writer);
        }

        if let Some(f32v) = Some(*self as f32).filter(|f32v| *f32v as f64 == *self) {
            return f32v.cbor_serialize_to(writer);
        }

        let mut buf = [ib::consts::IB_FLOAT_64, 0, 0, 0, 0, 0, 0, 0, 0];
        buf[1..].copy_from_slice(&self.to_be_bytes());
        writer.write(&buf).map_err(Into::into)
    }
}
