# Fuzzing

Targets:

- `value` — general `Value` decode.
- `serde` — general serde decode.
- `checking` — general decode, preferred-plus/deterministic checking decode, strict re-encode,
  and the checking skip scanner.

Run with the seeds from the draft-09 Appendix K vectors:

```text
cargo +nightly fuzz run checking fuzz/seeds/checking
```

`cargo fuzz` writes new corpus entries next to the supplied directory; keep generated corpus
out of the repository (it is ignored).
