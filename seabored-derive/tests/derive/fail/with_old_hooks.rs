#![allow(dead_code)]

mod old {
    use seabored::error::{SeaboredDeError, SeaboredSerError};
    use seabored::reexports::parsio::{DepthAwareReaderGuard, Read, Write};

    pub fn cbor_serialize_to<W: Write>(
        _value: &u32,
        _writer: &mut W,
    ) -> Result<usize, SeaboredSerError> {
        Ok(0)
    }

    pub fn cbor_deserialize_from<'de, R: Read<'de>>(
        _reader: &mut DepthAwareReaderGuard<'_, R>,
    ) -> Result<u32, SeaboredDeError> {
        Ok(0)
    }
}

#[derive(seabored::CborSerialize, seabored::CborDeserialize)]
struct Bad {
    #[cbor(with = old)]
    value: u32,
}

fn main() {}
