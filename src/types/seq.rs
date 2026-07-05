use crate::{ib, mt::MajorType, ser::CborSerialize, types::CborIntegerValue};
use parsio::Write;

const MAX_PREALLOC_CAPACITY: usize = 256;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct CborSequence<T: CborSerialize> {
    inner: Vec<T>,
    pub mt: MajorType,
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

impl<T: CborSerialize> CborSequence<T> {
    #[inline(always)]
    pub fn new_finite(mt: MajorType, len: usize) -> Self {
        Self {
            inner: Vec::with_capacity(len.min(MAX_PREALLOC_CAPACITY)),
            mt,
            is_indefinite: false,
        }
    }

    #[inline(always)]
    pub fn new_indefinite(mt: MajorType) -> Self {
        Self {
            // Preallocate 4 entries (the allocation strategy is 0, 1, 4, 8, ...)
            // so we amortize-ish ahead because indefinite collections are usually
            // not small
            inner: Vec::with_capacity(MAX_PREALLOC_CAPACITY),
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
