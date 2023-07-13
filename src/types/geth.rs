use {
    super::Bytes,
    std::{collections::BTreeMap, iter},
    neon_cli_lib::types::{
        trace::{TracedCall, VMTrace, VMOperation},
        Address,
    },
    serde::{self, Deserialize, Serialize},
    ethnum::U256,
};

#[derive(Deserialize, Default, PartialEq, Debug)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
/// Represents the arguments to construct a new transaction or a message call
pub struct TransactionArgs {
    /// From
    pub from: Option<Address>,
    /// To
    pub to: Option<Address>,
    /// Gas
    pub gas: Option<U256>,
    /// Gas Price
    pub gas_price: Option<U256>,
    /// Max fee per gas
    pub max_fee_per_gas: Option<U256>,
    /// Miner bribe
    pub max_priority_fee_per_gas: Option<U256>,
    /// Value
    pub value: Option<U256>,
    /// Nonce
    pub nonce: Option<U256>,
    /// Input
    #[serde(alias = "data")]
    pub input: Option<Bytes>,
    /// Access list
    //#[serde(skip_serializing_if = "Option::is_none")]
    //pub access_list: Option<AccessList>,
    /// Chain id
    pub chain_id: Option<U256>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
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


// #[derive(Serialize, Debug)]
// #[serde(untagged, rename_all = "camelCase")]
// pub enum Trace {
//     Logs(ExecutionResult),
    // JsTrace(serde_json::Value),
// }

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


impl From<TracedCall> for ExecutionResult {
    // TODO: move to trace_call with merging
    fn from(traced_call: TracedCall) -> Self {
        let failed = false; // TODO: NDEV-1206, NDEV-1207
        let gas = traced_call.used_gas;
        let return_value = String::new(); // TODO NDEV-1206, NDEV-1207

        let struct_logs = match traced_call.vm_trace {
            Some(vm_trace) => StructLog::from_trace_with_depth(vm_trace, 1).collect(),
            None => vec![],
        };

        Self {
            failed,
            gas,
            return_value,
            struct_logs,
        }
    }
}

impl ExecutionResult {
    #[allow(unused)]
    pub fn new(traced_call: TracedCall, options: &TraceTransactionOptions) -> Self {
        let failed = false; // TODO: NDEV-1206, NDEV-1207
        let gas = traced_call.used_gas;
        let return_value = String::new(); // TODO NDEV-1206, NDEV-1207

        let mut logs: Vec<StructLog> = match traced_call.vm_trace {
            Some(vm_trace) => StructLog::from_trace_with_depth(vm_trace, 1).collect(),
            None => vec![],
        };

        let data = traced_call.full_trace_data;
        assert_eq!(logs.len(), data.len());

        logs.iter_mut().zip(data.into_iter()).for_each(|(l, d)| {
            if !options.disable_stack {
                l.stack = Some(d.stack.iter().map(|entry|{ U256::from_le_bytes(*entry) }).collect());
            }

            if options.enable_memory && !d.memory.is_empty() {
                l.memory = Some(
                    d.memory
                        .chunks(32)
                        .map(|slice| slice.to_vec().into())
                        .collect(),
                );
            }

            if !options.disable_storage  {
                l.storage = Some(d.storage.into_iter().map(|(k, v)| { (k, U256::from_le_bytes(v)) }).collect());
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
    pub memory: Option<Vec<Bytes>>, // U256 sized chunks
    /// Snapshot of the current stack sate
    #[serde(skip_serializing_if = "Option::is_none")]
    // pub stack: Option<Vec<[u8; 32]>>,
    pub stack: Option<Vec<U256>>,
    /// Snapshot of the current storage
    #[serde(skip_serializing_if = "Option::is_none")]
    // pub storage: Option<BTreeMap<U256, [u8; 32]>>,
    pub storage: Option<BTreeMap<U256, U256>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl StructLog {
    // use boxing bc of the recursive opaque type
    fn from_trace_with_depth(
        vm_trace: VMTrace,
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


impl From<(usize, VMOperation)> for StructLog {
    fn from((depth, vm_operation): (usize, VMOperation)) -> Self {
        let pc = vm_operation.pc as u64;
        let op_name = "";
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
