#![allow(unused, clippy::too_many_arguments)]

use std::net::Ipv4Addr;
use std::str::FromStr;
use std::sync::Arc;

use jsonrpsee::http_server::{HttpServerBuilder, RpcModule};
use secret_value::Secret;
use structopt::StructOpt;
use tracing::{info, warn};
use tracing_subscriber::{EnvFilter, fmt};
use crate::metrics::start_monitoring;
use tokio::signal;

use crate::v1::geth::types::trace as geth;
use crate::service::{ eip1898::EIP1898Server, neon_proxy::NeonProxyServer };
use crate::neon::tracer_core::TracerCore;
use crate::stop_handle::StopHandle;

mod db;
mod neon;
mod v1;
mod syscall_stubs;
mod service;
mod metrics;
mod config;
mod evm_runtime;
mod stop_handle;

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

async fn run() {
    use crate::db::DbClient;
    use std::str::FromStr;

    let options = config::read_config();

    init_logs();

    info!(?options, "starting");

    let server = HttpServerBuilder::default()
        .build(options.addr.parse().unwrap())
        .unwrap();

    let tracer_db_client = Arc::new(DbClient::new(&options.tracer_db_config).await);
    let indexer_db_client = Arc::new(DbClient::new(&options.indexer_db_config).await);

    let transport = web3::transports::Http::new(&options.web3_proxy);
    if transport.is_err() {
        warn!("Failed to initialize HTTP transport for Web3 Proxy client");
        return;
    }

    let web3_client = Arc::new(web3::Web3::new(transport.unwrap()));

    let evm_runtime = Arc::new(evm_runtime::EVMRuntime::new(
        &options.evm_runtime_config,
        tracer_db_client.clone(),
    ).await.unwrap_or_else(|err| panic!("{:?}", err)));

    let serv_impl = TracerCore::new(
        options.evm_loader,
        tracer_db_client.clone(),
        indexer_db_client.clone(),
        web3_client.clone(),
        evm_runtime.clone(),
    );

    let mut module = RpcModule::new(());
    module.merge(EIP1898Server::into_rpc(serv_impl.clone()));
    module.merge(NeonProxyServer::into_rpc(serv_impl));

    let monitor_handle = start_monitoring(
        indexer_db_client.clone(),
        web3_client.clone(),
        options.metrics_ip,
        options.metrics_port
    );

    let evm_runtime_handle = (*evm_runtime).clone().start();
    let server_handle = server.start(module).unwrap();

    let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();
    let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt()).unwrap();
    tokio::select! {
        _ = sigterm.recv() => {}
        _ = sigint.recv() => {}
    }

    let handles = vec![
        server_handle.stop().unwrap(),
        evm_runtime_handle.stop().unwrap(),
        monitor_handle.stop().unwrap(),
    ];

    futures::future::join_all(handles).await;
}

fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(run());
}
