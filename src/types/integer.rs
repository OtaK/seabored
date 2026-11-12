use crate::{
    de::CborDeserialize,
    error::{SeaboredDeError, SeaboredSerError},
    ib::{self, AdditionalInfo, InitialByte},
    io::ReadExt as _,
    mt::MajorType,
    profile::{DecodeOptions, Profile},
    ser::CborSerialize,
};

use parsio::{DepthAwareReaderGuard, Read, Write};
use std::borrow::Cow;

pub(crate) const IB_LIMIT: u8 = 0x17;

#[inline]
fn read_uint_argument<'data, R: Read<'data>>(
    reader: &mut DepthAwareReaderGuard<'_, R>,
    ai: AdditionalInfo,
) -> Result<u64, SeaboredDeError> {
    Ok(match ai.action()? {
        crate::ib::AdditionalInfoAction::DoNothing => ai.raw() as u64,
        crate::ib::AdditionalInfoAction::Uint8 => reader.read_byte()? as u64,
        crate::ib::AdditionalInfoAction::Uint16 => reader.read_be_u16()? as u64,
        crate::ib::AdditionalInfoAction::Uint32 => reader.read_be_u32()? as u64,
        crate::ib::AdditionalInfoAction::Uint64 => reader.read_be_u64()?,
        crate::ib::AdditionalInfoAction::IndefiniteLenSeq => {
            return Err(SeaboredDeError::IllegalAdditionalInfo(ai.raw()));
        }
    })
}

