use rustc_hex::{FromHex, ToHex};
use serde::{
    de::Error, de::MapAccess, de::Visitor, de::SeqAccess, Deserialize, Deserializer, Serialize, Serializer,
};
use std::{str::FromStr, fmt};
use evm_loader::{H160, H256};
use super::{H160T, H256T, U256T};

/// Represents rpc api block number param.
#[derive(Debug, PartialEq, Eq)]
pub enum BlockNumber {
    /// Hash
    Hash {
        /// block hash
        hash: H256T,
        /// only return blocks part of the canon chain
        require_canonical: bool,
    },
    /// Number
    Num(u64),
    /// Latest block
    Latest,
    /// Earliest block (genesis)
    Earliest,
    /// Pending block (being mined)
    Pending,
}

impl Default for BlockNumber {
    fn default() -> Self {
        BlockNumber::Latest
    }
}

impl<'a> Deserialize<'a> for BlockNumber {
    fn deserialize<D>(deserializer: D) -> Result<BlockNumber, D::Error>
        where
            D: Deserializer<'a>,
    {
        deserializer.deserialize_any(BlockNumberVisitor)
    }
}

impl Serialize for BlockNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        match &*self {
            BlockNumber::Hash {
                hash,
                require_canonical,
            } => serializer.serialize_str(&format!(
                "{{ 'hash': '{}', 'requireCanonical': '{}'  }}",
                hash.0.to_string(), require_canonical
            )),
            BlockNumber::Num(ref x) => serializer.serialize_str(&format!("0x{:x}", x)),
            BlockNumber::Latest => serializer.serialize_str("latest"),
            BlockNumber::Earliest => serializer.serialize_str("earliest"),
            BlockNumber::Pending => serializer.serialize_str("pending"),
        }
    }
}

struct BlockNumberVisitor;

impl<'a> Visitor<'a> for BlockNumberVisitor {
    type Value = BlockNumber;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "a block number or 'latest', 'earliest' or 'pending'"
        )
    }

    fn visit_map<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
        where
            V: MapAccess<'a>,
    {
        let (mut require_canonical, mut block_number, mut block_hash) =
            (false, None::<u64>, None::<H256T>);

        loop {
            let key_str: Option<String> = visitor.next_key()?;

            match key_str {
                Some(key) => match key.as_str() {
                    "blockNumber" => {
                        let value: String = visitor.next_value()?;
                        if value.starts_with("0x") {
                            let number = u64::from_str_radix(&value[2..], 16).map_err(|e| {
                                Error::custom(format!("Invalid block number: {}", e))
                            })?;

                            block_number = Some(number);
                            break;
                        }
                        return Err(Error::custom(
                            "Invalid block number: missing 0x prefix".to_string(),
                        ));
                    }
                    "blockHash" => {
                        block_hash = Some(visitor.next_value()?);
                    }
                    "requireCanonical" => {
                        require_canonical = visitor.next_value()?;
                    }
                    key => return Err(Error::custom(format!("Unknown key: {}", key))),
                },
                None => break,
            };
        }

        if let Some(number) = block_number {
            return Ok(BlockNumber::Num(number));
        }

        if let Some(hash) = block_hash {
            return Ok(BlockNumber::Hash {
                hash,
                require_canonical,
            });
        }

        return Err(Error::custom("Invalid input"));
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: Error,
    {
        match value {
            "latest" => Ok(BlockNumber::Latest),
            "earliest" => Ok(BlockNumber::Earliest),
            "pending" => Ok(BlockNumber::Pending),
            _ if value.starts_with("0x") => u64::from_str_radix(&value[2..], 16)
                .map(BlockNumber::Num)
                .map_err(|e| Error::custom(format!("Invalid block number: {}", e))),
            _ => Err(Error::custom(
                "Invalid block number: missing 0x prefix".to_string(),
            )),
        }
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: Error,
    {
        self.visit_str(value.as_ref())
    }
}

/// Wrapper structure around vector of bytes.
#[derive(Debug, PartialEq, Eq, Default, Hash, Clone)]
pub struct Bytes(pub Vec<u8>);

impl Bytes {
    /// Simple constructor.
    pub fn new(bytes: Vec<u8>) -> Bytes {
        Bytes(bytes)
    }
}

impl From<Vec<u8>> for Bytes {
    fn from(bytes: Vec<u8>) -> Bytes {
        Bytes(bytes)
    }
}

impl Into<Vec<u8>> for Bytes {
    fn into(self) -> Vec<u8> {
        self.0
    }
}

impl Serialize for Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        let mut value = "0x".to_owned();
        value.push_str(self.0.to_hex::<String>().as_ref());
        serializer.serialize_str(value.as_ref())
    }
}

impl<'a> Deserialize<'a> for Bytes {
    fn deserialize<D>(deserializer: D) -> Result<Bytes, D::Error>
        where
            D: Deserializer<'a>,
    {
        deserializer.deserialize_any(BytesVisitor)
    }
}

struct BytesVisitor;

