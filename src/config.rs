use {
    crate::evm_runtime::EVMRuntimeConfig,
    solana_sdk::pubkey::Pubkey,
    std::{net::Ipv4Addr, str::FromStr },
    neon_cli_lib::types::DbConfig,
};

// Environment variables
const EVM_RUNTIME_DOCKER_SOCKET: &str = "EVM_RUNTIME_DOCKER_SOCKET";
const EVM_RUNTIME_DOCKER_SOCKET_DEFAULT: &str = "/var/run/docker.sock";
const EVM_RUNTIME_DOCKER_CONNECT_TOUT: &str = "EVM_RUNTIME_DOCKER_CONNECT_TOUT";
const EVM_RUNTIME_DOCKER_CONNECT_TOUT_DEFAULT: &str = "15";
const EVM_RUNTIME_UPDATE_INTERVAL_SEC: &str = "EVM_RUNTIME_UPDATE_INTERVAL_SEC";
const EVM_RUNTIME_UPDATE_INTERVAL_SEC_DEFAULT: &str = "5";
const EVM_RUNTIME_DOCKER_CLIENT_VERSION_MINOR: &str = "EVM_RUNTIME_DOCKER_CLIENT_VERSION_MINOR";
const EVM_RUNTIME_DOCKER_CLIENT_VERSION_MINOR_DEFAULT: &str = "29";
const EVM_RUNTIME_DOCKER_CLIENT_VERSION_MAJOR: &str = "EVM_RUNTIME_DOCKER_CLIENT_VERSION_MAJOR";
const EVM_RUNTIME_DOCKER_CLIENT_VERSION_MAJOR_DEFAULT: &str = "1";
const EVM_RUNTIME_RUNNING_TO_SUSPENDED_TIME_SEC: &str = "EVM_RUNTIME_RUNNING_TO_SUSPENDED_TIME_SEC";
const EVM_RUNTIME_RUNNING_TO_SUSPENDED_TIME_SEC_DEFAULT: &str = "300"; // 10 minutes
const EVM_RUNTIME_SUSPENDED_TO_STOPPED_TIME_SEC: &str = "EVM_RUNTIME_SUSPENDED_TO_STOPPED_TIME_SEC";
const EVM_RUNTIME_SUSPENDED_TO_STOPPED_TIME_SEC_DEFAULT: &str = "1200"; // 20 minutes
const EVM_RUNTIME_STOPPED_TO_DEAD_TIME_SEC: &str = "EVM_RUNTIME_STOPPED_TO_DEAD_TIME_SEC";
const EVM_RUNTIME_STOPPED_TO_DEAD_TIME_SEC_SEC_DEFAULT: &str = "2400"; // 40 minutes
const EVM_RUNTIME_KNOWN_REVISIONS: &str = "EVM_RUNTIME_KNOWN_REVISIONS";
const EVM_RUNTIME_KNOWN_REVISIONS_DEFAULT: &str = "[]";
const EVM_RUNTIME_DB_CONFIG_TAR: &str = "EVM_RUNTIME_DB_CONFIG_TAR";
const EVM_RUNTIME_NETWORK_NAME: &str = "EVM_RUNTIME_NETWORK_NAME";

