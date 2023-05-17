use std::{sync::Arc, time::Instant};

use crate::api_client::Result;
use ethnum::U256;
use evm_loader::types::Address;
use log::debug;
use reqwest::{
    header::{ACCEPT, CONTENT_TYPE},
    Client as ReqwestClient, Response,
};
use serde::Serialize;
use solana_sdk::pubkey::Pubkey;

use super::{
    errors::NeonAPIClientError,
    models::{GetEtherAccountDataRequest, GetStorageAtRequest, NeonApiResponse, TxParamsRequest},
};

#[derive(Clone)]
pub struct Client {
    pub neon_api_url: String,
    pub http_client: Arc<ReqwestClient>,
}

impl Client {
    /// Creates a new [`NeonAPIClient`].
    pub fn new(neon_api_url: impl Into<String>) -> Client {
        Client {
            neon_api_url: neon_api_url.into(),
            http_client: Arc::new(ReqwestClient::new()),
        }
    }

    async fn get_request<T: Serialize + Sized + std::fmt::Debug>(
        &self,
        uri: &str,
        query: T,
    ) -> Result<NeonApiResponse> {
        let full_url = format!("{0}{1}", self.neon_api_url, uri);
        debug!("get_request: {:?}, parameters: {:?}", full_url, query);

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
        let duration = start.elapsed();

        let processed_response = self.process_response(response).await;

        if processed_response.is_ok() {
            debug!(
                "Response for request {} (duration {} ms): {:?}",
                &full_url,
                &duration.as_millis().to_string(),
                &processed_response,
            );
        } else {
            debug!(
                "Error response for request {} (duration {} ms): {:?}",
                &full_url,
                &duration.as_millis().to_string(),
                &processed_response
            );
        }

        processed_response
    }

    async fn post_request<T: Serialize + Sized + std::fmt::Debug>(
        &self,
        uri: &str,
        req_body: T,
    ) -> Result<NeonApiResponse> {
        let full_url = format!("{0}{1}", self.neon_api_url, uri);
        debug!("post_request: {:?}, parameters: {:?}", full_url, req_body);

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
        let duration = start.elapsed();

        let processed_response = self.process_response(response).await;

        if processed_response.is_ok() {
            debug!(
                "Response for request {} (duration {} ms): {:?}",
                &full_url,
                &duration.as_millis().to_string(),
                &processed_response,
            );
        } else {
            debug!(
                "Error response for request {} (duration {} ms): {:?}",
                &full_url,
                &duration.as_millis().to_string(),
                &processed_response
            );
        }

        processed_response
    }

    async fn process_response(&self, response: Response) -> Result<NeonApiResponse> {
        let body = match response.status() {
            reqwest::StatusCode::OK => {
                // on success, parse our JSON to an NeonApiResponse
                match response.json::<NeonApiResponse>().await {
                    Ok(body) => body,
                    Err(e) => {
                        return Err(NeonAPIClientError::ParseResponseError(e.to_string()));
                    }
                }
            }
            reqwest::StatusCode::BAD_REQUEST => {
                // on parameters error, parse our JSON to an NeonApiResponse
                match response.json::<NeonApiResponse>().await {
                    Ok(body) => body,
                    Err(e) => return Err(NeonAPIClientError::ParseResponseError(e.to_string())),
                }
            }
            reqwest::StatusCode::INTERNAL_SERVER_ERROR => {
                // on error, parse our JSON to an NeonApiResponse
                match response.json::<NeonApiResponse>().await {
                    Ok(body) => body,
                    Err(e) => return Err(NeonAPIClientError::ParseResponseError(e.to_string())),
                }
            }
            other => return Err(NeonAPIClientError::OtherResponseStatusError(other)),
        };

        Ok(body)
    }

    pub async fn get_ether_account_data(
        &self,
        address: Address,
        slot: Option<u64>,
    ) -> Result<NeonApiResponse> {
        let params = GetEtherAccountDataRequest {
            ether: address,
            slot,
        };

        self.get_request::<GetEtherAccountDataRequest>("/api/get-ether-account-data", params)
            .await
    }

