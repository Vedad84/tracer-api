use {
    secret_value::Secret,
    std::{ net::Ipv4Addr, str::FromStr },
};

#[derive(std::fmt::Debug)]
pub struct Options {
    pub addr: String,
    pub db_host: String,
    pub db_port: String,
    pub db_password: String,
    pub db_user: String,
    pub db_database: String,
    pub evm_loader: solana_sdk::pubkey::Pubkey,
    pub web3_proxy: String,
    pub metrics_ip: Ipv4Addr,
    pub metrics_port: u16,
}

pub fn read_config() -> Options {
    let read_env = |var_name: &str| std::env::var(var_name)
        .expect(format!("Failed to read env var {}", var_name).as_str());

    let addr = read_env("LISTEN_ADDR");
    let db_host = read_env("TRACER_DB_HOST");
    let db_port = read_env("TRACER_DB_PORT");
    let db_user = read_env("TRACER_DB_USER");
    let db_password = read_env("TRACER_DB_PASSWORD");
    let db_name = read_env("TRACER_DB_NAME");
    let evm_loader = read_env("EVM_LOADER");
    let web3_proxy = read_env("WEB3_PROXY");
    let metrics_ip = read_env("METRICS_IP");
    let metrics_port = read_env("METRICS_PORT");

    Options {
        addr,
        db_host,
        db_port,
        db_password,
        db_user,
        db_database: db_name,
        evm_loader: solana_sdk::pubkey::Pubkey::from_str(evm_loader.as_str())
            .expect(format!("Failed to parse EVM_LOADER {}", evm_loader).as_str()),
        web3_proxy,
        metrics_ip: Ipv4Addr::from_str(metrics_ip.as_str())
            .expect(format!("Failed to parse METRICS_IP {}", metrics_ip).as_str()),
        metrics_port: metrics_port.parse::<u16>()
            .expect(format!("Failed to parse metrics port {}", metrics_port).as_str()),
    }
}