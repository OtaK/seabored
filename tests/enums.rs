//! Externally tagged enums (serde's convention): unit = bare string, others = 1-entry map.
//! Every well-formed spelling decodes; maps of any other length error instead of desyncing.

use seabored::error::SeaboredDeError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
enum E {
    A(u8),
    B,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct S {
    e: E,
    tail: u8,
}

/// `{ "e": <enum bytes>, "tail": 7 }` — a following field makes a desync visible
fn nested(enum_bytes: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0xa2, 0x61, b'e'];
    bytes.extend_from_slice(enum_bytes);
    bytes.extend_from_slice(&[0x64, b't', b'a', b'i', b'l', 0x07]);
    bytes
}

#[test]
fn every_one_entry_map_header_decodes() {
    for enum_bytes in [
        &[0xa1, 0x61, b'A', 0x01][..],       // 1-entry map, one-byte count
        &[0xb8, 0x01, 0x61, b'A', 0x01][..], // long-form count
        &[0xb9, 0x00, 0x01, 0x61, b'A', 0x01][..], // two-byte count
        &[0xbf, 0x61, b'A', 0x01, 0xff][..], // indefinite-length map
    ] {
        assert_eq!(
            seabored::serde::from_slice::<S>(&nested(enum_bytes)).unwrap(),
            S {
                e: E::A(1),
                tail: 7
            },
            "rejected {enum_bytes:02x?}"
        );
    }
}

#[test]
fn maps_that_are_not_one_entry_are_rejected() {
    // Two entries, definite
    assert!(matches!(
        seabored::serde::from_slice::<S>(&nested(&[0xa2, 0x61, b'A', 0x01, 0x61, b'B', 0x02])),
        Err(SeaboredDeError::WrongLen {
            expected: 1,
            got: 2
        })
    ));
    // Two entries, indefinite
    assert!(matches!(
        seabored::serde::from_slice::<S>(&nested(&[
            0xbf, 0x61, b'A', 0x01, 0x61, b'B', 0x02, 0xff
        ])),
        Err(SeaboredDeError::TooManyElements { expected: 1 })
    ));
    // Empty map
    assert!(matches!(
        seabored::serde::from_slice::<S>(&nested(&[0xa0])),
        Err(SeaboredDeError::WrongLen {
            expected: 1,
            got: 0
        })
    ));
}

#[test]
fn wire_shapes_match_the_documented_convention() {
    // Unit variant: a bare text string
    assert_eq!(seabored::serde::to_vec(&E::B).unwrap(), b"\x61B");
    assert_eq!(seabored::serde::from_slice::<E>(b"\x61B").unwrap(), E::B);
    // Any other variant: a 1-entry map
    assert_eq!(seabored::serde::to_vec(&E::A(1)).unwrap(), b"\xa1\x61A\x01");
}