#[inline]
fn decode_signed_value<'data, R: Read<'data>>(
    reader: &mut DepthAwareReaderGuard<'_, R>,
    options: DecodeOptions,
) -> Result<i128, SeaboredDeError> {
    let ib = InitialByte::cbor_deserialize_from(reader)?;
    let (mt, ai) = ib.mt_ai();
    let negative = match mt {
        MajorType::Uint => false,
        MajorType::NegativeUint => true,
        _ => {
            return Err(SeaboredDeError::IncorrectMajorType {
                actual: mt,
                expected: &[MajorType::Uint, MajorType::NegativeUint],
            });
        }
    };

    let raw = read_uint_argument(reader, ai)?;
    ai.ensure_shortest(raw, options, "integer")?;
    Ok(if negative {
        -1 - i128::from(raw)
    } else {
        i128::from(raw)
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub(crate) enum CborIntegerSize {
    MergeIntoIB = 0,
    U8 = 1,
    U16 = 1 << 1,
    U32 = 1 << 2,
    U64 = 1 << 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
/// u64-backed CBOR integer
pub struct CborIntegerValue(u64);

macro_rules! impl_int_conversions_cbor_int_value {
    (uint $repr:ty) => {
        impl From<$repr> for CborIntegerValue {
            #[inline(always)]
            fn from(value: $repr) -> Self {
                Self(value as u64)
            }
        }

        impl TryFrom<CborIntegerValue> for $repr {
            type Error = std::num::TryFromIntError;
            #[inline(always)]
            fn try_from(value: CborIntegerValue) -> Result<$repr, Self::Error> {
                value.0.try_into()
            }
        }
    };
    (infaillible uint $repr:ty ) => {
        impl From<$repr> for CborIntegerValue {
            #[inline(always)]
            fn from(value: $repr) -> Self {
                Self(value)
            }
        }

        impl From<CborIntegerValue> for $repr {
            #[inline(always)]
            fn from(civ: CborIntegerValue) -> $repr {
                civ.0
            }
        }
    };
}

impl_int_conversions_cbor_int_value!(uint u8);
impl_int_conversions_cbor_int_value!(uint u16);
impl_int_conversions_cbor_int_value!(uint u32);
impl_int_conversions_cbor_int_value!(infaillible uint u64);

impl From<usize> for CborIntegerValue {
    #[inline(always)]
    fn from(value: usize) -> Self {
        Self(value as u64)
    }
}

impl TryFrom<CborIntegerValue> for usize {
    type Error = std::num::TryFromIntError;
    #[inline(always)]
    fn try_from(value: CborIntegerValue) -> Result<usize, Self::Error> {
        value.0.try_into()
    }
}

impl CborIntegerValue {
    #[inline(always)]
    /// Size and InitialByte offset
    pub(crate) fn size(&self) -> (CborIntegerSize, u8) {
        if self.0 <= IB_LIMIT as u64 {
            (CborIntegerSize::MergeIntoIB, 0)
        } else if self.0 <= u8::MAX as u64 {
            (CborIntegerSize::U8, IB_LIMIT + 1)
        } else if self.0 <= u16::MAX as u64 {
            (CborIntegerSize::U16, IB_LIMIT + 2)
        } else if self.0 <= u32::MAX as u64 {
            (CborIntegerSize::U32, IB_LIMIT + 3)
        } else {
            (CborIntegerSize::U64, IB_LIMIT + 4)
        }
    }

    /// Vectored write of preamble + bytes (faster [`Self::serialize_complex_mt_preamble`])
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    pub(crate) fn serialize_inline_bytes<W: Write>(
        bytes: &[u8],
        mt: MajorType,
        writer: &mut W,
    ) -> Result<usize, SeaboredSerError> {
        debug_assert!(
            matches!(mt, MajorType::Bytes | MajorType::String),
            "[IMPLEMENTATION ERROR] MajorType used must be Bytes or String"
        );

        let len = CborIntegerValue::from(bytes.len());
        let (size, ib_offset) = len.size();

        let ib = InitialByte::from(mt);

        Ok(match size {
            CborIntegerSize::MergeIntoIB => {
                writer.write_vectored(&[&[ib.0 + len.0 as u8], bytes])?
            }
            _ => writer.write_vectored(&[
                &[ib.0 + ib_offset],
                &len.0.to_be_bytes()[(8 - size as u8) as usize..],
                bytes,
            ])?,
        })
    }

    #[cfg_attr(feature = "inline-nontrivial", inline)]
    pub(crate) fn serialize_complex_mt_preamble<W: Write>(
        &self,
        mt: MajorType,
        writer: &mut W,
    ) -> Result<usize, SeaboredSerError> {
        debug_assert!(
            matches!(
                mt,
                MajorType::Bytes
                    | MajorType::String
                    | MajorType::Array
                    | MajorType::Tagged
                    | MajorType::Map
            ),
            "MajorType must be a complex type, including: Bytes, String, Array, Map or Tagged"
        );

        let (size, ib_offset) = self.size();
        let ib = InitialByte::from(mt);

        Ok(match size {
            CborIntegerSize::MergeIntoIB => writer.write(&[ib.0 + self.0 as u8])?,
            _ => writer.write_vectored(&[
                &[ib.0 + ib_offset],
                &self.0.to_be_bytes()[(8 - size as u8) as usize..],
            ])?,
        })
    }
}

/// Tag 2/3 payload to `u128`.
///
/// The byte string is a network byte order integer, and leading zeroes are allowed
/// (RFC 8949 §3.4.3), so they get skipped before the payload is left-padded to 16 bytes.
#[cfg_attr(feature = "inline-nontrivial", inline)]
fn bignum_to_u128(wire_bytes: &[u8], options: DecodeOptions) -> Result<u128, SeaboredDeError> {
    if options.is_checking()
        && (wire_bytes.is_empty() || wire_bytes[0] == 0 || wire_bytes.len() <= 8)
    {
        return Err(SeaboredDeError::NonPreferredEncoding { what: "bignum" });
    }

    let significant = match wire_bytes.iter().position(|&byte| byte != 0) {
        Some(first) => &wire_bytes[first..],
        None => return Ok(0),
    };

    if significant.len() > 16 {
        return Err(SeaboredDeError::BignumTooLarge(significant.len()));
    }

    let mut bytes = [0u8; 16];
    bytes[16 - significant.len()..].copy_from_slice(significant);
    Ok(u128::from_be_bytes(bytes))
}

/// A CBOR bignum: tag 2 (positive) or tag 3 (negative) around a byte string.
///
/// The magnitude is normalized: big-endian with no leading zeroes. An empty magnitude is
/// `0` for tag 2 and `-1` for tag 3, because tag 3 means `-1 - magnitude`; bignums have no
/// negative zero. Decoding strips any leading zeroes the wire had, so re-encoding is
/// canonical even under [`Profile::General`](crate::Profile::General).
///
/// draft-09 §4.2 makes the empty-payload rule explicit (0 for tag 2, -1 for tag 3) in
/// [cbor-wg PR #101](https://github.com/cbor-wg/draft-ietf-cbor-serialization/pull/101);
/// the published -09 text still says "the value zero".
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CborBigInteger<'a> {
    negative: bool,
    magnitude: Cow<'a, [u8]>,
}

impl<'a> CborBigInteger<'a> {
    /// Builds a bignum, stripping leading zeroes from `magnitude`.
    ///
    /// `new(true, [])` is `-1`, not negative zero.
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    pub fn new(negative: bool, magnitude: impl Into<Cow<'a, [u8]>>) -> Self {
        let magnitude = magnitude.into();
        let first = magnitude
            .iter()
            .position(|&byte| byte != 0)
            .unwrap_or(magnitude.len());
        let magnitude = match magnitude {
            Cow::Borrowed(bytes) => Cow::Borrowed(&bytes[first..]),
            Cow::Owned(mut bytes) => {
                bytes.drain(..first);
                Cow::Owned(bytes)
            }
        };
        Self {
            negative,
            magnitude,
        }
    }

    /// Whether the tag was 3.
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    pub fn is_negative(&self) -> bool {
        self.negative
    }

    /// The big-endian magnitude, without leading zeroes; empty means `0` (tag 2) or `-1`
    /// (tag 3).
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    pub fn magnitude(&self) -> &[u8] {
        &self.magnitude
    }

    /// Whether this is the integer zero: tag 2 with an empty magnitude.
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    pub fn is_zero(&self) -> bool {
        !self.negative && self.magnitude.is_empty()
    }

    /// Drops the borrow.
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    pub fn into_owned(self) -> CborBigInteger<'static> {
        CborBigInteger {
            negative: self.negative,
            magnitude: Cow::Owned(self.magnitude.into_owned()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CborInteger {
    pub(crate) value: CborIntegerValue,
    pub(crate) negative: bool,
}

impl CborInteger {
    /// The unsigned CBOR argument: the value for major type 0, `-1 - value`
    /// for major type 1. Used by strict-profile header checks.
    #[inline(always)]
    pub fn argument(&self) -> u64 {
        self.value.0
    }

    #[inline(always)]
    fn base_ib(&self) -> u8 {
        (self.negative as u8) << 5u8
    }
}

impl std::fmt::Display for CborInteger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.negative {
            write!(f, "-{}", self.value.0 as u128 + 1)
        } else {
            write!(f, "{}", self.value.0)
        }
    }
}

impl CborSerialize for CborInteger {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        let (size, ib_offset) = self.value.size();
        let ib = self.base_ib();

        Ok(match size {
            CborIntegerSize::MergeIntoIB => writer.write(&[ib + self.value.0 as u8])?,
            _ => writer.write_vectored(&[
                &[ib + ib_offset],
                &self.value.0.to_be_bytes()[8 - size as u8 as usize..],
            ])?,
        })
    }

    #[inline]
    fn cbor_serialize_to_with_profile<W: Write>(
        &self,
        writer: &mut W,
        _profile: Profile,
    ) -> Result<usize, SeaboredSerError> {
        self.cbor_serialize_to(writer)
    }
}

impl<'a> CborDeserialize<'a> for CborInteger {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from_with_profile<R: Read<'a>>(
        reader: &mut DepthAwareReaderGuard<'_, R>,
        options: DecodeOptions,
    ) -> Result<Self, crate::error::SeaboredDeError>
    where
        Self: Sized + 'a,
    {
        let ib = InitialByte::cbor_deserialize_from(reader)?;
        let (mt, ai) = ib.mt_ai();

        let negative = match mt {
            MajorType::Uint => false,
            MajorType::NegativeUint => true,
            _ => {
                return Err(SeaboredDeError::IncorrectMajorType {
                    actual: mt,
                    expected: &[MajorType::Uint, MajorType::NegativeUint],
                });
            }
        };

        let value = read_uint_argument(reader, ai)?;
        ai.ensure_shortest(value, options, "integer")?;

        Ok(Self {
            value: value.into(),
            negative,
        })
    }
}

macro_rules! impl_int_conversion_cbor_int {
    (uint $repr:ty) => {
        impl From<$repr> for CborInteger {
            #[inline(always)]
            fn from(value: $repr) -> Self {
                Self {
                    value: value.into(),
                    negative: false,
                }
            }
        }

        impl TryFrom<CborInteger> for $repr {
            type Error = std::num::TryFromIntError;
            #[inline(always)]
            fn try_from(value: CborInteger) -> Result<$repr, Self::Error> {
                value.value.try_into()
            }
        }

        impl TryFrom<&CborInteger> for $repr {
            type Error = std::num::TryFromIntError;
            #[inline(always)]
            fn try_from(value: &CborInteger) -> Result<$repr, Self::Error> {
                value.value.try_into()
            }
        }
    };

    (infaillible uint $repr:ty ) => {
        impl From<$repr> for CborInteger {
            #[inline(always)]
            fn from(value: $repr) -> Self {
                Self {
                    value: value.into(),
                    negative: false,
                }
            }
        }

        impl From<CborInteger> for $repr {
            #[inline(always)]
            fn from(ci: CborInteger) -> $repr {
                ci.value.into()
            }
        }

        impl From<&CborInteger> for $repr {
            #[inline(always)]
            fn from(ci: &CborInteger) -> $repr {
                ci.value.into()
            }
        }
    };

    (int $repr:ty) => {
        impl From<$repr> for CborInteger {
            #[cfg_attr(feature = "inline-nontrivial", inline)]
            fn from(value: $repr) -> Self {
                let negative = value.is_negative();
                Self {
                    negative,
                    value: value.unsigned_abs().saturating_sub(negative as _).into(),
                }
            }
        }

        impl TryFrom<CborInteger> for $repr {
            type Error = std::num::TryFromIntError;
            #[cfg_attr(feature = "inline-nontrivial", inline)]
            fn try_from(value: CborInteger) -> Result<$repr, Self::Error> {
                let magnitude: $repr = value.value.0.try_into()?;
                Ok(-(value.negative as $repr) ^ magnitude)
            }
        }

        impl TryFrom<&CborInteger> for $repr {
            type Error = std::num::TryFromIntError;
            #[cfg_attr(feature = "inline-nontrivial", inline)]
            fn try_from(value: &CborInteger) -> Result<$repr, Self::Error> {
                let magnitude: $repr = value.value.0.try_into()?;
                Ok(-(value.negative as $repr) ^ magnitude)
            }
        }
    };
}

impl_int_conversion_cbor_int!(uint u8);
impl_int_conversion_cbor_int!(uint u16);
impl_int_conversion_cbor_int!(uint u32);
impl_int_conversion_cbor_int!(infaillible uint u64);
impl_int_conversion_cbor_int!(uint usize);
impl_int_conversion_cbor_int!(int i8);
impl_int_conversion_cbor_int!(int i16);
impl_int_conversion_cbor_int!(int i32);
impl_int_conversion_cbor_int!(int i64);

impl TryFrom<i128> for CborInteger {
    type Error = std::num::TryFromIntError;

    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn try_from(value: i128) -> Result<Self, Self::Error> {
        let negative = value.is_negative();
        let abs: u64 = value.abs().try_into()?;
        Ok(Self {
            value: abs.into(),
            negative,
        })
    }
}

impl CborSerialize for u8 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        if self <= &IB_LIMIT {
            writer.write(&[*self])
        } else {
            writer.write(&[ib::consts::IB_UINT_8, *self])
        }
        .map_err(Into::into)
    }

    #[inline]
    fn cbor_serialize_to_with_profile<W: Write>(
        &self,
        writer: &mut W,
        _profile: Profile,
    ) -> Result<usize, SeaboredSerError> {
        self.cbor_serialize_to(writer)
    }
}

