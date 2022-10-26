use {
    secret_value::Secret,
    std::{ net::Ipv4Addr, str::FromStr },
};

#[derive(std::fmt::Debug)]
pub struct Options {
    pub addr: String,
    pub tracer_db_host: String,
    pub tracer_db_port: String,
    pub tracer_db_password: String,
    pub tracer_db_user: String,
    pub tracer_db_database: String,
    pub indexer_db_host: String,
    pub indexer_db_port: String,
    pub indexer_db_password: String,
    pub indexer_db_user: String,
    pub indexer_db_database: String,
    pub evm_loader: solana_sdk::pubkey::Pubkey,
    pub web3_proxy: String,
    pub metrics_ip: Ipv4Addr,
    pub metrics_port: u16,
}

pub fn read_config() -> Options {
    let read_env = |var_name: &str| std::env::var(var_name)
        .expect(format!("Failed to read env var {}", var_name).as_str());

    let addr = read_env("LISTEN_ADDR");
    let tracer_db_host = read_env("TRACER_DB_HOST");
    let tracer_db_port = read_env("TRACER_DB_PORT");
    let tracer_db_user = read_env("TRACER_DB_USER");
    let tracer_db_password = read_env("TRACER_DB_PASSWORD");
    let tracer_db_database = read_env("TRACER_DB_NAME");
    let indexer_db_host = read_env("INDEXER_DB_HOST");
    let indexer_db_port = read_env("INDEXER_DB_PORT");
    let indexer_db_user = read_env("INDEXER_DB_USER");
    let indexer_db_password = read_env("INDEXER_DB_PASSWORD");
    let indexer_db_database = read_env("INDEXER_DB_NAME");
    let evm_loader = read_env("EVM_LOADER");
    let web3_proxy = read_env("WEB3_PROXY");
    let metrics_ip = read_env("METRICS_IP");
    let metrics_port = read_env("METRICS_PORT");

    Options {
        addr,
        tracer_db_host,
        tracer_db_port,
        tracer_db_password,
        tracer_db_user,
        tracer_db_database,
        indexer_db_host,
        indexer_db_port,
        indexer_db_password,
        indexer_db_user,
        indexer_db_database,
        evm_loader: solana_sdk::pubkey::Pubkey::from_str(evm_loader.as_str())
            .expect(format!("Failed to parse EVM_LOADER {}", evm_loader).as_str()),
        web3_proxy,
        metrics_ip: Ipv4Addr::from_str(metrics_ip.as_str())
            .expect(format!("Failed to parse METRICS_IP {}", metrics_ip).as_str()),
        metrics_port: metrics_port.parse::<u16>()
            .expect(format!("Failed to parse metrics port {}", metrics_port).as_str()),
    }
}