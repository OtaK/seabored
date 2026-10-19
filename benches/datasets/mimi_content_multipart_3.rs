use std::borrow::Cow;

use indexmap::IndexMap;
use serde::ser::SerializeSeq as _;

use crate::datasets::HasSample;

pub const BYTES: &[u8] = include_bytes!("cbor/mimi-content-multipart-3.cbor");
pub const ID: &str = "mimi-content-multipart-3";

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct MessageId<'a>(#[serde(borrow)] Cow<'a, serde_bytes::ByteArray<32>>);

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct MessageIdOwned(serde_bytes::ByteArray<32>);

#[derive(PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Tstr<'a>(#[serde(borrow)] Cow<'a, str>);

#[derive(PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct TstrOwned(String);

#[derive(serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Bstr<'a>(#[serde(with = "serde_bytes", borrow)] Cow<'a, [u8]>);

#[derive(serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct BstrOwned(#[serde(with = "serde_bytes")] Vec<u8>);

#[derive(serde_tuple::Deserialize_tuple, serde_tuple::Serialize_tuple)]
pub struct Expiration {
    pub relative: bool,
    pub time: u32,
}

#[derive(PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Name<'a> {
    Int(i64),
    Str(#[serde(borrow)] Tstr<'a>),
}

#[derive(PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum NameOwned {
    Int(i64),
    Str(TstrOwned),
}

#[derive(serde_repr::Serialize_repr, serde_repr::Deserialize_repr)]
#[repr(u8)]
pub enum PartSemantics {
    ChooseOne = 0,
    SingleUnit = 1,
    ProcessAll = 2,
}

#[derive(serde_tuple::Deserialize_tuple, serde_tuple::Serialize_tuple)]
pub struct SinglePart<'a> {
    #[serde(borrow)]
    pub content_type: Tstr<'a>,
    #[serde(borrow)]
    pub content: Bstr<'a>,
}

#[derive(serde_tuple::Deserialize_tuple, serde_tuple::Serialize_tuple)]
pub struct SinglePartOwned {
    pub content_type: TstrOwned,
    pub content: BstrOwned,
}

#[derive(serde_tuple::Deserialize_tuple, serde_tuple::Serialize_tuple)]
pub struct ExternalPart<'a> {
    #[serde(borrow)]
    pub content_type: Tstr<'a>,
    #[serde(borrow)]
    pub url: Tstr<'a>,
    pub expires: u32,
    pub size: u64,
    pub enc_alg: u16,
    #[serde(borrow)]
    pub key: Bstr<'a>,
    #[serde(borrow)]
    pub nonce: Bstr<'a>,
    #[serde(borrow)]
    pub aad: Bstr<'a>,
    pub hash_alg: u8,
    #[serde(borrow)]
    pub content_hash: Bstr<'a>,
    #[serde(borrow)]
    pub description: Tstr<'a>,
    #[serde(borrow)]
    pub filename: Tstr<'a>,
}

#[derive(serde_tuple::Deserialize_tuple, serde_tuple::Serialize_tuple)]
pub struct ExternalPartOwned {
    pub content_type: TstrOwned,
    pub url: TstrOwned,
    pub expires: u32,
    pub size: u64,
    pub enc_alg: u16,
    pub key: BstrOwned,
    pub nonce: BstrOwned,
    pub aad: BstrOwned,
    pub hash_alg: u8,
    pub content_hash: BstrOwned,
    pub description: TstrOwned,
    pub filename: TstrOwned,
}

#[derive(serde_tuple::Serialize_tuple)]
pub struct MultiPart<'a> {
    pub part_semantics: PartSemantics,
    #[serde(borrow)]
    pub parts: Vec<NestedPart<'a>>,
}

#[derive(serde_tuple::Serialize_tuple)]
pub struct MultiPartOwned {
    pub part_semantics: PartSemantics,
    pub parts: Vec<NestedPartOwned>,
}

