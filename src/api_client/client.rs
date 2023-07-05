use std::{sync::Arc, time::Instant};

use crate::api_client::{config::Config, models::{NeonApiResponse, NeonApiError}, Result};
use ethnum::U256;
use evm_loader::types::Address;
use log::debug;

use neon_cli_lib::types::{
    request_models::{
        EmulateHashRequestModel, EmulateRequestModel, EmulationParamsRequestModel, GetEtherRequest,
        GetStorageAtRequest, TraceHashRequestModel, TraceRequestModel, TxParamsRequestModel,
        TraceNextBlockRequestModel,
    },
    trace::{TraceCallConfig, TraceConfig},
};

use reqwest::{
    header::{ACCEPT, CONTENT_TYPE},
    Client as ReqwestClient, Response,
};
use serde::Serialize;
use solana_sdk::pubkey::Pubkey;

use super::errors::NeonAPIClientError;

#[derive(Clone)]
pub struct Client {
    config: Arc<Config>,
    pub neon_api_url: String,
    pub http_client: Arc<ReqwestClient>,
}

impl Client {
    /// Creates a new [`NeonAPIClient`].
    pub fn new(config: Arc<Config>, neon_api_url: impl Into<String>) -> Client {
        Client {
            config,
            neon_api_url: neon_api_url.into(),
            http_client: Arc::new(ReqwestClient::new()),
        }
    }

    async fn get_request<T: Serialize + Sized + std::fmt::Debug>(
        &self,
        uri: &str,
        query: T,
        id: u16,
    ) -> Result<NeonApiResponse> {
        let full_url = format!("{0}{1}", self.neon_api_url, uri);
        info!("id {:?}: get_request: {:?}, parameters: {:?}", id, full_url, query);

        let start = Instant::now();
        let response = self
            .http_client
            .clone()
            .get(full_url.clone())
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json")
            .query(&query)
            .send()
            .await?;

        self.process_response(&full_url, response, &start).await
    }

    async fn post_request<T: Serialize + Sized + std::fmt::Debug>(
        &self,
        uri: &str,
        req_body: T,
        id: u16,
    ) -> Result<NeonApiResponse> {
        let full_url = format!("{0}{1}", self.neon_api_url, uri);
        info!("id {:?}: post_request: {:?}, parameters: {:?}", id, full_url, req_body);

        let start = Instant::now();
        let response = self
            .http_client
            .clone()
            .post(full_url.clone())
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json")
            .json(&req_body)
            .send()
            .await?;

        self.process_response(&full_url, response, &start).await
    }

    async fn process_response(&self, full_url: &str, response: Response, start: &Instant) -> Result<NeonApiResponse> {
        let duration = start.elapsed();
        let response_status = response.status();
        let response_str = response.text().await?;
        debug!("Raw response for request {full_url}: {response_str}");

        let processed_response = match response_status {
            reqwest::StatusCode::OK |
            reqwest::StatusCode::BAD_REQUEST |
            reqwest::StatusCode::INTERNAL_SERVER_ERROR => {
                // Try to parse our JSON to an NeonApiResponse
                match serde_json::from_str::<NeonApiResponse>(&response_str) {
                    Ok(body) => Ok(body),
                    Err(e) => Err(NeonAPIClientError::ParseResponseError(e.to_string(), response_str))
                }
            }
            other => Err(NeonAPIClientError::OtherResponseStatusError(other)),
        };

        debug!(
            "id {:?}: Processed response for request {} (duration {} ms): {:?}",
            id,
            &full_url,
            &duration.as_millis().to_string(),
            &processed_response,
        );

        processed_response
    }

    pub async fn get_ether_account_data(
        &self,
        address: Address,
        slot: Option<u64>,
        id: u16,
    ) -> Result<NeonApiResponse> {
        let params = GetEtherRequest {
            ether: address,
            slot,
        };

        self.get_request("/api/get-ether-account-data", params, id)
            .await
    }