impl CborSerialize for u16 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        if let Ok(u8_v) = u8::try_from(*self) {
            return u8_v.cbor_serialize_to(writer);
        }

        let mut buf = [ib::consts::IB_UINT_16, 0, 0];
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

impl CborSerialize for u32 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        if let Ok(u16_v) = u16::try_from(*self) {
            return u16_v.cbor_serialize_to(writer);
        }

        let mut buf = [ib::consts::IB_UINT_32, 0, 0, 0, 0];
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

impl CborSerialize for u64 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        if let Ok(u32_v) = u32::try_from(*self) {
            return u32_v.cbor_serialize_to(writer);
        }

        let mut buf = [ib::consts::IB_UINT_64, 0, 0, 0, 0, 0, 0, 0, 0];
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

impl CborSerialize for u128 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        if let Ok(u64_v) = u64::try_from(*self) {
            return u64_v.cbor_serialize_to(writer);
        }

        let bytes = self.to_be_bytes();
        let offset = bytes.iter().position(|&b| b != 0x00).unwrap_or(bytes.len());

        // Tag 2: fits the IB, then the bytes
        Ok(writer.write(&[ib::consts::IB_UNSIGNED_BIG_NUM])?
            + CborIntegerValue::serialize_inline_bytes(&bytes[offset..], MajorType::Bytes, writer)?)
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

