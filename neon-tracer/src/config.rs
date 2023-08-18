use {
    neon_cli_lib::types::ChDbConfig,
    std::{net::Ipv4Addr, str::FromStr},
};

#[derive(std::fmt::Debug)]
pub struct Options {
    pub addr: String,
    pub db_config: ChDbConfig,
    pub web3_proxy: String,
    pub metrics_ip: Ipv4Addr,
    pub metrics_port: u16,
}

pub fn read_config() -> Options {
    let read_env = |var_name: &str| {
        std::env::var(var_name).unwrap_or_else(|_| panic!("Failed to read env var {var_name}"))
    };

    let clickhouse_url = std::env::var("NEON_DB_CLICKHOUSE_URLS")
        .map(|urls| {
            urls.split(';')
                .map(std::borrow::ToOwned::to_owned)
                .collect::<Vec<String>>()
        })
        .expect("NEON_DB_CLICKHOUSE_URLS not found");

    let clickhouse_user = std::env::var("NEON_DB_CLICKHOUSE_USER")
        .map(Some)
        .unwrap_or(None);

    let clickhouse_password = std::env::var("NEON_DB_CLICKHOUSE_PASSWORD")
        .map(Some)
        .unwrap_or(None);

    let indexer_host = read_env("NEON_DB_INDEXER_HOST");
    let indexer_port = read_env("NEON_DB_INDEXER_PORT");
    let indexer_database = read_env("NEON_DB_INDEXER_DATABASE");
    let indexer_user = read_env("NEON_DB_INDEXER_USER");
    let indexer_password = read_env("NEON_DB_INDEXER_PASSWORD");

    let db_config = ChDbConfig {
        clickhouse_url,
        clickhouse_user,
        clickhouse_password,
        indexer_host,
        indexer_port,
        indexer_database,
        indexer_user,
        indexer_password,
    };
    let addr = read_env("LISTEN_ADDR");
    let web3_proxy = read_env("WEB3_PROXY");
    let metrics_ip = read_env("METRICS_IP");
    let metrics_ip = Ipv4Addr::from_str(metrics_ip.as_str())
        .unwrap_or_else(|_| panic!("Failed to parse METRICS_IP {metrics_ip}"));
    let metrics_port = read_env("METRICS_PORT");
    let metrics_port = metrics_port
        .parse::<u16>()
        .unwrap_or_else(|_| panic!("Failed to parse metrics port {metrics_port}"));

    Options {
        addr,
        db_config,
        web3_proxy,
        metrics_ip,
        metrics_port,
    }
}
