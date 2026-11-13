use std::collections::{BTreeSet, BinaryHeap, HashSet, LinkedList, VecDeque};

use seabored::error::{SeaboredDeError, SeaboredSerError};
use seabored::profile::Profile;
use seabored::sequence::{CborSeqDeserialize as _, CborSeqSerialize as _};

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(sequence)]
struct Values(Vec<u8>);

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(sequence)]
struct Deque(VecDeque<u8>);

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(sequence)]
struct List(LinkedList<u8>);

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(sequence)]
struct Sorted(BTreeSet<u8>);

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(sequence, unordered)]
struct Unique(HashSet<u8>);

#[derive(Debug, seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(sequence, unordered)]
struct Heap(BinaryHeap<u8>);

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(sequence, profile = "pref-plus")]
struct Strict(Vec<u8>);

#[derive(Debug, PartialEq, Default)]
struct Bounded<T>(Vec<T>);

impl<T> Extend<T> for Bounded<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl<'a, T> IntoIterator for &'a Bounded<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(sequence)]
struct Custom(Bounded<u8>);

#[test]
fn vec_round_trips_without_a_header() {
    let values = Values(vec![1, 2, 3]);
    let bytes = values.cbor_sequence_serialize().unwrap();
    assert_eq!(bytes, [0x01, 0x02, 0x03]);
    assert_eq!(Values::cbor_sequence_from_bytes(&bytes).unwrap(), values);
}

#[test]
fn empty_sequence_round_trips() {
    let values = Values(vec![]);
    assert!(values.cbor_sequence_serialize().unwrap().is_empty());
    assert_eq!(Values::cbor_sequence_from_bytes(&[]).unwrap(), values);
}

#[test]
fn other_standard_containers_are_supported() {
    let bytes = Deque(VecDeque::from([1, 2]))
        .cbor_sequence_serialize()
        .unwrap();
    assert_eq!(Deque::cbor_sequence_from_bytes(&bytes).unwrap().0, [1, 2]);

    let bytes = List(LinkedList::from([1, 2]))
        .cbor_sequence_serialize()
        .unwrap();
    assert_eq!(
        List::cbor_sequence_from_bytes(&bytes)
            .unwrap()
            .0
            .into_iter()
            .collect::<Vec<_>>(),
        [1, 2]
    );

    let bytes = Custom(Bounded(vec![1, 2]))
        .cbor_sequence_serialize()
        .unwrap();
    assert_eq!(bytes, [0x01, 0x02]);
    assert_eq!(
        Custom::cbor_sequence_from_bytes(&bytes).unwrap(),
        Custom(Bounded(vec![1, 2]))
    );
    assert_eq!(
        Custom::cbor_sequence_from_bytes(&[0x01, 0x01]).unwrap(),
        Custom(Bounded(vec![1, 1]))
    );
}

#[test]
fn sets_reject_duplicate_items() {
    let err = Sorted::cbor_sequence_from_bytes(&[0x01, 0x01]).unwrap_err();
    assert!(matches!(err, SeaboredDeError::DuplicateSequenceItem));

    // Duplicate bails before reading the malformed trailing byte.
    let err = Sorted::cbor_sequence_from_bytes(&[0x01, 0x01, 0x18]).unwrap_err();
    assert!(matches!(err, SeaboredDeError::DuplicateSequenceItem));

    let bytes = Sorted(BTreeSet::from([2, 1]))
        .cbor_sequence_serialize()
        .unwrap();
    assert_eq!(bytes, [0x01, 0x02]);
}

#[test]
fn hash_sets_reject_duplicates_and_deterministic_encoding() {
    let err = Unique::cbor_sequence_from_bytes(&[0x01, 0x01]).unwrap_err();
    assert!(matches!(err, SeaboredDeError::DuplicateSequenceItem));

    let err = Unique(HashSet::from([1]))
        .cbor_sequence_serialize_with_profile(Profile::Deterministic)
        .unwrap_err();
    assert!(matches!(
        err,
        SeaboredSerError::ProfileUnsupported {
            profile: Profile::Deterministic,
            ..
        }
    ));
}

#[test]
fn heap_is_supported_but_unordered() {
    let heap = Heap(BinaryHeap::from([2, 1]));
    let bytes = heap.cbor_sequence_serialize().unwrap();
    let decoded = Heap::cbor_sequence_from_bytes(&bytes).unwrap();
    let mut items: Vec<_> = decoded.0.into_iter().collect();
    items.sort_unstable();
    assert_eq!(items, [1, 2]);

    let err = heap
        .cbor_sequence_serialize_with_profile(Profile::Deterministic)
        .unwrap_err();
    assert!(matches!(
        err,
        SeaboredSerError::ProfileUnsupported {
            profile: Profile::Deterministic,
            ..
        }
    ));
}

#[test]
fn profile_attribute_sets_default_options() {
    assert_eq!(
        Values::cbor_sequence_from_bytes(&[0x18, 0x01]).unwrap(),
        Values(vec![1])
    );

    let err = Strict::cbor_sequence_from_bytes(&[0x18, 0x01]).unwrap_err();
    assert!(matches!(err, SeaboredDeError::NonPreferredEncoding { .. }));
    assert_eq!(Strict(vec![1]).cbor_sequence_serialize().unwrap(), [0x01]);
}
