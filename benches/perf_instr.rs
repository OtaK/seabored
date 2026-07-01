use gungraun::prelude::*;
use hex_literal::hex;
use seabored::{de::CborDeserialize, ib::InitialByte};
use std::hint::black_box;

#[library_benchmark]
fn de_ib() -> InitialByte {
    black_box(InitialByte::cbor_deserialize_from(black_box(&mut &hex!("FF")[..])).unwrap())
}

#[library_benchmark]
fn de_u8() -> u8 {
    black_box(u8::cbor_deserialize_from(black_box(&mut &hex!("18FF")[..])).unwrap())
}

#[library_benchmark]
fn de_i8() -> i8 {
    black_box(i8::cbor_deserialize_from(black_box(&mut &hex!("3818")[..])).unwrap())
}

#[library_benchmark]
fn de_i64() -> i64 {
    black_box(i64::cbor_deserialize_from(black_box(&mut &hex!("3bffffffffffffffff")[..])).unwrap())
}

#[library_benchmark]
fn de_bool() -> bool {
    black_box(bool::cbor_deserialize_from(black_box(&mut &hex!("f5")[..])).unwrap())
}

library_benchmark_group!(name = de_framing, benchmarks = [de_ib]);
library_benchmark_group!(name = de_ints, benchmarks = [de_u8, de_i8, de_i64]);
library_benchmark_group!(name = de_sv, benchmarks = [de_bool]);

main!(library_benchmark_groups = [de_framing, de_ints, de_sv]);
