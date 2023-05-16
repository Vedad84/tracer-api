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
        std::env::var(var_name).unwrap_or_else(|_| panic!("Failed to read env var {}", var_name))
    };

    let path = read_env("DB_CONFIG");
    let path = path.as_str();
    let db_config: ChDbConfig =
        solana_cli_config::load_config_file(path).expect("load db-config error");
    let addr = read_env("LISTEN_ADDR");
    let web3_proxy = read_env("WEB3_PROXY");
    let metrics_ip = read_env("METRICS_IP");
    let metrics_ip = Ipv4Addr::from_str(metrics_ip.as_str())
        .unwrap_or_else(|_| panic!("Failed to parse METRICS_IP {}", metrics_ip));
    let metrics_port = read_env("METRICS_PORT");
    let metrics_port = metrics_port
        .parse::<u16>()
        .unwrap_or_else(|_| panic!("Failed to parse metrics port {}", metrics_port));

    Options {
        addr,
        db_config,
        web3_proxy,
        metrics_ip: metrics_ip,
        metrics_port,
    }
}
