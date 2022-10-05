#![allow(unused, clippy::too_many_arguments)]

use std::net::Ipv4Addr;
use std::str::FromStr;
use std::sync::Arc;

use jsonrpsee::http_server::{HttpServerBuilder, RpcModule};
use secret_value::Secret;
use structopt::StructOpt;
use tracing::{info, warn};
use tracing_subscriber::{EnvFilter, fmt};
use web3;
use crate::metrics::start_monitoring;

use crate::v1::geth::types::trace as geth;
use crate::service::{ eip1898::EIP1898Server };
use crate::neon::TracerCore;

mod db;
mod neon;
mod v1;
mod syscall_stubs;
mod service;
mod metrics;
mod config;

fn parse_secret<T: FromStr>(input: &str) -> std::result::Result<Secret<T>, T::Err> {
    T::from_str(input).map(Secret::from)
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

    let options = config::read_config();

    init_logs();

    info!(?options, "starting");

    let server = HttpServerBuilder::default()
        .build(options.addr.parse().unwrap())
        .unwrap();

    let client = Arc::new(DbClient::new(
        &options.db_host.clone(),
        &options.db_port.clone(),
        options.db_user.clone(),
        options.db_password.clone(),
        options.db_database.clone(),
    ).await);

    let transport = web3::transports::Http::new(&options.web3_proxy);
    if transport.is_err() {
        warn!("Failed to initialize HTTP transport for Web3 Proxy client");
        return;
    }

    let web3_client = Arc::new(web3::Web3::new(transport.unwrap()));

    let serv_impl = neon::TracerCore::new(
        options.evm_loader,
        client.clone(),
        web3_client.clone(),
    );

    let mut module = RpcModule::new(());
    module.merge(EIP1898Server::into_rpc(serv_impl));

    info!("before start monitoring");

    let _handle = server.start(module).unwrap();
    start_monitoring(
        client.clone(),
        web3_client.clone(),
        options.metrics_ip,
        options.metrics_port
    ).await;
}