impl<'de> serde::Deserialize<'de> for MultiPart<'de> {
    #[inline(always)]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct MultiPartVisitor;

        impl<'de> serde::de::Visitor<'de> for MultiPartVisitor {
            type Value = MultiPart<'de>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "A MultiPart MIMI-Content struct")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let part_semantics = seq.next_element()?.unwrap();
                let parts = seq.next_element()?.unwrap();
                Ok(MultiPart {
                    part_semantics,
                    parts,
                })
            }
        }

        deserializer.deserialize_seq(MultiPartVisitor)
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde_repr::Serialize_repr, serde_repr::Deserialize_repr,
)]
#[repr(u8)]
// Names mirror the schema they model
#[allow(clippy::enum_variant_names)]
pub enum NestedPartContentCardinality {
    NullPart = 0,
    SinglePart = 1,
    ExternalPart = 2,
    MultiPart = 3,
}

#[allow(clippy::enum_variant_names)]
pub enum NestedPartContent<'a> {
    NullPart,
    SinglePart(SinglePart<'a>),
    ExternalPart(ExternalPart<'a>),
    MultiPart(MultiPart<'a>),
}

impl NestedPartContent<'_> {
    #[inline(always)]
    pub fn cardinality(&self) -> NestedPartContentCardinality {
        match self {
            Self::NullPart => NestedPartContentCardinality::NullPart,
            Self::SinglePart(_) => NestedPartContentCardinality::SinglePart,
            Self::ExternalPart(_) => NestedPartContentCardinality::ExternalPart,
            Self::MultiPart(_) => NestedPartContentCardinality::MultiPart,
        }
    }
}

#[allow(clippy::enum_variant_names)]
pub enum NestedPartContentOwned {
    NullPart,
    SinglePart(SinglePartOwned),
    ExternalPart(ExternalPartOwned),
    MultiPart(MultiPartOwned),
}

impl NestedPartContentOwned {
    #[inline(always)]
    pub fn cardinality(&self) -> NestedPartContentCardinality {
        match self {
            Self::NullPart => NestedPartContentCardinality::NullPart,
            Self::SinglePart(_) => NestedPartContentCardinality::SinglePart,
            Self::ExternalPart(_) => NestedPartContentCardinality::ExternalPart,
            Self::MultiPart(_) => NestedPartContentCardinality::MultiPart,
        }
    }
}

pub struct NestedPart<'a> {
    pub disposition: u8,
    pub language: Tstr<'a>,
    pub part_content: NestedPartContent<'a>,
}

impl<'de> serde::Deserialize<'de> for NestedPart<'de> {
    #[inline(always)]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct NestedPartVisitor;
        impl<'de> serde::de::Visitor<'de> for NestedPartVisitor {
            type Value = NestedPart<'de>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "a NestedPart struct formatted as a tuple value")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<NestedPart<'de>, V::Error>
            where
                V: serde::de::SeqAccess<'de>,
            {
                let disposition = seq.next_element()?.unwrap();
                let language = seq.next_element()?.unwrap();
                let cardinality: NestedPartContentCardinality = seq.next_element()?.unwrap();

                let part_content = match cardinality {
                    NestedPartContentCardinality::NullPart => NestedPartContent::NullPart,
                    NestedPartContentCardinality::SinglePart => {
                        NestedPartContent::SinglePart(SinglePart {
                            content_type: seq.next_element()?.unwrap(),
                            content: seq.next_element()?.unwrap(),
                        })
                    }
                    NestedPartContentCardinality::ExternalPart => {
                        NestedPartContent::ExternalPart(ExternalPart {
                            content_type: seq.next_element()?.unwrap(),
                            url: seq.next_element()?.unwrap(),
                            expires: seq.next_element()?.unwrap(),
                            size: seq.next_element()?.unwrap(),
                            enc_alg: seq.next_element()?.unwrap(),
                            key: seq.next_element()?.unwrap(),
                            nonce: seq.next_element()?.unwrap(),
                            aad: seq.next_element()?.unwrap(),
                            hash_alg: seq.next_element()?.unwrap(),
                            content_hash: seq.next_element()?.unwrap(),
                            description: seq.next_element()?.unwrap(),
                            filename: seq.next_element()?.unwrap(),
                        })
                    }
                    NestedPartContentCardinality::MultiPart => {
                        NestedPartContent::MultiPart(MultiPart {
                            part_semantics: seq.next_element()?.unwrap(),
                            parts: seq.next_element()?.unwrap(),
                        })
                    }
                };

                Ok(NestedPart {
                    disposition,
                    language,
                    part_content,
                })
            }
        }

        deserializer.deserialize_seq(NestedPartVisitor)
    }
}

