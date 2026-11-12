#![no_main]

use libfuzzer_sys::fuzz_target;
use seabored::{
    DecodeOptions, Value, de,
    reexports::parsio::DepthAwareReader,
    ser::CborSerialize as _,
};

// Every decode path plus the checked skip scanner must reject or accept without panicking.
fuzz_target!(|data: &[u8]| {
    let data = core::hint::black_box(data);

    let _ = de::from_bytes::<Value>(data);
    let _ = seabored::serde::from_slice::<Value>(data);

    for options in [
        DecodeOptions::PREFERRED_PLUS_CHECKED,
        DecodeOptions::DETERMINISTIC_CHECKED,
    ] {
        if let Ok(value) = de::from_bytes_with_options::<Value>(data, options) {
            let _ = value.cbor_serialize_with_profile(options.profile);
        }
        let _ = seabored::serde::from_slice_with_options::<Value>(data, options);
    }

    let mut reader = DepthAwareReader::from_reader(data);
    if let Ok(mut guard) = reader.enter() {
        let _ = de::skip_value_with_options(&mut guard, DecodeOptions::DETERMINISTIC_CHECKED);
    }
});
