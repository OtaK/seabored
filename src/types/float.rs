use half::f16;

use crate::{
    de::CborDeserialize,
    error::{SeaboredDeError, SeaboredSerError},
    ib::{self, InitialByte},
    io::ReadExt as _,
    profile::{DecodeOptions, Profile},
    ser::CborSerialize,
};

use parsio::{DepthAwareReaderGuard, Read, Write};

/// Validate a decoded float against the selected serialization profile.
///
/// `ib` is the float's initial byte and `bits` its raw payload (16, 32, or 64 bits, low-aligned).
/// Under a checking decoder this enforces draft-09 §4.1 item 4: the shortest exact width and the
/// canonical half-precision NaN (`f9 7e 00`) are the only permitted encodings.
#[cfg_attr(feature = "inline-nontrivial", inline)]
pub(crate) fn validate_float_encoding(
    ib: InitialByte,
    bits: u64,
    options: DecodeOptions,
) -> Result<(), SeaboredDeError> {
    if !options.is_checking() {
        return Ok(());
    }

    match ib.0 {
        ib::consts::IB_FLOAT_16 => {
            let bits = bits as u16;
            if f16::from_bits(bits).is_nan() && bits != f16::NAN.to_bits() {
                return Err(SeaboredDeError::NonPreferredEncoding { what: "float NaN" });
            }
        }
        ib::consts::IB_FLOAT_32 => {
            let value = f32::from_bits(bits as u32);
            if value.is_nan() {
                return Err(SeaboredDeError::NonPreferredEncoding { what: "float NaN" });
            }

            if f16::from_f32(value).to_f32().to_bits() == value.to_bits() {
                return Err(SeaboredDeError::NonPreferredEncoding {
                    what: "float width",
                });
            }
        }
        ib::consts::IB_FLOAT_64 => {
            let value = f64::from_bits(bits);
            if value.is_nan() {
                return Err(SeaboredDeError::NonPreferredEncoding { what: "float NaN" });
            }

            if (value as f32) as f64 == value || f16::from_f64(value).to_f64() == value {
                return Err(SeaboredDeError::NonPreferredEncoding {
                    what: "float width",
                });
            }
        }
        _ => {}
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
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
    pub fn try_cast_f32(&self) -> Result<f32, SeaboredDeError> {
        let float_f32 = self.0 as f32;
        if float_f32 as f64 == self.0 {
            Ok(float_f32)
        } else {
            Err(SeaboredDeError::FloatPrecisionLoss)
        }
    }

    #[cfg_attr(feature = "inline-nontrivial", inline)]
    pub fn try_cast_f16(&self) -> Result<f16, SeaboredDeError> {
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
        // Preferred-plus permits exactly one NaN, with no sign or payload variation
        if self.0.is_nan() {
            return f16::NAN.cbor_serialize_to(writer);
        }

        // Smallest lossless representation wins.
        let maybe_f16 =
            Some(f16::from_f64(self.0)).filter(|f16_value| f16_value.to_f64() == self.0);

        if let Some(f16_value) = maybe_f16 {
            f16_value.cbor_serialize_to(writer)
        } else if let Ok(f32_value) = self.try_cast_f32() {
            f32_value.cbor_serialize_to(writer)
        } else {
            self.0.cbor_serialize_to(writer)
        }
    }

    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to_with_profile<W: Write>(
        &self,
        writer: &mut W,
        _profile: Profile,
    ) -> Result<usize, SeaboredSerError> {
        self.cbor_serialize_to(writer)
    }
}

impl<'a> CborDeserialize<'a> for CborFloat {
    #[inline]
    fn cbor_deserialize_from_with_profile<R: Read<'a>>(
        reader: &mut DepthAwareReaderGuard<'_, R>,
        options: DecodeOptions,
    ) -> Result<Self, crate::error::SeaboredDeError> {
        Ok(Self(f64::cbor_deserialize_from_with_profile(
            reader, options,
        )?))
    }
}

impl<'a> CborDeserialize<'a> for f16 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from_with_profile<R: Read<'a>>(
        reader: &mut DepthAwareReaderGuard<'_, R>,
        options: DecodeOptions,
    ) -> Result<Self, SeaboredDeError>
    where
        Self: Sized + 'a,
    {
        let ib = InitialByte::cbor_deserialize_from(reader)?;

        match ib.0 {
            ib::consts::IB_FLOAT_16 => {
                let bits = reader.read_be_u16()?;
                validate_float_encoding(ib, bits as u64, options)?;
                Ok(f16::from_bits(bits))
            }
            _ => Err(SeaboredDeError::IncorrectInitialByteSet {
                actual: ib.0,
                expected: &[ib::consts::IB_FLOAT_16],
            }),
        }
    }
}

