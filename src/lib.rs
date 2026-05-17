// Copyright 2020-2026 Velithris
// SPDX-License-Identifier: MIT

#![forbid(unsafe_code)]

/// Error types.
pub mod error;
pub use error::*;

use weakauras_codec_ace_serialize::{Deserializer, Serializer};
use weakauras_codec_base64::error::DecodeError as Base64DecodeError;
pub use weakauras_codec_lua_value::LuaValue;

#[derive(Clone, Copy, PartialEq, Eq)]
enum StringVersion {
    #[cfg(feature = "legacy-strings-decoding")]
    Legacy, // base64
    Deflate, // ! + base64
}

pub fn decode(data: &[u8], max_size: Option<usize>) -> Result<Option<LuaValue>, DecodeError> {
    let (base64_data, version) = match data {
        [b'!', rest @ ..] => (rest, StringVersion::Deflate),
        _ => {
            #[cfg(feature = "legacy-strings-decoding")]
            {
                (data, StringVersion::Legacy)
            }

            #[cfg(not(feature = "legacy-strings-decoding"))]
            return Err(DecodeError::InvalidPrefix);
        }
    };

    let compressed_data = match weakauras_codec_base64::decode_to_vec(base64_data) {
        Ok(compressed_data) => compressed_data,
        Err(Base64DecodeError::InvalidByte(invalid_byte_at)) => {
            let prefix_len = base64_data.as_ptr().addr() - data.as_ptr().addr();

            return Err(DecodeError::Base64DecodeError(
                Base64DecodeError::InvalidByte(prefix_len + invalid_byte_at),
            ));
        }
        Err(e) => return Err(e.into()),
    };

    let max_size = max_size.unwrap_or(16 * 1024 * 1024);
    #[cfg(feature = "legacy-strings-decoding")]
    {
        if version == StringVersion::Legacy {
            let decoded = weakauras_codec_lib_compress::decompress(&compressed_data, max_size)?;
            return Deserializer::from_str(&String::from_utf8_lossy(&decoded))
                .deserialize_first()
                .map_err(Into::into);
        }
    }

    let decoded = {
        use flate2::read::DeflateDecoder;
        use std::io::prelude::*;

        let mut result = Vec::new();
        let mut inflater = DeflateDecoder::new(&compressed_data[..]).take(max_size as u64);

        inflater.read_to_end(&mut result)?;

        #[allow(clippy::unbuffered_bytes)] // inflater wraps in-memory data
        if result.len() == max_size && inflater.into_inner().bytes().next().is_some() {
            return Err(DecodeError::DataExceedsMaxSize);
        }

        result
    };

    Ok(Deserializer::from_str(&String::from_utf8_lossy(&decoded)).deserialize_first()?)
}

pub fn encode(value: &LuaValue) -> Result<String, EncodeError> {
    let serialized = Serializer::serialize_one(value, None).map(|v| v.into_bytes())?;

    let compressed = {
        use flate2::{Compression, read::DeflateEncoder};
        use std::io::prelude::*;

        let mut result = Vec::new();
        let mut deflater = DeflateEncoder::new(serialized.as_slice(), Compression::best());

        deflater.read_to_end(&mut result)?;
        result
    };

    Ok(weakauras_codec_base64::encode_to_string_with_prefix(
        &compressed,
        "!",
    )?)
}
