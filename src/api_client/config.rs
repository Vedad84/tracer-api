use std::str::FromStr;

use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone)]
pub struct Config {
    pub neon_api_url: String,
    pub chain_id: u64,
    pub token_mint: Pubkey,
}

pub fn read_api_client_config_from_enviroment() -> Config {
    let read_env = |var_name: &str| {
        std::env::var(var_name).unwrap_or_else(|_| panic!("Failed to read env var {}", var_name))
    };

    let neon_api_url =
        std::env::var("NEON_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());

    let token_mint = read_env("NEON_TOKEN_MINT");
    let token_mint = Pubkey::from_str(token_mint.as_str())
        .unwrap_or_else(|_| panic!("Failed to parse NEON_TOKEN_MINT {}", token_mint));
    let chain_id = read_env("NEON_CHAIN_ID");
    let chain_id = chain_id
        .parse::<u64>()
        .unwrap_or_else(|_| panic!("Failed to parse NEON_CHAIN_ID"));

    Config {
        neon_api_url,
        chain_id,
        token_mint,
    }
}