    pub async fn get_storage_at(
        &self,
        address: Address,
        index: Option<U256>,
        slot: Option<u64>,
    ) -> Result<NeonApiResponse> {
        let params = GetStorageAtRequest {
            contract_id: address,
            index,
            slot,
        };

        self.get_request::<GetStorageAtRequest>("/api/get-storage-at", params)
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
        token_mint: Option<Pubkey>,
        chain_id: Option<u64>,
        max_steps_to_execute: Option<u64>,
        cached_accounts: Option<Vec<Address>>,
        solana_accounts: Option<Vec<Pubkey>>,
        slot: Option<u64>,
    ) -> Result<NeonApiResponse> {
        let value = value.map(|v| v.to_string());
        let gas_limit = gas_limit.map(|v| v.to_string());
        let token_mint = token_mint.map(|v| v.to_string());
        let solana_accounts =
            solana_accounts.map(|vec| vec.into_iter().map(|v| v.to_string()).collect());

        let params = TxParamsRequest {
            sender,
            contract,
            data,
            value,
            gas_limit,
            token_mint,
            chain_id,
            max_steps_to_execute,
            cached_accounts,
            solana_accounts,
            slot,
            hash: None,
        };

        self.post_request::<TxParamsRequest>("/api/emulate", params)
            .await
    }

    #[allow(unused)]
    #[allow(clippy::too_many_arguments)]
    pub async fn emulate_hash(
        &self,
        sender: Address,
        contract: Option<Address>,
        data: Option<Vec<u8>>,
        value: Option<U256>,
        gas_limit: Option<U256>,
        token_mint: Option<Pubkey>,
        chain_id: Option<u64>,
        max_steps_to_execute: Option<u64>,
        cached_accounts: Option<Vec<Address>>,
        solana_accounts: Option<Vec<Pubkey>>,
        slot: Option<u64>,
        hash: String,
    ) -> Result<NeonApiResponse> {
        let value = value.map(|v| v.to_string());
        let gas_limit = gas_limit.map(|v| v.to_string());
        let token_mint = token_mint.map(|v| v.to_string());
        let solana_accounts =
            solana_accounts.map(|vec| vec.into_iter().map(|v| v.to_string()).collect());

        let params = TxParamsRequest {
            sender,
            contract,
            data,
            value,
            gas_limit,
            token_mint,
            chain_id,
            max_steps_to_execute,
            cached_accounts,
            solana_accounts,
            slot,
            hash: Some(hash),
        };

        self.post_request::<TxParamsRequest>("/api/emulate_hash", params)
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
        token_mint: Option<Pubkey>,
        chain_id: Option<u64>,
        max_steps_to_execute: Option<u64>,
        cached_accounts: Option<Vec<Address>>,
        solana_accounts: Option<Vec<Pubkey>>,
        slot: Option<u64>,
    ) -> Result<NeonApiResponse> {
        let value = value.map(|v| v.to_string());
        let gas_limit = gas_limit.map(|v| v.to_string());
        let token_mint = token_mint.map(|v| v.to_string());
        let solana_accounts =
            solana_accounts.map(|vec| vec.into_iter().map(|v| v.to_string()).collect());

        let params = TxParamsRequest {
            sender,
            contract,
            data,
            value,
            gas_limit,
            token_mint,
            chain_id,
            max_steps_to_execute,
            cached_accounts,
            solana_accounts,
            slot,
            hash: None,
        };

        self.post_request::<TxParamsRequest>("/api/trace", params)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn trace_hash(
        &self,
        sender: Address,
        contract: Option<Address>,
        data: Option<Vec<u8>>,
        value: Option<U256>,
        gas_limit: Option<U256>,
        token_mint: Option<Pubkey>,
        chain_id: Option<u64>,
        max_steps_to_execute: Option<u64>,
        cached_accounts: Option<Vec<Address>>,
        solana_accounts: Option<Vec<Pubkey>>,
        slot: Option<u64>,
        hash: String,
    ) -> Result<NeonApiResponse> {
        let value = value.map(|v| v.to_string());
        let gas_limit = gas_limit.map(|v| v.to_string());
        let token_mint = token_mint.map(|v| v.to_string());
        let solana_accounts =
            solana_accounts.map(|vec| vec.into_iter().map(|v| v.to_string()).collect());

        let params = TxParamsRequest {
            sender,
            contract,
            data,
            value,
            gas_limit,
            token_mint,
            chain_id,
            max_steps_to_execute,
            cached_accounts,
            solana_accounts,
            slot,
            hash: Some(hash),
        };

        self.post_request::<TxParamsRequest>("/api/trace_hash", params)
            .await
    }
}
