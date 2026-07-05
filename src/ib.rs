use crate::{
    de::CborDeserialize, error::SeaboredDeError, io::ReadExt as _, mt::MajorType,
    types::CborIntegerValue,
};
use parsio::Read;

pub mod consts {
    pub const IB_SMALL_UINT: u8 = 0x00;
    pub const IB_UINT_8: u8 = 0x18;
    pub const IB_UINT_16: u8 = 0x19;
    pub const IB_UINT_32: u8 = 0x1A;
    pub const IB_UINT_64: u8 = 0x1B;
    pub const IB_SMALL_NEGATIVE_UINT: u8 = 0x20;
    pub const IB_NEGATIVE_UINT_8: u8 = 0x38;
    pub const IB_NEGATIVE_UINT_16: u8 = 0x39;
    pub const IB_NEGATIVE_UINT_32: u8 = 0x3A;
    pub const IB_NEGATIVE_UINT_64: u8 = 0x3B;
    pub const IB_SMALL_BYTE_STRING: u8 = 0x40;
    pub const IB_BYTE_STRING_UINT_8_LEN: u8 = 0x58;
    pub const IB_BYTE_STRING_UINT_16_LEN: u8 = 0x59;
    pub const IB_BYTE_STRING_UINT_32_LEN: u8 = 0x5A;
    pub const IB_BYTE_STRING_UINT_64_LEN: u8 = 0x5B;
    pub const IB_BYTE_STRING_SEQUENCE_START: u8 = 0x5F;
    pub const IB_SMALL_UTF_8_STRING: u8 = 0x60;
    pub const IB_UTF_8_STRING_UINT_8_LEN: u8 = 0x78;
    pub const IB_UTF_8_STRING_UINT_16_LEN: u8 = 0x79;
    pub const IB_UTF_8_STRING_UINT_32_LEN: u8 = 0x7A;
    pub const IB_UTF_8_STRING_UINT_64_LEN: u8 = 0x7B;
    pub const IB_UTF_8_STRING_SEQUENCE_START: u8 = 0x7F;
    pub const IB_SMALL_ARRAY: u8 = 0x80;
    pub const IB_ARRAY_UINT_8_LEN: u8 = 0x98;
    pub const IB_ARRAY_UINT_16_LEN: u8 = 0x99;
    pub const IB_ARRAY_UINT_32_LEN: u8 = 0x9A;
    pub const IB_ARRAY_UINT_64_LEN: u8 = 0x9B;
    pub const IB_ARRAY_SEQUENCE_START: u8 = 0x9F;
    pub const IB_SMALL_MAP: u8 = 0xA0;
    pub const IB_MAP_UINT_8_LEN: u8 = 0xB8;
    pub const IB_MAP_UINT_16_LEN: u8 = 0xB9;
    pub const IB_MAP_UINT_32_LEN: u8 = 0xBA;
    pub const IB_MAP_UINT_64_LEN: u8 = 0xBB;
    pub const IB_MAP_SEQUENCE_START: u8 = 0xBF;
    pub const IB_TAG_START_OR_DATE_TIME_TEXT: u8 = 0xC0;
    pub const IB_DATE_TIME_EPOCH: u8 = 0xC1;
    pub const IB_UNSIGNED_BIG_NUM: u8 = 0xC2;
    pub const IB_NEGATIVE_BIG_NUM: u8 = 0xC3;
    pub const IB_DECIMAL_FRACTION: u8 = 0xC4;
    pub const IB_BIG_FLOAT: u8 = 0xC5;
    pub const IB_EXPECTED_CONVERSION_BASE_64_NO_PAD: u8 = 0xD5;
    pub const IB_EXPECTED_CONVERSION_BASE_64_PAD: u8 = 0xD6;
    pub const IB_EXPECTED_CONVERSION_HEX_UPPER: u8 = 0xD7;
    pub const IB_TAG_NEXT_UINT_8: u8 = 0xD8;
    pub const IB_TAG_NEXT_UINT_16: u8 = 0xD9;
    pub const IB_TAG_NEXT_UINT_32: u8 = 0xDA;
    pub const IB_TAG_NEXT_UINT_64: u8 = 0xDB;
    pub const IB_SIMPLE_VALUE: u8 = 0xE0;
    pub const IB_FALSE: u8 = 0xF4;
    pub const IB_TRUE: u8 = 0xF5;
    pub const IB_NULL: u8 = 0xF6;
    pub const IB_UNDEFINED: u8 = 0xF7;
    pub const IB_SIMPLE_VALUE_NEXT_BYTE: u8 = 0xF8;
    pub const IB_FLOAT_16: u8 = 0xF9;
    pub const IB_FLOAT_32: u8 = 0xFA;
    pub const IB_FLOAT_64: u8 = 0xFB;
    pub const IB_BREAK: u8 = 0xFF;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct InitialByte(pub(crate) u8);

impl From<MajorType> for InitialByte {
    #[inline(always)]
    fn from(value: MajorType) -> Self {
        // SAFETY: CBOR defines the relationship between Major Types and InitialByte
        // Since Major Types go from 0 to 7 included, the max value when shifted by 5 becomes 224
        // which is the InitialByte for SimpleValues
        // Additionally, since [`InitialByte`] has a repr(transparent) over its u8 member, this
        // is a valid transmute
        unsafe { std::mem::transmute((value as u8) << 5) }
    }
}

impl<'a> CborDeserialize<'a> for InitialByte {
    #[inline(always)]
    fn cbor_deserialize_from<R: Read<'a>>(reader: &mut R) -> Result<Self, SeaboredDeError<'a>> {
        reader.read_byte().map(InitialByte).map_err(Into::into)
    }
}

impl std::ops::Deref for InitialByte {
    type Target = u8;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl InitialByte {
    #[inline(always)]
    pub fn mt(&self) -> MajorType {
        MajorType::from(*self)
    }

    #[inline(always)]
    pub fn ai(&self) -> AdditionalInfo {
        AdditionalInfo::from(*self)
    }

    #[inline(always)]
    pub fn mt_ai(&self) -> (MajorType, AdditionalInfo) {
        (self.mt(), self.ai())
    }

    #[inline(always)]
    pub fn peek<'a, R: Read<'a>>(reader: &mut R) -> Result<Self, SeaboredDeError<'a>> {
        reader.peek_byte().map(Self).map_err(Into::into)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct AdditionalInfo(pub(crate) u8);

impl AdditionalInfo {
    #[inline(always)]
    pub(crate) fn action<'a>(&self) -> Result<AdditionalInfoAction, SeaboredDeError<'a>> {
        Ok(match self.0 {
            24 => AdditionalInfoAction::Uint8,
            25 => AdditionalInfoAction::Uint16,
            26 => AdditionalInfoAction::Uint32,
            27 => AdditionalInfoAction::Uint64,
            28..31 => return Err(SeaboredDeError::IllegalAdditionalInfo(self.0)),
            31 => AdditionalInfoAction::IndefiniteLenSeq,
            _ => AdditionalInfoAction::DoNothing,
        })
    }

    #[cfg_attr(feature = "inline-nontrivial", inline)]
    pub(crate) fn find_subsequent_len<'data, R: Read<'data>>(
        &self,
        reader: &mut R,
    ) -> Result<CborIntegerValue, SeaboredDeError<'data>> {
        Ok(match self.action()? {
            crate::ib::AdditionalInfoAction::DoNothing => CborIntegerValue::from(self.0),
            crate::ib::AdditionalInfoAction::Uint8 => CborIntegerValue::from(reader.read_byte()?),
            crate::ib::AdditionalInfoAction::Uint16 => {
                CborIntegerValue::from(reader.read_be_u16()?)
            }
            crate::ib::AdditionalInfoAction::Uint32 => {
                CborIntegerValue::from(reader.read_be_u32()?)
            }
            crate::ib::AdditionalInfoAction::Uint64 => {
                CborIntegerValue::from(reader.read_be_u64()?)
            }
            crate::ib::AdditionalInfoAction::IndefiniteLenSeq => {
                return Err(SeaboredDeError::IndefiniteLen);
            }
        })
    }
}

impl From<InitialByte> for AdditionalInfo {
    #[inline(always)]
    fn from(value: InitialByte) -> Self {
        Self(value.0 & 0x1f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub(crate) enum AdditionalInfoAction {
    #[default]
    DoNothing = 0,
    Uint8 = 1 << 0,
    Uint16 = 1 << 1,
    Uint32 = 1 << 2,
    Uint64 = 1 << 3,
    IndefiniteLenSeq = consts::IB_BREAK, // We'll look for BREAK code which is 0xFF
}
