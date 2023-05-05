use {
    std::{time::Duration, sync::Arc},
    log::*,
    evm_loader::types::Address,
    neon_cli_lib::types::{
        trace::{TracedCall, TraceCallConfig, TraceConfig},
        {TraceBlockBySlotParams, TransactionHashParams, TransactionParams},
    },
    crate::{evm_runtime::EVMRuntime, service::Result},
    ethnum::U256,
    super::ERR,
};

const NUM_STEPS_TO_EXECUTE: u32 = 500_000;

#[derive(Clone)]
pub struct NeonCli {
    chain_id: String,
    steps_to_execute: String,
    db_config: String,
    evm_loader: String,
    token_mint: String,
    pub evm_runtime: Arc<EVMRuntime>,
}

impl NeonCli{
    pub fn new(evm_runtime: Arc<EVMRuntime>) -> Self {
        NeonCli{
            chain_id : format!("{}", evm_runtime.config.chain_id),
            steps_to_execute: format!("{}", NUM_STEPS_TO_EXECUTE),
            db_config: format!("/opt/db_config.yaml"),
            evm_loader: evm_runtime.config.evm_loader.to_string(),
            token_mint: evm_runtime.config.token_mint.to_string(),
            evm_runtime,
        }
    }

    async fn execute<T, F: FnOnce(serde_json::Value) -> Result<T> > (
        &self,
        cmd: Vec<&str>,
        payload: Option<impl serde::Serialize>,
        slot: u64,
        tout: &Duration,
        parse_result: F,
        default: Option<T>
    ) -> Result<T> {
        let result =  self.evm_runtime.run_command_with_slot_revision(cmd, payload, slot, tout)
            .await.map_err(|e| ERR(&e.to_string()))?;

        let stderr = std::str::from_utf8(&result.stderr).map_err(|_| ERR("read neon-cli stderr"))?;

        if !result.stdout.is_empty() {
            let stdout = std::str::from_utf8(&result.stdout).map_err(|_| ERR("read neon-cli stdout"))?;
            debug!("neon_cli STDOUT: {}", stdout)
        };
        debug!("neon_cli STDERR: {}", stderr);

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(stderr){
            if let serde_json::Value::Object(map) = json{

                if let serde_json::Value::String(result) = map.get("result")
                    .ok_or_else(|| ERR("get neon-cli json.result"))? {

                    if result == "success" {
                        let value = map.get("value").ok_or_else(|| ERR("get neon-cli json.value"))?;
                        parse_result(value.clone())
                    } else {
                        default.ok_or_else(|| {
                            debug!("neon_cli STDERR: {}", stderr);
                            ERR("neon-cli json.result != success")
                        })
                    }
                } else {
                    Err(ERR("cast neon-cli json.result->String"))
                }
            }else{
                Err(ERR("cast neon-cli json->{}"))
            }
        }
        else{
            Err(ERR("parse neon-cli json"))
        }
    }

    pub async fn emulate(
        &self,
        from: Option<Address>,
        to: Option<Address>,
        value: Option<U256>,
        data: Option<Vec<u8>>,
        slot: u64,
        tout: &Duration,
    ) -> Result<String> {
        let slot_ = slot.to_string();
        let from = from.unwrap_or_default().to_string();
        let to = to.map_or("deploy".to_string(), |v| v.to_string());
        let value = value.unwrap_or_default().to_string();

        let cmd = vec![
            "neon-cli",
            "--db_config", &self.db_config,
            "--slot", &slot_,
            "--evm_loader", &self.evm_loader,
            "emulate",
            "--token_mint", &self.token_mint,
            "--chain_id", &self.chain_id,
            "--max_steps_to_execute", &self.steps_to_execute,
            &from,
            &to,
            &value,
        ];
        let f = |value|-> Result<String> {
            if let serde_json::Value::Object(map) = value {
                if let serde_json::Value::String(result) = map.get("result").ok_or_else(|| ERR("get neon-cli json.value.result"))? {
                    Ok(format!("0x{}", result))
                } else {
                    Err(ERR("cast neon-cli json.value.result->String"))
                }
            } else {
                Err(ERR("cast neon-cli json.value->{}"))
            }
        };

        let transaction_params = TransactionParams { data: data.map(Into::into), trace_config: None };
        self.execute(cmd, Some(transaction_params), slot, tout, f, None).await
    }

    #[allow(unused)]
    pub async fn trace (
        &self,
        from: Option<Address>,
        to: Option<Address>,
        value: Option<U256>,
        data: Option<Vec<u8>>,
        gas_limit: Option<U256>,
        slot: u64,
        trace_config: Option<TraceCallConfig>,
        tout: &Duration,
    ) -> Result<TracedCall> {
        let slot_ = slot.to_string();
        let from = from.unwrap_or_default().to_string();
        let to = to.map_or("deploy".to_string(), |v| v.to_string());
        let value = value.unwrap_or_default().to_string();

        let mut cmd = vec![
            "neon-cli",
            "--db_config", &self.db_config,
            "--slot", &slot_,
            "--evm_loader", &self.evm_loader,
            "trace",
            "--token_mint", &self.token_mint,
            "--chain_id", &self.chain_id,
            "--max_steps_to_execute", &self.steps_to_execute,
        ];
        let gas;
        if let Some(value) = gas_limit {
            gas = value.to_string();
            cmd.extend(vec!["--gas_limit", &gas])
        }
        let a: Vec<&str> = vec![&from, &to, &value];
        cmd.extend(a);

        let f = |value|-> Result<TracedCall> {
            serde_json::from_value(value).map_err(|_| ERR("deserialize neon-cli json.value to TraceCall"))
        };

        let transaction_params = TransactionParams { data: data.map(Into::into), trace_config };
        self.execute(cmd, Some(transaction_params), slot, tout, f, None).await
    }

