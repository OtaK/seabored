use gungraun::prelude::*;
use hex_literal::hex;
use std::hint::black_box;
use std::sync::LazyLock;

/// Array of 1000 items, each written with a 16-bit argument (`0x19`/`0x39` + 2 bytes), which is
/// the wide-argument arm of the narrow-integer decoders.
fn wide_array<const N: usize>(header_byte: u8, arg: [u8; N]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(3 + 1000 * (1 + N));
    bytes.extend_from_slice(&[0x99, 0x03, 0xe8]); // array of 1000
    for _ in 0..1000 {
        bytes.push(header_byte);
        bytes.extend_from_slice(&arg);
    }
    bytes
}

static I8_ARRAY: LazyLock<Vec<u8>> = LazyLock::new(|| wide_array(0x39, [0x00, 0x00])); // -1
static U8_ARRAY: LazyLock<Vec<u8>> = LazyLock::new(|| wide_array(0x19, [0x00, 0xff])); // 255
static I16_ARRAY: LazyLock<Vec<u8>> = LazyLock::new(|| wide_array(0x39, [0x00, 0x00])); // -1

#[library_benchmark]
fn serde_vec_i8_1k_wide() -> Vec<i8> {
    black_box(seabored::serde::from_slice(black_box(&I8_ARRAY[..])).unwrap())
}

#[library_benchmark]
fn serde_vec_u8_1k_wide() -> Vec<u8> {
    black_box(seabored::serde::from_slice(black_box(&U8_ARRAY[..])).unwrap())
}

#[library_benchmark]
fn serde_vec_i16_1k_wide() -> Vec<i16> {
    black_box(seabored::serde::from_slice(black_box(&I16_ARRAY[..])).unwrap())
}

#[library_benchmark]
fn de_i8_short() -> i8 {
    black_box(seabored::de::from_bytes::<i8>(black_box(&hex!("3818"))).unwrap())
}

#[library_benchmark]
fn de_i8_wide() -> i8 {
    black_box(seabored::de::from_bytes::<i8>(black_box(&hex!("390000"))).unwrap())
}

mod datasets;

#[library_benchmark]
fn mk48_serde() -> datasets::mk48::Updates {
    black_box(seabored::serde::from_slice(black_box(datasets::mk48::BYTES)).unwrap())
}

#[library_benchmark]
fn minecraft_serde() -> datasets::minecraft_savedata::BorrowPlayers<'static> {
    black_box(seabored::serde::from_slice(black_box(datasets::minecraft_savedata::BYTES)).unwrap())
}

#[library_benchmark]
fn serde_vec_u64_1k_wide() -> Vec<u64> {
    static U64: std::sync::LazyLock<Vec<u8>> =
        std::sync::LazyLock::new(|| wide_array(0x1b, [0, 0, 0, 0, 0, 0, 0, 0]));
    black_box(seabored::serde::from_slice(black_box(&U64[..])).unwrap())
}

library_benchmark_group!(
    name = h3_ints,
    benchmarks = [
        serde_vec_i8_1k_wide,
        serde_vec_u8_1k_wide,
        serde_vec_i16_1k_wide,
        de_i8_short,
        de_i8_wide,
        mk48_serde,
        minecraft_serde,
        serde_vec_u64_1k_wide,
    ]
);

main!(library_benchmark_groups = [h3_ints]);