impl serde::Serialize for NestedPart<'_> {
    #[inline(always)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let len = 3 + match &self.part_content {
            NestedPartContent::NullPart => 0,
            NestedPartContent::SinglePart(_) => 2,
            NestedPartContent::ExternalPart(_) => 12,
            NestedPartContent::MultiPart(_) => 2,
        };
        let mut seq = serializer.serialize_seq(Some(len))?;
        seq.serialize_element(&self.disposition)?;
        seq.serialize_element(&self.language)?;
        seq.serialize_element(&self.part_content.cardinality())?;
        match &self.part_content {
            NestedPartContent::NullPart => {}
            NestedPartContent::SinglePart(single_part_ref) => {
                seq.serialize_element(&single_part_ref.content_type)?;
                seq.serialize_element(&single_part_ref.content)?;
            }
            NestedPartContent::ExternalPart(external_part_ref) => {
                seq.serialize_element(&external_part_ref.content_type)?;
                seq.serialize_element(&external_part_ref.url)?;
                seq.serialize_element(&external_part_ref.expires)?;
                seq.serialize_element(&external_part_ref.size)?;
                seq.serialize_element(&external_part_ref.enc_alg)?;
                seq.serialize_element(&external_part_ref.key)?;
                seq.serialize_element(&external_part_ref.nonce)?;
                seq.serialize_element(&external_part_ref.aad)?;
                seq.serialize_element(&external_part_ref.hash_alg)?;
                seq.serialize_element(&external_part_ref.content_hash)?;
                seq.serialize_element(&external_part_ref.description)?;
                seq.serialize_element(&external_part_ref.filename)?;
            }
            NestedPartContent::MultiPart(multi_part_ref) => {
                seq.serialize_element(&multi_part_ref.part_semantics)?;
                seq.serialize_element(&multi_part_ref.parts)?;
            }
        }
        seq.end()
    }
}

pub struct NestedPartOwned {
    pub disposition: u8,
    pub language: TstrOwned,
    pub part_content: NestedPartContentOwned,
}

impl<'de> serde::Deserialize<'de> for NestedPartOwned {
    #[inline(always)]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct NestedPartVisitor;
        impl<'de> serde::de::Visitor<'de> for NestedPartVisitor {
            type Value = NestedPartOwned;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "a NestedPart struct formatted as a tuple value")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<NestedPartOwned, V::Error>
            where
                V: serde::de::SeqAccess<'de>,
            {
                let disposition = seq.next_element()?.unwrap();
                let language = seq.next_element()?.unwrap();
                let cardinality: NestedPartContentCardinality = seq.next_element()?.unwrap();

                let part_content = match cardinality {
                    NestedPartContentCardinality::NullPart => NestedPartContentOwned::NullPart,
                    NestedPartContentCardinality::SinglePart => {
                        NestedPartContentOwned::SinglePart(SinglePartOwned {
                            content_type: seq.next_element()?.unwrap(),
                            content: seq.next_element()?.unwrap(),
                        })
                    }
                    NestedPartContentCardinality::ExternalPart => {
                        NestedPartContentOwned::ExternalPart(ExternalPartOwned {
                            content_type: seq.next_element()?.unwrap(),
                            url: seq.next_element()?.unwrap(),
                            expires: seq.next_element()?.unwrap(),
                            size: seq.next_element()?.unwrap(),
                            enc_alg: seq.next_element()?.unwrap(),
                            key: seq.next_element()?.unwrap(),
                            nonce: seq.next_element()?.unwrap(),
                            aad: seq.next_element()?.unwrap(),
                            hash_alg: seq.next_element()?.unwrap(),
                            content_hash: seq.next_element()?.unwrap(),
                            description: seq.next_element()?.unwrap(),
                            filename: seq.next_element()?.unwrap(),
                        })
                    }
                    NestedPartContentCardinality::MultiPart => {
                        NestedPartContentOwned::MultiPart(MultiPartOwned {
                            part_semantics: seq.next_element()?.unwrap(),
                            parts: seq.next_element()?.unwrap(),
                        })
                    }
                };

                Ok(NestedPartOwned {
                    disposition,
                    language,
                    part_content,
                })
            }
        }

        deserializer.deserialize_seq(NestedPartVisitor)
    }
}

