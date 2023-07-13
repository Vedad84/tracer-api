use std::{sync::Arc, time::{Instant, Duration}};

use crate::api_client::{config::Config, models::{NeonApiResponse, NeonApiError}, Result};
use ethnum::U256;
use log::{info, warn};
use reqwest::{
    header::{ACCEPT, CONTENT_TYPE},
    Client as ReqwestClient, Response,
};
use serde::{Serialize, de::DeserializeOwned};
use solana_sdk::pubkey::Pubkey;

use super::errors::NeonAPIClientError;
use neon_cli_lib::{
    commands::{
        get_storage_at::GetStorageAtReturn,
        get_ether_account_data::GetEtherAccountDataReturn,
        emulate::EmulateReturn,
        // trace::TraceReturn,
    },
    types::{
        request_models::{
            EmulateHashRequestModel, EmulateRequestModel, EmulationParamsRequestModel, GetEtherRequest,
            GetStorageAtRequest, /*TraceHashRequestModel, TraceRequestModel,*/ TxParamsRequestModel,
        },
        Address,
    }
};

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

    async fn post_request<T, R>(
        &self,
        uri: &str,
        req_body: T,
        id: u64,
    ) -> Result<R>
    where
        T: Serialize + Sized + std::fmt::Debug,
        R: DeserializeOwned + std::fmt::Display,
    {
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
        let duration = start.elapsed();

        self.process_response(response, &duration, &full_url, id).await
    }

    async fn get_request<T, R> (
        &self,
        uri: &str,
        query: T,
        id: u64,
    ) -> Result<R>
    where
        T: Serialize + Sized + std::fmt::Debug,
        R: DeserializeOwned + std::fmt::Display,
    {
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
        let duration = start.elapsed();
        self.process_response(response, &duration, &full_url, id).await
    }

    async fn process_response<T>(&self, response: Response, duration: &Duration, full_url: &String, id: u64) -> Result<T>
    where
        T: DeserializeOwned + std::fmt::Display,
    {

        info!(
            "id {:?}: found response for request {} (duration {} ms)",
            id,
            full_url,
            duration.as_millis().to_string(),
        );
        let status = response.status();
        let response_str = response.text().await?;

        match status {
            reqwest::StatusCode::OK => {
                match serde_json::from_str::<NeonApiResponse<T>>(&response_str) {
                    Ok(response) => {
                        if response.result != "success" {
                            warn!("id {:?}: NeonApiResponse.result != success", id);
                            Err(NeonAPIClientError::NeonApiError(format!("result != success")))
                        } else {
                            info!("id {:?}: NeonApiResponse.value: {}", id, response.value);
                            Ok(response.value)
                        }
                    },
                    Err(e) => {
                        warn!("id {:?}: error to deserialize response.json to NeonApiResponse: {:?}", id, e.to_string());
                        Err(NeonAPIClientError::ParseResponseError(e.to_string()))
                    }
                }
            },
            reqwest::StatusCode::BAD_REQUEST | reqwest::StatusCode::INTERNAL_SERVER_ERROR => {
                warn!("id {:?}: neon-api response.status() is BAD_REQUEST or INTERNAL_SERVER_ERROR", id);
                match serde_json::from_str::<NeonApiError>(&response_str) {
                    Ok(body) => {
                        warn!("id {:?}: response.json: {:?}", id, body);
                        Err(NeonAPIClientError::NeonApiError(serde_json::json!(body).to_string()))
                    },
                    Err(e) => {
                        warn!("id {:?}: error to deserialize response.json to NeonApiError: {:?}", id, e.to_string());
                        Err(NeonAPIClientError::ParseResponseError(e.to_string()))
                    },
                }
            },
            other => {
                warn!("id {:?}: unknown neon-api response.status(): {:?}", id, status
                );
                Err(NeonAPIClientError::OtherResponseStatusError(other))
            },
        }
    }

    pub async fn get_ether_account_data(
        &self,
        address: Address,
        slot: Option<u64>,
        id: u64,
    ) -> Result<GetEtherAccountDataReturn> {
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
        id: u64,
    ) -> Result<GetStorageAtReturn> {
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
        id: u64,
    ) -> Result<EmulateReturn> {
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
        id: u64,
    ) -> Result<EmulateReturn> {
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

        self.post_request("/api/emulate_hash", params, id)
            .await
    }

    // #[allow(clippy::too_many_arguments)]
    // pub async fn trace(
    //     &self,
    //     sender: Address,
    //     contract: Option<Address>,
    //     data: Option<Vec<u8>>,
    //     value: Option<U256>,
    //     gas_limit: Option<U256>,
    //     max_steps_to_execute: u64,
    //     cached_accounts: Option<Vec<Address>>,
    //     solana_accounts: Option<Vec<Pubkey>>,
    //     slot: Option<u64>,
    //     id: u64,
    // ) -> Result<TraceReturn> {
    //     let tx_params = TxParamsRequestModel {
    //         sender,
    //         contract,
    //         data,
    //         value,
    //         gas_limit,
    //     };
    //
    //     let emulation_params = EmulationParamsRequestModel::new(
    //         Some(self.config.token_mint),
    //         Some(self.config.chain_id),
    //         max_steps_to_execute,
    //         cached_accounts,
    //         solana_accounts,
    //     );
    //
    //     let emulate_request = EmulateRequestModel {
    //         tx_params,
    //         emulation_params,
    //         slot,
    //     };
    //
    //     let params = TraceRequestModel {
    //         emulate_request,
    //     };
    //
    //     self.post_request("/api/trace", params, id)
    //         .await
    // }

    // #[allow(clippy::too_many_arguments)]
    // pub async fn trace_hash(
    //     &self,
    //     max_steps_to_execute: u64,
    //     cached_accounts: Option<Vec<Address>>,
    //     solana_accounts: Option<Vec<Pubkey>>,
    //     hash: String,
    //     id: u64,
    // ) -> Result<TraceReturn> {
    //     let emulation_params = EmulationParamsRequestModel::new(
    //         Some(self.config.token_mint),
    //         Some(self.config.chain_id),
    //         max_steps_to_execute,
    //         cached_accounts,
    //         solana_accounts,
    //     );
    //
    //     let emulate_hash_request = EmulateHashRequestModel {
    //         emulation_params,
    //         hash,
    //     };
    //
    //     let params = TraceHashRequestModel {
    //         emulate_hash_request,
    //     };
    //
    //     self.post_request("/api/trace_hash", params, id)
    //         .await
    // }
}
