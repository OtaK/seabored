//! Errors must describe what the decoder actually accepted: the accepted initial bytes,
//! not a major type that the offending value already belongs to.

use seabored::error::SeaboredDeError;

fn expected_set<T: seabored::de::CborDeserialize<'static> + core::fmt::Debug + 'static>(
    bytes: &'static [u8],
) -> (u8, &'static [u8]) {
    match seabored::de::from_bytes::<T>(bytes) {
        Err(SeaboredDeError::IncorrectInitialByteSet { actual, expected }) => (actual, expected),
        other => panic!("expected an initial-byte set error, got {other:?}"),
    }
}

#[test]
fn bool_names_its_two_simple_values() {
    // A float is a simple value too, so "expected SimpleValueOrFloat" said nothing
    let (actual, expected) = expected_set::<bool>(&[0xf9, 0x00, 0x00]);
    assert_eq!(actual, 0xf9);
    assert_eq!(expected, &[0xf4, 0xf5]);
}

#[test]
fn floats_name_every_width_they_accept() {
    // f64 used to claim `IB_FLOAT_32` while accepting f16/f32/f64
    let (actual, expected) = expected_set::<f64>(&[0xf4]);
    assert_eq!(actual, 0xf4);
    assert_eq!(expected, &[0xf9, 0xfa, 0xfb]);

    let (actual, expected) = expected_set::<f32>(&[0xf4]);
    assert_eq!(actual, 0xf4);
    assert_eq!(expected, &[0xf9, 0xfa]);

    let (actual, expected) = expected_set::<half::f16>(&[0xf4]);
    assert_eq!(actual, 0xf4);
    assert_eq!(expected, &[0xf9]);
}
