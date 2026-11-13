# Seabored

[![Crates.io](https://img.shields.io/crates/v/seabored.svg)](https://crates.io/crates/seabored)
[![docs.rs](https://docs.rs/seabored/badge.svg)](https://docs.rs/seabored)

## Description

Implementation of the CBOR data format. Serde bridge and native derive (optional).

Compatible with WASM.

Complies to the following RFCs:

- [RFC8949](https://www.rfc-editor.org/rfc/rfc8949.html) - The canonical CBOR RFC
- [RFC8742](https://www.rfc-editor.org/rfc/rfc8742.html) - CBOR Sequences
- [draft-ietf-cbor-serialization-09](https://datatracker.ietf.org/doc/draft-ietf-cbor-serialization/) (28 September 2026) - Defines preferred-plus and deterministic serialization.

## Documentation

Here: [https://docs.rs/seabored](https://docs.rs/seabored)

Not great as of now, will get better in subsequent releases.
If you want examples, please head to the `benches` folder.

### Native derive

With the `derive` feature, the native traits can be derived without going through
Serde. Attributes are under `#[cbor(...)]`.

```rust,ignore
use seabored::de::from_bytes;
use seabored::ser::CborSerialize;

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
struct Record {
    id: u32,
    #[cbor(key = "payload", tag = 24)]
    payload: Vec<u8>,
    #[cbor(skip, default)]
    cache: u64,
}

let bytes = Record { id: 1, payload: vec![1, 2], cache: 0 }
    .cbor_serialize()
    .unwrap();
let decoded: Record = from_bytes(&bytes).unwrap();
```

Profiles are selected per container: `"rfc"` (default, permissive decode),
`"pref-plus"` (shortest headers on decode), and `"deterministic"` (canonical map
key order, definite containers, exact key set). Runtime profile calls
(`cbor_serialize_with_profile`, `from_bytes_with_options`) override the attribute for
that item. See `seabored-derive/README.md`.

### Serialization profiles

Draft-09 conformance is opt-in. The original entry points — `de::from_bytes`,
`CborDeserialize::cbor_deserialize_from`, `CborSerialize::cbor_serialize*`,
`serde::from_slice`, and `serde::to_vec` — remain general, permissive, and wire-preserving for
compatibility, except that tag 2/3 bignums normalize to `Value::Integer`/`Value::BigInteger`
(see Quirks). New protocols should not rely on that default (draft-09 §3.1); choose
preferred-plus unless deterministic map ordering is needed.

```rust,ignore
use seabored::{DecodeOptions, Profile, de, ser};

// Encode.
let bytes = ser::to_vec_preferred_plus(&value)?;
let bytes = ser::to_vec_deterministic(&value)?;
let bytes = ser::to_vec_with_profile(&value, Profile::PreferredPlus)?;
let bytes = seabored::serde::to_vec_with_profile(&value, Profile::PreferredPlus)?;

// Decode and check the requested profile.
let value: T = de::from_bytes_preferred_plus(&bytes)?;
let value: T = de::from_bytes_deterministic_checked(&bytes)?;
let value: T = de::from_bytes_with_profile(&bytes, Profile::PreferredPlus)?;
let value: T = de::from_bytes_with_options(&bytes, DecodeOptions::DETERMINISTIC_CHECKED)?;
let value: T =
    seabored::serde::from_slice_with_profile(&bytes, Profile::PreferredPlus)?;
```

`PreferredPlus` emits shortest arguments, definite containers, canonical NaNs, and normalized
bignums. `Deterministic` adds recursive bytewise sorting of map keys. `Value::Map`, `CborMap`,
serde maps, and derived maps sort under `Deterministic`.

`Profile` selects the target serialization; `DecodeOptions.checking` says whether to reject input
that does not conform. The `*_preferred_plus` and `*_deterministic_checked` aliases check, as do
`from_bytes_with_profile` and `from_slice_with_profile`. Passing a non-checking `DecodeOptions`
accepts the profile's required floor instead. `Profile::General` ignores `checking`.

CDDL can bind a type to these profiles with `.serial "prefp"` or `.serial "dtrm"`
(draft-09 §8).

`from_reader_with_options` / `from_reader_with_profile` cannot buffer a streaming item, so
checking reader decodes return `ProfileUnsupported`; use the slice entry points.

A `#[cbor(with = module)]` field must provide both hooks:

```rust,ignore
pub fn cbor_serialize_to_with_profile<W: Write>(
    value: &T,
    writer: &mut W,
    profile: seabored::Profile,
) -> Result<usize, SeaboredSerError>;

pub fn cbor_deserialize_from_with_profile<'de, R: Read<'de>>(
    reader: &mut DepthAwareReaderGuard<'_, R>,
    options: seabored::DecodeOptions,
) -> Result<T, SeaboredDeError>;
```

### CBOR Sequences (RFC 8742)

A CBOR Sequence is zero or more CBOR items concatenated in one stream. It is not a
data item: there is no array header, count, or terminator. `Vec<T>` still serializes
as a CBOR array.

```rust,ignore
use seabored::{sequence, Value};
use seabored::reexports::parsio::DepthAwareReader;

let reader = DepthAwareReader::from_reader(&bytes[..]);
let items: Vec<Value> = sequence::Iter::new(reader)
    .collect::<Result<_, _>>()?;

let mut out = Vec::new();
sequence::to_writer(&mut out, &items)?;
sequence::to_writer_with_profile(&mut out, &items, seabored::Profile::PreferredPlus)?;
```

The `DepthAwareReader` supplies the depth limit; each item starts at level 1.
`sequence::Iter` decodes one item per `next`, so large sequences need no full
allocation. `to_writer` writes each item back to back; empty input and an empty slice
are valid empty sequences. Use `Value` when items have different types.

A newtype can own a sequence through the derive:

```rust,ignore
use seabored::sequence::CborSeqSerialize as _;

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(sequence)]
struct Values(Vec<u32>);

let bytes = Values(vec![1, 2, 3]).cbor_sequence_serialize()?;
let values: Values = Values::cbor_sequence_from_bytes(&bytes)?;
```

The generated `CborSeqSerialize` / `CborSeqDeserialize` traits are separate from
the normal item traits, so a sequence newtype cannot be nested. The inner type
needs exactly one type parameter plus `Default + Extend<T>` and
`&Inner: IntoIterator<Item = &T>`. This covers `Vec`, `VecDeque`, `LinkedList`,
`BTreeSet`, `HashSet`, `BinaryHeap`, and matching custom containers. Standard
sets reject duplicate decoded items at `insert`. Custom containers use `Extend`
semantics. Mark unordered containers with
`#[cbor(sequence, unordered)]`; deterministic encoding then returns an error.

The Serde bridge returns a lazy `Iter`:

```rust,ignore
let items: Vec<MyType> = seabored::serde::sequence::from_reader(
    DepthAwareReader::from_reader(&bytes[..]),
)
.collect::<Result<_, _>>()?;
let mut out = Vec::new();
seabored::serde::sequence::to_writer(&mut out, &items)?;
```

`serde::sequence::from_reader` returns a lazy `Iter`; collect it or consume one item
at a time. `from_reader_with_options` returns a `Result<Iter, _>` because checking
options are rejected up front. Serde checking options are unsupported for this
streaming bridge; use the native `sequence::Iter` for checking decodes.

## Quirks

- It's *fast*. Like, *seriously fast*. See [BENCHMARKS.md](benches/BENCHMARKS.md)
- Recursion depth is tracked on both decode paths; the default limit is 256 levels.
- As always, Serde de/ser is quirky as hell with CBOR since the Rust types do not map very well to CBOR primitives.
  - `Option::None` is serialized as CBOR Null SimpleValue
  - `()` unit type is serialized as CBOR Undefined SimpleValue
  - Enums follow serde's external tagging: a unit variant is a bare text string
    (`"Variant"`), any other variant a 1-entry map of { variant_name => contents }
  - TODO: list more quirks?
- Encoding follows what a `Value` says under `Profile::General`: every constructor — `FromIterator` included — builds the definite-length form, while a container that came off the wire keeps the form it had, so a general decode → encode round trip reproduces indefinite-length items as they arrived. Strict profiles ignore that wire form: they force definite containers and strings, canonicalize floats and bignums, and sort maps for `Deterministic`.
- Tags 2/3 do not stay generic tags in `Value`: they decode to `Value::Integer` when the magnitude fits CBOR's major types 0/1, and to `Value::BigInteger` (normalized, leading zeroes dropped) otherwise. Non-byte tag 2/3 payloads stay `Value::Tagged` unless checking. Because decoding normalizes, general decode → encode also canonicalizes bignums.
- There's some `unsafe` here and there to juice out more performance where applicable. If you don't like it, then use something else. Fuzzing is in place to minimize risk here.

## Features

- `inline-nontrivial`: Enabled by default, adds the `#[inline]` attributes to most non-trivial functions. This is for performance at the cost of codesize (even though those reports are usually greatly exaggerated). Disable it (by using `default-features = false`) if you absolutely need smol code size.
- `serde`: Enables Serde compatibility
- `derive`: Enables the native `CborSerialize` / `CborDeserialize` derive macros
- `hazmat`: Enables dangerous features (`RawValue`)

## Roadmap

- [x] Billion Laughs protection (recursion depth tracking)
- [ ] Add CBOR RFC compat feature flag, to reduce branches
- [x] Native derive for people who don't care about serde
  - CBOR-specific control: tags, simple values, map keys, profiles, defaults/skips
- [x] ~~Facet compat~~ Not happening.
- [ ] Improve performance (yes, there's still some to get)
- [ ] Docs improvements, examples, etc

## AI Disclaimer

Unlike a lot of things being created currently, this library was written WITHOUT the use of any LLM.

Yes crazy I know, but I'm an actual engineer, not a meat proxy to a bunch of GPUs.

## License

Licensed under either of these:

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
  [https://www.apache.org/licenses/LICENSE-2.0](https://www.apache.org/licenses/LICENSE-2.0))
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  [https://opensource.org/licenses/MIT](https://opensource.org/licenses/MIT))
