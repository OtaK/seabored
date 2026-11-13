# seabored-derive

Derive macros for `seabored`'s native CBOR traits.

Enable the root crate's `derive` feature. The generated code uses `::seabored`,
so a renamed dependency needs a `use` alias.

```toml
[dependencies]
seabored = { version = "0.2", features = ["derive"] }
```

```rust,ignore
#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
struct Record {
    id: u32,
    #[cbor(key = "payload", tag = 24)]
    payload: Vec<u8>,
    #[cbor(skip, default)]
    cache: u64,
}
```

This is not the Serde bridge and does not use the Serde data model. A type can
derive both.

## Container attributes

- `#[cbor(tag = N)]`: CBOR tag around the whole item.
- `#[cbor(profile = "rfc" | "pref-plus" | "deterministic")]`: default `"rfc"`.
- `#[cbor(map)]` / `#[cbor(array)]`: named structs only; default is map.
- `#[cbor(sequence)]`: tuple newtype; emits an RFC 8742 sequence instead of one
  CBOR item. `#[cbor(sequence, unordered)]` also rejects deterministic encoding.

## Field attributes

- `key = "text"` or `key = N`: map key; defaults to the field name.
- `tag = N`: tag around the field value.
- `simple_value`: `u8` field encoded as a CBOR simple value.
- `with = module`: encode through `module::cbor_serialize_to_with_profile(value, writer, profile)`; decode through `module::cbor_deserialize_from_with_profile(reader, options)`.
- `default` or `default = expr`: value for a missing map key.
- `skip`: omit on encode, fill from `default` on decode.
- `indefinite`: encode `Vec`/`BTreeSet`/`HashSet` fields as `9f … ff`.

## CBOR sequences

```rust,ignore
use seabored::sequence::CborSeqSerialize as _;

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(sequence)]
struct Values(Vec<u32>);

let bytes = Values(vec![1, 2, 3]).cbor_sequence_serialize()?;
assert_eq!(bytes, [0x01, 0x02, 0x03]);

let values: Values = Values::cbor_sequence_from_bytes(&bytes)?;
```

`#[cbor(sequence)]` generates `sequence::CborSeqSerialize` and
`sequence::CborSeqDeserialize`, not `CborSerialize` / `CborDeserialize`. A
sequence type cannot be used as a field, array element, or sequence item.

The inner type is accepted by its bounds, not by name. It needs exactly one type
parameter and must satisfy:

- `Default + Extend<T>` for decoding;
- `&Inner: IntoIterator<Item = &T>` for encoding;
- `T: CborSerialize` / `T: CborDeserialize` for the items.

That covers `Vec<T>`, `VecDeque<T>`, `LinkedList<T>`, `BTreeSet<T>`,
`HashSet<T>`, `BinaryHeap<T>`, and custom containers with the same bounds.
`Box<[T]>`, arrays, and maps do not fit.

Each item is written back to back with no array header, length, or terminator.
An empty container writes zero bytes. Decoding consumes the whole reader, so it
is top-level only. `BTreeSet` and `HashSet` return
`SeaboredDeError::DuplicateSequenceItem` as soon as `insert` reports a
duplicate. Other containers use `Extend`: their own insertion semantics apply,
including any lossy behavior.

Mark hash-based or otherwise unordered containers:

```rust,ignore
#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(sequence, unordered)]
struct Unique(std::collections::HashSet<u32>);
```

`unordered` rejects `Profile::Deterministic` at encode time. Without it,
deterministic output is only as stable as the container's iteration order.

`tag`, `map`, `array`, field attributes, and `profile = "deterministic"` with
`unordered` are rejected.

## Enums

Externally tagged, like Serde: a unit variant is its bare key; other variants
are one-entry maps `{ key: payload }`. Variant `tag` wraps the payload inside
that entry.

## Profiles

- `rfc`: permissive decode.
- `pref-plus`: checking decode. Rejects non-shortest arguments, headers, tags,
  and declared keys; rejects indefinite containers; validates floats, strings,
  bignums, and skipped/unknown values.
- `deterministic`: `pref-plus` plus canonical map key order, exact declared key
  set, and no default filling.

These are attribute defaults. Runtime profile calls
(`cbor_serialize_with_profile`, `from_bytes_with_options`) override them for that
item.

`#[cbor(indefinite)]` is rejected under `pref-plus` and `deterministic`.

## Supported field types

Native impls exist for integers, floats, `bool`, `String`, `Cow<str>`,
`Cow<[u8]>`, `Vec<T>`, `BTreeSet<T>`, `HashSet<T>`, `Option<T>`, `Box<T>`,
`CborInteger`, `CborFloat`, `CborContainer<T>` (serialize), `CborMap<K, V>`, `CborSimpleValue`,
`RawValue`, and `Value`.

`Vec<u8>` encodes as an array of integers. Use `Cow<[u8]>` for byte strings.
