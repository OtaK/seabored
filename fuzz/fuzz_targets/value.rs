#![no_main]

use libfuzzer_sys::fuzz_target;
use seabored::de::from_bytes;

fuzz_target!(|data: &[u8]| {
    let _ = core::hint::black_box(from_bytes::<seabored::Value>(core::hint::black_box(data)));
});