impl CborSerialize for i8 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        let mut ui = (self >> (i8::BITS - 1)) as u8;
        let mt = ui & ib::consts::IB_SMALL_NEGATIVE_UINT;
        ui ^= *self as u8;

        if ui <= IB_LIMIT {
            writer.write(&[mt | ui])
        } else {
            writer.write(&[mt | ib::consts::IB_UINT_8, ui])
        }
        .map_err(Into::into)
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

impl CborSerialize for i16 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        if *self >= 0 {
            return (*self as u16).cbor_serialize_to(writer);
        }
        if let Ok(i8_v) = i8::try_from(*self) {
            return i8_v.cbor_serialize_to(writer);
        }

        let mut ui = (self >> (i16::BITS - 1)) as u16;
        let mt = (ui & ib::consts::IB_SMALL_NEGATIVE_UINT as u16) as u8;
        ui ^= *self as u16;
        let mut buf = [mt | ib::consts::IB_UINT_16, 0, 0];
        buf[1..].copy_from_slice(&ui.to_be_bytes());
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

impl CborSerialize for i32 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        if *self >= 0 {
            return (*self as u32).cbor_serialize_to(writer);
        }
        if let Ok(i16_v) = i16::try_from(*self) {
            return i16_v.cbor_serialize_to(writer);
        }

        let mut ui = (self >> (i32::BITS - 1)) as u32;
        let mt = (ui & ib::consts::IB_SMALL_NEGATIVE_UINT as u32) as u8;
        ui ^= *self as u32;
        let mut buf = [mt | ib::consts::IB_UINT_32, 0, 0, 0, 0];
        buf[1..].copy_from_slice(&ui.to_be_bytes());
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