    pub async fn get_storage_at(
        &self,
        address: Address,
        index: U256,
        slot: Option<u64>,
        id: u16,
    ) -> Result<NeonApiResponse> {
        let params = GetStorageAtRequest {
            contract_id: address,
            index,
            slot,
        };

        self.get_request("/api/get-storage-at", params, id)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn emulate(
        &self,
        sender: Address,
        contract: Option<Address>,
        data: Option<Vec<u8>>,
        value: Option<U256>,
        gas_limit: Option<U256>,
        max_steps_to_execute: u64,
        cached_accounts: Option<Vec<Address>>,
        solana_accounts: Option<Vec<Pubkey>>,
        slot: Option<u64>,
        id: u16,
    ) -> Result<NeonApiResponse> {
        let tx_params = TxParamsRequestModel {
            sender,
            contract,
            data,
            value,
            gas_limit,
        };

        let emulation_params = EmulationParamsRequestModel::new(
            Some(self.config.token_mint),
            Some(self.config.chain_id),
            max_steps_to_execute,
            cached_accounts,
            solana_accounts,
        );

        let params = EmulateRequestModel {
            tx_params,
            emulation_params,
            slot,
        };

        self.post_request("/api/emulate", params, id)
            .await
    }

    #[allow(unused)]
    pub async fn emulate_hash(
        &self,
        gas_limit: Option<U256>,
        max_steps_to_execute: u64,
        cached_accounts: Option<Vec<Address>>,
        solana_accounts: Option<Vec<Pubkey>>,
        hash: String,
        id: u16,
    ) -> Result<NeonApiResponse> {
        let emulation_params = EmulationParamsRequestModel::new(
            Some(self.config.token_mint),
            Some(self.config.chain_id),
            max_steps_to_execute,
            cached_accounts,
            solana_accounts,
        );

        let params = EmulateHashRequestModel {
            emulation_params,
            hash,
        };

        self.post_request("/api/emulate-hash", params, id)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn trace(
        &self,
        sender: Address,
        contract: Option<Address>,
        data: Option<Vec<u8>>,
        value: Option<U256>,
        gas_limit: Option<U256>,
        max_steps_to_execute: u64,
        cached_accounts: Option<Vec<Address>>,
        solana_accounts: Option<Vec<Pubkey>>,
        slot: Option<u64>,
        trace_call_config: Option<TraceCallConfig>,
        id: u16,
    ) -> Result<NeonApiResponse> {
        let tx_params = TxParamsRequestModel {
            sender,
            contract,
            data,
            value,
            gas_limit,
        };

        let emulation_params = EmulationParamsRequestModel::new(
            Some(self.config.token_mint),
            Some(self.config.chain_id),
            max_steps_to_execute,
            cached_accounts,
            solana_accounts,
        );

        let emulate_request = EmulateRequestModel {
            tx_params,
            emulation_params,
            slot,
        };

        let params = TraceRequestModel {
            emulate_request,
            trace_call_config,
        };

        self.post_request("/api/trace", params, id)
            .await
    }

    pub async fn trace_hash(
        &self,
        max_steps_to_execute: u64,
        cached_accounts: Option<Vec<Address>>,
        solana_accounts: Option<Vec<Pubkey>>,
        hash: String,
        trace_config: Option<TraceConfig>,
        id: u16,
    ) -> Result<NeonApiResponse> {
        let emulation_params = EmulationParamsRequestModel::new(
            Some(self.config.token_mint),
            Some(self.config.chain_id),
            max_steps_to_execute,
            cached_accounts,
            solana_accounts,
        );

        let emulate_hash_request = EmulateHashRequestModel {
            emulation_params,
            hash,
        };

        let params = TraceHashRequestModel {
            emulate_hash_request,
            trace_config,
        };

        self.post_request("/api/trace-hash", params)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn trace_next_block(
        &self,
        max_steps_to_execute: u64,
        cached_accounts: Option<Vec<Address>>,
        solana_accounts: Option<Vec<Pubkey>>,
        slot: u64,
        trace_config: Option<TraceConfig>,
        id: u16,
    ) -> Result<NeonApiResponse> {
        let emulation_params = EmulationParamsRequestModel::new(
            Some(self.config.token_mint),
            Some(self.config.chain_id),
            max_steps_to_execute,
            cached_accounts,
            solana_accounts,
        );

        let params = TraceNextBlockRequestModel {
            emulation_params,
            slot,
            trace_config,
        };

        self.post_request("/api/trace-next-block", params, id)
            .await
    }
}
