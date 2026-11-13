//! RFC 8742 CBOR Sequences: concatenated CBOR data items, basically.

use std::marker::PhantomData;

use parsio::{DepthAwareReader, Read};

use crate::{
    de::CborDeserialize,
    error::{SeaboredDeError, SeaboredSerError},
    profile::{DecodeOptions, Profile},
    ser::CborSerialize,
};

/// Iterator over the CBOR items in an RFC 8742 sequence.
///
/// Empty input is a valid empty sequence.
/// After an error the iterator is fused, so it yields no further items.
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

    /// Set the decoding options, for example the CBOR Profile (deterministic, pref-plus, etc)
    #[inline]
    pub fn with_options(mut self, options: DecodeOptions) -> Self {
        self.options = options;
        self
    }
}

impl<'de, R, T> Iterator for Iter<'de, R, T>
where
    R: Read<'de>,
    T: CborDeserialize<'de> + 'de,
{
    type Item = Result<T, SeaboredDeError>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        match self.reader.is_eof() {
            Ok(false) => {}
            Ok(true) => {
                self.done = true;
                return None;
            }
            Err(error) => {
                self.done = true;
                return Some(Err(error.into()));
            }
        }

        let mut guard = match self.reader.enter() {
            Ok(guard) => guard,
            Err(error) => {
                self.done = true;
                return Some(Err(error.into()));
            }
        };

        let item = T::cbor_deserialize_from_with_profile(&mut guard, self.options);
        if item.is_err() {
            self.done = true;
        }
        Some(item)
    }
}

impl<'de, R, T> std::iter::FusedIterator for Iter<'de, R, T>
where
    R: Read<'de>,
    T: CborDeserialize<'de> + 'de,
{
}

/// Write an RFC 8742 sequence with [`Profile::General`].
///
/// Item bytes are written back to back without any separator.
#[inline]
pub fn to_writer<'a, W, I, T>(writer: &mut W, items: I) -> Result<usize, SeaboredSerError>
where
    W: parsio::Write,
    I: IntoIterator<Item = &'a T>,
    T: CborSerialize + 'a,
{
    to_writer_with_profile(writer, items, Profile::General)
}

/// Write an RFC 8742 sequence with `profile`, applied to each item.
///
/// Item bytes are written back to back without any separator.
#[inline]
pub fn to_writer_with_profile<'a, W, I, T>(
    writer: &mut W,
    items: I,
    profile: Profile,
) -> Result<usize, SeaboredSerError>
where
    W: parsio::Write,
    I: IntoIterator<Item = &'a T>,
    T: CborSerialize + 'a,
{
    let mut written = 0;
    for item in items {
        written += item.cbor_serialize_to_with_profile(writer, profile)?;
    }
    Ok(written)
}
