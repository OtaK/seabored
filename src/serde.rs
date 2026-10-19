use crate::error::SeaboredDeError;
use parsio::{DepthAwareReader, DepthAwareReaderGuard, Read, Write};

// Lil hack to pass over the serde fence
thread_local! {
    pub(crate) static TAG: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
    /// Tag of the value being serialized, handed over by [`serialize_tagged`]
    pub(crate) static SER_TAG: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
}

/// Serializes `value` as a CBOR simple value, handing the `u8` over as the sentinel payload
#[cfg_attr(feature = "inline-nontrivial", inline)]
pub(crate) fn serialize_simple_value<S: ::serde_core::Serializer>(
    serializer: S,
    value: u8,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_newtype_struct(SimpleValue::TYP_NAME, &value)
}

/// Serializes `value` as CBOR tag `tag`, handing the tag over through [`SER_TAG`]
#[cfg_attr(feature = "inline-nontrivial", inline)]
pub(crate) fn serialize_tagged<S, T>(serializer: S, tag: u64, value: &T) -> Result<S::Ok, S::Error>
where
    S: ::serde_core::Serializer,
    T: ?Sized + ::serde_core::Serialize,
{
    SER_TAG.set(Some(tag));
    let result = serializer.serialize_newtype_struct(DYN_TAGGED_TYP_NAME, value);
    // A foreign serializer may have ignored the sentinel name
    SER_TAG.set(None);
    result
}

mod serde_de;
mod serde_ser;

/// CBOR deserializer over a depth-tracking reader: one guard per level, containers hand their
/// entered guard to a child.
pub struct Deserializer<'de, 'a, R: Read<'de>> {
    reader: DepthAwareReaderGuard<'a, 'de, R>,
}

impl<'de, 'a, R: Read<'de>> Deserializer<'de, 'a, R> {
    /// Wraps `reader`, entering level 1; errors on depth overflow.
    #[inline]
    pub fn new(reader: &'a mut DepthAwareReader<'de, R>) -> Result<Self, SeaboredDeError> {
        Ok(Self {
            reader: reader.enter()?,
        })
    }

    /// Wraps an already-entered guard. Used to descend one level per container.
    #[inline]
    pub(crate) fn from_guard(reader: DepthAwareReaderGuard<'a, 'de, R>) -> Self {
        Self { reader }
    }

    /// Current nesting depth (1 for the top-level item).
    #[inline]
    pub fn depth(&self) -> u16 {
        self.reader.depth()
    }

    /// Maximum nesting depth this deserializer will accept.
    #[inline]
    pub fn limit(&self) -> u16 {
        self.reader.limit()
    }

    /// How many bytes are left in the input, if the underlying reader can tell without I/O.
    #[inline]
    pub fn remaining(&self) -> Option<usize> {
        self.reader.remaining()
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
pub fn from_slice<'de, T: ::serde_core::Deserialize<'de>>(
    buf: &'de [u8],
) -> Result<T, crate::error::SeaboredDeError> {
    from_reader(buf)
}

#[inline]
/// Read a serde-enabled data structure from a slice, allowing at most `limit` levels of nesting.
pub fn from_slice_with_limit<'de, T: ::serde_core::Deserialize<'de>>(
    buf: &'de [u8],
    limit: u16,
) -> Result<T, crate::error::SeaboredDeError> {
    from_reader_with_limit(buf, limit)
}

#[inline(always)]
/// Read a serde-enabled data structure from a slice, erroring out when bytes are left over
pub fn from_slice_complete<'de, T: ::serde_core::Deserialize<'de>>(
    buf: &'de [u8],
) -> Result<T, crate::error::SeaboredDeError> {
    from_reader_complete(buf)
}

#[inline(always)]
/// Read a serde-enabled data structure from a type that implements parsio's [`Read`] trait.
///
/// Wrap a [`std::io::Read`] source in [`parsio::StdReader`] to use it here.
///
/// Nesting is limited to [`parsio::DepthAwareReader::DEFAULT_LIMIT`] levels
pub fn from_reader<'de, T: ::serde_core::Deserialize<'de>, R: Read<'de>>(
    reader: R,
) -> Result<T, crate::error::SeaboredDeError> {
    from_reader_with_limit(reader, DepthAwareReader::<'de, R>::DEFAULT_LIMIT)
}

#[inline]
/// Read a serde-enabled data structure, allowing at most `limit` levels of nesting.
pub fn from_reader_with_limit<'de, T: ::serde_core::Deserialize<'de>, R: Read<'de>>(
    reader: R,
    limit: u16,
) -> Result<T, crate::error::SeaboredDeError> {
    let mut reader = DepthAwareReader::from_reader_with_limit(reader, limit);
    let mut deserializer = Deserializer::new(&mut reader)?;
    ::serde_core::Deserialize::deserialize(&mut deserializer)
}

