# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## 0.2.0 - unreleased

Hardening, native derive, recursion limits, bignum decoding, serde sentinel path. No byte changes for valid data;
every break is a new rejection or removed UB.

### Added

- `OwnedValue`: a `Value` that owns its contents and is `DeserializeOwned`, for decoders that cannot lend (`ciborium::from_reader`, streaming). `Value` borrows and can never satisfy that bound; its docs say so.
- `Value::into_owned()`: drops the borrow, keeping major type and definiteness.
- `CborSequence`: `IntoIterator` (owned and `&`), `Extend`, `FromIterator`. `FromIterator` builds definite-length; `Extend` keeps `mt`/`is_indefinite`.
- Depth limit (256, `parsio::DepthAwareReader::DEFAULT_LIMIT`) on both paths: `de::from_bytes(_with_limit)`, `serde::from_slice_with_limit` / `from_reader_with_limit`, `serde::from_slice_complete` / `from_reader_complete` (exactly one item), `serde::Deserializer::{depth, limit, remaining}`.
- `SeaboredDeError::{TrailingBytes(usize), BignumTooLarge(usize), MissingField, DuplicateField, UnexpectedKey, NonPreferredEncoding}`.
- Native derive behind the `derive` feature: `CborSerialize` / `CborDeserialize` on structs and enums, with `#[cbor(key, tag, simple_value, with, default, skip, indefinite, profile, map, array)]`. Profiles `rfc` (default), `pref-plus`, and `deterministic`; deterministic output sorts map keys by encoded bytes and strict decode enforces/validates them.
- `InitialByte::raw`, `AdditionalInfo::{raw, is_indefinite, length, is_shortest}`, `ser::write_preamble`, and `de::skip_value` (depth-aware well-formedness skip).
- `CborSimpleValue`, plus `CborInteger::{argument, Display}`.
- Native impls for `String`, `Cow<str>`/`Cow<[u8]>` serialize, `Vec<T>`, `BTreeSet<T>`, `HashSet<T>`, `Option<T>` (null only), and `Box<T>`; `Vec<u8>` stays an array, byte strings use `Cow<[u8]>`.
- Integer decoders accept every argument width for a value the target holds (RFC 8949 §4.1, §5.5): `19 007f` decodes as `127` in an `i8`.
- Tests: Appendix F.1 kinds 1–3 on both paths plus well-formed counterexamples; depth limit; bignum lengths; argument widths; allocation bounds; indefinite strings; `OwnedValue`; simple values and tags; tuple length.

### Fixed

- UB in the serde serializer: the `SimpleValue`, `RawValue` and dynamic `Value` sentinels reinterpreted the erased `&T` from `serialize_newtype_struct`; safe code could read misaligned/out of bounds.
  - Payloads now travel as values (u8, byte string, tagged value) through a verbatim serializer; forged sentinel names error; those types serialize through non-CBOR formats instead of stack-overflowing.
