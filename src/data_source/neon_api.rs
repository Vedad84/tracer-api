use std::{sync::Arc, time::Duration};

use crate::api_client::{client::Client, config::Config};
use crate::service::Result;
use ethnum::U256;
use evm_loader::types::Address;
use log::debug;
use neon_cli_lib::types::trace::{TraceCallConfig, TraceConfig, TracedCall};

use super::ERR;

const NUM_STEPS_TO_EXECUTE: u64 = 500_000;

#[derive(Clone)]
pub struct NeonAPIDataSource {
    pub config: Arc<Config>,
    pub api_client: Arc<Client>,
    pub steps_to_execute: u64,
}

impl NeonAPIDataSource {
    pub fn new(config: Arc<Config>, client: Client) -> Self {
        NeonAPIDataSource {
            config,
            api_client: Arc::new(client),
            steps_to_execute: NUM_STEPS_TO_EXECUTE,
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(unused)]
    pub async fn emulate(
        &self,
        from: Option<Address>,
        to: Option<Address>,
        value: Option<U256>,
        data: Option<Vec<u8>>,
        slot: u64,
        tout: &Duration,
        id: u16,
    ) -> Result<String> {
        let sender = from.unwrap_or_default();
        let contract = to;
        let slot = Some(slot);
        let token_mint = Some(self.config.clone().token_mint);
        let chain_id = Some(self.config.clone().chain_id);
        let max_steps_to_execute = self.steps_to_execute;
        let gas_limit = None;
        let cached_accounts = None;
        let solana_accounts = None;

        let response = self
            .api_client
            .clone()
            .emulate(
                sender,
                contract,
                data,
                value,
                gas_limit,
                max_steps_to_execute,
                cached_accounts,
                solana_accounts,
                slot,
            )
            .await
            .map_err(|e| jsonrpsee::types::error::Error::Custom(e.to_string()))?;

        if response.result != "success" {
            debug!("id {:?}: neon_api ERR: {}", id, response.value);
            return Err(ERR("result != success", id));
        }

        let value = serde_json::from_str(&response.value)?;

        if let serde_json::Value::Object(map) = value {
            if let serde_json::Value::String(result) = map
                .get("result")
                .ok_or_else(|| ERR("get neon-cli json.value.result", id))?
            {
                Ok(format!("0x{result}"))
            } else {
                Err(ERR("cast neon-cli json.value.result->String", id))
            }
        } else {
            Err(ERR("cast neon-cli json.value->{}", id))
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(unused)]
    pub async fn trace(
        &self,
        from: Option<Address>,
        to: Option<Address>,
        value: Option<U256>,
        data: Option<Vec<u8>>,
        gas_limit: Option<U256>,
        slot: u64,
        trace_call_config: Option<TraceCallConfig>,
        tout: &Duration,
        id: u16,
    ) -> Result<TracedCall> {
        let response = self
            .api_client
            .clone()
            .trace(
                from.unwrap_or_default(),
                to,
                data,
                value,
                gas_limit,
                self.steps_to_execute,
                None,
                None,
                Some(slot),
                trace_call_config,
            )
            .await
            .map_err(|e| jsonrpsee::types::error::Error::Custom(e.to_string()))?;

        if response.result != "success" {
            debug!("id {:?}: neon_api ERR: {}", id, response.value);
            return Err(ERR("result != success", id));
        }

        serde_json::from_str(&response.value)
            .map_err(|_| ERR("deserialize neon-cli json.value to TracedCall", id))
    }

    #[allow(unused)]
    pub async fn trace_hash(
        &self,
        hash: U256,
        slot: u64,
        trace_config: Option<TraceConfig>,
        tout: &Duration,
        id: u16,
    ) -> Result<TracedCall> {
        let hash = hash.to_be_bytes();
        let hash = format!("0x{}", hex::encode(hash));

        let response = self
            .api_client
            .clone()
            .trace_hash(
                self.steps_to_execute,
                None,
                None,
                hash,
                trace_config,
            )
            .await
            .map_err(|e| jsonrpsee::types::error::Error::Custom(e.to_string()))?;

        if response.result != "success" {
            debug!("id {:?}: neon_api ERR: {}", id, response.value);
            return Err(ERR("result != success", id));
        }

        serde_json::from_str(&response.value)
            .map_err(|_| ERR("deserialize neon-cli json.value to TracedCall", id))
    }

    #[allow(unused)]
    pub async fn trace_next_block(
        &self,
        slot: u64,
        trace_config: Option<TraceConfig>,
        tout: &Duration,
        id: u16,
    ) -> Result<Vec<TracedCall>> {
        let response = self
            .api_client
            .clone()
            .trace_next_block(
                self.steps_to_execute,
                None,
                None,
                slot,
                trace_config,
            )
            .await
            .map_err(|e| jsonrpsee::types::error::Error::Custom(e.to_string()))?;

        if response.result != "success" {
            debug!("id {:?}: neon_api ERR: {}", id, response.value);
            return Err(ERR("result != success", id));
        }

        serde_json::from_str(&response.value)
            .map_err(|_| ERR("deserialize neon-cli json.value to TracedCall", id))
    }

    #[allow(unused)]
    pub async fn get_storage_at(
        &self,
        to: Address,
        index: U256,
        slot: u64,
        tout: &Duration,
        id: u16,
    ) -> Result<U256> {
        let response = self
            .api_client
            .clone()
            .get_storage_at(to, index, Some(slot))
            .await
            .map_err(|e| jsonrpsee::types::error::Error::Custom(e.to_string()))?;

        if response.result != "success" {
            debug!("id {:?}: neon_api ERR: {}", id, response.value);
            return Ok(U256::default());
        }

        let value = serde_json::from_str(&response.value)?;

        let value: String = serde_json::from_value(value)
            .map_err(|_| ERR("cast neon-cli json.value->String", id))?;
        U256::from_str_hex(&format!("0x{}", &value))
            .map_err(|_| ERR("cast neon-cli json.value->U256", id))
    }

    #[allow(unused)]
    async fn get_ether_account_data<F, D>(
        &self,
        address: Address,
        slot: u64,
        tout: &Duration,
        f: F,
        id: u16,
    ) -> Result<D>
    where
        F: FnOnce(serde_json::Value) -> Result<D>,
        D: std::default::Default,
    {
        let response = self
            .api_client
            .clone()
            .get_ether_account_data(address, Some(slot))
            .await
            .map_err(|e| jsonrpsee::types::error::Error::Custom(e.to_string()))?;

        if response.result != "success" {
            debug!("id {:?}: neon_api ERR: {}", id, response.value);
            return Ok(Default::default());
        }

        let value = serde_json::from_str(&response.value)?;

        f(value)
    }

    #[allow(unused)]
    pub async fn get_balance(
        &self,
        address: Address,
        slot: u64,
        tout: &Duration,
        id: u16,
    ) -> Result<U256> {
        let f = |value| -> Result<U256> {
            if let serde_json::Value::Object(map) = value {
                if let serde_json::Value::String(balance) = map
                    .get("balance")
                    .ok_or_else(|| ERR("get neon-cli json.value.balance", id))?
                {
                    U256::from_str_prefixed(balance)
                        .map_err(|_| ERR("cast neon-cli json.value.balance->U256", id))
                } else {
                    Err(ERR("cast neon-cli json.value.balance->String", id))
                }
            } else {
                Err(ERR("cast neon-cli json.value->{}", id))
            }
        };

        self.get_ether_account_data(address, slot, tout, f, id)
            .await
    }

    #[allow(unused)]
    pub async fn get_trx_count(
        &self,
        address: Address,
        slot: u64,
        tout: &Duration,
        id: u16,
    ) -> Result<U256> {
        let f = |value| -> Result<U256> {
            if let serde_json::Value::Object(map) = value {
                if let serde_json::Value::Number(trx_count) = map
                    .get("trx_count")
                    .ok_or_else(|| ERR("get neon-cli json.value.trx_count", id))?
                {
                    let trx_count = trx_count
                        .as_u64()
                        .ok_or_else(|| ERR("cast neon-cli json.value.trx_count->u64", id))?;
                    Ok(U256::new(trx_count.into()))
                } else {
                    Err(ERR("cast neon-cli json.value.trx_count->Number", id))
                }
            } else {
                Err(ERR("cast neon-cli json.value->{}", id))
            }
        };

        self.get_ether_account_data(address, slot, tout, f, id)
            .await
    }

    #[allow(unused)]
    pub async fn get_code(
        &self,
        address: Address,
        slot: u64,
        tout: &Duration,
        id: u16,
    ) -> Result<String> {
        let f = |value| -> Result<String> {
            if let serde_json::Value::Object(map) = value {
                if let serde_json::Value::String(code) = map
                    .get("code")
                    .ok_or_else(|| ERR("get neon-cli json.value.code", id))?
                {
                    Ok(code.clone())
                } else {
                    Err(ERR("cast neon-cli json.value.code->String", id))
                }
            } else {
                Err(ERR("cast neon-cli json.value->{}", id))
            }
        };

        self.get_ether_account_data(address, slot, tout, f, id)
            .await
            .map(|code| format!("0x{}", &code))
    }
}
