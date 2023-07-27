use std::{num::ParseIntError, time::Duration};

use clap::{Parser, Subcommand};
use tracing::info;

#[derive(Parser, Debug)]
#[command(about)]
pub struct Params {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Measures the average time between the transaction’s processing by a validator and its
    /// appearance in Clickhouse.
    TransactionLatency {
        /// A number of transactions to check.
        num_transactions: usize,
        /// A timeout in seconds after that a transaction is ignored.
        #[arg(short, long, default_value = "10", value_parser = |arg: &str| -> Result<Duration, ParseIntError> {Ok(Duration::from_secs(arg.parse()?))})]
        timeout: Duration,
    },
    /// Measures accounts queries performance.
    QueryAccount {
        /// A number of queries to perform.
        num_queries: usize,
    },
}

impl Params {
    pub fn parse() -> Self {
        let params = <Self as Parser>::parse();
        info!("Starting with the following parameters:\n {params:#?}");
        params
    }
}