impl CborSerialize for i64 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        if *self >= 0 {
            return (*self as u64).cbor_serialize_to(writer);
        }
        if let Ok(i32_v) = i32::try_from(*self) {
            return i32_v.cbor_serialize_to(writer);
        }

        let mut ui = (self >> (i64::BITS - 1)) as u64;
        let mt = (ui & ib::consts::IB_SMALL_NEGATIVE_UINT as u64) as u8;
        ui ^= *self as u64;
        let mut buf = [mt | ib::consts::IB_UINT_64, 0, 0, 0, 0, 0, 0, 0, 0];
        buf[1..].copy_from_slice(&ui.to_be_bytes());
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

impl CborSerialize for i128 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        if *self >= 0 {
            return (*self as u128).cbor_serialize_to(writer);
        }
        if let Ok(i64_v) = i64::try_from(*self) {
            return i64_v.cbor_serialize_to(writer);
        }
        // Major type 1 still holds down to -2^64 (n = -1 - value <= u64::MAX)
        let n = -(self + 1) as u128;
        if let Ok(n) = u64::try_from(n) {
            return CborInteger {
                value: CborIntegerValue::from(n),
                negative: true,
            }
            .cbor_serialize_to(writer);
        }

        let mut ui = (self >> (i128::BITS - 1)) as u128;
        let mt = self.is_negative() as u8;
        ui ^= *self as u128;
        let bytes = ui.to_be_bytes();
        let offset = bytes.iter().position(|&b| b != 0x00).unwrap_or(bytes.len());

        // Tag 2/3: fits the IB, then the bytes
        Ok(writer.write(&[mt | ib::consts::IB_UNSIGNED_BIG_NUM])?
            + CborIntegerValue::serialize_inline_bytes(&bytes[offset..], MajorType::Bytes, writer)?)
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

impl<'a> CborDeserialize<'a> for u8 {
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
        if mt != MajorType::Uint {
            return Err(SeaboredDeError::IncorrectMajorType {
                actual: mt,
                expected: &[MajorType::Uint],
            });
        }

        let raw = read_uint_argument(reader, ai)?;
        ai.ensure_shortest(raw, options, "integer")?;
        u8::try_from(raw).map_err(Into::into)
    }
}

impl<'a> CborDeserialize<'a> for u16 {
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
        if mt != MajorType::Uint {
            return Err(SeaboredDeError::IncorrectMajorType {
                actual: mt,
                expected: &[MajorType::Uint],
            });
        }

