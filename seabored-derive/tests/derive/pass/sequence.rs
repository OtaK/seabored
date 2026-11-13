#![allow(dead_code)]

use std::collections::{BTreeSet, BinaryHeap, HashSet, LinkedList, VecDeque};

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(sequence)]
struct VecSeq<T>(Vec<T>);

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(sequence)]
struct DequeSeq<T>(VecDeque<T>);

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(sequence)]
struct ListSeq<T>(LinkedList<T>);

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(sequence)]
struct SortedSeq<T>(BTreeSet<T>);

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(sequence, unordered)]
struct UniqueSeq<T>(HashSet<T>);

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(sequence, unordered)]
struct HeapSeq<T>(BinaryHeap<T>);

#[derive(Default)]
struct Custom<T>(Vec<T>);

impl<T> Extend<T> for Custom<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl<'a, T> IntoIterator for &'a Custom<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(sequence)]
struct CustomSeq<T>(Custom<T>);

#[derive(seabored::CborSerialize)]
#[cbor(sequence)]
struct BorrowedSeq<'a>(Vec<&'a str>);

#[derive(seabored::CborSerialize)]
#[cbor(sequence)]
struct SerOnly(Vec<u8>);

#[derive(seabored::CborDeserialize)]
#[cbor(sequence)]
struct DeOnly(Vec<u8>);

fn main() {}
