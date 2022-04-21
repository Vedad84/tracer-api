#![allow(unused, clippy::too_many_arguments)]

use std::str::FromStr;
use std::sync::Arc;

use jsonrpsee::http_server::{HttpServerBuilder, RpcModule};
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::types::error::Error;
use secret_value::Secret;
use structopt::StructOpt;
use tracing::{info, instrument};
use tracing_subscriber::{EnvFilter, fmt};


use crate::neon::provider::DbProvider;
use crate::v1::geth::types::trace as geth;
use crate::v1::types::{
    BlockNumber, EthCallObject,
};
use evm::{H160, U256, H256};
use crate::v1::geth::types::trace::{H160T, H256T, U256T};

type Result<T> = std::result::Result<T, Error>;

mod db;
mod neon;
mod utils;
mod v1;
mod syscall_stubs;

#[derive(Debug, StructOpt)]
struct Options {
    #[structopt(short = "l", long = "listen", default_value = "127.0.0.1:8080")]
    addr: String,
    #[structopt(short = "c", long = "db-addr", default_value = "127.0.0.1:8123")]
    ch_addr: String,
    #[structopt(short = "p", long = "ch-password", parse(try_from_str = parse_secret))]
    ch_password: Option<Secret<String>>,
    #[structopt(short = "u", long = "ch-user")]
    ch_user: Option<String>,
    #[structopt(short = "d", long = "ch-database")]
    ch_database: Option<String>,
    #[structopt(long = "evm-loader")]
    evm_loader: solana_sdk::pubkey::Pubkey,
}

fn parse_secret<T: FromStr>(input: &str) -> std::result::Result<Secret<T>, T::Err> {
    T::from_str(input).map(Secret::from)
}

#[rpc(server)]
pub trait EIP1898 {
    #[method(name = "eth_call")]
    fn eth_call(
        &self,
        object: EthCallObject,
        tag: BlockNumber,
    ) -> Result<String>;

    #[method(name = "eth_getStorageAt")]
    fn eth_get_storage_at(
        &self,
        contract_id: H160T,
        index: U256T,
        tag: BlockNumber,
    ) -> Result<U256T>;

    #[method(name = "eth_getBalance")]
    fn eth_get_balance(
        &self,
        address: H160T,
        tag: BlockNumber,
    ) -> Result<U256T>;

    #[method(name = "eth_getCode")]
    fn eth_get_code(
        &self,
        address: H160T,
        tag: BlockNumber,
    ) -> Result<String>;

    #[method(name = "eth_getTransactionCount")]
    fn eth_get_transaction_count(
        &self,
        contract_id: H160T,
        block_number: u64,
    ) -> Result<U256T>;
}

#[derive(Debug, Clone)]
pub struct ServerImpl {
    neon_config: neon::Config,
}

impl ServerImpl {
    fn get_slot_by_block(&self, bn: BlockNumber) -> Option<u64> {
        match bn {
            BlockNumber::Num(num) => Some(num),
            BlockNumber::Latest => None,
            _ => todo!(),
        }
    }
}

impl EIP1898Server for ServerImpl {
    #[instrument]
    fn eth_call(
        &self,
        object: EthCallObject,
        tag: BlockNumber,
    ) -> Result<String> {
        let provider = DbProvider::new(
            Arc::clone(&self.neon_config.rpc_client_after),
            self.neon_config.evm_loader,
        );

        match tag {
            BlockNumber::Num(block_number) =>
                neon::eth_call(
                    provider,
                    object.from.map(|v| v.0),
                    object.to.0,
                    object.gas.map(|v| v.0),
                    object.value.map(|v| v.0),
                    object.data.map(|v| v.0),
                    block_number,
                ).map_err(|err| Error::Custom(err.to_string())),
            _ => todo!()
        }

    }

    #[instrument]
    fn eth_get_storage_at(
        &self,
        contract_id: H160T,
        index: U256T,
        tag: BlockNumber,
    ) -> Result<U256T> {
        let provider = DbProvider::new(
            self.neon_config.rpc_client_after.clone(),
            self.neon_config.evm_loader,
        );

        match tag {
            BlockNumber::Num(number) => {
                print!("Block number {:?}", number);
                return Ok(U256T(neon::get_storage_at(
                    provider,
                    &contract_id.0,
                    &index.0,
                    number)));
            },
            _ => todo!()
        }
    }

    #[instrument]
    fn eth_get_balance(
        &self,
        address: H160T,
        tag: BlockNumber,
    ) -> Result<U256T> {

        let provider = DbProvider::new(
            self.neon_config.rpc_client_after.clone(),
            self.neon_config.evm_loader,
        );

        match tag {
            BlockNumber::Num(block_number) =>
                Ok(U256T(neon::get_balance(
                    provider,
                    &address.0,
                    block_number))),
            _ => todo!()
        }
    }

    #[instrument]
    fn eth_get_code(
        &self,
        address: H160T,
        tag: BlockNumber,
    ) -> Result<String> {
        let provider = DbProvider::new(
            Arc::clone(&self.neon_config.rpc_client_after),
            self.neon_config.evm_loader,
        );

        match tag {
            BlockNumber::Num(block_number) => {
                let code = neon::get_code(provider, &address.0, block_number);
                Ok(format!("0x{}", hex::encode(code)))
            }
            _ => todo!()
        }
    }

    #[instrument]
    fn eth_get_transaction_count(
        &self,
        account_id: H160T,
        block_number: u64) -> Result<U256T> {

        let provider = DbProvider::new(
            self.neon_config.rpc_client_after.clone(),
            self.neon_config.evm_loader,
        );

        Ok(U256T(neon::get_transaction_count(
            provider,
            &account_id.0,
            block_number)))
    }
}

fn init_logs() {
    let writer = std::io::stdout;
    let subscriber = fmt::Subscriber::builder()
        .with_writer(writer)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();
    tracing_log::LogTracer::init().unwrap();
}

#[tokio::main]
async fn main() {
    use crate::db::DbClient;
    use std::str::FromStr;

    let options = Options::from_args();

    init_logs();

    info!(?options, "starting");

    let server = HttpServerBuilder::default()
        .build(options.addr.parse().unwrap())
        .unwrap();

    let client = DbClient::new(
        options.ch_addr.clone(),
        options.ch_user.clone(),
        options.ch_password.clone().map(Secret::inner),
        options.ch_database.clone(),
        false
    );

    let client_after = DbClient::new(
        options.ch_addr,
        options.ch_user,
        options.ch_password.map(Secret::inner),
        options.ch_database,
        true
    );

    let serv_impl = ServerImpl {
        neon_config: neon::Config {
            evm_loader: options.evm_loader,
            rpc_client: Arc::new(client),
            rpc_client_after: Arc::new(client_after),
        },
    };

    let mut module = RpcModule::new(());
    module.merge(EIP1898Server::into_rpc(serv_impl));

    let _handle = server.start(module).unwrap();
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
