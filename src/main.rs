#![deny(warnings)]
#![deny(clippy::all, clippy::pedantic)]

use {
    std::sync::Arc,
    jsonrpsee::http_server::{HttpServerBuilder, RpcModule},
    tracing::{info, warn},
    tracing_subscriber::{EnvFilter, fmt},
    tokio::signal,
    crate::{
        service::{ eip1898::EIP1898Server, neon_proxy::NeonProxyServer },
        neon::tracer_core::TracerCore,
        metrics::start_monitoring,
    }
};

mod db;
mod neon;
mod service;
mod metrics;
mod config;
mod evm_runtime;
mod stop_handle;
mod types;

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
    module.merge(EIP1898Server::into_rpc(serv_impl.clone())).expect("EIP1898Server error");
    module.merge(NeonProxyServer::into_rpc(serv_impl)).expect("NeonProxyServer error");

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
