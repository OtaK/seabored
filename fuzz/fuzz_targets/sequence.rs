#![no_main]

use libfuzzer_sys::fuzz_target;
use seabored::reexports::parsio::DepthAwareReader;
use seabored::{Value, sequence};

fuzz_target!(|data: &[u8]| {
    let reader = DepthAwareReader::from_reader(core::hint::black_box(data));
    let _ = sequence::Iter::new(reader).collect::<Result<Vec<Value>, _>>();

    let reader = DepthAwareReader::from_reader(core::hint::black_box(data));
    let _ = seabored::serde::sequence::from_reader::<Value, _>(reader)
        .collect::<Result<Vec<Value>, _>>();
});