impl<'a> CborDeserialize<'a> for f32 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from_with_profile<R: Read<'a>>(
        reader: &mut DepthAwareReaderGuard<'_, R>,
        options: DecodeOptions,
    ) -> Result<Self, SeaboredDeError>
    where
        Self: Sized + 'a,
    {
        let ib = InitialByte::cbor_deserialize_from(reader)?;

        match ib.0 {
            ib::consts::IB_FLOAT_16 => {
                let bits = reader.read_be_u16()?;
                validate_float_encoding(ib, bits as u64, options)?;
                Ok(f16::from_bits(bits).to_f32())
            }
            ib::consts::IB_FLOAT_32 => {
                let bits = reader.read_be_u32()?;
                validate_float_encoding(ib, bits as u64, options)?;
                Ok(f32::from_bits(bits))
            }
            _ => Err(SeaboredDeError::IncorrectInitialByteSet {
                actual: ib.0,
                expected: &[ib::consts::IB_FLOAT_16, ib::consts::IB_FLOAT_32],
            }),
        }
    }
}

impl<'a> CborDeserialize<'a> for f64 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from_with_profile<R: Read<'a>>(
        reader: &mut DepthAwareReaderGuard<'_, R>,
        options: DecodeOptions,
    ) -> Result<Self, SeaboredDeError>
    where
        Self: Sized + 'a,
    {
        let ib = InitialByte::cbor_deserialize_from(reader)?;

        match ib.0 {
            ib::consts::IB_FLOAT_16 => {
                let bits = reader.read_be_u16()?;
                validate_float_encoding(ib, bits as u64, options)?;
                Ok(f16::from_bits(bits).to_f64())
            }
            ib::consts::IB_FLOAT_32 => {
                let bits = reader.read_be_u32()?;
                validate_float_encoding(ib, bits as u64, options)?;
                Ok(f32::from_bits(bits) as f64)
            }
            ib::consts::IB_FLOAT_64 => {
                let bits = reader.read_be_u64()?;
                validate_float_encoding(ib, bits, options)?;
                Ok(f64::from_bits(bits))
            }
            _ => Err(SeaboredDeError::IncorrectInitialByteSet {
                actual: ib.0,
                expected: &[
                    ib::consts::IB_FLOAT_16,
                    ib::consts::IB_FLOAT_32,
                    ib::consts::IB_FLOAT_64,
                ],
            }),
        }
    }
}

impl CborSerialize for f16 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        // Preferred-plus permits exactly one NaN, with no sign or payload variation
        let value = if self.is_nan() { f16::NAN } else { *self };
        let mut buf = [ib::consts::IB_FLOAT_16, 0, 0];
        buf[1..].copy_from_slice(&value.to_be_bytes());
        writer.write(&buf).map_err(Into::into)
    }

    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to_with_profile<W: Write>(
        &self,
        writer: &mut W,
        _profile: Profile,
    ) -> Result<usize, SeaboredSerError> {
        self.cbor_serialize_to(writer)
    }
}

impl CborSerialize for f32 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        // Preferred-plus: NaN is always f16
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

    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to_with_profile<W: Write>(
        &self,
        writer: &mut W,
        _profile: Profile,
    ) -> Result<usize, SeaboredSerError> {
        self.cbor_serialize_to(writer)
    }
}

