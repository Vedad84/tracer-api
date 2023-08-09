use ethnum::U256;
use serde::{
    de::Error, de::MapAccess, de::Visitor, Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt;

/// Represents rpc api block number param.
#[derive(Default, Debug, PartialEq, Eq)]
pub enum BlockNumber {
    /// Hash
    Hash {
        /// block hash
        hash: U256,
        /// only return blocks part of the canon chain
        require_canonical: bool,
    },
    /// Number
    Num(u64),
    /// Latest block
    #[default]
    Latest,
    /// Earliest block (genesis)
    Earliest,
    /// Pending block (being mined)
    Pending,
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
        match self {
            BlockNumber::Hash {
                hash,
                require_canonical,
            } => serializer.serialize_str(&format!(
                "{{ 'hash': '{hash}', 'requireCanonical': '{require_canonical}'  }}"
            )),
            BlockNumber::Num(ref x) => serializer.serialize_str(&format!("0x{x:x}")),
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
            (false, None::<u64>, None::<U256>);

        loop {
            let key_str: Option<String> = visitor.next_key()?;

            match key_str {
                Some(key) => match key.as_str() {
                    "blockNumber" => {
                        let value: String = visitor.next_value()?;
                        if let Some(value) = value.strip_prefix("0x") {
                            let number = u64::from_str_radix(value, 16)
                                .map_err(|e| Error::custom(format!("Invalid block number: {e}")))?;

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
                    key => return Err(Error::custom(format!("Unknown key: {key}"))),
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

        Err(Error::custom("Invalid input"))
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
                .map_err(|e| Error::custom(format!("Invalid block number: {e}"))),
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
