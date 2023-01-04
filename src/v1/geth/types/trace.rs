use std::{collections::BTreeMap, iter};

use evm::{H160, U256, H256};
use serde::{self, Deserialize, Serialize};

use crate::neon;
use serde::de;
use std::fmt;
use byte_slice_cast::AsByteSlice;

mod string {
    use serde::{de, Deserialize, Deserializer, Serializer};
    use std::fmt::Display;
    use std::str::FromStr;

    pub trait HasRadix: Sized {
        type Error;
        fn from_radix(s: &str, radix: u32) -> Result<Self, std::num::ParseIntError>;
    }
    macro_rules! impl_radix {
        ($t: ty) => {
            impl HasRadix for $t {
                type Error = std::num::ParseIntError;

                fn from_radix(s: &str, radix: u32) -> Result<$t, Self::Error> {
                    <$t>::from_str_radix(s, radix)
                }
            }
        };
    }
    impl_radix!(u64);

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
        where
            T: Display,
            S: Serializer,
    {
        serializer.collect_str(value)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
        where
            T: HasRadix,
            T::Error: Display,
            D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if let Some(value) = value.strip_prefix("0x") {
            let number = HasRadix::from_radix(value, 16)
                .map_err(|e| serde::de::Error::custom(format!("Invalid block number: {}", e)))?;
            Ok(number)
        } else {
            Err(serde::de::Error::custom(
                "Invalid block number: missing 0x prefix".to_string(),
            ))
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BlockNumber(#[serde(with = "string")] u64);

impl From<BlockNumber> for u64 {
    fn from(b: BlockNumber) -> u64 {
        b.0
    }
}

pub type Bytes = crate::v1::types::Bytes;


#[derive(Debug, Default, Serialize, Deserialize)]
#[derive(std::cmp::PartialEq, std::cmp::Eq)]
pub struct H160T(
    #[serde(deserialize_with = "deserialize_hex_h160", serialize_with = "serialize_hex_h160")]
    pub H160
);

impl std::cmp::PartialOrd for H160T {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[derive(std::cmp::PartialEq, std::cmp::Eq)]
pub struct U256T(
    #[serde(deserialize_with = "deserialize_hex_u256", serialize_with = "serialize_hex_u256")]
    pub U256
);

impl From<U256> for U256T {
    fn from(value: U256) -> Self {
        U256T(value)
    }
}

impl std::cmp::PartialOrd for U256T {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[derive(std::cmp::PartialEq, std::cmp::Eq)]
pub struct H256T(
    #[serde(deserialize_with = "deserialize_hex_h256", serialize_with = "serialize_hex_h256")]
    pub H256
);

impl From<H256> for H256T {
    fn from(value: H256) -> Self {
        H256T(value)
    }
}

impl std::cmp::PartialOrd for H256T {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

fn deserialize_hex_h160<'de, D>(deserializer: D) -> Result<H160, D::Error>
    where
        D: de::Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = H160;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string containing json data")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
        {
            if !v.starts_with("0x") || v.len() < 3 {
                return Err(E::custom("Invalid bytes format. Expected a 0x-prefixed hex string".to_string()));
            }

            let v = v.split_at(2).1;
            let v = if v.len() & 1 != 0 {
                "0".to_owned() +v
            }
            else{
                v.to_string()
            };

            match hex::decode(v){
                Ok(a) =>  {
                    let address = H160::from_slice(a.as_slice());
                    Ok(address)
                }
                Err(e) => Err(E::custom(format!("Invalid hex format: {}", e)))
            }
        }
    }

    deserializer.deserialize_any(Visitor)
}


fn deserialize_hex_u256<'de, D>(deserializer: D) -> Result<U256, D::Error>
    where
        D: de::Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = U256;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string containing json data")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
        {

            if !v.starts_with("0x") || v.len() < 3 {
                return Err(E::custom("Invalid bytes format. Expected a 0x-prefixed hex string".to_string()));
            }

            let v = v.split_at(2).1;
            let v = if v.len() & 1 != 0 {
                "0".to_owned() +v
            }
            else{
                v.to_string()
            };

            let value = U256::from_str_radix(&v, 16)
                .map_err(|e| E::custom(format!("Invalid hex format: {}", e)))?;
            Ok(value)
        }
    }

    deserializer.deserialize_any(Visitor)
}

fn deserialize_hex_h256<'de, D>(deserializer: D) -> Result<H256, D::Error>
    where
        D: de::Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = H256;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string containing json data")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
        {

            if !v.starts_with("0x") || v.len() < 3 {
                return Err(E::custom("Invalid bytes format. Expected a 0x-prefixed hex string".to_string()));
            }

            let v = v.split_at(2).1;
            let v = if v.len() & 1 != 0 {
                "0".to_owned() +v
            }
            else{
                v.to_string()
            };

            match hex::decode(v){
                Ok(a) =>  {
                    let address = H256::from_slice(a.as_slice());
                    Ok(address)
                }
                Err(e) => Err(E::custom(format!("Invalid hex format: {}", e)))
            }
        }
    }

    deserializer.deserialize_any(Visitor)
}

fn serialize_hex_u256<S>(value: &U256, serializer: S)  -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
    let mut tmp = value.as_byte_slice().iter().cloned().rev().collect::<Vec<u8>>();
    serializer.serialize_str(format!("0x{}", hex::encode(tmp.as_slice())).as_str())
}

fn serialize_hex_h256<S>(value: &H256, serializer: S)  -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
    serializer.serialize_str(format!("0x{}", hex::encode(value.as_bytes())).as_str())
}

fn serialize_hex_h160<S>(value: &H160, serializer: S)  -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
    serializer.serialize_str(format!("0x{}", hex::encode(value.as_bytes())).as_str())
}

