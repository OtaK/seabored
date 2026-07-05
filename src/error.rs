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
#[error("{inner:?} with external cause {external:?}")]
pub struct WinnowError<'a> {
    inner: winnow::error::ContextError<&'a [u8]>,
    external: Option<&'a (dyn std::error::Error + 'static)>,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SeaboredDeError<'a> {
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
    #[error("The parser needs more bytes: {0:?}")]
    Incomplete(winnow::error::Needed),
    #[error("Parsing error {0}")]
    Parsing(WinnowError<'a>),
    #[error("Recoverable error {0}")]
    Recoverable(WinnowError<'a>),
    #[error("Unrecoverable error {0}")]
    Unrecoverable(WinnowError<'a>),
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
    #[error(transparent)]
    IntConversionError(#[from] std::num::TryFromIntError),
    #[cfg(feature = "serde")]
    #[error("Serde error: {0}")]
    Serde(String),
    #[cfg(feature = "facet")]
    #[error("The Facet Scalar Type {0:?} is unsupported")]
    UnsupportedFacetScalar(facet::ScalarType),
    #[cfg(feature = "facet")]
    #[error(transparent)]
    FacetReflectError(#[from] facet_reflect::ReflectError),
}

impl<'a> From<winnow::error::ContextError<&'a [u8]>> for SeaboredDeError<'a> {
    fn from(value: winnow::error::ContextError<&'a [u8]>) -> Self {
        Self::Parsing(WinnowError {
            inner: value,
            external: None,
        })
    }
}

impl<'a> From<winnow::error::ErrMode<winnow::error::ContextError<&'a [u8]>>>
    for SeaboredDeError<'a>
{
    fn from(value: winnow::error::ErrMode<winnow::error::ContextError<&'a [u8]>>) -> Self {
        match value {
            winnow::error::ErrMode::Incomplete(needed) => Self::Incomplete(needed),
            winnow::error::ErrMode::Backtrack(e) => Self::Recoverable(WinnowError {
                inner: e,
                external: None,
            }),
            winnow::error::ErrMode::Cut(e) => Self::Unrecoverable(WinnowError {
                inner: e,
                external: None,
            }),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SeaboredError<'a> {
    #[error(transparent)]
    Ser(#[from] SeaboredSerError),
    #[error(transparent)]
    De(SeaboredDeError<'a>),
}

impl<'a> From<SeaboredDeError<'a>> for SeaboredError<'a> {
    fn from(value: SeaboredDeError<'a>) -> Self {
        Self::De(value)
    }
}
