//! Serde bridge for RFC 8742 CBOR Sequences.

use std::marker::PhantomData;

use parsio::{DepthAwareReader, Read, Write};

use crate::{
    error::{SeaboredDeError, SeaboredSerError},
    profile::{DecodeOptions, Profile},
    serde::Deserializer,
};

/// RFC 8742 sequence lazy serde deserializer
///
/// Each `next` decodes exactly one item. Empty input yields no items. After an error
/// the iterator is fused.
pub struct Iter<'de, R: Read<'de>, T> {
    reader: DepthAwareReader<R>,
    options: DecodeOptions,
    done: bool,
    _boo: PhantomData<fn() -> &'de T>,
}

impl<'de, R: Read<'de>, T> Iter<'de, R, T> {
    /// Decode with [`DecodeOptions::GENERAL`].
    #[inline]
    pub fn new(reader: DepthAwareReader<R>) -> Self {
        Self {
            reader,
            options: DecodeOptions::GENERAL,
            done: false,
            _boo: PhantomData,
        }
    }

    /// Replace the decoding options.
    ///
    /// Checking options cannot be supported by this streaming bridge; the first `next`
    /// returns [`SeaboredDeError::ProfileUnsupported`].
    #[inline]
    pub fn with_options(mut self, options: DecodeOptions) -> Self {
        self.options = options;
        self
    }
}

impl<'de, R, T> Iterator for Iter<'de, R, T>
where
    R: Read<'de>,
    T: ::serde_core::Deserialize<'de> + 'de,
{
    type Item = Result<T, SeaboredDeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        if self.options.is_checking() {
            self.done = true;
            return Some(Err(SeaboredDeError::ProfileUnsupported {
                profile: self.options.profile,
                what: "checking serde sequence from_reader",
            }));
        }

        match self.reader.is_eof() {
            Ok(true) => {
                self.done = true;
                return None;
            }
            Ok(false) => {}
            Err(error) => {
                self.done = true;
                return Some(Err(error.into()));
            }
        }

        let mut deserializer = match Deserializer::new_with_options(&mut self.reader, self.options)
        {
            Ok(deserializer) => deserializer,
            Err(error) => {
                self.done = true;
                return Some(Err(error));
            }
        };

        let item = T::deserialize(&mut deserializer);
        if item.is_err() {
            self.done = true;
        }
        Some(item)
    }
}

impl<'de, R, T> std::iter::FusedIterator for Iter<'de, R, T>
where
    R: Read<'de>,
    T: ::serde_core::Deserialize<'de> + 'de,
{
}

/// RFC 8742 sequence lazy serde deserializer
#[inline]
pub fn from_reader<'de, T, R>(reader: DepthAwareReader<R>) -> Iter<'de, R, T>
where
    R: Read<'de>,
    T: ::serde_core::Deserialize<'de> + 'de,
{
    Iter::new(reader)
}

/// RFC 8742 sequence lazy serde deserializer with explicit decoding options.
///
/// Checking options need a complete item before deserializing, which this streaming
/// bridge cannot provide. They return [`SeaboredDeError::ProfileUnsupported`].
#[inline]
pub fn from_reader_with_options<'de, T, R>(
    reader: DepthAwareReader<R>,
    options: DecodeOptions,
) -> Result<Iter<'de, R, T>, SeaboredDeError>
where
    R: Read<'de>,
    T: ::serde_core::Deserialize<'de> + 'de,
{
    if options.is_checking() {
        return Err(SeaboredDeError::ProfileUnsupported {
            profile: options.profile,
            what: "checking serde sequence from_reader",
        });
    }
    Ok(Iter::new(reader).with_options(options))
}

/// RFC 8742 sequence serialize zero or more independent items with [`Profile::General`].
#[inline]
pub fn to_writer<'a, W, I, T>(writer: &mut W, items: I) -> Result<usize, SeaboredSerError>
where
    W: Write,
    I: IntoIterator<Item = &'a T>,
    T: ::serde_core::Serialize + 'a,
{
    to_writer_with_profile(writer, items, Profile::General)
}

/// RFC 8742 sequence serialize zero or more independent items with `profile`, applied to each item.
#[inline]
pub fn to_writer_with_profile<'a, W, I, T>(
    writer: &mut W,
    items: I,
    profile: Profile,
) -> Result<usize, SeaboredSerError>
where
    W: Write,
    I: IntoIterator<Item = &'a T>,
    T: ::serde_core::Serialize + 'a,
{
    let mut written = 0;
    for item in items {
        written += crate::serde::to_writer_with_profile(writer, item, profile)?;
    }
    Ok(written)
}
