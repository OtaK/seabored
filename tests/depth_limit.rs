//! Recursion-limit (billion laughs) tests for both decoding paths
//!
//! Top-level item is level 1, every container descent enters one more

use seabored::{Value, de, serde as sb};

/// `depth` nested single-element arrays around a scalar: `[[[…1…]]]`
fn nested_arrays(depth: usize) -> Vec<u8> {
    let mut bytes = vec![0x81u8; depth];
    bytes.push(0x01);
    bytes
}

/// Levels `bytes` occupies on the serde path
fn levels_used(bytes: &[u8]) -> Option<u16> {
    (1..32u16).find(|limit| sb::from_slice_with_limit::<Value>(bytes, *limit).is_ok())
}

/// Same, through the `CborDeserialize` path.
fn levels_used_native(bytes: &[u8]) -> Option<u16> {
    (1..32u16).find(|limit| de::from_bytes_with_limit::<Value>(bytes, *limit).is_ok())
}

#[test]
fn depth_limit_fires_on_both_paths() {
    // Well under the default limit: both paths accept it
    for depth in [1usize, 10, 200, 255] {
        let bytes = nested_arrays(depth);
        sb::from_slice::<Value>(&bytes).expect("serde path must accept");
        de::from_bytes::<Value>(&bytes).expect("value path must accept");
    }

    // Past it: both paths must error, not blow the stack
    for depth in [256usize, 257, 1000, 100_000] {
        let bytes = nested_arrays(depth);
        let serde_err = sb::from_slice::<Value>(&bytes).expect_err("serde path must reject");
        let value_err = de::from_bytes::<Value>(&bytes).expect_err("value path must reject");
        for err in [&serde_err, &value_err] {
            assert!(
                err.to_string().contains("depth"),
                "expected a depth error, got: {err}"
            );
        }
    }
}

#[test]
fn every_container_enters_exactly_one_level() {
    // Scalars sit at level 1, each container adds one
    assert_eq!(levels_used(&[0x01]), Some(1), "scalar");
    assert_eq!(levels_used(&[0x81, 0x01]), Some(2), "[1]");
    assert_eq!(levels_used(&[0x81, 0x81, 0x01]), Some(3), "[[1]]");
    assert_eq!(levels_used(&[0xa1, 0x01, 0x01]), Some(2), "map");
    assert_eq!(levels_used(&[0xc2, 0x01]), Some(2), "tag");
    assert_eq!(
        levels_used(&nested_arrays(24)),
        Some(25),
        "24 nested arrays + the scalar"
    );

    // Both paths must count the same levels
    for bytes in [
        vec![0x01],
        vec![0xf3], // simple value (native path only; serde has its own tests)
        vec![0x81, 0x01],
        vec![0x81, 0x81, 0x01],
        vec![0xa1, 0x01, 0x01],
        vec![0xc2, 0x01],
        nested_arrays(24),
    ] {
        assert_eq!(
            levels_used_native(&bytes),
            Some(match bytes[0] {
                0xf3 => 1, // the serde path cannot decode simple values into `Value` yet
                _ => levels_used(&bytes).expect("serde path must decode this"),
            }),
            "paths disagree on {bytes:02x?}"
        );
    }
}

#[test]
fn nesting_below_the_limit_still_round_trips() {
    let bytes = nested_arrays(200);
    let value = sb::from_slice::<Value>(&bytes).unwrap();

    let mut current = &value;
    let mut seen = 0;
    while let Value::Sequence(seq) = current {
        seen += 1;
        current = seq.first().unwrap();
    }
    assert_eq!(seen, 200);
    assert_eq!(current, &Value::Integer(1u8.into()));
}

#[test]
fn trailing_bytes_are_rejected_by_the_strict_entry_points() {
    // Two items in one slice is a sequence, not an item
    sb::from_slice::<Value>(&[0x01, 0x02]).expect("lenient entry point ignores the extra byte");
    let err = sb::from_slice_complete::<Value>(&[0x01, 0x02]).expect_err("strict entry point");
    assert!(matches!(
        err,
        seabored::error::SeaboredDeError::TrailingBytes(1)
    ));
    sb::from_slice_complete::<Value>(&[0x01]).expect("exactly one item is fine");
}

/// The fuzzer artifact that used to abort the process must now be a recoverable error
#[test]
fn committed_fuzz_artifact_is_rejected() {
    let path = "fuzz/artifacts/serde/oom-5e4c2c97785b85c392d946e015e071a65babb61f";
    let Ok(data) = std::fs::read(path) else {
        // Not shipped in the published crate
        return;
    };

    let err = sb::from_slice::<Value>(&data).expect_err("the artifact must be rejected");
    assert!(
        err.to_string().contains("depth"),
        "expected a depth error, got: {err}"
    );
}