impl<'a> Visitor<'a> for BytesVisitor {
    type Value = Bytes;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a 0x-prefixed, hex-encoded vector of bytes")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: Error,
    {
        if value.len() >= 2 && value.starts_with("0x") && value.len() & 1 == 0 {
            Ok(Bytes::new(FromHex::from_hex(&value[2..]).map_err(|e| {
                Error::custom(format!("Invalid hex: {}", e))
            })?))
        } else {
            Err(Error::custom(
                "Invalid bytes format. Expected a 0x-prefixed hex string with even length",
            ))
        }
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: Error,
    {
        self.visit_str(value.as_ref())
    }
}



#[derive(Debug, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct EthCallObject {
    // (optional) String of the address the transaction is sent from.
    pub from: Option<H160T>,

    // String of the address the transaction is directed to.
    pub to: H160T,

    // (optional) Integer of the gas provided for the transaction execution.
    pub gas: Option<U256T>,

    // (optional) Integer of the gasPrice used for each paid gas encoded as a hexadecimal.
    pub gasprice: Option<U256T>,

    // (optional) Integer of the value sent with this transaction encoded as a hexadecimal.
    pub value: Option<U256T>,

    // (optional) String of the hash of the method signature and encoded parameters
    pub data: Option<Bytes>,
}

#[derive(Debug, PartialEq)]
pub enum FilterAddress {
    Single(H160T),
    Many(Vec<H160T>),
}

impl<'a> Deserialize<'a> for FilterAddress {
    fn deserialize<D>(deserializer: D) -> Result<FilterAddress, D::Error>
        where
            D: Deserializer<'a>,
    {
        deserializer.deserialize_any(FilterAddressVisitor)
    }
}

struct FilterAddressVisitor;

impl<'a> Visitor<'a> for FilterAddressVisitor {
    type Value = FilterAddress;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "H160 hex encoded address or sequence of H160 addresses expected"
        )
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
        where
            V: SeqAccess<'a>,
    {
        let mut addresses: Vec<H160T> = Vec::new();
        loop {
            let entry: Option<H160T> = seq.next_element()?;
            match entry {
                Some(entry) => addresses.push(entry),
                None => break,
            }
        }

        Ok(FilterAddress::Many(addresses))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: Error,
    {
        Ok(FilterAddress::Single(H160T(
            H160::from_str(value)
                .map_err(|err| E::custom(format!("Failed to deserialize H160T: {:?}", err)))?
        )))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: Error,
    {
        self.visit_str(value.as_ref())
    }
}

#[derive(Debug, PartialEq)]
pub enum FilterTopic {
    Single(H256T),
    Many(Vec<H256T>),
}

impl<'a> Deserialize<'a> for FilterTopic {
    fn deserialize<D>(deserializer: D) -> Result<FilterTopic, D::Error>
        where
            D: Deserializer<'a>,
    {
        deserializer.deserialize_any(FilterTopicVisitor)
    }
}

struct FilterTopicVisitor;

impl<'a> Visitor<'a> for FilterTopicVisitor
{
    type Value = FilterTopic;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "H256 hex encoded address or sequence of H256 addresses expected"
        )
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
        where
            V: SeqAccess<'a>,
    {
        let mut topics: Vec<H256T> = Vec::new();
        loop {
            let entry: Option<H256T> = seq.next_element()?;
            match entry {
                Some(entry) => topics.push(entry),
                None => break,
            }
        }

        Ok(FilterTopic::Many(topics))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: Error,
    {
        Ok(FilterTopic::Single(H256T(
            H256::from_str(value)
                .map_err(|err| E::custom(format!("Failed to deserialize H256T: {:?}", err)))?
        )))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: Error,
    {
        self.visit_str(value.as_ref())
    }
}

#[derive(Debug, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct FilterObject {
    // Integer block number encoded as a hexadecimal, "latest","pending", or "earliest" tags.
    pub from_block: Option<BlockNumber>,

    // Integer block number encoded as a hexadecimal, "latest","pending", or "earliest" tags.
    pub to_block: Option<BlockNumber>,

    // Contract address or a list of addresses from which logs should originate.
    pub address: Option<FilterAddress>,

    // Array of DATA topics. Topics are order-dependent.
    pub topics: Option<Vec<FilterTopic>>,

    // With the addition of EIP-234, blockHash will be a new filter option
    // which restricts the logs returned to the single block with the 32-byte
    // hash blockHash. Using blockHash is equivalent to
    // fromBlock = toBlock = the block number with hash blockHash.
    // If blockHash is present in in the filter criteria,
    // then neither fromBlock nor toBlock are allowed.
    pub block_hash: Option<H256T>,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct LogObject {
    // Boolean true if log was removed, due to a chain reorganization. false if its a valid log.
    pub removed: bool,

    // Integer of log index position in the block encoded as a hexadecimal. null if pending.
    pub log_index: String,

    // Integer of transactions index position log was created from. null if pending.
    pub transaction_index: String,

    pub transaction_log_index: String,

    // Hash of the transactions this log was created from. null if pending.
    pub transaction_hash: U256T,

    // Hash of the block where this log was in. null when its pending. null if pending.
    pub block_hash: U256T,

    // The block number where this log was, encoded as a hexadecimal. null if pending.
    pub block_number: String,

    // The address from which this log originated.
    pub address: H160T,

    // Contains one or more 32 Bytes non-indexed arguments of the log.
    pub data: String,

    // Array of 0 to 4 32 Bytes of indexed log arguments.
    pub topics: Vec<U256T>,
}
