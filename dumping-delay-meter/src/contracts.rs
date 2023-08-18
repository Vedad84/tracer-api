use {
    async_trait::async_trait,
    secp256k1::SecretKey,
    web3::{contract::Contract, transports::Http, types::H256},
};

#[async_trait]
pub trait TestFactoryContract {
    async fn create_new_contract(&self, caller: &SecretKey) -> web3::error::Result<H256>;
}

#[async_trait]
pub trait TestContract {
    async fn get_creation_block(&self, caller: &SecretKey) -> web3::error::Result<H256>;
}

pub struct BaseContract {
    instance: Contract<Http>,
}

impl BaseContract {
    pub fn new(instance: Contract<Http>) -> Self {
        Self { instance }
    }
}

#[async_trait]
impl TestFactoryContract for BaseContract {
    async fn create_new_contract(&self, caller: &SecretKey) -> web3::error::Result<H256> {
        let options = web3::contract::Options {
            gas: Some(web3::types::U256::from(20000000)),
            ..Default::default()
        };
        self.instance
            .signed_call(
                "createNewContract",
                (),
                options,
                web3::signing::SecretKeyRef::new(caller),
            )
            .await
    }
}

#[async_trait]
impl TestContract for BaseContract {
    async fn get_creation_block(&self, caller: &SecretKey) -> web3::error::Result<H256> {
        self.instance
            .signed_call(
                "getCreationBlock",
                (),
                web3::contract::Options::default(),
                web3::signing::SecretKeyRef::new(caller),
            )
            .await
    }
}
