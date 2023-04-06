#![deny(warnings)]
#![deny(clippy::all, clippy::pedantic)]

use {
    std::sync::Arc,
    jsonrpsee::http_server::{HttpServerBuilder, RpcModule},
    tracing::{info, warn},
    tracing_subscriber::{EnvFilter, fmt},
    tokio::signal,
    crate::{
        service::{ eip1898::EIP1898Server, geth::GethTraceServer},
        data_source::DataSource,
        metrics::start_monitoring,
        evm_runtime::EVMRuntime,
    },
    neon_cli_lib::types::{TracerDb, IndexerDb},
};

mod account_ordering;
mod config;
mod data_source;
mod evm_runtime;
mod metrics;
mod opcodes;
mod service;
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
    let options = config::read_config();

    init_logs();

    info!(?options, "starting");

    let server = HttpServerBuilder::default()
        .build(options.addr.parse().unwrap())
        .unwrap();

    let tracer_db = TracerDb::new(&options.db_config);
    let indexer_db = IndexerDb::new(&options.db_config);

    let transport = web3::transports::Http::new(&options.web3_proxy)
        .map_err(|e| warn!("Failed to initialize HTTP transport for Web3 Proxy client: {:?}", e))
        .unwrap();

    let web3_client = Arc::new(web3::Web3::new(transport));

    let evm_runtime = Arc::new(EVMRuntime::new(&options.evm_runtime_config,tracer_db.clone()).await
        .map_err(|e| warn!("Filed to init emv_runtime: {:?}", e))
        .unwrap());

    let source = DataSource::new(
        tracer_db.clone(),
        indexer_db.clone(),
        web3_client.clone(),
        evm_runtime.clone(),
    );

    let mut module = RpcModule::new(());
    module.merge(EIP1898Server::into_rpc(source.clone())).expect("EIP1898Server error");
    module.merge(GethTraceServer::into_rpc(source.clone())).expect("GethTraceServer error");

    let monitor_handle = start_monitoring(
        tracer_db.clone(),
        web3_client.clone(),
        options.metrics_ip,
        options.metrics_port
    );

    let evm_runtime_handle = (*evm_runtime).clone().start();
    let server_handle = server.start(module)
        .expect("Failed to start JSON RPC Server");
    let acc_ord_job_handle = if options.enable_acc_ord_job {
        Some(account_ordering::start_account_ordering(tracer_db.clone()))
    } else {
        None
    };

    let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
        .expect("Failed to initialize SIGTERM handler");
    let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
        .expect("Failed to initialize SIGINT handler");
    tokio::select! {
        _ = sigterm.recv() => {}
        _ = sigint.recv() => {}
    }

    let mut handles = vec![
        server_handle.stop().expect("Failed to stop JSON RPC Server"),
        evm_runtime_handle.stop().expect("Failed to stop EVM Runtime"),
        monitor_handle.stop().expect("Failed to stop Monitoring"),
    ];

    if let Some(acc_ord_job_handle) = acc_ord_job_handle {
        handles.push(acc_ord_job_handle.stop().expect("Failed to stop Account Ordering Job"));
    }

    futures::future::join_all(handles).await;
}

fn main() {
    let rt = tokio::runtime::Runtime::new()
        .expect("Failed to initialize tokio runtime");
    rt.block_on(run());
}
