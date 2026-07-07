use parsio::{Read, Write};

// Lil hack to pass over the serde fence
thread_local! {
    pub(crate) static TAG: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
}

mod serde_de;
mod serde_ser;

pub struct Deserializer<'de, R: Read<'de>> {
    reader: R,
    _marker: std::marker::PhantomData<&'de ()>,
}

impl<'de, R: Read<'de>> Deserializer<'de, R> {
    #[inline]
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            _marker: Default::default(),
        }
    }

    #[inline]
    pub fn into_inner(self) -> R {
        self.reader
    }
}

pub struct Serializer<W: Write> {
    pub(crate) writer: W,
}

impl<W: Write> Serializer<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    pub fn into_inner(self) -> W {
        self.writer
    }
}

#[inline(always)]
/// Read a serde-enabled data structure from a slice
pub fn from_slice<'de, T: serde::Deserialize<'de>>(
    buf: &'de [u8],
) -> Result<T, crate::error::SeaboredDeError<'de>> {
    from_reader(buf)
}

#[inline(always)]
/// Read a serde-enabled data structure from a type that implements our [`io::Read`] trait
// You might want to use the [`io::StdReader`] adapter if you need that
pub fn from_reader<'de, T: serde::Deserialize<'de>, R: Read<'de>>(
    reader: R,
) -> Result<T, crate::error::SeaboredDeError<'de>> {
    let mut deserializer = Deserializer::new(reader);
    serde::Deserialize::deserialize(&mut deserializer)
}

#[inline(always)]
/// Serialize a data structure to a Writer that implements our [`io::Write`] trait
/// You might want to use the [`io::StdWriter`] adapter if you need that
pub fn to_writer<W: Write, T: serde::Serialize>(
    writer: &mut W,
    value: &T,
) -> Result<usize, crate::error::SeaboredSerError> {
    let mut serializer = Serializer::new(writer);
    value.serialize(&mut serializer)
}

#[inline(always)]
/// Serialize a data structure to a Vec
pub fn to_vec<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, crate::error::SeaboredSerError> {
    let mut buf = vec![];
    let written = to_writer(&mut buf, value)?;
    debug_assert_eq!(written, buf.len());
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use crate::{
        Value,
        adapters::{SimpleValue, Tagged},
    };

    struct Repo;
    impl Repo {
        pub const TV: u64 = 99999;
    }

    #[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    struct SubTest<'a> {
        sub: (),
        #[serde(with = "serde_bytes")]
        data: &'a [u8],
    }

    #[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    enum Mune {
        Nothing,
        Whatever { id: u32 },
    }

    const TEST_CONST: u64 = 9999999;
    #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    struct Test<'a> {
        v1: bool,
        v2: String,
        v3: (),
        v4: Option<u64>,
        #[serde(with = "serde_bytes")]
        v5: Vec<u8>,
        // No serde_bytes
        v5_a: Vec<u8>,
        v6: f32,
        v6_f64: f64,
        v7: Option<i64>,
        v7_pos: Option<i64>,
        #[serde(borrow)]
        v8: Tagged<'a, { Repo::TV }, bool>,
        #[serde(borrow)]
        v9: Tagged<'a, 12u64, std::borrow::Cow<'a, serde_bytes::Bytes>>,
        v10: SimpleValue,
        #[serde(borrow)]
        v11: Tagged<'a, { TEST_CONST }, bool>,
        #[serde(borrow)]
        v12: Tagged<'a, { u64::MAX }, ()>,
        #[serde(borrow)]
        v13: Tagged<'a, 69, Option<u64>>, // nice
        v14: Option<()>,
        v15: Tagged<'a, 420, Value<'a>>, // blaze it
        v16: Value<'a>,
        #[serde(with = "serde_bytes")]
        v17: [u8; 32],
        // No serde_bytes
        v17_a: [u8; 32],
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
                v5: vec![1, 2, 3, 4, 5, 6],
                v5_a: vec![1, 2, 3, 4, 5, 6],
                v6: std::f32::consts::PI,
                v6_f64: f64::MAX,
                v7: Some(-100),
                v7_pos: Some(100),
                v8: false.into(),
                v9: Tagged::from(std::borrow::Cow::Owned(vec![7, 8, 9].into())),
                v10: SimpleValue(59),
                v11: true.into(),
                v12: ().into(),
                v13: Some(64).into(),
                v14: Some(()),
                v15: Tagged::from(Value::Bool(false)),
                v16: Value::Tagged(((u32::MAX as u64).into(), Box::new(Value::Bool(true)))),
                v17: [0x00; 32],
                v17_a: [0x00; 32],
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
    fn can_roundtrip() {
        use pretty_assertions::assert_eq;
        let value = Test::default();

        let mut buf = vec![];
        super::to_writer(&mut buf, &value).unwrap();
        let value2 = super::from_slice::<Test>(&buf).unwrap();
        assert_eq!(value, value2);
    }
}
