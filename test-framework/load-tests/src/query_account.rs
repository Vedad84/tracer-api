use std::time::{Duration, Instant};

use tracing::info;

use neon_test_framework::TestFramework;

use crate::utils::median;

pub async fn measure(num_queries: usize) {
    info!("Measuring query account performance ({num_queries} tries)...");

    let tf = TestFramework::new();

    let keys = prepare_keys(&tf, num_queries).await;

    // Perform some requests, so the potential CLickHouse caching wouldn't affect the following
    // benchmarking.
    info!("Invalidating cache...");
    for offset in 0..100 {
        tf.account_pubkey(offset).await;
    }

    info!("Starting benchmark...");
    let mut times = Vec::with_capacity(num_queries);
    for key in keys {
        let now = Instant::now();
        tf.account(&key).await;
        times.push(now.elapsed());
    }

    times.sort_unstable();
    eprintln!("Measured {num_queries} queries");
    if times.len() > 0 {
        eprintln!("Min = {:?}", times.first().expect("Empty latency vec"));
        eprintln!("Max = {:?}", times.last().expect("Empty latency vec"));
        eprintln!(
            "Average = {:?}",
            times.iter().sum::<Duration>() / times.len() as u32
        );
        eprintln!("Median = {:?}", median(&times));
    }
}

/// Prepares a list of public keys that will be used in the benchmark.
async fn prepare_keys(tf: &TestFramework<()>, num_queries: usize) -> Vec<Vec<u8>> {
    info!("Preparing keys...");

    let num_accounts = tf.count_accounts().await;
    info!("Total number of accounts = {num_accounts}");
    // The number of accounts in the database should greatly exceed the number of queries.
    assert!(num_accounts > num_queries);

    // Prepare a list of public keys that will be used in the benchmark.
    let increment = num_accounts / num_queries;
    let mut offset = 0;
    let mut keys = Vec::with_capacity(num_queries);
    for _ in 0..num_queries {
        keys.push(tf.account_pubkey(offset).await);
        offset += increment;
    }
    keys
}
