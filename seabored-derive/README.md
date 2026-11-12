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

## Field attributes

- `key = "text"` or `key = N`: map key; defaults to the field name.
- `tag = N`: tag around the field value.
- `simple_value`: `u8` field encoded as a CBOR simple value.
- `with = module`: module with `cbor_serialize_to` / `cbor_deserialize_from`.
- `default` or `default = expr`: value for a missing map key.
- `skip`: omit on encode, fill from `default` on decode.
- `indefinite`: encode `Vec`/`BTreeSet`/`HashSet` fields as `9f … ff`.

## Enums

Externally tagged, like Serde: a unit variant is its bare key; other variants
are one-entry maps `{ key: payload }`. Variant `tag` wraps the payload inside
that entry.

## Profiles

- `rfc`: permissive decode.
- `pref-plus`: rejects non-shortest headers, tags, and declared keys the derive
  reads; tolerates indefinite containers; skips unknown keys unchecked.
- `deterministic`: `pref-plus` plus definite containers, canonical map key order,
  exact declared key set, and no default filling.

`#[cbor(indefinite)]` is rejected under `pref-plus` and `deterministic`.

## Supported field types

Native impls exist for integers, floats, `bool`, `String`, `Cow<str>`,
`Cow<[u8]>`, `Vec<T>`, `BTreeSet<T>`, `HashSet<T>`, `Option<T>`, `Box<T>`,
`CborInteger`, `CborFloat`, `CborSequence<T>` (serialize), `CborSimpleValue`,
`RawValue`, and `Value`.

`Vec<u8>` encodes as an array of integers. Use `Cow<[u8]>` for byte strings.
