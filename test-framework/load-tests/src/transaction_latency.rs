use std::time::Duration;

use time::{OffsetDateTime, UtcOffset};
use tracing::{info, warn};

use neon_test_framework::TestFramework;

pub async fn measure(num_transactions: usize, timeout: Duration) {
    assert!(num_transactions < u32::MAX as usize);
    info!("Measuring transactions latency...");

    let mut tf = TestFramework::new().await;

    let (secret_key_1, address_1) = tf.make_wallet(10 * num_transactions).await;
    let (_secret_key_2, address_2) = tf.make_wallet(0).await;

    let mut timeouts = 0;
    let mut values = Vec::with_capacity(num_transactions);

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
        values.push(now - retrieved_time);
    }

    // TODO: Better report.
    values.sort_unstable();
    eprintln!(
        "Measured ({}/{num_transactions}) transactions ({timeouts} timeouts)",
        values.len(),
    );
    if values.len() > 0 {
        eprintln!("Min = {}", values.first().expect("Empty latency vec"));
        eprintln!("Max = {}", values.last().expect("Empty latency vec"));
        eprintln!(
            "Average = {}",
            values.iter().sum::<time::Duration>() / values.len() as u32
        );
        eprintln!("Median = {}", median(&values));
    }
}

fn median(values: &[time::Duration]) -> time::Duration {
    assert!(values.len() > 0);

    let middle = values.len() / 2;
    if values.len() % 2 == 1 {
        values[middle]
    } else {
        (values[middle - 1] + values[middle]) / 2
    }
}