        let raw = read_uint_argument(reader, ai)?;
        ai.ensure_shortest(raw, options, "integer")?;
        u16::try_from(raw).map_err(Into::into)
    }
}

impl<'a> CborDeserialize<'a> for u32 {
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
        if mt != MajorType::Uint {
            return Err(SeaboredDeError::IncorrectMajorType {
                actual: mt,
                expected: &[MajorType::Uint],
            });
        }

        let raw = read_uint_argument(reader, ai)?;
        ai.ensure_shortest(raw, options, "integer")?;
        u32::try_from(raw).map_err(Into::into)
    }
}

impl<'a> CborDeserialize<'a> for u64 {
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
        if mt != MajorType::Uint {
            return Err(SeaboredDeError::IncorrectMajorType {
                actual: mt,
                expected: &[MajorType::Uint],
            });
        }

        let raw = read_uint_argument(reader, ai)?;
        ai.ensure_shortest(raw, options, "integer")?;
        Ok(raw)
    }
}

impl<'a> CborDeserialize<'a> for u128 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from_with_profile<R: Read<'a>>(
        reader: &mut DepthAwareReaderGuard<'_, R>,
        options: DecodeOptions,
    ) -> Result<Self, SeaboredDeError>
    where
        Self: Sized + 'a,
    {
        let tag_ib = InitialByte::peek(reader)?;
        // Not a bignum, fall back to the uXX chain
        if tag_ib.mt() == MajorType::Uint {
            return u64::cbor_deserialize_from_with_profile(reader, options).map(Into::into);
        }

        // Check if it's the correct tag
        if tag_ib.0 != ib::consts::IB_UNSIGNED_BIG_NUM {
            return Err(SeaboredDeError::IncorrectInitialByte {
                actual: tag_ib.0,
                expected: ib::consts::IB_UNSIGNED_BIG_NUM,
            });
        }
        tag_ib.ai().ensure_shortest(2, options, "tag")?;

        // Skip over the Tag IB
        reader.advance(1)?;

        let wire_bytes =
            std::borrow::Cow::<[u8]>::cbor_deserialize_from_with_profile(reader, options)?;
        bignum_to_u128(&wire_bytes, options)
    }
}

impl<'a> CborDeserialize<'a> for i8 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from_with_profile<R: Read<'a>>(
        reader: &mut DepthAwareReaderGuard<'_, R>,
        options: DecodeOptions,
    ) -> Result<Self, SeaboredDeError>
    where
        Self: Sized + 'a,
    {
        i8::try_from(decode_signed_value(reader, options)?).map_err(Into::into)
    }
}

impl<'a> CborDeserialize<'a> for i16 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from_with_profile<R: Read<'a>>(
        reader: &mut DepthAwareReaderGuard<'_, R>,
        options: DecodeOptions,
    ) -> Result<Self, SeaboredDeError>
    where
        Self: Sized + 'a,
    {
        i16::try_from(decode_signed_value(reader, options)?).map_err(Into::into)
    }
}

impl<'a> CborDeserialize<'a> for i32 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from_with_profile<R: Read<'a>>(
        reader: &mut DepthAwareReaderGuard<'_, R>,
        options: DecodeOptions,
    ) -> Result<Self, SeaboredDeError>
    where
        Self: Sized + 'a,
    {
        i32::try_from(decode_signed_value(reader, options)?).map_err(Into::into)
    }
}

impl<'a> CborDeserialize<'a> for i64 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from_with_profile<R: Read<'a>>(
        reader: &mut DepthAwareReaderGuard<'_, R>,
        options: DecodeOptions,
    ) -> Result<Self, SeaboredDeError>
    where
        Self: Sized + 'a,
    {
        i64::try_from(decode_signed_value(reader, options)?).map_err(Into::into)
    }
}

