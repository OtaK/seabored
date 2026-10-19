//! Non-preferred argument widths (RFC 8949 §4.1): a narrow integer is reachable through any
//! wider width, and the value must still fit the target type.

use seabored::de::from_bytes;

#[test]
fn wide_argument_values() {
    // In-range through every width: -1 as i8 via u16/u32/u64 arguments
    assert_eq!(from_bytes::<i8>(&[0x39, 0x00, 0x00]).unwrap(), -1);
    assert_eq!(from_bytes::<i8>(&[0x3a, 0, 0, 0, 0]).unwrap(), -1);
    assert_eq!(
        from_bytes::<i8>(&[0x3b, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap(),
        -1
    );
    assert_eq!(from_bytes::<u8>(&[0x19, 0x00, 0xff]).unwrap(), 255);
    assert_eq!(from_bytes::<u16>(&[0x1a, 0, 0, 0x01, 0x00]).unwrap(), 256);
    assert_eq!(from_bytes::<i16>(&[0x39, 0x00, 0xff]).unwrap(), -256);
}

#[test]
fn wide_argument_overflow_is_rejected() {
    // The value leaves the target type's range
    assert!(from_bytes::<i8>(&[0x39, 0x01, 0x00]).is_err());
    assert!(from_bytes::<i8>(&[0x3a, 0, 0, 0x01, 0x00]).is_err());
    assert!(from_bytes::<u8>(&[0x19, 0x01, 0x00]).is_err());
    assert!(from_bytes::<u16>(&[0x1b, 0, 0, 0, 0, 0, 1, 0, 0]).is_err());
    // -32769 is one below i16::MIN
    assert!(from_bytes::<i16>(&[0x39, 0x80, 0x00]).is_err());
}

#[test]
fn reserved_argument_is_rejected() {
    // ai=31 is reserved on integer major types (RFC 8949 §3.1)
    assert!(from_bytes::<i8>(&[0x1f]).is_err());
    assert!(from_bytes::<u8>(&[0x1f]).is_err());
}
