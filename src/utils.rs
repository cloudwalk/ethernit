use serde::de::Error;
use serde::{Deserialize, Deserializer};

pub fn u64_from_hex<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    // extract value
    let value: String = Deserialize::deserialize(deserializer)?;
    let value_trimmed = value.trim_start_matches("0x");

    // parse value
    u64::from_str_radix(value_trimmed, 16).map_err(Error::custom)
}

pub fn i64_from_hex<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    // extract value
    let value: String = Deserialize::deserialize(deserializer)?;
    let value_trimmed = value.trim_start_matches("0x");

    // parse value
    i64::from_str_radix(value_trimmed, 16).map_err(Error::custom)
}

pub fn bytes_from_hex<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    // extract value
    let value: String = Deserialize::deserialize(deserializer)?;
    let value_trimmed = value.trim_start_matches("0x");

    // parse value
    hex::decode(value_trimmed).map_err(Error::custom)
}
