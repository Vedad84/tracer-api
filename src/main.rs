#![allow(unused, clippy::too_many_arguments)]

use std::str::FromStr;
use std::sync::Arc;

use jsonrpsee::http_server::{HttpServerBuilder, RpcModule};
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::types::error::Error;
use secret_value::Secret;
use structopt::StructOpt;
use tracing::{info, warn, instrument};
use tracing_subscriber::{EnvFilter, fmt};
use web3;

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
    #[structopt(short = "w", long = "web3-proxy")]
    web3_proxy: String,
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
        tag: BlockNumber,
    ) -> Result<U256T>;
}

#[derive(Debug, Clone)]
pub struct ServerImpl {
    tracer_core: neon::TracerCore,
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
        self.tracer_core.eth_call(
            object.from.map(|v| v.0),
            object.to.0,
            object.gas.map(|v| v.0),
            object.value.map(|v| v.0),
            object.data.map(|v| v.0),
            tag,
        )
            .map_err(|err| Error::Custom(err.to_string()))
    }

    #[instrument]
    fn eth_get_storage_at(
        &self,
        contract_id: H160T,
        index: U256T,
        tag: BlockNumber,
    ) -> Result<U256T> {
        self.tracer_core.get_storage_at(&contract_id, &index, tag)
            .map_err(|err| Error::Custom(err.to_string()))
    }

    #[instrument]
    fn eth_get_balance(
        &self,
        address: H160T,
        tag: BlockNumber,
    ) -> Result<U256T> {
        self.tracer_core.get_balance(&address, tag)
            .map_err(|err|Error::Custom(err.to_string()))
    }

    #[instrument]
    fn eth_get_code(
        &self,
        address: H160T,
        tag: BlockNumber,
    ) -> Result<String> {
        self.tracer_core.get_code(&address, tag)
            .map_err(|err|Error::Custom(err.to_string()))
    }

    #[instrument]
    fn eth_get_transaction_count(
        &self,
        account_id: H160T,
        tag: BlockNumber,
    ) -> Result<U256T> {
        self.tracer_core.get_transaction_count(&account_id, tag)
            .map_err(|err|Error::Custom(err.to_string()))
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

    let transport = web3::transports::Http::new(&options.web3_proxy);
    if transport.is_err() {
        warn!("Failed to initialize HTTP transport for Web3 Proxy client");
        return;
    }

    let web3_client = web3::Web3::new(transport.unwrap());

    let serv_impl = ServerImpl {
        tracer_core: neon::TracerCore {
            evm_loader: options.evm_loader,
            db_client: Arc::new(client),
            db_client_after: Arc::new(client_after),
            web3: Arc::new(web3_client),
        },
    };

    let mut module = RpcModule::new(());
    module.merge(EIP1898Server::into_rpc(serv_impl));

    let _handle = server.start(module).unwrap();
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
