mod contracts;
mod event_monitor;
mod generator;
mod stop_handle;

use {
    crate::{event_monitor::EventMonitor, generator::Generator},
    std::{fs, str::FromStr, sync::Arc},
    tracing::info,
    tracing_subscriber::{fmt, EnvFilter},
    web3::{contract::Contract, transports::Http},
};

struct Config {
    web3_client: web3::Web3<Http>,
    factory_contract: Contract<Http>,
    test_contract_abi: web3::ethabi::Contract,
    caller: secp256k1::SecretKey,
    generation_interval_ms: u64,
    monitoring_interval_ms: u64,
    read_delay_spread_slots: u64,
    pg_connection_string: String,
}

const WEB3_URL_ENV: &str = "WEB3_URL";
const FACTORY_ADDRESS_ENV: &str = "FACTORY_ADDRESS";
const FACTORY_ABI_ENV: &str = "FACTORY_ABI";
const TEST_CONTRACT_ABI_ENV: &str = "TEST_CONTRACT_ABI";
const CALLER_ENV: &str = "CALLER";
const GENERATION_INTERVAL_ENV: &str = "GENERATION_INTERVAL";
const MONITORING_INTERVAL_ENV: &str = "MONITORING_INTERVAL";
const READ_DELAY_SPREAD_SLOTS_ENV: &str = "READ_DELAY_SPREAD_SLOTS";
const PG_CONNECTION_STRING_ENV: &str = "PG_CONNECTION_STRING";

fn contract_abi_from_file(filename: String) -> web3::ethabi::Contract {
    let abi = fs::read_to_string(filename.clone())
        .unwrap_or_else(|_| panic!("Failed to read ABI file {filename}"));
    let abi: serde_json::Value =
        serde_json::from_str(abi.as_str()).unwrap_or_else(|_| panic!("Failed to parse ABI {abi}"));

    let abi: web3::ethabi::Contract = if let serde_json::Value::Object(map) = abi {
        if let Some(abi) = map.get("abi") {
            serde_json::from_str(abi.to_string().as_str())
                .unwrap_or_else(|_| panic!("Failed to parse ABI file {abi}"))
        } else {
            panic!(
                "Failed to create Contract ABI: expected ABI file object to contain field 'abi'"
            );
        }
    } else {
        panic!("Failed to create Contract ABI: expected ABI file content to be a JSON object");
    };

    abi
}

impl Config {
    pub fn new_from_env() -> Self {
        let read_env_var = |var_name: &str, default: Option<&str>| {
            std::env::var(var_name)
                .map_err(|e| {
                    if let Some(value) = default {
                        Ok(value)
                    } else {
                        Err(e)
                    }
                })
                .unwrap_or_else(|_| panic!("Unable to read env var {var_name}"))
        };

        let web3_url = read_env_var(WEB3_URL_ENV, None);
        let web3_client = web3::Web3::new(
            Http::new(web3_url.as_str())
                .unwrap_or_else(|_| panic!("Failed to connect to {web3_url}")),
        );

        let factory_address =
            web3::types::Address::from_str(read_env_var(FACTORY_ADDRESS_ENV, None).as_str())
                .unwrap_or_else(|_| panic!("Unable to parse {FACTORY_ADDRESS_ENV}"));

        let factory_abi = read_env_var(FACTORY_ABI_ENV, None);
        let factory_abi = contract_abi_from_file(factory_abi);
        let factory_contract: Contract<Http> =
            Contract::new(web3_client.eth(), factory_address, factory_abi);

        let test_contract_abi = read_env_var(TEST_CONTRACT_ABI_ENV, None);
        let test_contract_abi = contract_abi_from_file(test_contract_abi);

        let caller = secp256k1::SecretKey::from_str(read_env_var(CALLER_ENV, None).as_str())
            .unwrap_or_else(|_| panic!("Unable to parse {CALLER_ENV}"));

        let generation_interval_ms =
            u64::from_str(read_env_var(GENERATION_INTERVAL_ENV, Some("1")).as_str())
                .expect("Failed to parse GENERATION_INTERVAL");

        let monitoring_interval_ms =
            u64::from_str(read_env_var(MONITORING_INTERVAL_ENV, Some("1")).as_str())
                .expect("Failed to parse MONITORING_INTERVAL");

        let read_delay_spread_slots =
            u64::from_str(read_env_var(READ_DELAY_SPREAD_SLOTS_ENV, None).as_str())
                .expect("Failed to parse READ_DELAY_SPREAD_SLOTS");

        let pg_connection_string = read_env_var(PG_CONNECTION_STRING_ENV, None);

        Self {
            web3_client,
            factory_contract,
            test_contract_abi,
            caller,
            generation_interval_ms,
            monitoring_interval_ms,
            read_delay_spread_slots,
            pg_connection_string,
        }
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
    init_logs();

    info!("Reading configuration...");
    let config = Config::new_from_env();

    info!("Creating Generator...");
    let generator = Generator::new(config.factory_contract.clone(), config.caller);

    let (pg_client, connection) =
        tokio_postgres::connect(&config.pg_connection_string, tokio_postgres::NoTls)
            .await
            .unwrap();

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    pg_client.execute(
        "CREATE TABLE IF NOT EXISTS delay_stat (delay_slots BIGINT,success BOOLEAN,err VARCHAR(512))", &[]
    ).await.expect("Failed to create delay_stat table");

    info!("Creating Event Monitor...");
    let event_monitor = EventMonitor::new(
        &config.web3_client,
        config.factory_contract.address(),
        config.test_contract_abi,
        Arc::new(pg_client),
    );

    info!("Starting...");
    let generator_handle = generator.run(config.generation_interval_ms);
    let event_monitor_handle = event_monitor.run(
        config.monitoring_interval_ms,
        config.read_delay_spread_slots,
    );

    let mut sigterm =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();

    let mut sigint =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).unwrap();

    tokio::select! {
        _ = sigterm.recv() => {}
        _ = sigint.recv() => {}
    }

    let handles = vec![
        generator_handle.stop().unwrap(),
        event_monitor_handle.stop().unwrap(),
    ];

    futures::future::join_all(handles).await;
}
