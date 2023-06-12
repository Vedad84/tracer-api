#![deny(warnings)]
#![deny(clippy::all, clippy::pedantic)]

use {
    crate::{
        data_source::DataSource,
        metrics::start_monitoring,
        service::{eip1898::EIP1898Server, geth::GethTraceServer},
    },
    jsonrpsee::http_server::{HttpServerBuilder, RpcModule},
    neon_cli_lib::types::{IndexerDb, TracerDb},
    std::sync::Arc,
    tokio::signal,
    tracing::{info, warn},
    tracing_subscriber::{fmt, EnvFilter},
};

mod api_client;
mod config;
mod data_source;
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
        .map_err(|e| {
            warn!(
                "Failed to initialize HTTP transport for Web3 Proxy client: {:?}",
                e
            );
        })
        .unwrap();

    let web3_client = Arc::new(web3::Web3::new(transport));

    let neon_client_config = Arc::new(api_client::config::read_api_client_config_from_enviroment());
    let neon_api_url = neon_client_config.neon_api_url.clone();
    let neon_client = api_client::client::Client::new(Arc::clone(&neon_client_config), neon_api_url.as_str());

    let source = DataSource::new(
        tracer_db.clone(),
        indexer_db.clone(),
        web3_client.clone(),
        neon_client_config,
        neon_client,
    );

    let mut module = RpcModule::new(());
    module
        .merge(EIP1898Server::into_rpc(source.clone()))
        .expect("EIP1898Server error");
    module
        .merge(GethTraceServer::into_rpc(source.clone()))
        .expect("GethTraceServer error");

    let monitor_handle = start_monitoring(
        tracer_db.clone(),
        web3_client.clone(),
        options.metrics_ip,
        options.metrics_port,
    );

    let server_handle = server
        .start(module)
        .expect("Failed to start JSON RPC Server");

    let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
        .expect("Failed to initialize SIGTERM handler");
    let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
        .expect("Failed to initialize SIGINT handler");
    tokio::select! {
        _ = sigterm.recv() => {}
        _ = sigint.recv() => {}
    }

    let handles = vec![
        server_handle
            .stop()
            .expect("Failed to stop JSON RPC Server"),
        monitor_handle.stop().expect("Failed to stop Monitoring"),
    ];

    futures::future::join_all(handles).await;
}

#[tokio::main]
async fn main() {
    run().await;
}
