//! RFC 8742 CBOR Sequences: concatenated items with no array header.

use std::borrow::Cow;

use seabored::{
    Value,
    error::SeaboredDeError,
    mt::MajorType,
    profile::{DecodeOptions, Profile},
    reexports::parsio::DepthAwareReader,
    sequence,
    types::CborContainer,
};

fn decode<'a>(bytes: &'a [u8]) -> Vec<Value<'a>> {
    let reader = DepthAwareReader::from_reader(bytes);
    sequence::Iter::new(reader)
        .collect::<Result<_, _>>()
        .expect("well-formed sequence")
}

fn encode(items: &[Value<'_>]) -> Vec<u8> {
    let mut out = Vec::new();
    sequence::to_writer(&mut out, items).expect("sequence encodes");
    out
}

fn int(value: u8) -> Value<'static> {
    Value::Integer(value.into())
}

fn array(items: Vec<Value<'_>>) -> Value<'_> {
    Value::Array(CborContainer::from(items))
}

#[test]
fn empty_sequence_has_no_bytes_and_no_items() {
    let empty: [Value; 0] = [];
    assert!(encode(&empty).is_empty());
    assert!(decode(&[]).is_empty());
}

#[test]
fn items_are_concatenated_without_a_header() {
    let mut out = Vec::new();
    let written = sequence::to_writer(&mut out, &[1u8, 2, 3]).expect("encodes");
    assert_eq!(written, 3);
    assert_eq!(out, [0x01, 0x02, 0x03]);

    let items = decode(&out);
    assert_eq!(items, [int(1), int(2), int(3)]);
    assert_eq!(encode(&items), out);
}

#[test]
fn writer_accepts_an_iterator_of_references() {
    let items = [int(1), int(2)];
    let mut out = Vec::new();
    let written = sequence::to_writer(&mut out, items.iter()).expect("encodes");
    assert_eq!(written, 2);
    assert_eq!(out, [0x01, 0x02]);
}

#[test]
fn item_boundaries_are_not_confused_with_trailing_bytes() {
    let items = decode(&[0x82, 0x01, 0x02, 0x03]);
    assert_eq!(items, [array(vec![int(1), int(2)]), int(3)]);
    assert_eq!(encode(&items), [0x82, 0x01, 0x02, 0x03]);
}

#[test]
fn truncated_item_is_an_error_not_eof() {
    let reader = DepthAwareReader::from_reader(&[0x01, 0x18][..]);
    let mut iter = sequence::Iter::new(reader);

    let first: Value = iter.next().unwrap().expect("first item");
    assert_eq!(first, int(1));
    assert!(iter.next().unwrap().is_err());
    assert!(iter.next().is_none());
}

#[test]
fn top_level_break_is_malformed() {
    let reader = DepthAwareReader::from_reader(&[0xff][..]);
    let err = sequence::Iter::<'_, _, Value>::new(reader)
        .next()
        .unwrap()
        .expect_err("BREAK is not an item");
    assert!(matches!(
        err,
        SeaboredDeError::IncorrectMajorType {
            actual: MajorType::SimpleValueOrFloat,
            ..
        }
    ));
}

#[test]
fn depth_resets_for_each_item() {
    // Each [1] is two levels; a cumulative depth would reject the second item.
    let bytes = [0x81, 0x01, 0x81, 0x02];
    let reader = DepthAwareReader::from_reader_with_limit(&bytes[..], 2);
    let items: Vec<Value> = sequence::Iter::new(reader)
        .collect::<Result<_, _>>()
        .expect("each item has its own depth budget");

    assert_eq!(items, [array(vec![int(1)]), array(vec![int(2)])]);
}

#[test]
fn options_apply_per_item() {
    let reader = DepthAwareReader::from_reader(&[0x9f, 0x01, 0xff][..]);
    let mut iter = sequence::Iter::<'_, _, Value>::new(reader)
        .with_options(DecodeOptions::PREFERRED_PLUS_CHECKED);

    assert!(matches!(
        iter.next().unwrap(),
        Err(SeaboredDeError::NonPreferredEncoding { .. })
    ));
    assert!(iter.next().is_none());
}

#[test]
fn decoded_strings_can_borrow_from_the_input() {
    let bytes = [0x61, b'a', 0x61, b'b'];
    let reader = DepthAwareReader::from_reader(&bytes[..]);
    let items = sequence::Iter::new(reader)
        .collect::<Result<Vec<Cow<'_, str>>, _>>()
        .expect("two strings");

    assert_eq!(items, [Cow::Borrowed("a"), Cow::Borrowed("b")]);
}

#[test]
fn strict_encoding_rejects_an_indefinite_item() {
    let mut out = Vec::new();
    let indefinite: CborContainer<Value<'_>> = CborContainer::new_indefinite(MajorType::Array);
    let err = sequence::to_writer_with_profile(&mut out, &[indefinite], Profile::PreferredPlus)
        .expect_err("strict profiles reject indefinite containers");

    assert!(matches!(
        err,
        seabored::error::SeaboredSerError::IndefiniteForbidden { .. }
    ));
}