#[inline]
/// Strict counterpart of [`from_reader`]: errors out when the input holds more than one item.
pub fn from_reader_complete<'de, T: ::serde_core::Deserialize<'de>, R: Read<'de>>(
    reader: R,
) -> Result<T, crate::error::SeaboredDeError> {
    let mut reader = DepthAwareReader::from_reader(reader);
    let mut deserializer = Deserializer::new(&mut reader)?;
    let value = ::serde_core::Deserialize::deserialize(&mut deserializer)?;

    match deserializer.remaining() {
        Some(0) | None => Ok(value),
        Some(remaining) => Err(crate::error::SeaboredDeError::TrailingBytes(remaining)),
    }
}

#[inline(always)]
/// Serialize a data structure to a writer that implements parsio's [`Write`] trait.
///
/// Wrap a [`std::io::Write`] sink in [`parsio::StdWriter`] to use it here.
pub fn to_writer<W: Write, T: ::serde_core::Serialize>(
    writer: &mut W,
    value: &T,
) -> Result<usize, crate::error::SeaboredSerError> {
    let mut serializer = Serializer::new(writer);
    value.serialize(&mut serializer)
}

#[inline(always)]
/// Serialize a data structure to a Vec
pub fn to_vec<T: ::serde_core::Serialize>(
    value: &T,
) -> Result<Vec<u8>, crate::error::SeaboredSerError> {
    let mut buf = vec![];
    let written = to_writer(&mut buf, value)?;
    debug_assert_eq!(written, buf.len());
    Ok(buf)
}

/// Tagged CBOR value: the tag is emitted on serialize and expected on deserialize.
///
/// ## Example (serde)
/// ```rust,ignore
/// const MY_TAG: u64 = 123456789;
///
/// #[derive(serde::Serialize, serde::Deserialize)]
/// struct MyStruct<'a> {
///     thing: seabored::serde::Tagged<'a, { MY_TAG }, &'a str>,
/// }
/// ```
pub struct Tagged<'a, const TAG: u64, V: 'a> {
    inner: V,
    _marker: std::marker::PhantomData<&'a ()>,
}

pub(crate) const DYN_TAGGED_TYP_NAME: &str = "seabored::serde::DynamicTaggedValue";

/// What [`parse_tag_from_typ`] found in a `serialize_newtype_struct` / `deserialize_newtype_struct`
/// name.
pub(crate) enum TypeNameTag {
    /// Not one of our `Tagged<…>` names: a foreign newtype struct.
    NotOurs,
    /// Ours, but the const generic did not parse. `type_name` has no stability guarantee:
    /// caller-visible error, never a panic.
    Unparsable,
    Tag(u64),
}

/// Parses the tag value from the `seabored::serde::Tagged<'life, TAG, V>` form of a type name.
#[inline(always)]
pub(crate) fn parse_tag_from_typ(typ: &str) -> TypeNameTag {
    const TAGGED_VALUE_TYP_ROOT_NAME: &str = "seabored::serde::Tagged";
    // Split at the generics boundary to get (`seabored::serde::Tagged`, `'life, TAG, V>`)
    let Some((tname, targs)) = typ.split_once('<') else {
        return TypeNameTag::NotOurs;
    };
    if tname != TAGGED_VALUE_TYP_ROOT_NAME {
        return TypeNameTag::NotOurs;
    }

    // Skip the lifetime, then read the const generic; anything else is an unknown rendering
    let Some((_, targs)) = targs.split_once(',') else {
        return TypeNameTag::Unparsable;
    };
    let Some((tag_str, _)) = targs.split_once(',') else {
        return TypeNameTag::Unparsable;
    };
    let tag_str = tag_str.trim();
    if tag_str == "u64::MAX" {
        return TypeNameTag::Tag(u64::MAX);
    }
    match tag_str.parse() {
        Ok(tag) => TypeNameTag::Tag(tag),
        Err(_) => TypeNameTag::Unparsable,
    }
}

impl<'a, const TAG: u64, V: ::serde_core::Serialize + 'a> ::serde_core::Serialize
    for Tagged<'a, TAG, V>
{
    #[inline(always)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde_core::Serializer,
    {
        serializer.serialize_newtype_struct(std::any::type_name::<Self>(), &self.inner)
    }
}

impl<'a, 'de: 'a, const TAG: u64, V: ::serde_core::Deserialize<'de> + 'a>
    ::serde_core::Deserialize<'de> for Tagged<'a, TAG, V>
{
    #[inline(always)]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde_core::Deserializer<'de>,
    {
        struct TaggedValueVisitor<'a, 'de: 'a, const TAG: u64, V: ::serde_core::Deserialize<'de>>(
            std::marker::PhantomData<(&'a V, &'de ())>,
        );

        impl<'a, 'de: 'a, const TAG: u64, V: ::serde_core::Deserialize<'de>>
            ::serde_core::de::Visitor<'de> for TaggedValueVisitor<'a, 'de, TAG, V>
        {
            type Value = Tagged<'a, TAG, V>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "A CBOR Tagged Value")
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: ::serde_core::Deserializer<'de>,
            {
                let inner = V::deserialize(deserializer)?;
                Ok(inner.into())
            }
        }

        deserializer.deserialize_newtype_struct(
            std::any::type_name::<Self>(),
            TaggedValueVisitor::<'a, 'de, TAG, V>(Default::default()),
        )
    }
}

