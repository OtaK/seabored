use crate::{ib, mt::MajorType, ser::CborSerialize, types::CborIntegerValue};
use parsio::Write;

const MAX_PREALLOC_CAPACITY: usize = 256;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct CborSequence<T: CborSerialize> {
    inner: Vec<T>,
    pub mt: MajorType,
    /// Indefinite form (`9f`/`bf` … `ff`, RFC 8949 §3.2.2). Decoding preserves it; set `false`
    /// to re-encode definite, as the profile requires.
    pub is_indefinite: bool,
}

impl<T: CborSerialize> Default for CborSequence<T> {
    #[inline(always)]
    fn default() -> Self {
        Self {
            inner: Default::default(),
            mt: MajorType::Array,
            is_indefinite: Default::default(),
        }
    }
}

impl<T: CborSerialize> IntoIterator for CborSequence<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    /// Consuming access; `Deref` gives borrowed access.
    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a, T: CborSerialize> IntoIterator for &'a CborSequence<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    /// Borrowed access, no clone.
    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl<T: CborSerialize> Extend<T> for CborSequence<T> {
    /// Appends items; `mt` and `is_indefinite` are kept.
    #[inline(always)]
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.inner.extend(iter);
    }
}

impl<T: CborSerialize> FromIterator<T> for CborSequence<T> {
    /// Definite-length array (RFC 8949 §3.2.2, profile §4.1 item 2). Indefinite stays explicit:
    /// [`CborSequence::new_indefinite`].
    #[inline(always)]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::from(iter.into_iter().collect::<Vec<T>>())
    }
}

impl<T: CborSerialize> CborSequence<T> {
    #[inline(always)]
    pub fn new_finite(mt: MajorType, len: usize) -> Self {
        Self {
            inner: Vec::with_capacity(len.min(MAX_PREALLOC_CAPACITY)),
            mt,
            is_indefinite: false,
        }
    }

    /// The indefinite-length form, which the profile forbids encoding — hence explicit.
    #[inline(always)]
    pub fn new_indefinite(mt: MajorType) -> Self {
        Self {
            // Lazy: pre-allocating here is capacity an attacker picks for free (RFC 8949 §10)
            inner: Vec::new(),
            mt,
            is_indefinite: true,
        }
    }

    #[inline(always)]
    pub fn with_mt(mut self, mt: MajorType) -> Self {
        self.mt = mt;
        self
    }
}

impl<T: CborSerialize> std::ops::Deref for CborSequence<T> {
    type Target = Vec<T>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: CborSerialize> std::ops::DerefMut for CborSequence<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: CborSerialize> From<Vec<T>> for CborSequence<T> {
    #[inline(always)]
    fn from(inner: Vec<T>) -> Self {
        Self {
            inner,
            mt: MajorType::Array,
            is_indefinite: false,
        }
    }
}

impl<T: CborSerialize> CborSerialize for CborSequence<T> {
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to<W: Write>(
        &self,
        writer: &mut W,
    ) -> Result<usize, crate::error::SeaboredSerError> {
        let mut written = if self.is_indefinite {
            writer.write(&[((self.mt as u8) << 5u8) | 0x1F])?
        } else {
            CborIntegerValue::from(self.inner.len())
                .serialize_complex_mt_preamble(self.mt, writer)?
        };

        for value in &self.inner {
            written += value.cbor_serialize_to(writer)?;
        }

        if self.is_indefinite {
            written += writer.write(&[ib::consts::IB_BREAK])?;
        }

        Ok(written)
    }
}
