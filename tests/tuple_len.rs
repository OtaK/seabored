//! A tuple declares its field count, so the wire array must agree. Otherwise the unread
//! element lands in the *next field* of the enclosing structure and decodes to wrong values.
use seabored::error::SeaboredDeError;
use serde::Deserialize;

fn de<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, SeaboredDeError> {
    seabored::serde::from_slice(bytes)
}

#[test]
fn tuple_length_must_match_the_wire() {
    // Matching lengths: the definite and the indefinite spelling of the same array
    assert_eq!(de::<(u8, u8)>(&[0x82, 0x01, 0x02]).unwrap(), (1, 2));
    assert_eq!(de::<(u8, u8)>(&[0x9f, 0x01, 0x02, 0xff]).unwrap(), (1, 2));

    // Nested, consistent: the tuple is the first element of a two-element array
    assert_eq!(
        de::<((u8, u8), u8)>(&[0x82, 0x82, 0x01, 0x02, 0x03]).unwrap(),
        ((1, 2), 3)
    );

    // One element too many, with an exact count for the definite case
    assert!(matches!(
        de::<(u8, u8)>(&[0x83, 0x01, 0x02, 0x03]),
        Err(SeaboredDeError::WrongLen {
            expected: 2,
            got: 3
        })
    ));
    // and without one for the indefinite case: the run is not counted
    assert!(matches!(
        de::<(u8, u8)>(&[0x9f, 0x01, 0x02, 0x03, 0xff]),
        Err(SeaboredDeError::TooManyElements { expected: 2 })
    ));

    // One element too few, both spellings
    assert!(de::<(u8, u8)>(&[0x81, 0x01]).is_err());
    assert!(de::<(u8, u8)>(&[0x9f, 0x01, 0xff]).is_err());

    // The desync this finding is about: a 3-element array inside the tuple position used to
    // hand the leftover element to the outer array's second field and return Ok(((1, 2), 3))
    assert!(de::<((u8, u8), u8)>(&[0x82, 0x83, 0x01, 0x02, 0x03, 0x04]).is_err());

    // A `Vec` declares no field count, so the same bytes are a perfectly good array
    assert_eq!(
        de::<Vec<u8>>(&[0x83, 0x01, 0x02, 0x03]).unwrap(),
        vec![1, 2, 3]
    );
}