- Bignums (tags 2/3) panicked unless the payload was 16 bytes and truncated longer ones. Now skips leading zeroes (RFC 8949 §3.4.3), left-pads up to 16 bytes, errors beyond; negative values outside `i128` no longer decode as `0`.
- Unbounded recursion: KBs of nesting overflowed the stack. Both paths enforce the depth limit; the committed fuzz artifact is the regression test.
- Indefinite strings through serde: `String`, `serde_bytes` and `Cow` decoders rejected `5f`/`7f` while `Value` accepted them. Now concatenates chunks (RFC 8949 §3.2.3), each a definite string of the same major type (split multi-byte chars rejected); definite strings stay borrowed.
- Bare simple values failed to decode into `Value` through serde (`f0`, `f3`, `f8 3b` encoded but errored). The visitor now separates the two newtype sentinels by payload shape; `DynamicTaggedValue` is deleted (no construction site); a tagless tagged sentinel errors.
- A serde-decoded `Value::Map` carried `mt: Array`, so re-serialising wrote an array header (`81 a1` for `a1 a1`): silent in release, `debug_assert` panic in debug.
- Four pre-existing gaps found on re-verification: `Value` serde `Serialize` panicked below `i64::MIN` (now i128); `CborFloat` emitted `f9 fe 00` for negative/payload NaN (now always `f9 7e 00`); `i128` encoding used tag 3 for major-type-1 values (`−2^64 … i64::MIN−1`, now `3b ff…`); serde `Value` decode rejected those integers (now i128).
- `Tagged::parse_tag_from_typ` panicked on an unparsable `type_name`; now a `Serde` error, foreign newtypes pass through.
- Unknown struct fields: `deserialize_ignored_any` materialised a full `Value` and dropped it; now `IgnoredAny`: 40 960 → 0 bytes for a 1019-byte document with a 1000-element unknown array.
- Errors name the accepted bytes: a bool reports `IncorrectInitialByteSet { actual: 0xf9, expected: [0xf4, 0xf5] }`; float decoders list every width (`f64`: `[f9, fa, fb]`). New `IncorrectInitialByteSet`; the enum is `#[non_exhaustive]`.
- Clippy under `--all-features --all-targets`: bench slicing/lifetime/constant lints, a stray `&mut`, test module placement, float literals at exact widths; `black_box` closures keep their shape.
- `CborInteger` conversions are `TryFrom` instead of `TryInto`, so `i64::try_from(ci)` and `TryFrom` bounds work; `TryInto` still works through the blanket impl.
- `from_reader` / `to_writer` docs point at `parsio::StdReader` / `StdWriter`; four broken intra-doc links fixed (`RUSTDOCFLAGS="-D warnings" cargo doc` clean).
- Enums decode every well-formed 1-entry map spelling: long-form counts (`b8 01`, `b9 0001`) and indefinite maps (`bf … ff`) were rejected. Extra entries are `WrongLen` / `TooManyElements`; the README enum quirk is clarified.
- Tuple field count is checked against the wire array: a mismatch is `WrongLen` instead of leaving the unread element for the enclosing struct; indefinite arrays check the break (`TooManyElements`); `Vec` is unaffected.
- Not-well-formed input is rejected and no longer emitted:
  - `f8` below 0x20; nested or non-string chunks in indefinite strings; encoder rejects `SimpleValue(20..=31)`, while `Bool` / `Null` / `Undefined` still write `f4` / `f6` / `f7`. New `NotWellFormed`, `ReservedSimpleValue`.
- Heap amplification: an indefinite container preallocated 256 entries (`5f ff`, 10 KiB) and size hints were trusted (`9b ff…ff`, 10 KiB). Containers start empty; lengths and hints are clamped by unread bytes (>= 1 byte per item).
- Signed decoders wrapped out-of-range arguments (`38 ff` → `0` in `i8`; `3b ff…ff` → `0` in `i128`). The target range decides now; `i128` no longer routes major type 1 through `i64`.

### Changed

- **Breaking:** `SeaboredSerError::ReservedSimpleValue(u8)`; `SimpleValue(20..=31)` errors.
- **Breaking:** `Value::from_iter` builds definite-length (`82 01 02`, not `9f 01 02 ff`; profile §4.1 item 2). Indefinite is explicit via `CborSequence::new_indefinite`; decoded containers keep `is_indefinite`.
- **Breaking:** `CborDeserialize::cbor_deserialize_from` takes `&mut DepthAwareReaderGuard<'_, 'data, R>`.
  - Migrate: `T::cbor_deserialize_from(&mut &bytes[..])` → `de::from_bytes::<T>(&bytes)`. Custom readers: `let mut r = DepthAwareReader::from_reader(reader); let mut guard = r.enter()?;`, then pass `&mut guard`.
- **Breaking:** `serde::Deserializer::new` takes `&mut DepthAwareReader` and returns `Result`; `into_inner` is gone. Use the entry points, or `remaining` for trailing bytes.
- **Breaking:** `SeaboredDeError::AllowedDepthOverflow` is gone (never constructed); overflow surfaces as `Parsio(ReadError::AllowedDepthOverflow { depth, limit })`.
- Deeper than the limit errors; 256 levels covers real documents, and `_with_limit` raises it per call.
- Integer decoding reads each argument once: wide arms narrow inside `action()`. Bulk decode 29% cheaper for `Vec<i8>`, 22.5% for `Vec<u8>`, 10% for `Vec<i16>`, single wide `i8` 31% (callgrind); `i8`'s `#[inline(never)]` workaround is gone.
- Seven of `serde_de.rs`'s eight `inline-nontrivial` items are `inline(always)`: mk48 −1.0%, minecraft_savedata −0.8% instructions. `deserialize_newtype_struct` stays plain; forcing it in overflows the stack on legal deep nesting.
- **Breaking:** an integer outside the target range errors instead of wrapping: `18 80` (128) into `i8` was `-128`, `38 ff` (-256) was `0`, `3b ff…ff` (-2^64) was `0` even via i128.
  - Use `i128` / `CborInteger` / `Value` for wide ranges; coerce on top if a protocol wants saturation or clamping.
