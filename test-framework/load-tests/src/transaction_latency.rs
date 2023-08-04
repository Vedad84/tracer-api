use std::time::Duration;

use time::{OffsetDateTime, UtcOffset};
use tracing::{info, warn};

use neon_test_framework::TestFramework;

use crate::utils::median;

pub async fn measure(num_transactions: usize, timeout: Duration) {
    assert!(num_transactions < u32::MAX as usize);
    info!("Measuring transactions latency...");

    let mut tf = TestFramework::with_indexer().await;

    info!("Creating wallets...");
    let (secret_key_1, address_1) = tf.make_wallet(10 * num_transactions).await;
    let (_secret_key_2, address_2) = tf.make_wallet(0).await;

    info!("Starting benchmark...");
    let mut timeouts = 0;
    let mut times = Vec::with_capacity(num_transactions);

    for _ in 0..num_transactions {
        let tx = tf
            .make_transfer_transaction(address_1, address_2, 1.into(), &secret_key_1)
            .await;
        tf.send_raw_transaction(tx.raw_transaction).await;

        if !tf
            .wait_for_transaction(tx.transaction_hash, timeout, None)
            .await
        {
            warn!("Ignoring the timed out transaction");
            timeouts += 1;
            continue;
        }
        let now = OffsetDateTime::now_utc();
        let retrieved_time = tf
            .transaction_retrieved_time(tx.transaction_hash)
            .await
            .to_offset(UtcOffset::UTC);
        times.push(now - retrieved_time);
    }

    times.sort_unstable();
    let times: Vec<_> = times.into_iter().map(|t| t.unsigned_abs()).collect();
    eprintln!(
        "Measured ({}/{num_transactions}) transactions ({timeouts} timeouts)",
        times.len(),
    );
    if times.len() > 0 {
        eprintln!("Min = {:?}", times.first().expect("Empty times vec"));
        eprintln!("Max = {:?}", times.last().expect("Empty times vec"));
        eprintln!(
            "Average = {:?}",
            times.iter().sum::<Duration>() / times.len() as u32
        );
        eprintln!("Median = {:?}", median(&times));
    }
}