impl<'a> CborDeserialize<'a> for i128 {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from_with_profile<R: Read<'a>>(
        reader: &mut DepthAwareReaderGuard<'_, R>,
        options: DecodeOptions,
    ) -> Result<Self, SeaboredDeError>
    where
        Self: Sized + 'a,
    {
        let tag_ib = InitialByte::peek(reader)?;
        // Tag 2 on the wire: decode as u128, then cast
        if tag_ib.0 == ib::consts::IB_UNSIGNED_BIG_NUM {
            return Ok(Self::try_from(u128::cbor_deserialize_from_with_profile(
                reader, options,
            )?)?);
        }

        let tag_mt = tag_ib.mt();

        match tag_mt {
            MajorType::Uint => {
                return u64::cbor_deserialize_from_with_profile(reader, options).map(Into::into);
            }
            // Not via `i64`: `3b ff..ff` is `-2^64` (RFC 8949 Appendix A), which only `i128` holds
            MajorType::NegativeUint => {
                let ai = tag_ib.ai();
                reader.advance(1)?;
                let n = read_uint_argument(reader, ai)?;
                ai.ensure_shortest(n, options, "integer")?;
                return Ok(-1 - i128::from(n));
            }
            MajorType::Tagged => {}
            _ => {
                return Err(SeaboredDeError::IncorrectMajorType {
                    actual: tag_mt,
                    expected: &[MajorType::Uint, MajorType::NegativeUint, MajorType::Tagged],
                });
            }
        }

        // Check if it's the correct tag
        if tag_ib.0 != ib::consts::IB_NEGATIVE_BIG_NUM {
            return Err(SeaboredDeError::IncorrectInitialByte {
                actual: tag_ib.0,
                expected: ib::consts::IB_NEGATIVE_BIG_NUM,
            });
        }
        tag_ib.ai().ensure_shortest(3, options, "tag")?;

        // Skip over the Tag IB
        reader.advance(1)?;

        let wire_bytes =
            std::borrow::Cow::<[u8]>::cbor_deserialize_from_with_profile(reader, options)?;
        let value_u128 = bignum_to_u128(&wire_bytes, options)?;
        // Tag 3 means -1 - n, so n has to fit i128's positive range
        Ok(-1 - i128::try_from(value_u128)?)
    }
}

#[cfg(test)]
mod tests {

    use super::CborBigInteger;
    use crate::{de::from_bytes, ser::CborSerialize};

    #[test]
    fn bignums_of_every_length() {
        use crate::de::from_bytes;
        use hex_literal::hex;

        // RFC 8949 §3.4.3: tag 2, 9-byte payload for 2^64
        assert_eq!(
            from_bytes::<u128>(&hex!("c249010000000000000000")).unwrap(),
            1 << 64
        );
        assert_eq!(from_bytes::<u128>(&hex!("c24101")).unwrap(), 1);
        assert_eq!(from_bytes::<u128>(&hex!("c240")).unwrap(), 0);
        assert_eq!(
            from_bytes::<i128>(&hex!("c249010000000000000000")).unwrap(),
            1 << 64
        );

        // Leading zeroes are allowed
        let mut leading_zeros = vec![0xc2, 0x51, 0x00];
        leading_zeros.extend_from_slice(&[0xff; 16]);
        assert_eq!(from_bytes::<u128>(&leading_zeros).unwrap(), u128::MAX);

        // 2^128 does not fit
        let mut too_big = vec![0xc2, 0x51, 0x01];
        too_big.extend_from_slice(&[0x00; 16]);
        assert!(from_bytes::<u128>(&too_big).is_err());

        // Tag 3 is -1 - n
        assert_eq!(from_bytes::<i128>(&hex!("c34101")).unwrap(), -2);
        assert_eq!(from_bytes::<i128>(&hex!("c340")).unwrap(), -1);

        let mut i128_min = vec![0xc3, 0x50, 0x7f];
        i128_min.extend_from_slice(&[0xff; 15]);
        assert_eq!(from_bytes::<i128>(&i128_min).unwrap(), i128::MIN);

        // -2^128 does not fit (used to decode as 0)
        let mut under_flow = vec![0xc3, 0x50];
        under_flow.extend_from_slice(&[0xff; 16]);
        assert!(from_bytes::<i128>(&under_flow).is_err());
    }

