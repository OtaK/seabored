# Seabored

[![Crates.io](https://img.shields.io/crates/v/seabored.svg)](https://crates.io/crates/seabored)
[![docs.rs](https://docs.rs/seabored/badge.svg)](https://docs.rs/seabored)

## Description

Implementation of the CBOR data format. Serde bridge and native derive (optional).

Compatible with WASM.

Complies to the following RFCs:

- [RFC8949](https://www.rfc-editor.org/rfc/rfc8949.html) - The canonical CBOR RFC
- [draft-ietf-cbor-serialization-draft-06](https://datatracker.ietf.org/doc/draft-ietf-cbor-serialization/) - Defines the preferred-plus profile that is being implemented while tolerating things like indefinite length sequences.

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
key order, definite containers, exact key set). See `seabored-derive/README.md`.

## Quirks

- It's *fast*. Like, *seriously fast*. See [BENCHMARKS.md](benches/BENCHMARKS.md)
- Recursion depth is tracked on both decode paths; the default limit is 256 levels.
- As always, Serde de/ser is quirky as hell with CBOR since the Rust types do not map very well to CBOR primitives.
  - `Option::None` is serialized as CBOR Null SimpleValue
  - `()` unit type is serialized as CBOR Undefined SimpleValue
  - Enums follow serde's external tagging: a unit variant is a bare text string
    (`"Variant"`), any other variant a 1-entry map of { variant_name => contents }
  - TODO: list more quirks?
- Encoding follows what a `Value` says. Every constructor — `FromIterator` included — builds the definite-length form the preferred-plus scheme requires, while a container that came off the wire keeps the form it had, so a decode → encode round trip reproduces indefinite-length items as they arrived. To re-encode a decoded container in the preferred form, clear `CborSequence::is_indefinite`; the items themselves are untouched.
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
