use crate::types::CborIntegerValue;

#[cfg(feature = "serde")]
pub mod serde;

#[cfg(feature = "facet")]
pub mod facet;

/// Wrapper for Tagged CBOR values
/// This makes sure the tag is used when deserializing and emitted when serializing
///
/// ## Example (serde)
/// ```rust,ignore
/// const MY_TAG: u64 = 123456789;
///
/// #[derive(serde::Serialize, serde::Deserialize)]
/// struct MyStruct<'a> {
///     thing: seabored::adapters::Tagged<'a, { MY_TAG }, &'a str>,
/// }
/// ```
pub struct Tagged<'a, const TAG: u64, V: 'a> {
    inner: V,
    _marker: std::marker::PhantomData<&'a ()>,
}

#[derive(Debug)]
pub(crate) struct DynamicTaggedValue<'a> {
    pub(crate) tag: CborIntegerValue,
    pub(crate) value: std::borrow::Cow<'a, crate::Value<'a>>,
}

pub(crate) const DYN_TAGGED_TYP_NAME: &'static str = "seabored::adapters::DynamicTaggedValue";

#[cfg(feature = "serde")]
impl<'de> ::serde::Deserialize<'de> for DynamicTaggedValue<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct DynamicTaggedValueVisitor;

        impl<'de> ::serde::de::Visitor<'de> for DynamicTaggedValueVisitor {
            type Value = DynamicTaggedValue<'de>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "A Dynamically tagged CBOR Value")
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                use ::serde::Deserialize as _;
                let tag = crate::adapters::serde::TAG
                    .get()
                    .expect("This should never happen");
                let value = crate::Value::deserialize(deserializer)?;
                Ok(DynamicTaggedValue {
                    tag: tag.into(),
                    value: std::borrow::Cow::Owned(value),
                })
            }
        }

        deserializer.deserialize_newtype_struct(DYN_TAGGED_TYP_NAME, DynamicTaggedValueVisitor)
    }
}

#[cfg(feature = "serde")]
impl<'a> ::serde::Serialize for DynamicTaggedValue<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        serializer.serialize_newtype_struct(DYN_TAGGED_TYP_NAME, self)
    }
}

/// Internal only: Parses the tag value from the "seabored::serde::Tagged<'life, TAG, V>" form
/// Returns `None` if non-matching
///
/// ## Warning
/// Can panic if fed anything else than our own internals, hence the visibility of this fn
#[inline(always)]
pub(crate) fn parse_tag_from_typ(typ: &str) -> Option<u64> {
    const TAGGED_VALUE_TYP_ROOT_NAME: &'static str = "seabored::adapters::Tagged";
    // Split at the generics boundary to get (`seabored::serde::Tagged`, `'life, TAG, V>`)
    let (tname, targs) = typ.split_once('<')?;
    if tname != TAGGED_VALUE_TYP_ROOT_NAME {
        return None;
    }

    let (_, targs) = targs.split_once(',').unwrap(); // SAFETY: Lifetime is always present in the type, skip over lifetime
    let (tag_str, _) = targs.split_once(',').unwrap(); // SAFETY: const TAG: u64 is next
    let tag_str = tag_str.trim();
    Some(if tag_str == "u64::MAX" {
        u64::MAX
    } else {
        tag_str.parse().unwrap() // SAFETY: Tag cannot be anything else than u64
    })
}

#[cfg(feature = "serde")]
impl<'a, const TAG: u64, V: ::serde::Serialize + 'a> ::serde::Serialize for Tagged<'a, TAG, V> {
    #[inline(always)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        serializer.serialize_newtype_struct(std::any::type_name::<Self>(), &self.inner)
    }
}

#[cfg(feature = "serde")]
impl<'a, 'de: 'a, const TAG: u64, V: ::serde::Deserialize<'de> + 'a> ::serde::Deserialize<'de>
    for Tagged<'a, TAG, V>
{
    #[inline(always)]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct TaggedValueVisitor<'a, 'de: 'a, const TAG: u64, V: ::serde::Deserialize<'de>>(
            std::marker::PhantomData<(&'a V, &'de ())>,
        );

        impl<'a, 'de: 'a, const TAG: u64, V: ::serde::Deserialize<'de>> ::serde::de::Visitor<'de>
            for TaggedValueVisitor<'a, 'de, TAG, V>
        {
            type Value = Tagged<'a, TAG, V>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "A CBOR Tagged Value")
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: ::serde::Deserializer<'de>,
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

/// Much like the [`TaggedValue`] wrapper, this is a wrapper around SimpleValues, since they need to be treated a tad differently
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct SimpleValue(pub u8);

impl SimpleValue {
    pub(crate) const TYP_NAME: &'static str = "seabored::adapters::SimpleValue";
}

#[cfg(feature = "serde")]
impl ::serde::Serialize for SimpleValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        serializer.serialize_newtype_struct(Self::TYP_NAME, self)
    }
}

#[cfg(feature = "serde")]
impl<'de> ::serde::Deserialize<'de> for SimpleValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct SimpleValueVisitor;
        impl<'de> ::serde::de::Visitor<'de> for SimpleValueVisitor {
            type Value = SimpleValue;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "A CBOR SimpleValue")
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                use ::serde::Deserialize as _;
                Ok(SimpleValue(u8::deserialize(deserializer)?))
            }
        }

        deserializer.deserialize_newtype_struct(Self::TYP_NAME, SimpleValueVisitor)
    }
}
