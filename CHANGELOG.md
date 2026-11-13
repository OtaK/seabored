# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## 0.2.0 - unreleased

Adds draft-09 serialization profiles and native derives, enforces recursion limits, and
hardens bignum, serde, and malformed-input handling. General entry points stay
permissive; breaks are listed under Changed.

### Added

- `derive` feature: `CborSerialize` / `CborDeserialize` derives for structs and enums,
  with `#[cbor(key, tag, simple_value, with, default, skip, indefinite, profile, map,
  array)]` and `rfc` / `pref-plus` / `deterministic` profiles. `with` hooks receive the
  active profile/options.
- Draft-09 `Profile` and `DecodeOptions`: strict encode/decode entry points for
  preferred-plus and deterministic output, recursive header/container/float/bignum
  checking, and bytewise map-key sorting under `Deterministic`.
- `CborBigInteger` and `Value::BigInteger` for normalized tag 2/3 values.
- `CborMap<K, V>` with deterministic key ordering.
- `OwnedValue` and `Value::into_owned()` for decoders that cannot lend.
- Depth limits on native and serde decoding (default 256), `_with_limit` variants, and
  complete-input serde entry points.
- `CborContainer` iteration/extend support and `CborSimpleValue`.
- Native RFC 8742 CBOR Sequences: `sequence::Iter`, `sequence::to_writer`,
  `sequence::to_writer_with_profile`, `CborSeqSerialize`, and
  `CborSeqDeserialize`.
  - `#[cbor(sequence)]` derive for tuple newtypes, with generic container bounds
    (`Vec`, `VecDeque`, `LinkedList`, `BTreeSet`, `HashSet`, `BinaryHeap`, and
    compatible custom containers). `BTreeSet` and `HashSet` reject duplicate
    decoded items at `insert`; other containers use `Extend` semantics.
  - `#[cbor(sequence, unordered)]` rejects deterministic encoding.
  - Also added a serde RFC 8742 bridge: lazy `serde::sequence::Iter`, `from_reader`,
    `from_reader_with_options`, `to_writer`, and `to_writer_with_profile`.
- New error variants for profiles, bignums, trailing bytes, struct keys, and
  duplicate sequence items.

### Fixed

- Removed UB in the serde sentinel path for `SimpleValue`, `RawValue`,
  and dynamic values.
- Bignum decoding no longer panics or truncates; oversized magnitudes are rejected and
  leading zeroes are normalized.
- Deeply nested input no longer overflows the stack.
- Indefinite strings and bare simple values now decode correctly through serde.
- `Value::Map` re-serializes as a map, not an array.
- Negative/payload NaNs encode canonically; `i128` boundaries serialize and decode
  correctly.
- Unknown struct fields are skipped without building a `Value`.
- Tuple/enum lengths, non-well-formed simple values, invalid string chunks, and
  out-of-range integers are rejected instead of silently misdecoded.
- Untrusted size hints no longer cause large heap preallocations.

### Changed

- **Breaking:** `CborDeserialize` now requires
  `cbor_deserialize_from_with_profile(reader, options)`; `cbor_deserialize_from` takes
  `&mut DepthAwareReaderGuard<'_, R>`. Use `de::from_bytes` or
  `from_bytes_with_profile` for the common cases.
- **Breaking:** `serde::Deserializer::new` takes `&mut DepthAwareReader<R>` and returns
  `Result`; `into_inner` is removed.
- **Breaking:** `parsio` is pinned to `0.1.0-rc.7`; `DepthAwareReader` is no longer
  lifetime-parameterized.
- **Breaking:** `Value` no longer keeps tags 2/3 as `Value::Tagged`; fitting values
  become `Value::Integer`, larger ones `Value::BigInteger`, and leading zeroes are
  normalized.
- **Breaking:** `Value::from_iter` builds definite-length containers. Use
  `CborContainer::new_indefinite` for the indefinite form.
- **Breaking:** `CborSequence` is renamed to `CborContainer`, `Value::Sequence` to
  `Value::Array`, and `Value::as_sequence` to `Value::as_array`. The old names described
  CBOR containers, not RFC 8742 sequences.
- **Breaking:** `SeaboredDeError::AllowedDepthOverflow` is removed (overflow now comes
  from `parsio`); `SimpleValue(20..=31)` is rejected; `SeaboredSerError` is now
  `#[non_exhaustive]`.
- **Breaking:** integers outside the target type now error instead of wrapping around.
- The general entry points (`de::from_bytes`, `CborSerialize::cbor_serialize*`,
  `serde::from_slice`, `serde::to_vec`) remain permissive; draft-09 conformance requires
  the profile entry points.
- Integer decoding is 10–31% faster on wide arguments.
