use crate::{
    de::CborDeserialize,
    error::{SeaboredDeError, SeaboredSerError},
    ib::{self, InitialByte},
    ser::CborSerialize,
};

use parsio::{DepthAwareReaderGuard, Read, Write};

const SIMPLE_INITIAL_BYTES: &[u8] = &[
    0xe0, 0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xeb, 0xec, 0xed, 0xee, 0xef,
    0xf0, 0xf1, 0xf2, 0xf3, 0xf8,
];

/// A CBOR simple value (RFC 8949 §3.3).
///
/// Values 0..=19 are the one-byte form; 32..=255 use `f8`. Values 20..=23 are
/// assigned to false/true/null/undefined, and 24..=31 are reserved. All of
/// 20..=31 are rejected by encode; the public field can still hold them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CborSimpleValue(pub u8);

impl From<u8> for CborSimpleValue {
    #[inline(always)]
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl From<CborSimpleValue> for u8 {
    #[inline(always)]
    fn from(value: CborSimpleValue) -> Self {
        value.0
    }
}

impl std::fmt::Display for CborSimpleValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "simple({})", self.0)
    }
}

impl CborSerialize for CborSimpleValue {
    #[inline]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        match self.0 {
            0..=19 => Ok(writer.write(&[ib::consts::IB_SIMPLE_VALUE | self.0])?),
            32..=u8::MAX => Ok(writer.write(&[ib::consts::IB_SIMPLE_VALUE_NEXT_BYTE, self.0])?),
            reserved => Err(SeaboredSerError::ReservedSimpleValue(reserved)),
        }
    }
}

impl<'de> CborDeserialize<'de> for CborSimpleValue {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from<R: Read<'de>>(
        reader: &mut DepthAwareReaderGuard<'_, 'de, R>,
    ) -> Result<Self, SeaboredDeError> {
        let ib = InitialByte::cbor_deserialize_from(reader)?;
        let raw = ib.raw();
        match raw {
            ib::consts::IB_SIMPLE_VALUE..=0xf3 => Ok(Self(raw - ib::consts::IB_SIMPLE_VALUE)),
            ib::consts::IB_SIMPLE_VALUE_NEXT_BYTE => {
                let value = reader.read_byte()?;
                if value < 0x20 {
                    Err(SeaboredDeError::NotWellFormed(
                        "two-byte simple value with an argument below 0x20",
                    ))
                } else {
                    Ok(Self(value))
                }
            }
            ib::consts::IB_FALSE..=ib::consts::IB_UNDEFINED => Err(
                SeaboredDeError::ReservedSimpleValue(raw - ib::consts::IB_SIMPLE_VALUE),
            ),
            actual => Err(SeaboredDeError::IncorrectInitialByteSet {
                actual,
                expected: SIMPLE_INITIAL_BYTES,
            }),
        }
    }
}