pub fn read_evm_runtime_config(
    evm_loader: &Pubkey,
    token_mint: &Pubkey,
    chain_id: u16,
) -> EVMRuntimeConfig {
    let docker_socket = std::env::var(EVM_RUNTIME_DOCKER_SOCKET)
        .unwrap_or_else(|_| EVM_RUNTIME_DOCKER_SOCKET_DEFAULT.to_string());

    let docker_tout = std::env::var(EVM_RUNTIME_DOCKER_CONNECT_TOUT)
        .unwrap_or_else(|_| EVM_RUNTIME_DOCKER_CONNECT_TOUT_DEFAULT.to_string())
        .parse::<u64>()
        .unwrap_or_else(|err| panic!("Failed to parse EVM_RUNTIME_DOCKER_CONNECT_TOUT_DEFAULT: {:?}", err));

    let docker_version_minor = std::env::var(EVM_RUNTIME_DOCKER_CLIENT_VERSION_MINOR)
        .unwrap_or_else(|_| EVM_RUNTIME_DOCKER_CLIENT_VERSION_MINOR_DEFAULT.to_string())
        .parse::<usize>()
        .unwrap_or_else(|err| panic!("Failed to parse EVM_RUNTIME_DOCKER_CLIENT_VERSION_MINOR: {:?}", err));

    let docker_version_major = std::env::var(EVM_RUNTIME_DOCKER_CLIENT_VERSION_MAJOR)
        .unwrap_or_else(|_| EVM_RUNTIME_DOCKER_CLIENT_VERSION_MAJOR_DEFAULT.to_string())
        .parse::<usize>()
        .unwrap_or_else(|err| panic!("Failed to parse EVM_RUNTIME_DOCKER_CLIENT_VERSION_MAJOR: {:?}", err));

    let read_env_var = |var_name: &str, default_value: &str| -> u64 {
        let result = std::env::var(var_name)
            .unwrap_or_else(|_| default_value.to_string());
        result.parse::<u64>()
            .unwrap_or_else(|err| panic!("Failed to parse {}: {:?}", var_name, err))
    };

    let update_interval_sec: u64 = read_env_var(
        EVM_RUNTIME_UPDATE_INTERVAL_SEC,
        EVM_RUNTIME_UPDATE_INTERVAL_SEC_DEFAULT,
    );

    let running_to_suspended_time_sec: u64 = read_env_var(
        EVM_RUNTIME_RUNNING_TO_SUSPENDED_TIME_SEC,
        EVM_RUNTIME_RUNNING_TO_SUSPENDED_TIME_SEC_DEFAULT,
    );

    let suspended_to_stopped_time_sec: u64 = read_env_var(
        EVM_RUNTIME_SUSPENDED_TO_STOPPED_TIME_SEC,
        EVM_RUNTIME_SUSPENDED_TO_STOPPED_TIME_SEC_DEFAULT,
    );

    let stopped_to_dead_time_sec: u64 = read_env_var(
        EVM_RUNTIME_STOPPED_TO_DEAD_TIME_SEC,
        EVM_RUNTIME_STOPPED_TO_DEAD_TIME_SEC_SEC_DEFAULT,
    );

    let known_revisions = std::env::var(EVM_RUNTIME_KNOWN_REVISIONS)
        .unwrap_or_else(|_| EVM_RUNTIME_KNOWN_REVISIONS_DEFAULT.to_string());

    let db_config_tar = std::env::var(EVM_RUNTIME_DB_CONFIG_TAR)
        .unwrap();

    let network_name = std::env::var(EVM_RUNTIME_NETWORK_NAME).ok();

    EVMRuntimeConfig {
        docker_socket,
        docker_tout,
        docker_version_minor,
        docker_version_major,
        update_interval_sec,
        running_to_suspended_time_sec,
        suspended_to_stopped_time_sec,
        stopped_to_dead_time_sec,
        known_revisions,
        db_config_tar,
        evm_loader: evm_loader.clone(),
        token_mint: token_mint.clone(),
        chain_id,
        network_name,
    }
}

#[derive(std::fmt::Debug)]
pub struct Options {
    pub addr: String,
    pub db_config: DbConfig,
    pub evm_loader: solana_sdk::pubkey::Pubkey,
    pub web3_proxy: String,
    pub metrics_ip: Ipv4Addr,
    pub metrics_port: u16,
    pub evm_runtime_config: EVMRuntimeConfig,
}

pub fn read_config() -> Options {
    let read_env = |var_name: &str| std::env::var(var_name)
        .unwrap_or_else(|_| panic!("Failed to read env var {}", var_name));

    let path = read_env("DB_CONFIG");
    let path = path.as_str();
    let db_config: DbConfig = solana_cli_config::load_config_file(path).expect("load db-config error");

    let addr = read_env("LISTEN_ADDR");
    let evm_loader = read_env("EVM_LOADER");
    let evm_loader = Pubkey::from_str(evm_loader.as_str())
        .unwrap_or_else(|_| panic!("Failed to parse EVM_LOADER {}", evm_loader));
    let web3_proxy = read_env("WEB3_PROXY");
    let metrics_ip = read_env("METRICS_IP");
    let metrics_port = read_env("METRICS_PORT");
    let token_mint = read_env("NEON_TOKEN_MINT");
    let token_mint = Pubkey::from_str(token_mint.as_str())
        .unwrap_or_else(|_| panic!("Failed to parse NEON_TOKEN_MINT {}", token_mint));
    let chain_id = read_env("NEON_CHAIN_ID");
    let chain_id = chain_id.parse::<u16>()
        .unwrap_or_else(|_| panic!("Failed to parse NEON_CHAIN_ID"));
    let evm_runtime_config = read_evm_runtime_config(&evm_loader, &token_mint, chain_id);


    Options {
        addr,
        db_config,
        evm_loader: evm_loader.clone(),
        web3_proxy,
        metrics_ip: Ipv4Addr::from_str(metrics_ip.as_str())
            .unwrap_or_else(|_| panic!("Failed to parse METRICS_IP {}", metrics_ip)),
        metrics_port: metrics_port.parse::<u16>()
            .unwrap_or_else(|_| panic!("Failed to parse metrics port {}", metrics_port)),
        evm_runtime_config
    }
}