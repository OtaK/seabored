//! The integer conversions are `TryFrom`-first: `TryInto` comes from the standard blanket impl,
//! and generic code bounded on either trait can consume a `CborInteger`.

use seabored::types::CborInteger;

fn via_try_from<T: TryFrom<CborInteger>>(value: CborInteger) -> Result<T, T::Error> {
    T::try_from(value)
}

fn via_try_into<T>(value: CborInteger) -> Result<T, <CborInteger as TryInto<T>>::Error>
where
    CborInteger: TryInto<T>,
{
    value.try_into()
}

#[test]
fn try_from_is_the_source_of_truth() {
    // `i64::try_from(ci)` used to fail to compile: only `TryInto` was implemented
    assert_eq!(i64::try_from(CborInteger::from(-5)).unwrap(), -5);
    assert_eq!(i8::try_from(&CborInteger::from(-128)).unwrap(), -128);
    assert_eq!(usize::try_from(CborInteger::from(7u8)).unwrap(), 7);

    // Out of range still errors
    assert!(u8::try_from(CborInteger::from(300u16)).is_err());
    assert!(i8::try_from(&CborInteger::from(128)).is_err());

    // Both traits work in generic code now
    assert_eq!(via_try_from::<i32>(CborInteger::from(-2)).unwrap(), -2);
    assert_eq!(via_try_into::<i32>(CborInteger::from(-2)).unwrap(), -2);
}