impl serde::Serialize for NestedPartOwned {
    #[inline(always)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let len = 3 + match &self.part_content {
            NestedPartContentOwned::NullPart => 0,
            NestedPartContentOwned::SinglePart(_) => 2,
            NestedPartContentOwned::ExternalPart(_) => 12,
            NestedPartContentOwned::MultiPart(_) => 2,
        };
        let mut seq = serializer.serialize_seq(Some(len))?;
        seq.serialize_element(&self.disposition)?;
        seq.serialize_element(&self.language)?;
        seq.serialize_element(&self.part_content.cardinality())?;
        match &self.part_content {
            NestedPartContentOwned::NullPart => {}
            NestedPartContentOwned::SinglePart(single_part_ref) => {
                seq.serialize_element(&single_part_ref.content_type)?;
                seq.serialize_element(&single_part_ref.content)?;
            }
            NestedPartContentOwned::ExternalPart(external_part_ref) => {
                seq.serialize_element(&external_part_ref.content_type)?;
                seq.serialize_element(&external_part_ref.url)?;
                seq.serialize_element(&external_part_ref.expires)?;
                seq.serialize_element(&external_part_ref.size)?;
                seq.serialize_element(&external_part_ref.enc_alg)?;
                seq.serialize_element(&external_part_ref.key)?;
                seq.serialize_element(&external_part_ref.nonce)?;
                seq.serialize_element(&external_part_ref.aad)?;
                seq.serialize_element(&external_part_ref.hash_alg)?;
                seq.serialize_element(&external_part_ref.content_hash)?;
                seq.serialize_element(&external_part_ref.description)?;
                seq.serialize_element(&external_part_ref.filename)?;
            }
            NestedPartContentOwned::MultiPart(multi_part_ref) => {
                seq.serialize_element(&multi_part_ref.part_semantics)?;
                seq.serialize_element(&multi_part_ref.parts)?;
            }
        }
        seq.end()
    }
}

#[derive(serde_tuple::Serialize_tuple)]
pub struct MimiContent<'a, ValueType: serde::Serialize + serde::de::Deserialize<'a>> {
    #[serde(with = "serde_bytes")]
    pub salt: &'a [u8; 16],
    pub replaces: Option<MessageId<'a>>,
    pub topic_id: Bstr<'a>,
    pub expires: Option<Expiration>,
    pub in_reply_to: Option<MessageId<'a>>,
    pub extensions: IndexMap<Name<'a>, ValueType>,
    pub nested_part: NestedPart<'a>,
}

