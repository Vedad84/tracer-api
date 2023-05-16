use ethnum::U256;
use evm_loader::types::Address;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct NeonApiResponse {
    pub result: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct GetEtherAccountDataRequest {
    pub(crate) ether: Address,
    pub(crate) slot: Option<u64>,
}

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct GetStorageAtRequest {
    pub(crate) contract_id: Address,
    pub(crate) index: Option<U256>,
    pub(crate) slot: Option<u64>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub(crate) struct TxParamsRequest {
    pub(crate) sender: Address,
    pub(crate) contract: Option<Address>,
    pub(crate) data: Option<Vec<u8>>,
    pub(crate) value: Option<String>,
    pub(crate) gas_limit: Option<String>,
    pub(crate) token_mint: Option<String>,
    pub(crate) chain_id: Option<u64>,
    pub(crate) max_steps_to_execute: Option<u64>,
    pub(crate) cached_accounts: Option<Vec<Address>>,
    pub(crate) solana_accounts: Option<Vec<String>>,
    pub(crate) slot: Option<u64>,
    pub(crate) hash: Option<String>,
}
