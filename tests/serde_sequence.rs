#![cfg(feature = "serde")]

//! Serde bridge for RFC 8742 CBOR Sequences.

use seabored::{
    error::SeaboredDeError,
    profile::{DecodeOptions, Profile},
    reexports::parsio::DepthAwareReader,
    serde::sequence,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Point {
    x: u8,
    y: u8,
}

#[test]
fn scalars_are_written_back_to_back() {
    let mut bytes = Vec::new();
    let written = sequence::to_writer(&mut bytes, &[1u8, 2, 3]).expect("encodes");
    assert_eq!(written, 3);
    assert_eq!(bytes, [0x01, 0x02, 0x03]);

    let reader = DepthAwareReader::from_reader(&bytes[..]);
    let items: Vec<u8> = sequence::from_reader(reader)
        .collect::<Result<_, _>>()
        .expect("decodes");
    assert_eq!(items, [1, 2, 3]);
}

#[test]
fn writer_accepts_an_iterator_of_references() {
    let points = [Point { x: 1, y: 2 }, Point { x: 3, y: 4 }];
    let mut bytes = Vec::new();
    let written = sequence::to_writer(&mut bytes, points.iter()).expect("encodes");
    assert_eq!(written, bytes.len());
    assert_eq!(bytes[0], 0xa2);
}

#[test]
fn structs_are_independent_items() {
    let points = [Point { x: 1, y: 2 }, Point { x: 3, y: 4 }];
    let mut bytes = Vec::new();
    sequence::to_writer(&mut bytes, &points).expect("encodes");

    // Each point is a map (`a2`), not one array containing both (`82`).
    assert_eq!(bytes[0], 0xa2);

    let reader = DepthAwareReader::from_reader(&bytes[..]);
    let decoded: Vec<Point> = sequence::from_reader(reader)
        .collect::<Result<_, _>>()
        .expect("decodes");
    assert_eq!(decoded, points);
}

#[test]
fn empty_sequence_round_trips() {
    let empty: [Point; 0] = [];
    let mut bytes = Vec::new();
    assert_eq!(sequence::to_writer(&mut bytes, &empty).unwrap(), 0);
    assert!(bytes.is_empty());

    let reader = DepthAwareReader::from_reader(&bytes[..]);
    let decoded: Vec<Point> = sequence::from_reader(reader)
        .collect::<Result<_, _>>()
        .expect("empty sequence");
    assert!(decoded.is_empty());
}

#[test]
fn reader_is_lazy() {
    let reader = DepthAwareReader::from_reader(&[0x01, 0x18][..]);
    let mut iter = sequence::from_reader::<u8, _>(reader);

    assert_eq!(iter.next().unwrap().unwrap(), 1);
    assert!(iter.next().unwrap().is_err());
    assert!(iter.next().is_none());
}

#[test]
fn truncated_item_is_an_error() {
    let reader = DepthAwareReader::from_reader(&[0x01, 0x18][..]);
    let error = sequence::from_reader::<u8, _>(reader)
        .collect::<Result<Vec<_>, _>>()
        .expect_err("truncated tail");
    assert!(matches!(
        error,
        SeaboredDeError::Parsio(parsio::ReadError::IoError(
            std::io::ErrorKind::UnexpectedEof
        ))
    ));
}

#[test]
fn depth_resets_for_each_item() {
    let bytes = [0x81, 0x01, 0x81, 0x02];
    let reader = DepthAwareReader::from_reader_with_limit(&bytes[..], 2);
    let items: Vec<Vec<u8>> = sequence::from_reader(reader)
        .collect::<Result<_, _>>()
        .expect("two arrays");
    assert_eq!(items, [vec![1], vec![2]]);
}

#[test]
fn borrowed_strings_decode_from_a_slice() {
    let bytes = [0x61, b'a', 0x61, b'b'];
    let reader = DepthAwareReader::from_reader(&bytes[..]);
    let items: Vec<&str> = sequence::from_reader(reader)
        .collect::<Result<_, _>>()
        .expect("two strings");
    assert_eq!(items, ["a", "b"]);
}

#[test]
fn non_checking_options_are_applied() {
    let bytes = [0x9f, 0x01, 0xff];
    let reader = DepthAwareReader::from_reader(&bytes[..]);
    let items: Vec<Vec<u8>> =
        sequence::from_reader_with_options(reader, DecodeOptions::PREFERRED_PLUS)
            .expect("profile floor accepts indefinite arrays")
            .collect::<Result<_, _>>()
            .expect("decodes");
    assert_eq!(items, [vec![1]]);
}

#[test]
fn checking_options_are_unsupported() {
    let reader = DepthAwareReader::from_reader(&[0x01][..]);
    let error =
        sequence::from_reader_with_options::<u8, _>(reader, DecodeOptions::PREFERRED_PLUS_CHECKED)
            .err()
            .expect("streaming checking is unsupported");

    assert!(matches!(error, SeaboredDeError::ProfileUnsupported { .. }));

    let reader = DepthAwareReader::from_reader(&[0x01][..]);
    let mut iter = sequence::Iter::<'_, _, u8>::new(reader)
        .with_options(DecodeOptions::PREFERRED_PLUS_CHECKED);
    assert!(matches!(
        iter.next().unwrap(),
        Err(SeaboredDeError::ProfileUnsupported { .. })
    ));
    assert!(iter.next().is_none());
}

#[test]
fn strict_profiles_apply_to_each_item() {
    let mut bytes = Vec::new();
    let written = sequence::to_writer_with_profile(&mut bytes, &[1u8, 2], Profile::PreferredPlus)
        .expect("encodes");
    assert_eq!(written, 2);
    assert_eq!(bytes, [0x01, 0x02]);
}