impl<'de, ValueType: serde::Serialize + serde::Deserialize<'de> + 'de> serde::Deserialize<'de>
    for MimiContent<'de, ValueType>
{
    #[inline(always)]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct MimiContentVisitor<'de, ValueType: serde::Serialize + serde::Deserialize<'de> + 'de>(
            std::marker::PhantomData<&'de ValueType>,
        );

        impl<'de, ValueType: serde::Serialize + serde::Deserialize<'de> + 'de> Default
            for MimiContentVisitor<'de, ValueType>
        {
            #[inline(always)]
            fn default() -> Self {
                Self(Default::default())
            }
        }

        impl<'de, ValueType: serde::Serialize + serde::Deserialize<'de>> serde::de::Visitor<'de>
            for MimiContentVisitor<'de, ValueType>
        {
            type Value = MimiContent<'de, ValueType>;

            #[inline(always)]
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "A MimiContent value")
            }

            #[inline(always)]
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let salt: &serde_bytes::ByteArray<16> = seq.next_element()?.unwrap();
                let replaces = seq.next_element()?.unwrap();
                let topic_id = seq.next_element()?.unwrap();
                let expires = seq.next_element()?.flatten();
                let in_reply_to = seq.next_element()?.flatten();
                let extensions = seq.next_element()?.unwrap_or_default();
                let nested_part = seq.next_element()?.unwrap();

                Ok(MimiContent {
                    salt,
                    replaces,
                    topic_id,
                    expires,
                    in_reply_to,
                    extensions,
                    nested_part,
                })
            }
        }

        deserializer.deserialize_seq(MimiContentVisitor::default())
    }
}

#[derive(serde_tuple::Serialize_tuple)]
pub struct MimiContentOwned<ValueType: serde::Serialize + serde::de::DeserializeOwned> {
    pub salt: serde_bytes::ByteArray<16>,
    pub replaces: Option<MessageIdOwned>,
    pub topic_id: BstrOwned,
    pub expires: Option<Expiration>,
    pub in_reply_to: Option<serde_bytes::ByteArray<32>>,
    pub extensions: IndexMap<NameOwned, ValueType>,
    pub nested_part: NestedPartOwned,
}

impl<'de, ValueType: serde::Serialize + serde::de::DeserializeOwned + 'de> serde::Deserialize<'de>
    for MimiContentOwned<ValueType>
{
    #[inline(always)]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct MimiContentOwnedVisitor<ValueType: serde::Serialize + serde::de::DeserializeOwned>(
            std::marker::PhantomData<ValueType>,
        );

        impl<ValueType: serde::Serialize + serde::de::DeserializeOwned> Default
            for MimiContentOwnedVisitor<ValueType>
        {
            #[inline(always)]
            fn default() -> Self {
                Self(Default::default())
            }
        }

        impl<'de, ValueType: serde::Serialize + serde::de::DeserializeOwned> serde::de::Visitor<'de>
            for MimiContentOwnedVisitor<ValueType>
        {
            type Value = MimiContentOwned<ValueType>;

            #[inline(always)]
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "A MimiContent value")
            }

            #[inline(always)]
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let salt = seq.next_element()?.unwrap();
                let replaces = seq.next_element()?.unwrap();
                let topic_id = seq.next_element()?.unwrap();
                let expires = seq.next_element()?.flatten();
                let in_reply_to = seq.next_element()?.flatten();
                let extensions = seq.next_element()?.unwrap_or_default();
                let nested_part = seq.next_element()?.unwrap();

                Ok(MimiContentOwned {
                    salt,
                    replaces,
                    topic_id,
                    expires,
                    in_reply_to,
                    extensions,
                    nested_part,
                })
            }
        }

        deserializer.deserialize_seq(MimiContentOwnedVisitor::default())
    }
}

impl<ValueType: serde::Serialize + serde::de::DeserializeOwned> HasSample
    for MimiContentOwned<ValueType>
{
    #[inline(always)]
    fn sample() -> &'static [u8] {
        BYTES
    }
}

impl<'a, ValueType: serde::Serialize + serde::de::Deserialize<'a>> HasSample
    for MimiContent<'a, ValueType>
{
    #[inline(always)]
    fn sample() -> &'static [u8] {
        BYTES
    }
}
