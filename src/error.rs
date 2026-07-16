use crate::mt::MajorType;

#[derive(Debug, thiserror::Error)]
pub enum SeaboredSerError {
    #[error(transparent)]
    Parsio(#[from] parsio::WriteError),
    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("I/O Error: {0}")]
    IoKind(std::io::ErrorKind),
    #[error("Unexpected Major type for inline bytes, expected Bytes or String, got: {0}")]
    UnexpectedInlineBytesMajorType(MajorType),
    #[error(
        "Unexpected Major type for complex type preamble, expected one of (Bytes, String, Array, Map, Tagged), but got: {0}"
    )]
    UnexpectedPreambleMajorType(MajorType),
    #[cfg(feature = "serde")]
    #[error("Serde error: {0}")]
    Serde(String),
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SeaboredDeError {
    #[error(transparent)]
    Parsio(#[from] parsio::ReadError),
    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("I/O Error: {0}")]
    IoKind(std::io::ErrorKind),
    #[error(
        "Allowed depth exceeded (billion laughs detected?), we are {depth} levels deep but only {limit} is allowed"
    )]
    AllowedDepthOverflow { depth: usize, limit: usize },
    #[error("Wrong MajorType encountered, got {actual}, expected one of {expected:?}")]
    IncorrectMajorType {
        actual: MajorType,
        expected: &'static [MajorType],
    },
    #[error("Wrong InitialByte encountered, got {actual}, expected {expected}")]
    IncorrectInitialByte { actual: u8, expected: u8 },
    #[error(transparent)]
    SimdUtf8Error(#[from] simdutf8::basic::Utf8Error),
    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error(transparent)]
    OwnedUtf8Error(#[from] std::string::FromUtf8Error),
    #[error("Illegal state: the {0} additional info has been found, which is malformed")]
    IllegalAdditionalInfo(u8),
    #[error("Cannot deserialize float: precision loss detected")]
    FloatPrecisionLoss,
    #[error("Wrong Tag, got {actual}, expected {expected}")]
    WrongTag { actual: u64, expected: u64 },
    #[error("Wrong SimpleValue used, as it is reserved for another RFC-described type: {0}")]
    ReservedSimpleValue(u8),
    #[error(
        "This SimpleValue is not supported (yet, please ask for support or contribute it yourself)"
    )]
    UnsupportedSimpleValue(u8),
    #[error("Indefinite len value - not an error that is supposed to be user-visible")]
    IndefiniteLen,
    #[error("A CBOR Sequence has the wrong number of elements, expected {expected}, but got {got}")]
    WrongLen { expected: usize, got: usize },
    #[error(transparent)]
    IntConversionError(#[from] std::num::TryFromIntError),
    #[cfg(feature = "serde")]
    #[error("Serde error: {0}")]
    Serde(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SeaboredError {
    #[error(transparent)]
    Ser(#[from] SeaboredSerError),
    #[error(transparent)]
    De(SeaboredDeError),
}

impl From<SeaboredDeError> for SeaboredError {
    fn from(value: SeaboredDeError) -> Self {
        Self::De(value)
    }
}
