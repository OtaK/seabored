//! `CborSequence` satisfies the standard sequence traits; `Deref` alone only bought method calls.

use seabored::{Value, mt::MajorType, ser::CborSerialize, types::CborSequence};

fn small() -> CborSequence<Value<'static>> {
    let mut seq = CborSequence::new_finite(MajorType::Array, 2);
    seq.push(Value::Integer(1u8.into()));
    seq.push(Value::Integer(2u8.into()));
    seq
}

#[test]
fn from_iterator_builds_the_profile_form() {
    let seq: CborSequence<Value> = (1u8..=2).map(|n| Value::Integer(n.into())).collect();
    // A finite array: `82 01 02`, not the forbidden indefinite `9f 01 02 ff`
    assert_eq!(seq.cbor_serialize().unwrap(), [0x82, 0x01, 0x02]);
    assert!(!seq.is_indefinite);
    assert_eq!(seq.mt, MajorType::Array);
}

#[test]
fn extend_keeps_the_existing_form() {
    // Definite stays definite
    let mut finite = small();
    finite.extend([Value::Integer(3u8.into())]);
    assert_eq!(finite.cbor_serialize().unwrap(), [0x83, 0x01, 0x02, 0x03]);
    assert!(!finite.is_indefinite);

    // Indefinite stays indefinite, and a map stays a map
    let mut map = CborSequence::new_indefinite(MajorType::Map);
    map.extend([(Value::Integer(1u8.into()), Value::Integer(2u8.into()))]);
    let bytes = map.cbor_serialize().unwrap();
    assert_eq!(bytes, [0xbf, 0x01, 0x02, 0xff]);
    assert!(map.is_indefinite);
    assert_eq!(map.mt, MajorType::Map);
}

#[test]
fn borrowed_and_owned_iteration_and_generic_bounds() {
    let seq = small();

    // `for x in &seq` used to be a compile error
    let mut borrowed = 0;
    for item in &seq {
        borrowed += 1;
        let _: &Value = item;
    }
    assert_eq!(borrowed, 2);

    // Generic code bounded on the standard traits
    fn count<S: IntoIterator>(s: S) -> usize {
        s.into_iter().count()
    }
    fn push_all<S: Extend<Value<'static>>>(mut s: S) -> S {
        s.extend([Value::Integer(9u8.into())]);
        s
    }
    assert_eq!(count(&seq), 2);
    assert_eq!(count(seq.clone()), 2);
    let extended = push_all(seq.clone());
    assert_eq!(extended.len(), 3);
    let _ = seq.into_iter();
}
