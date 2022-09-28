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

#[derive(Debug, StructOpt)]
struct Options {
    #[structopt(short = "l", long = "listen", default_value = "127.0.0.1:8080")]
    addr: String,
    #[structopt(short = "h", long = "db-host", default_value = "127.0.0.1")]
    ch_host: String,
    #[structopt(short = "P", long = "db-port", default_value = "5432")]
    ch_port: String,
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
    #[structopt(short = "i", long = "metrics-ip", default_value = "0.0.0.0")]
    metrics_ip: Ipv4Addr,
    #[structopt(short = "m", long = "metrics-port", default_value = "9292")]
    metrics_port: u16,
}

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

    let options = Options::from_args();

    init_logs();

    info!(?options, "starting");

    let server = HttpServerBuilder::default()
        .build(options.addr.parse().unwrap())
        .unwrap();

    let client = Arc::new(DbClient::new(
        &options.ch_host.clone(),
        &options.ch_port.clone(),
        options.ch_user.clone(),
        options.ch_password.clone().map(Secret::inner),
        options.ch_database.clone(),
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
