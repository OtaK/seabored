# Seabored

[![Crates.io](https://img.shields.io/crates/v/seabored.svg)](https://crates.io/crates/seabored)
[![docs.rs](https://docs.rs/seabored/badge.svg)](https://docs.rs/seabored)

## Description

Implementation of the CBOR data format. Compatible with Serde (optional), Facet incoming.

Compatible with WASM.

Complies to the following RFCs:

- [RFC8949](https://www.rfc-editor.org/rfc/rfc8949.html) - The canonical CBOR RFC
- [draft-ietf-cbor-serialization-draft-06](https://datatracker.ietf.org/doc/draft-ietf-cbor-serialization/) - Defines the preferred-plus profile that is being implemented while tolerating things like indefinite length sequences.

## Documentation

Here: [https://docs.rs/seabored](https://docs.rs/seabored)

Not great as of now, will get better in subsequent releases.
If you want examples, please head to the `benches` folder.

## Quirks

- It's *fast*. Like, *seriously fast*. See [BENCHMARKS.md](benches/BENCHMARKS.md)
- As of now, it IS vulnerable to billion laughs attacks. There is no recursion depth tracking, but it will be addressed soon.
- As always, Serde de/ser is quirky as hell with CBOR since the Rust types do not map very well to CBOR primitives.
  - `Option::None` is serialized as CBOR Null SimpleValue
  - `()` unit type is serialized as CBOR Undefined SimpleValue
  - TODO: list more quirks?
- The library *might* serialize things a bit differently than what it deserialized. The reason being is that the library prefers the preferred-plus serialization scheme, and things like indefinite length sequences are forbidden in this scheme. We still tolerate them but *might* avoid reserializing them.
- There's some `unsafe` here and there to juice out more performance where applicable. If you don't like it, then use something else. Fuzzing is in place to minimize risk here.

## Features

- `inline-nontrivial`: Enabled by default, adds the `#[inline]` attributes to most non-trivial functions. This is for performance at the cost of codesize (even though those reports are usually greatly exaggerated). Disable it (by using `default-features = false`) if you absolutely need smol code size.
- `serde`: Enables Serde compatibility
- `facet`: Enables Facet compatibility (does nothing for now: TODO)
- `hazmat`: Enables dangerous features (`RawValue`)

## Roadmap

- [ ] Billion Laughs protection (recursion depth tracking)
- [ ] Add CBOR RFC compat feature flag, to reduce branches
- [ ] Make a homebrew derive for people who don't care about serde/facet/etc
  - This would have more power as you'll be able to be more in control as to how stuff gets de/serialized (like tags, simple values, etc)
- [ ] Facet compat
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
