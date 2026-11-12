use crate::{
    de::CborDeserialize,
    error::{SeaboredDeError, SeaboredSerError},
    ib::{self, InitialByte},
    mt::MajorType,
    profile::{DecodeOptions, Profile},
    ser::CborSerialize,
    types::CborIntegerValue,
};
use parsio::{DepthAwareReaderGuard, Read, Write};

const MAX_PREALLOC_CAPACITY: usize = 256;

/// A CBOR map with typed keys and values.
///
/// Unlike [`CborSequence`](crate::types::CborSequence), the key/value split is known, so
/// [`Profile::Deterministic`] can sort entries by their encoded key bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct CborMap<K: CborSerialize, V: CborSerialize> {
    inner: Vec<(K, V)>,
    /// Indefinite form (`bf` … `ff`, RFC 8949 §3.2.2). Decoding preserves it; set `false` to
    /// re-encode definite, as strict profiles require.
    pub is_indefinite: bool,
}

impl<K: CborSerialize, V: CborSerialize> Default for CborMap<K, V> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl<K: CborSerialize, V: CborSerialize> CborMap<K, V> {
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            inner: Vec::new(),
            is_indefinite: false,
        }
    }

    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Vec::with_capacity(capacity.min(MAX_PREALLOC_CAPACITY)),
            is_indefinite: false,
        }
    }

    /// The indefinite-length form. Strict profiles reject it.
    #[inline(always)]
    pub fn new_indefinite() -> Self {
        Self {
            inner: Vec::new(),
            is_indefinite: true,
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl<K: CborSerialize, V: CborSerialize> std::ops::Deref for CborMap<K, V> {
    type Target = Vec<(K, V)>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<K: CborSerialize, V: CborSerialize> std::ops::DerefMut for CborMap<K, V> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<K: CborSerialize, V: CborSerialize> IntoIterator for CborMap<K, V> {
    type Item = (K, V);
    type IntoIter = std::vec::IntoIter<(K, V)>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a, K: CborSerialize, V: CborSerialize> IntoIterator for &'a CborMap<K, V> {
    type Item = &'a (K, V);
    type IntoIter = std::slice::Iter<'a, (K, V)>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl<K: CborSerialize, V: CborSerialize> Extend<(K, V)> for CborMap<K, V> {
    #[inline(always)]
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        self.inner.extend(iter);
    }
}

impl<K: CborSerialize, V: CborSerialize> FromIterator<(K, V)> for CborMap<K, V> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self {
            inner: iter.into_iter().collect(),
            is_indefinite: false,
        }
    }
}

impl<K: CborSerialize, V: CborSerialize> From<Vec<(K, V)>> for CborMap<K, V> {
    #[inline(always)]
    fn from(inner: Vec<(K, V)>) -> Self {
        Self {
            inner,
            is_indefinite: false,
        }
    }
}

impl<K: CborSerialize, V: CborSerialize> CborSerialize for CborMap<K, V> {
    #[inline]
    fn cbor_serialize_to<W: Write>(&self, writer: &mut W) -> Result<usize, SeaboredSerError> {
        self.cbor_serialize_to_with_profile(writer, Profile::General)
    }

    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_serialize_to_with_profile<W: Write>(
        &self,
        writer: &mut W,
        profile: Profile,
    ) -> Result<usize, SeaboredSerError> {
        if self.is_indefinite {
            if profile.is_strict() {
                return Err(SeaboredSerError::IndefiniteForbidden {
                    profile,
                    what: "map",
                });
            }

            let mut written = writer.write(&[ib::consts::IB_MAP_SEQUENCE_START])?;
            for (key, value) in &self.inner {
                written += key.cbor_serialize_to(writer)?;
                written += value.cbor_serialize_to(writer)?;
            }
            written += writer.write(&[ib::consts::IB_BREAK])?;
            return Ok(written);
        }

        if profile == Profile::Deterministic {
            let mut entries = Vec::with_capacity(self.inner.len());
            for (key, value) in &self.inner {
                let mut key_bytes = Vec::new();
                key.cbor_serialize_to_with_profile(&mut key_bytes, profile)?;
                entries.push((key_bytes, value));
            }
            entries.sort_by(|left, right| left.0.cmp(&right.0));

            let mut written = CborIntegerValue::from(entries.len())
                .serialize_complex_mt_preamble(MajorType::Map, writer)?;
            for (key_bytes, value) in entries {
                written += writer.write(&key_bytes)?;
                written += value.cbor_serialize_to_with_profile(writer, profile)?;
            }
            return Ok(written);
        }

        let mut written = CborIntegerValue::from(self.inner.len())
            .serialize_complex_mt_preamble(MajorType::Map, writer)?;
        for (key, value) in &self.inner {
            written += key.cbor_serialize_to_with_profile(writer, profile)?;
            written += value.cbor_serialize_to_with_profile(writer, profile)?;
        }
        Ok(written)
    }
}

impl<'de, K, V> CborDeserialize<'de> for CborMap<K, V>
where
    K: CborSerialize + CborDeserialize<'de> + 'de,
    V: CborSerialize + CborDeserialize<'de> + 'de,
{
    #[cfg_attr(feature = "inline-nontrivial", inline)]
    fn cbor_deserialize_from_with_profile<R: Read<'de>>(
        reader: &mut DepthAwareReaderGuard<'_, R>,
        options: DecodeOptions,
    ) -> Result<Self, SeaboredDeError> {
        let ib = InitialByte::cbor_deserialize_from(reader)?;
        let (mt, ai) = ib.mt_ai();
        if mt != MajorType::Map {
            return Err(SeaboredDeError::IncorrectMajorType {
                actual: mt,
                expected: &[MajorType::Map],
            });
        }
        ai.ensure_finite_length(options, "map")?;

        match ai.length(reader)? {
            Some(len) => {
                ai.ensure_shortest(len, options, "map length")?;
                let len: usize = len.try_into()?;
                let cap = len
                    .min(reader.remaining().unwrap_or(usize::MAX))
                    .min(MAX_PREALLOC_CAPACITY);
                let mut map = Self::with_capacity(cap);
                let mut nested = reader.enter()?;
                let mut last_key: Option<Vec<u8>> = None;
                for _ in 0..len {
                    let key = K::cbor_deserialize_from_with_profile(&mut nested, options)?;
                    if options.is_checking()
                        && options.profile == crate::profile::Profile::Deterministic
                    {
                        let mut key_bytes = Vec::new();
                        key.cbor_serialize_to_with_profile(&mut key_bytes, options.profile)
                            .map_err(|_| SeaboredDeError::NonPreferredEncoding {
                                what: "map key order",
                            })?;
                        if let Some(previous) = &last_key
                            && key_bytes <= *previous
                        {
                            return Err(SeaboredDeError::NonPreferredEncoding {
                                what: "map key order",
                            });
                        }
                        last_key = Some(key_bytes);
                    }
                    let value = V::cbor_deserialize_from_with_profile(&mut nested, options)?;
                    map.push((key, value));
                }
                Ok(map)
            }
            None => {
                let mut map = Self::new_indefinite();
                let mut nested = reader.enter()?;
                loop {
                    if InitialByte::peek(&mut nested)?.raw() == ib::consts::IB_BREAK {
                        nested.advance(1)?;
                        break;
                    }
                    map.push(<(K, V)>::cbor_deserialize_from_with_profile(
                        &mut nested,
                        options,
                    )?);
                }
                Ok(map)
            }
        }
    }
}