impl CborSerialize for f64 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        // Preferred-plus: NaN is always f16
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

    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to_with_profile<W: Write>(
        &self,
        writer: &mut W,
        _profile: Profile,
    ) -> Result<usize, SeaboredSerError> {
        self.cbor_serialize_to(writer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        error::SeaboredDeError,
        ib::consts,
        profile::{DecodeOptions, Profile},
    };

    fn validate(ib: u8, bits: u64, options: DecodeOptions) -> Result<(), SeaboredDeError> {
        validate_float_encoding(InitialByte(ib), bits, options)
    }

    fn assert_float_nan(result: Result<(), SeaboredDeError>) {
        match result {
            Err(SeaboredDeError::NonPreferredEncoding { what }) => {
                assert_eq!(what, "float NaN");
            }
            Err(other) => panic!("expected float NaN rejection, got {other:?}"),
            Ok(()) => panic!("expected float NaN rejection, got Ok"),
        }
    }

    fn assert_float_width(result: Result<(), SeaboredDeError>) {
        match result {
            Err(SeaboredDeError::NonPreferredEncoding { what }) => {
                assert_eq!(what, "float width");
            }
            Err(other) => panic!("expected float width rejection, got {other:?}"),
            Ok(()) => panic!("expected float width rejection, got Ok"),
        }
    }

    #[test]
    fn f16_nan_is_canonicalized_on_encode() {
        // Sign, signaling bit, and payload all disappear: only f9 7e 00 is preferred-plus.
        for bits in [0x7e01_u16, 0x7dff, 0xfe00, 0x7c01, 0xfc01] {
            let encoded = f16::from_bits(bits).cbor_serialize().unwrap();
            assert_eq!(encoded, [0xf9, 0x7e, 0x00], "f16 bits {bits:#06x}");
        }
    }

    #[test]
    fn checking_rejects_non_canonical_f16_nan() {
        let checked = DecodeOptions::PREFERRED_PLUS_CHECKED;
        for bits in [0x7e01_u16, 0x7dff, 0xfe00, 0x7c01, 0xfc01] {
            assert_float_nan(validate(consts::IB_FLOAT_16, bits as u64, checked));
        }

        // The one permitted NaN, plus the only two infinities, are accepted.
        for bits in [0x7e00_u16, 0x7c00, 0xfc00] {
            validate(consts::IB_FLOAT_16, bits as u64, checked).unwrap();
        }
    }

    #[test]
    fn checking_rejects_f32_values_that_fit_f16() {
        let checked = DecodeOptions::PREFERRED_PLUS_CHECKED;

        // 0.0, -0.0, 1.0, +inf, -inf are all exactly representable as f16.
        for bits in [0x0000_u16, 0x8000, 0x3c00, 0x7c00, 0xfc00] {
            let value = f16::from_bits(bits).to_f32();
            assert_float_width(validate(
                consts::IB_FLOAT_32,
                value.to_bits() as u64,
                checked,
            ));
        }

        // NaNs must use f16 0x7e00, whether quiet or signaling.
        for bits in [0x7fc0_0000_u32, 0x7f80_0001, 0xffc0_0000] {
            assert_float_nan(validate(consts::IB_FLOAT_32, bits as u64, checked));
        }

        // Values that need f32 stay f32.
        for bits in [0x7f7f_ffff_u32, 0x0000_0001, 0x3f80_0001] {
            validate(consts::IB_FLOAT_32, bits as u64, checked).unwrap();
        }
    }

    #[test]
    fn checking_rejects_f64_values_that_fit_f32_or_f16() {
        let checked = DecodeOptions::DETERMINISTIC_CHECKED;

        // 0.0, -0.0, 1.0, 0.5, +inf, -inf are all exactly representable as f16/f32.
        for bits in [
            0x0000_0000_0000_0000_u64,
            0x8000_0000_0000_0000,
            0x3ff0_0000_0000_0000,
            0x3fe0_0000_0000_0000,
            0x7ff0_0000_0000_0000,
            0xfff0_0000_0000_0000,
        ] {
            assert_float_width(validate(consts::IB_FLOAT_64, bits, checked));
        }

        for bits in [0x7ff8_0000_0000_0000_u64, 0x7ff0_0000_0000_0001] {
            assert_float_nan(validate(consts::IB_FLOAT_64, bits, checked));
        }

        // f64::MAX, the smallest f64 subnormal, and 0.1 all require f64.
        for bits in [
            0x7fef_ffff_ffff_ffff_u64,
            0x0000_0000_0000_0001,
            0x3fb9_9999_9999_999a,
        ] {
            validate(consts::IB_FLOAT_64, bits, checked).unwrap();
        }
    }

    #[test]
    fn non_checking_decoders_accept_every_float_form() {
        for options in [
            DecodeOptions::GENERAL,
            DecodeOptions::PREFERRED_PLUS,
            DecodeOptions::DETERMINISTIC,
            DecodeOptions {
                profile: Profile::General,
                checking: true,
            },
        ] {
            for (ib, bits) in [
                (consts::IB_FLOAT_16, 0x7e01_u64),
                (consts::IB_FLOAT_16, 0xfe00),
                (consts::IB_FLOAT_32, 0x7fc0_0000),
                (consts::IB_FLOAT_32, 0x3f80_0000),
                (consts::IB_FLOAT_64, 0x7ff8_0000_0000_0000),
                (consts::IB_FLOAT_64, 0x3ff0_0000_0000_0000),
            ] {
                validate(ib, bits, options).unwrap();
            }
        }
    }
}