    #[test]
    fn integer_argument_widths_are_variation_tolerant() {
        use crate::de::from_bytes;
        use hex_literal::hex;

        // Same value in every width; RFC 8949 §5.5's own example (0 in a two-byte argument).
        for zero in [
            &hex!("00")[..],
            &hex!("1800"),
            &hex!("190000"),
            &hex!("1a00000000"),
            &hex!("1b0000000000000000"),
        ] {
            assert_eq!(from_bytes::<u8>(zero).unwrap(), 0);
            assert_eq!(from_bytes::<u64>(zero).unwrap(), 0);
            assert_eq!(from_bytes::<i8>(zero).unwrap(), 0);
            assert_eq!(from_bytes::<i64>(zero).unwrap(), 0);
        }

        // 127 fits every target, whichever width carried it (§4.1)
        for wide in [
            &hex!("187f")[..],
            &hex!("19007f"),
            &hex!("1a0000007f"),
            &hex!("1b000000000000007f"),
        ] {
            assert_eq!(from_bytes::<u8>(wide).unwrap(), 127);
            assert_eq!(from_bytes::<u16>(wide).unwrap(), 127);
            assert_eq!(from_bytes::<u32>(wide).unwrap(), 127);
            assert_eq!(from_bytes::<u64>(wide).unwrap(), 127);
            assert_eq!(from_bytes::<i8>(wide).unwrap(), 127);
            assert_eq!(from_bytes::<i16>(wide).unwrap(), 127);
            assert_eq!(from_bytes::<i32>(wide).unwrap(), 127);
            assert_eq!(from_bytes::<i64>(wide).unwrap(), 127);
        }

        // -128 on the negative major type, same story
        for wide in [
            &hex!("387f")[..],
            &hex!("39007f"),
            &hex!("3a0000007f"),
            &hex!("3b000000000000007f"),
        ] {
            assert_eq!(from_bytes::<i8>(wide).unwrap(), i8::MIN);
            assert_eq!(from_bytes::<i16>(wide).unwrap(), -128);
            assert_eq!(from_bytes::<i64>(wide).unwrap(), -128);
        }

        // `i128` holds the whole 1 + 8 byte range: RFC 8949 Appendix A's own vectors
        assert_eq!(
            from_bytes::<i128>(&hex!("1bffffffffffffffff")).unwrap(),
            (1 << 64) - 1
        );
        assert_eq!(
            from_bytes::<i128>(&hex!("3b7fffffffffffffff")).unwrap(),
            -(1 << 63)
        );
        assert_eq!(
            from_bytes::<i128>(&hex!("3b8000000000000000")).unwrap(),
            -(1 << 63) - 1
        );
        assert_eq!(
            from_bytes::<i128>(&hex!("3bffffffffffffffff")).unwrap(),
            -(1 << 64)
        );

        // Additional information 28..31 is not a well-formed integer (RFC 8949 §3.1, Appendix F.1)
        for reserved in [
            &hex!("1c")[..],
            &hex!("1d"),
            &hex!("1e"),
            &hex!("1f"),
            &hex!("3c"),
            &hex!("3d"),
            &hex!("3e"),
            &hex!("3f"),
        ] {
            assert!(from_bytes::<u8>(reserved).is_err());
            assert!(from_bytes::<i8>(reserved).is_err());
            assert!(from_bytes::<i64>(reserved).is_err());
            assert!(from_bytes::<i128>(reserved).is_err());
        }

        // Width accepted, value still range-checked
        assert!(from_bytes::<i8>(&hex!("190080")).is_err()); // 128
        assert!(from_bytes::<u8>(&hex!("190100")).is_err()); // 256
        assert!(from_bytes::<i16>(&hex!("1a00008000")).is_err()); // 32768
        assert!(from_bytes::<i8>(&hex!("390080")).is_err()); // -129
        assert!(from_bytes::<i64>(&hex!("3bffffffffffffffff")).is_err()); // -2^64
    }

    #[test]
    fn big_integer_normalizes_the_magnitude() {
        assert_eq!(
            CborBigInteger::new(false, vec![0, 0, 1]),
            CborBigInteger::new(false, vec![1])
        );
        assert_eq!(
            CborBigInteger::new(true, vec![0, 0]),
            CborBigInteger::new(true, Vec::new())
        );
        assert!(!CborBigInteger::new(true, vec![0, 0]).is_zero());
        assert!(CborBigInteger::new(false, vec![0, 0]).is_zero());
        assert_eq!(
            CborBigInteger::new(false, &b"\x00\x02"[..]).magnitude(),
            &[2]
        );
    }

    #[test]
    fn upper_bounded_unsigned_in_signed() {
        let u8_value = u8::MAX - 1;
        let i64_value = u8_value as i64;
        let serialized = i64_value.cbor_serialize().unwrap();
        let roundtripped_u8 = from_bytes::<u8>(&serialized).unwrap();
        assert_eq!(u8_value, roundtripped_u8);
    }
}