    #[allow(unused)]
    pub async fn trace_hash (
        &self,
        hash: U256,
        slot: u64,
        trace_config: Option<TraceConfig>,
        tout: &Duration,
    ) -> Result<TracedCall> {
        let hash = hash.to_be_bytes();
        let hash = format!("0x{}", hex::encode(hash));

        let mut cmd = vec![
            "neon-cli",
            "--db_config", &self.db_config,
            "--evm_loader", &self.evm_loader,
            "trace-hash",
            "--token_mint", &self.token_mint,
            "--chain_id", &self.chain_id,
            "--max_steps_to_execute", &self.steps_to_execute,
            &hash
        ];

        let f = |value|-> Result<TracedCall> {
            serde_json::from_value(value).map_err(|_| ERR("deserialize neon-cli json.value to TracedCall"))
        };

        let transaction_params = TransactionHashParams { trace_config };
        self.execute(cmd, Some(transaction_params), slot, tout, f, None).await
    }

    #[allow(unused)]
    pub async fn trace_block_by_slot (
        &self,
        slot: u64,
        trace_config: Option<TraceConfig>,
        tout: &Duration,
    ) -> Result<Vec<TracedCall>> {
        let slot_ = slot.to_string();
        let mut cmd = vec![
            "neon-cli",
            "--db_config", &self.db_config,
            "--slot", &slot_,
            "--evm_loader", &self.evm_loader,
            "trace-block-by-slot",
            "--token_mint", &self.token_mint,
            "--chain_id", &self.chain_id,
            "--max_steps_to_execute", &self.steps_to_execute,
        ];

        let f = |value|-> Result<Vec<TracedCall>> {
            serde_json::from_value(value).map_err(|_| ERR("deserialize neon-cli json.value to TracedCall"))
        };

        let trace_params = TraceBlockBySlotParams { trace_config };
        self.execute(cmd, Some(trace_params), slot, tout, f, None).await
    }

    pub async fn get_storage_at(&self, to: Address, index: U256, slot: u64, tout: &Duration) -> Result<U256> {
        let slot_ = slot.to_string();
        let to = to.to_string();
        let index = index.to_string();

        let cmd = vec![
            "neon-cli",
            "--db_config", &self.db_config,
            "--slot", &slot_,
            "--evm_loader", &self.evm_loader,
            "get-storage-at",
            &to,
            &index
        ];
        let f = |value| -> Result<U256> {
            let value :String = serde_json::from_value(value).map_err(|_| ERR("cast neon-cli json.value->String"))?;
            U256::from_str_hex(&format!("0x{}", &value) ).map_err(|_| ERR("cast neon-cli json.value->U256"))
        };

        self.execute(cmd, None::<()>, slot, tout, f, Some(U256::default())).await
    }

    pub async fn get_balance(&self, address: Address, slot: u64, tout: &Duration) -> Result<U256> {
        let f = |value|-> Result<U256> {
            if let serde_json::Value::Object(map) = value {
                if let serde_json::Value::String(balance) = map.get("balance").ok_or_else(||  ERR("get neon-cli json.value.balance"))? {
                    U256::from_str_prefixed(balance ).map_err(|_| ERR("cast neon-cli json.value.balance->U256"))
                } else {
                    Err(ERR("cast neon-cli json.value.balance->String"))
                }
            } else {
                Err(ERR("cast neon-cli json.value->{}"))
            }
        };

        self.get_ether_account_data(address, slot, tout, f).await
    }

    pub async fn get_trx_count(&self, address: Address, slot: u64, tout: &Duration) -> Result<U256> {
        let f = |value|-> Result<U256> {
            if let serde_json::Value::Object(map) = value {
                if let serde_json::Value::Number(trx_count) = map.get("trx_count").ok_or_else(|| ERR("get neon-cli json.value.trx_count"))? {
                    let trx_count = trx_count.as_u64().ok_or_else(|| ERR("cast neon-cli json.value.trx_count->u64"))?;
                    Ok(U256::new(trx_count.into()))
                } else {
                    Err(ERR("cast neon-cli json.value.trx_count->Number"))
                }
            } else {
                Err(ERR("cast neon-cli json.value->{}"))
            }
        };

        self.get_ether_account_data(address, slot, tout, f).await
    }

    pub async fn get_code(&self, address: Address, slot: u64, tout: &Duration) -> Result<String> {
        let f = |value|-> Result<String> {
            if let serde_json::Value::Object(map) = value {
                if let serde_json::Value::String(code) = map.get("code").ok_or_else(||  ERR("get neon-cli json.value.code"))? {
                    Ok(code.clone())
                } else {
                    Err(ERR("cast neon-cli json.value.code->String"))
                }
            } else {
                Err(ERR("cast neon-cli json.value->{}"))
            }
        };

        self.get_ether_account_data(address, slot, tout, f).await.map(|code| format!("0x{}", &code))
    }

    async fn get_ether_account_data <F, D>(&self, address: Address, slot: u64, tout: &Duration, f: F) -> Result<D>
        where F: FnOnce(serde_json::Value) -> Result<D>,
              D: std::default::Default
    {
        let slot_ = slot.to_string();
        let address = address.to_string();

        let cmd = vec![
            "neon-cli",
            "--db_config", &self.db_config,
            "--slot", &slot_,
            "--evm_loader", &self.evm_loader,
            "get-ether-account-data",
            &address,
        ];

        self.execute(cmd, None::<()>, slot, tout, f, Some(Default::default())).await
    }
}

