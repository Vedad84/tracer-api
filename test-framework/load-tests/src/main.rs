mod log;
mod params;
mod query_account;
mod transaction_latency;
mod utils;

use crate::params::{Command, Params};

#[tokio::main]
async fn main() {
    log::init();

    let params = Params::parse();
    match params.command {
        Command::TransactionLatency {
            num_transactions,
            timeout,
        } => transaction_latency::measure(num_transactions, timeout).await,
        Command::QueryAccount { num_queries } => query_account::measure(num_queries).await,
    }
}
