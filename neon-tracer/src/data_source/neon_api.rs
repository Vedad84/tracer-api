use std::{sync::Arc, time::Duration};

use crate::api_client::{client::Client, config::Config};
use crate::service::{Error, Result};
use ethnum::U256;
use evm_loader::evm::tracing::event_listener::trace::{TraceCallConfig, TraceConfig, TracedCall};
use jsonrpsee::types::error::ErrorCode;
use neon_cli_lib::{
    commands::{get_storage_at::GetStorageAtReturn, trace::TraceBlockReturn},
    types::Address,
};

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
        id: u64,
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
                id,
            )
            .await
            .map_err(|e| {
                Error::owned(ErrorCode::InternalError.code(), e.to_string(), None::<()>)
            })?;

        Ok(format!(
            "0x{}",
            hex::encode(response.emulation_result.result)
        ))
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
        id: u64,
    ) -> Result<TracedCall> {
        self.api_client
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
                id,
            )
            .await
            .map_err(|e| Error::owned(ErrorCode::InternalError.code(), e.to_string(), None::<()>))
    }

    #[allow(unused)]
    pub async fn trace_hash(
        &self,
        hash: U256,
        slot: u64,
        trace_config: Option<TraceConfig>,
        tout: &Duration,
        id: u64,
    ) -> Result<TracedCall> {
        let hash = hash.to_be_bytes();
        let hash = format!("0x{}", hex::encode(hash));

        self.api_client
            .clone()
            .trace_hash(self.steps_to_execute, None, None, hash, trace_config, id)
            .await
            .map_err(|e| Error::owned(ErrorCode::InternalError.code(), e.to_string(), None::<()>))
    }

    #[allow(unused)]
    pub async fn trace_next_block(
        &self,
        slot: u64,
        trace_config: Option<TraceConfig>,
        tout: &Duration,
        id: u64,
    ) -> Result<TraceBlockReturn> {
        self.api_client
            .clone()
            .trace_next_block(self.steps_to_execute, None, None, slot, trace_config, id)
            .await
            .map_err(|e| Error::owned(ErrorCode::InternalError.code(), e.to_string(), None::<()>))
    }

    #[allow(unused)]
    pub async fn get_storage_at(
        &self,
        to: Address,
        index: U256,
        slot: u64,
        tout: &Duration,
        id: u64,
    ) -> Result<U256> {
        self.api_client
            .clone()
            .get_storage_at(to, index, Some(slot), id)
            .await
            .map(|GetStorageAtReturn(arr)| U256::from_be_bytes(arr))
            .map_err(|e| Error::owned(ErrorCode::InternalError.code(), e.to_string(), None::<()>))
    }

    #[allow(unused)]
    pub async fn get_balance(
        &self,
        address: Address,
        slot: u64,
        tout: &Duration,
        id: u64,
    ) -> Result<U256> {
        let response = self
            .api_client
            .clone()
            .get_ether_account_data(address, Some(slot), id)
            .await;

        if response.is_err() {
            Ok(U256::default())
        } else {
            U256::from_str_prefixed(&response.unwrap().balance)
                .map_err(|_| ERR("cast GetEtherAccountDataReturn.balance->U256", id))
        }
    }

    #[allow(unused)]
    pub async fn get_trx_count(
        &self,
        address: Address,
        slot: u64,
        tout: &Duration,
        id: u64,
    ) -> Result<U256> {
        let response = self
            .api_client
            .clone()
            .get_ether_account_data(address, Some(slot), id)
            .await;

        if response.is_err() {
            Ok(U256::default())
        } else {
            Ok(U256::new(response.unwrap().trx_count.into()))
        }
    }

    #[allow(unused)]
    pub async fn get_code(
        &self,
        address: Address,
        slot: u64,
        tout: &Duration,
        id: u64,
    ) -> Result<String> {
        let response = self
            .api_client
            .clone()
            .get_ether_account_data(address, Some(slot), id)
            .await;

        if response.is_err() {
            Ok(String::default())
        } else {
            Ok(response.unwrap().code)
        }
    }
}
