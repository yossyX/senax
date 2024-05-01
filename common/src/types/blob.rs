use anyhow::Result;
use base64::alphabet;
use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
use base64::{engine::general_purpose, Engine as _};
use once_cell::sync::OnceCell;
use schemars::schema::{InstanceType, Schema, SchemaObject};
use schemars::JsonSchema;
use serde::Serialize;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{self, Display};
use std::str::FromStr;
use std::sync::RwLock;

pub static FILES: OnceCell<RwLock<HashMap<String, Cow<[u8]>>>> = OnceCell::new();

#[derive(Serialize, Clone, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord, JsonSchema)]
pub struct Blob(#[schemars(schema_with = "crate::types::blob::schema")] pub Vec<u8>);

pub(crate) fn schema(_: &mut schemars::gen::SchemaGenerator) -> Schema {
    let schema = SchemaObject {
        instance_type: Some(InstanceType::String.into()),
        ..Default::default()
    };
    Schema::Object(schema)
}

impl<'de> serde::Deserialize<'de> for Blob {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Visitor;

        struct IdVisitor;

        impl<'de> Visitor<'de> for IdVisitor {
            type Value = Blob;

            #[inline]
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an file path or base64")
            }

            #[inline]
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let files = FILES.get().map(|files| files.read().unwrap());
                if let Some(file) =
                    files.and_then(|files| files.get(v).map(|v| v.as_ref().to_owned()))
                {
                    return Ok(Blob(file));
                }
                if let Ok(decode) = DECODE.decode(v) {
                    return Ok(Blob(decode));
                }
                Err(serde::de::Error::custom(format_args!(
                    "file not found: {}",
                    v
                )))
            }
        }
        deserializer.deserialize_bytes(IdVisitor)
    }
}

impl From<Blob> for Vec<u8> {
    fn from(v: Blob) -> Self {
        v.0
    }
}

impl From<Vec<u8>> for Blob {
    fn from(v: Vec<u8>) -> Self {
        Blob(v)
    }
}

impl From<&[u8]> for Blob {
    fn from(v: &[u8]) -> Self {
        Blob(v.to_vec())
    }
}

impl FromStr for Blob {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Blob(DECODE.decode(s)?))
    }
}
impl Display for Blob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&general_purpose::STANDARD.encode(&self.0))
    }
}

pub trait BlobToApi {
    fn to_str(&self) -> String;
}
impl BlobToApi for std::sync::Arc<Vec<u8>> {
    fn to_str(&self) -> String {
        general_purpose::STANDARD_NO_PAD.encode(self.as_ref())
    }
}
pub trait ApiToBlob {
    fn to_vec(&self) -> Vec<u8>;
}
impl ApiToBlob for String {
    fn to_vec(&self) -> Vec<u8> {
        DECODE.decode(self).unwrap_or_default()
    }
}
pub const DECODE: GeneralPurpose = GeneralPurpose::new(
    &alphabet::STANDARD,
    GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(
            "SGVsbG8sIOOCu+OCq+OCpA==".to_string().to_vec(),
            "Hello, セカイ".as_bytes()
        );
        assert_eq!(
            "SGVsbG8sIOOCu+OCq+OCpA".to_string().to_vec(),
            "Hello, セカイ".as_bytes()
        );
    }
}
