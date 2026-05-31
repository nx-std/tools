//! Integer fields of the NPDM descriptor that are encoded as hexadecimal text.
//!
//! The descriptor JSON expresses program IDs, addresses, sizes and permission
//! masks as hexadecimal strings rather than JSON numbers (which cannot represent
//! the full `u64` range losslessly). These newtypes parse that representation at
//! the deserialization boundary so the rest of the descriptor works with decoded
//! `u64` values.

use std::fmt;

/// A `u64` encoded as a hexadecimal string in the descriptor JSON.
///
/// Accepts both prefixed (`"0x1F00"`) and unprefixed (`"1F00"`) forms on
/// deserialization and serializes back to the canonical `0x`-prefixed form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HexU64(u64);

impl HexU64 {
    /// Returns the decoded integer value.
    pub fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for HexU64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:x}", self.0)
    }
}

impl<'de> serde::Deserialize<'de> for HexU64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = <String as serde::Deserialize>::deserialize(deserializer)?;
        parse_hex_u64(&raw)
            .map(Self)
            .map_err(|_| serde::de::Error::custom(format!("invalid hexadecimal integer '{raw}'")))
    }
}

impl serde::Serialize for HexU64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

/// A `u64` that may appear either as a JSON number or as a hexadecimal string.
///
/// Some descriptor fields (syscall IDs, the minimum kernel version) are written
/// either way depending on the source template, so both forms are accepted.
/// Serializes back as a plain JSON number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct U64OrHex(u64);

impl U64OrHex {
    /// Returns the decoded integer value.
    pub fn get(self) -> u64 {
        self.0
    }
}

impl From<u64> for U64OrHex {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl<'de> serde::Deserialize<'de> for U64OrHex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = U64OrHex;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a u64 integer or a hexadecimal string")
            }

            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<U64OrHex, E> {
                Ok(U64OrHex(value))
            }

            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<U64OrHex, E> {
                u64::try_from(value)
                    .map(U64OrHex)
                    .map_err(|_| E::custom("integer must be non-negative"))
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<U64OrHex, E> {
                parse_hex_u64(value)
                    .map(U64OrHex)
                    .map_err(|_| E::custom(format!("invalid hexadecimal integer '{value}'")))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl serde::Serialize for U64OrHex {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u64(self.0)
    }
}

/// Parse a hexadecimal string with an optional `0x`/`0X` prefix.
fn parse_hex_u64(input: &str) -> Result<u64, std::num::ParseIntError> {
    let stripped = input
        .strip_prefix("0x")
        .or_else(|| input.strip_prefix("0X"))
        .unwrap_or(input);
    u64::from_str_radix(stripped, 16)
}

#[cfg(feature = "json-schema")]
mod json_schema {
    use std::borrow::Cow;

    use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};

    use super::{HexU64, U64OrHex};

    /// JSON Schema pattern matching an optionally `0x`-prefixed hexadecimal integer.
    const HEX_PATTERN: &str = "^(0[xX])?[0-9a-fA-F]+$";

    impl JsonSchema for HexU64 {
        fn inline_schema() -> bool {
            true
        }

        fn schema_name() -> Cow<'static, str> {
            "HexU64".into()
        }

        fn schema_id() -> Cow<'static, str> {
            concat!(module_path!(), "::HexU64").into()
        }

        fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
            json_schema!({
                "type": "string",
                "pattern": HEX_PATTERN,
                "description": "64-bit unsigned integer encoded as a hexadecimal string",
            })
        }
    }

    impl JsonSchema for U64OrHex {
        fn inline_schema() -> bool {
            true
        }

        fn schema_name() -> Cow<'static, str> {
            "U64OrHex".into()
        }

        fn schema_id() -> Cow<'static, str> {
            concat!(module_path!(), "::U64OrHex").into()
        }

        fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
            json_schema!({
                "anyOf": [
                    { "type": "integer", "minimum": 0 },
                    { "type": "string", "pattern": HEX_PATTERN },
                ],
                "description": "64-bit unsigned integer as a JSON number or hexadecimal string",
            })
        }
    }
}
