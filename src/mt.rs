use crate::ib::InitialByte;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
/// CBOR Major Type
pub enum MajorType {
    Uint = 0,
    NegativeUint = 1,
    Bytes = 2,
    String = 3,
    Array = 4,
    Map = 5,
    Tagged = 6,
    SimpleValueOrFloat = 7,
}

impl std::fmt::Display for MajorType {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MajorType::Uint => write!(f, "MajorType::Uint"),
            MajorType::NegativeUint => write!(f, "MajorType::NegativeUint"),
            MajorType::Bytes => write!(f, "MajorType::Bytes"),
            MajorType::String => write!(f, "MajorType::String"),
            MajorType::Array => write!(f, "MajorType::Array"),
            MajorType::Map => write!(f, "MajorType::Map"),
            MajorType::Tagged => write!(f, "MajorType::Tagged"),
            MajorType::SimpleValueOrFloat => write!(f, "MajorType::SimpleValueOrFloat"),
        }
    }
}

impl From<InitialByte> for MajorType {
    #[inline(always)]
    fn from(ib: InitialByte) -> Self {
        // SAFETY: `u8::MAX >> 5 == 7`, and MajorType is 0..=7.
        unsafe { std::mem::transmute(ib.0 >> 5u8) }
    }
}

impl From<&crate::types::CborInteger> for MajorType {
    #[inline(always)]
    fn from(value: &crate::types::CborInteger) -> Self {
        // SAFETY: `bool` is 0/1, matching the Uint/NegativeUint discriminants.
        unsafe { std::mem::transmute(value.negative as u8) }
    }
}
