mod facet_de;
mod facet_ser;

pub use facet_de::*;
pub use facet_ser::*;

#[cfg(test)]
mod tests {
    // use crate::{
    //     Value,
    //     adapters::{SimpleValue, Tagged},
    // };

    // struct Repo;
    // impl Repo {
    //     pub const TV: u64 = 99999;
    // }

    use std::borrow::Cow;

    #[derive(Debug, PartialEq, Eq, facet::Facet)]
    struct SubTest<'a> {
        sub: (),
        data: &'a [u8],
    }

    #[derive(Debug, PartialEq, Eq, facet::Facet)]
    #[repr(u8)]
    enum Mune {
        Nothing,
        Whatever { id: u32 },
    }

    // const TEST_CONST: u64 = 9999999;
    #[derive(Debug, PartialEq, facet::Facet)]
    struct Test<'a> {
        v1: bool,
        v2: String,
        v3: (),
        v4: Option<u64>,
        // v5: Vec<u8>,
        // // No serde_bytes
        // v5_a: Vec<u8>,
        v6: f32,
        v6_f64: f64,
        v7: Option<i64>,
        v7_pos: Option<i64>,
        // v8: Tagged<'a, { Repo::TV }, bool>,
        // v9: Tagged<'a, 12u64, std::borrow::Cow<'a, [u8]>>,
        // v10: SimpleValue,
        // v11: Tagged<'a, { TEST_CONST }, bool>,
        // v12: Tagged<'a, { u64::MAX }, ()>,
        // v13: Tagged<'a, 69, Option<u64>>, // nice
        v14: Option<()>,
        // v15: Tagged<'a, 420, Value<'a>>, // blaze it
        // v16: Value<'a>,
        v17: [u8; 32],
        v17_a: Cow<'a, [u8]>,
        data: SubTest<'a>,
        mune1: Mune,
        mune2: Mune,
        u8max: u8,
        u16max: u16,
        u32max: u32,
        u64max: u64,
        biguint: u128,
        i8min: i8,
        i16min: i16,
        i32min: i32,
        i64min: i64,
        bigsint: i128,
    }

    impl Default for Test<'_> {
        fn default() -> Self {
            Test {
                v1: false,
                v2: "Here is a test!".into(),
                v3: (),
                v4: None,
                // v5: vec![1, 2, 3, 4, 5, 6],
                // v5_a: vec![1, 2, 3, 4, 5, 6],
                v6: std::f32::consts::PI,
                v6_f64: f64::MAX,
                v7: Some(-100),
                v7_pos: Some(100),
                // v8: false.into(),
                // v9: Tagged::from(std::borrow::Cow::Owned(vec![7, 8, 9].into())),
                // v10: SimpleValue(59),
                // v11: true.into(),
                // v12: ().into(),
                // v13: Some(64).into(),
                v14: Some(()),
                // v15: Tagged::from(Value::Bool(false)),
                // v16: Value::Tagged(((u32::MAX as u64).into(), Box::new(Value::Bool(true)))),
                v17: [0x00; 32],
                v17_a: vec![0x00; 32].into(),
                data: SubTest {
                    sub: (),
                    data: &[1, 43, 35, 64],
                },
                mune1: Mune::Nothing,
                mune2: Mune::Whatever { id: 100 },
                u8max: u8::MAX,
                u16max: u16::MAX,
                u32max: u32::MAX,
                u64max: u64::MAX,
                biguint: u128::MAX,
                i8min: i8::MIN,
                i16min: i16::MIN,
                i32min: i32::MIN,
                i64min: i64::MIN,
                bigsint: i128::MIN,
            }
        }
    }

    #[wasm_bindgen_test::wasm_bindgen_test(unsupported = test)]
    #[ignore] // Facet is buggy or what
    // called `Result::unwrap()` on an `Err` value: FacetError(FacetReflectError(ReflectError { .. kind: ReflectErrorKind(Wrong shape: expected **[u8], but got [u8]**) })) <- wtf???
    fn can_roundtrip() {
        use pretty_assertions::assert_eq;
        let value = Test::default();

        let mut buf: Vec<u8> = vec![];
        super::to_writer(&mut buf, &value).unwrap();
        dbg!(&buf);
        let value2 = super::from_slice::<Test>(&buf).unwrap();
        assert_eq!(value, value2);
    }
}
