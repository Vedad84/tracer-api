use std::{collections::BTreeMap, iter};

use evm::{H160, U256, H256};
use serde::{self, Deserialize, Serialize};

use crate::neon;
use crate::types::ec::trace;
use serde::de;
use std::fmt;

#[derive(Serialize, Deserialize, Debug)]
pub struct BlockNumber(#[serde(with = "string")] u64);

impl From<BlockNumber> for u64 {
    fn from(b: BlockNumber) -> u64 {
        b.0
    }
}

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
        if value.starts_with("0x") {
            let number = HasRadix::from_radix(&value[2..], 16)
                .map_err(|e| serde::de::Error::custom(format!("Invalid block number: {}", e)))?;
            Ok(number)
        } else {
            return Err(serde::de::Error::custom(
                "Invalid block number: missing 0x prefix".to_string(),
            ));
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TraceTransactionOptions {
    #[serde(default)]
    pub enable_memory: bool,
    #[serde(default)]
    pub disable_storage: bool,
    #[serde(default)]
    pub disable_stack: bool,
    #[serde(default)]
    pub tracer: Option<String>,
    #[serde(default)]
    pub timeout: Option<String>,
}

pub type Bytes = crate::v1::types::Bytes;

#[derive(Serialize, Debug)]
#[serde(untagged, rename_all = "camelCase")]
pub enum Trace {
    Logs(ExecutionResult),
    JsTrace(serde_json::Value),
}

/// ExecutionResult groups all structured logs emitted by the EVM
/// while replaying a transaction in debug mode as well as transaction
/// execution status, the amount of gas used and the return value
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionResult {
    /// Is execution failed or not
    pub failed: bool,
    /// Total used gas but include the refunded gas
    pub gas: u64,
    /// The data after execution or revert reason
    pub return_value: String,
    /// Logs emitted during execution
    pub struct_logs: Vec<StructLog>,
}

impl From<neon::TracedCall> for ExecutionResult {
    // TODO: move to trace_call with merging
    fn from(traced_call: neon::TracedCall) -> Self {
        let failed = !traced_call.exit_reason.is_succeed();
        let gas = traced_call.used_gas;
        let return_value = hex::encode(traced_call.result);
        let struct_logs = traced_call.vm_trace.map_or(vec![], Into::into);

        Self {
            failed,
            gas,
            return_value,
            struct_logs,
        }
    }
}

impl ExecutionResult {
    pub fn new(traced_call: neon::TracedCall, options: &TraceTransactionOptions) -> Self {
        let failed = !traced_call.exit_reason.is_succeed();
        let gas = traced_call.used_gas;
        let return_value = hex::encode(traced_call.result);
        let mut logs = traced_call.vm_trace.map_or(vec![], Into::into);
        let data = traced_call.full_trace_data;

        assert_eq!(logs.len(), data.len());

        logs.iter_mut().zip(data.into_iter()).for_each(|(l, d)| {
            if !options.disable_stack {
                l.stack = Some(d.stack);
            }

            if options.enable_memory && !d.memory.is_empty() {
                l.memory = Some(d.memory.into());
            }

            if !options.disable_storage && !d.storage.is_none() {
                l.storage = Some(d.storage.into_iter().collect());
            }
        });

        Self {
            failed,
            gas,
            return_value,
            struct_logs: logs,
        }
    }
}

/// StructLog stores a structured log emitted by the EVM while replaying a
/// transaction in debug mode
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StructLog {
    /// Program counter.
    pub pc: u64,
    /// Operation name
    #[serde(rename(serialize = "op"))]
    pub op_name: &'static str,
    /// Amount of used gas
    pub gas: Option<u64>,
    /// Gas cost for this instruction.
    pub gas_cost: u64,
    /// Current depth
    pub depth: u32,
    /// Snapshot of the current memory sate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<Bytes>,
    /// Snapshot of the current stack sate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<Vec<U256>>,
    /// Snapshot of the current storage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<BTreeMap<U256, U256>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl StructLog {
    // use boxing bc of the recursive opaque type
    fn from_trace_with_depth(
        vm_trace: trace::VMTrace,
        depth: usize,
    ) -> Box<dyn Iterator<Item = Self>> {
        let operations = vm_trace.operations;
        let mut subs = vm_trace.subs.into_iter().peekable();

        Box::new(
            operations
                .into_iter()
                .enumerate()
                .map(move |(idx, operation)| {
                    let main_op = iter::once((depth, operation).into());
                    let mut subtrace_iter = None;
                    if subs
                        .peek()
                        .map_or(false, |subtrace| idx == subtrace.parent_step)
                    {
                        let subtrace = subs.next().expect("just peeked it");
                        subtrace_iter = Some(Self::from_trace_with_depth(subtrace, depth + 1));
                    }
                    main_op.chain(subtrace_iter.into_iter().flatten())
                })
                .flatten(),
        )
    }
}

impl From<trace::VMTrace> for Vec<StructLog> {
    fn from(vm_trace: trace::VMTrace) -> Self {
        StructLog::from_trace_with_depth(vm_trace, 1).collect()
    }
}

impl From<(usize, trace::VMOperation)> for StructLog {
    fn from((depth, vm_operation): (usize, trace::VMOperation)) -> Self {
        let pc = vm_operation.pc as u64;
        let op_name = trace::INSTRUCTIONS
            .get(vm_operation.instruction as usize)
            .copied()
            .flatten()
            .map_or("INVALID", |i| i.name);
        let gas = vm_operation
            .executed
            .as_ref()
            .map(|e| e.gas_used.as_u128() as u64);
        let gas_cost = vm_operation.gas_cost.as_u128() as u64;
        let depth = depth as u32;
        let memory = None;
        let stack = None;
        let storage = None;
        let error = None;

        Self {
            pc,
            op_name,
            gas,
            gas_cost,
            depth,
            memory,
            stack,
            storage,
            error,
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum TraceResult {
    Result(ExecutionResult),
    Error(String),
}

#[derive(Deserialize, Default, PartialEq, Debug)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
/// Represents the arguments to construct a new transaction or a message call
pub struct TransactionArgs {
    /// From
    pub from: Option<H160T>,
    /// To
    pub to: Option<H160T>,
    /// Gas
    pub gas: Option<U256T>,
    /// Gas Price
    pub gas_price: Option<U256T>,
    /// Max fee per gas
    pub max_fee_per_gas: Option<U256T>,
    /// Miner bribe
    pub max_priority_fee_per_gas: Option<U256T>,
    /// Value
    pub value: Option<U256T>,
    /// Nonce
    pub nonce: Option<U256T>,
    /// Input
    #[serde(alias = "data")]
    pub input: Option<Bytes>,
    /// Access list
    //#[serde(skip_serializing_if = "Option::is_none")]
    //pub access_list: Option<AccessList>,
    /// Chain id
    pub chain_id: Option<U256T>,
}


#[derive(Debug, Deserialize)]
#[derive(std::cmp::PartialEq)]
pub struct H160T(
    #[serde(deserialize_with = "deserialize_hex_h160")]
    pub H160
);

#[derive(Debug, Serialize, Deserialize)]
#[derive(std::cmp::PartialEq)]
pub struct U256T(
    #[serde(deserialize_with = "deserialize_hex_u256", serialize_with = "serialize_hex_u256")]
    pub U256
);

#[derive(Debug, Deserialize)]
#[derive(std::cmp::PartialEq)]
pub struct H256T(
    #[serde(deserialize_with = "deserialize_hex_h256")]
    pub H256
);


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
                return Err(E::custom(format!("Invalid bytes format. Expected a 0x-prefixed hex string")));
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
                return Err(E::custom(format!("Invalid bytes format. Expected a 0x-prefixed hex string")));
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
                return Err(E::custom(format!("Invalid bytes format. Expected a 0x-prefixed hex string")));
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

    static SYMTABLE: [char; 16] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f'];

    let string = (0..=31).rev().map(|idx| {
        let byte = value.byte(idx);
        format!("{}{}",
                SYMTABLE[usize::from(byte >> 4)],
                SYMTABLE[usize::from(byte & 0x0F)])
    }).collect::<Vec<String>>().concat();

    serializer.serialize_str(&string)
}


