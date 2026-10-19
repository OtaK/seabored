//! An indefinite string is the concatenation of its chunks (RFC 8949 §3.2.3); serde used to
//! reject them while `Value` accepted them.
//!
//! Byte strings reach `deserialize_bytes` only via `serde_bytes` or the native impls: serde's own
//! `Vec<u8>`/`Cow<[u8]>` are generic sequences, so they expect an array.
use seabored::Value;
use std::borrow::Cow;

fn unhex(hex: &str) -> Vec<u8> {
    assert_eq!(hex.len() % 2, 0, "odd-length hex string {hex}");
    (0..hex.len() / 2)
        .map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

#[test]
fn indefinite_byte_strings_concatenate() {
    // 5f 42 0102 43 030405 ff: chunks [1, 2] and [3, 4, 5]
    let bytes = unhex("5f42010243030405ff");

    assert_eq!(
        seabored::serde::from_slice::<serde_bytes::ByteBuf>(&bytes)
            .unwrap()
            .into_vec(),
        vec![1, 2, 3, 4, 5]
    );
    // The native decoder for the same type concatenates too
    assert_eq!(
        seabored::de::from_bytes::<Cow<[u8]>>(&bytes).unwrap(),
        vec![1, 2, 3, 4, 5]
    );
    assert_eq!(
        seabored::serde::from_slice::<Value>(&bytes).unwrap(),
        Value::Bytes(vec![1, 2, 3, 4, 5].into())
    );

    // Empty string, and a zero-length chunk in front: both are well-formed (§3.2.3), the
    // latter is simply not useful
    assert_eq!(
        seabored::serde::from_slice::<serde_bytes::ByteBuf>(&unhex("5fff"))
            .unwrap()
            .into_vec(),
        Vec::<u8>::new()
    );
    assert_eq!(
        seabored::serde::from_slice::<serde_bytes::ByteBuf>(&unhex("5f404100ff"))
            .unwrap()
            .into_vec(),
        vec![0]
    );

    // A definite-length string is still lent from the input, not copied
    assert!(matches!(
        seabored::de::from_bytes::<Cow<[u8]>>(&unhex("43030405")).unwrap(),
        Cow::Borrowed([3, 4, 5])
    ));
}

#[test]
fn indefinite_text_strings_concatenate() {
    // 7f 62 6869 63 212121 ff: "hi" + "!!!"
    let bytes = unhex("7f62686963212121ff");
    assert_eq!(
        seabored::serde::from_slice::<String>(&bytes).unwrap(),
        "hi!!!"
    );
    assert_eq!(
        seabored::de::from_bytes::<Cow<str>>(&bytes).unwrap(),
        "hi!!!"
    );

    // Empty, and a zero-length chunk in front: 7f 60 62 6162 ff
    assert_eq!(
        seabored::serde::from_slice::<String>(&unhex("7fff")).unwrap(),
        ""
    );
    assert_eq!(
        seabored::serde::from_slice::<String>(&unhex("7f60626162ff")).unwrap(),
        "ab"
    );

    // Only the assembled indefinite-length form has to own its bytes
    assert!(matches!(
        seabored::de::from_bytes::<Cow<str>>(&unhex("626869")).unwrap(),
        Cow::Borrowed("hi")
    ));
    assert!(matches!(
        seabored::de::from_bytes::<Cow<str>>(&unhex("7f626869ff")).unwrap(),
        Cow::Owned(s) if s == "hi"
    ));
}

#[test]
fn indefinite_string_chunks_must_be_well_formed() {
    // A multi-byte character split across chunks: the first chunk is not a text string
    assert!(seabored::serde::from_slice::<String>(&unhex("7f61c361a9ff")).is_err());
    // A text chunk inside a byte string, and a byte chunk inside a text string
    assert!(seabored::serde::from_slice::<serde_bytes::ByteBuf>(&unhex("5f6161ff")).is_err());
    assert!(seabored::serde::from_slice::<String>(&unhex("7f4161ff")).is_err());
    // Nested indefinite-length strings
    assert!(seabored::serde::from_slice::<serde_bytes::ByteBuf>(&unhex("5f5f40ffff")).is_err());
    assert!(seabored::serde::from_slice::<String>(&unhex("7f7f6161ffff")).is_err());
    // The native path agrees on all of the above
    assert!(seabored::de::from_bytes::<Value>(&unhex("5f6161ff")).is_err());
    assert!(seabored::de::from_bytes::<Value>(&unhex("7f61c361a9ff")).is_err());
}