impl<'a, const TAG: u64, V: 'a> From<V> for Tagged<'a, TAG, V> {
    #[inline(always)]
    fn from(inner: V) -> Self {
        Self {
            inner,
            _marker: Default::default(),
        }
    }
}

impl<'a, const TAG: u64, V: 'a> std::fmt::Debug for Tagged<'a, TAG, V>
where
    V: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tagged")
            .field("TAG", &TAG)
            .field("inner", &self.inner)
            .finish()
    }
}

impl<'a, const TAG: u64, V: 'a> PartialEq for Tagged<'a, TAG, V>
where
    V: PartialEq,
{
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<'a, const TAG: u64, V: 'a> Tagged<'a, TAG, V> {
    #[inline(always)]
    pub fn into_inner(self) -> V {
        self.inner
    }
}

/// Like [`Tagged`], a wrapper for simple values, which need special handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct SimpleValue(pub u8);

impl SimpleValue {
    pub(crate) const TYP_NAME: &'static str = "seabored::serde::SimpleValue";
}

impl ::serde_core::Serialize for SimpleValue {
    #[inline(always)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde_core::Serializer,
    {
        serialize_simple_value(serializer, self.0)
    }
}

impl<'de> ::serde_core::Deserialize<'de> for SimpleValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde_core::Deserializer<'de>,
    {
        struct SimpleValueVisitor;
        impl<'de> ::serde_core::de::Visitor<'de> for SimpleValueVisitor {
            type Value = SimpleValue;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "A CBOR SimpleValue")
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: ::serde_core::Deserializer<'de>,
            {
                use ::serde_core::Deserialize as _;
                Ok(SimpleValue(u8::deserialize(deserializer)?))
            }
        }

        deserializer.deserialize_newtype_struct(Self::TYP_NAME, SimpleValueVisitor)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Value,
        serde::{SimpleValue, Tagged, TypeNameTag, parse_tag_from_typ},
    };

    #[test]
    fn type_names_are_parsed_without_panicking() {
        assert!(matches!(
            parse_tag_from_typ("seabored::serde::Tagged<'_, 42, u8>"),
            TypeNameTag::Tag(42)
        ));
        assert!(matches!(
            parse_tag_from_typ("seabored::serde::Tagged<'_, u64::MAX, u8>"),
            TypeNameTag::Tag(u64::MAX)
        ));
        // Foreign newtype names are not ours
        assert!(matches!(
            parse_tag_from_typ("other::Tagged<'_, 42, u8>"),
            TypeNameTag::NotOurs
        ));
        assert!(matches!(parse_tag_from_typ("u8"), TypeNameTag::NotOurs));
        // Ours, but not a rendering this code understands: an error for the caller, never a panic
        for malformed in [
            "seabored::serde::Tagged<'a>",
            "seabored::serde::Tagged<'a, nope, u8>",
            "seabored::serde::Tagged<'a, 42>",
            "seabored::serde::Tagged<",
        ] {
            assert!(
                matches!(parse_tag_from_typ(malformed), TypeNameTag::Unparsable),
                "{malformed}"
            );
        }
    }

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

    /// A sentinel name does not license reinterpreting the erased `&T`: forging one errors.
    #[test]
    fn sentinel_names_are_not_a_licence_to_reinterpret() {
        use crate::error::SeaboredSerError;

        #[derive(Debug)]
        struct Impostor {
            name: &'static str,
            /// Laid out like no sentinel: the old casts read these bytes as one
            payload: [u8; 24],
        }

        impl ::serde_core::Serialize for Impostor {
            fn serialize<S: ::serde_core::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                debug_assert_eq!(self.payload.len(), 24);
                s.serialize_newtype_struct(self.name, &self.payload)
            }
        }

        let mut names = vec!["seabored::serde::SimpleValue", super::DYN_TAGGED_TYP_NAME];
        #[cfg(feature = "hazmat")]
        names.push(crate::types::RawValue::TYP_NAME);

        for name in names {
            let err = crate::serde::to_vec(&Impostor {
                name,
                payload: [0x11; 24],
            })
            .expect_err("an impostor must not reach the payload");

            assert!(
                matches!(err, SeaboredSerError::Serde(_)),
                "{name} returned {err:?}"
            );
        }

        // Legit paths still work afterwards
        pretty_assertions::assert_eq!(crate::serde::to_vec(&SimpleValue(19)).unwrap(), vec![0xf3]);
        pretty_assertions::assert_eq!(
            crate::serde::to_vec(&crate::Value::Tagged((
                2u64.into(),
                Box::new(crate::Value::Bool(true))
            )))
            .unwrap(),
            vec![0xc2, 0xf5]
        );
        #[cfg(feature = "hazmat")]
        pretty_assertions::assert_eq!(
            crate::serde::to_vec(&crate::types::RawValue::from_slice_unchecked(&[
                0x82, 0x01, 0x02
            ]))
            .unwrap(),
            vec![0x82, 0x01, 0x02]
        );
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
