use neon_test_framework::TestFramework;

// Submit a single transaction and check that it is present in ClickHouse.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transfer_transaction() {
    let mut tf = TestFramework::new().await;

    let (secret_key_1, address_1) = tf.make_wallet(1).await;
    let (_secret_key_2, address_2) = tf.make_wallet(0).await;
    let tx = tf
        .make_transfer_transaction(address_1, address_2, 1.into(), &secret_key_1)
        .await;
    assert!(!tf.is_known_transaction(tx.transaction_hash).await);

    tf.send_raw_transaction(tx.raw_transaction).await;
    assert!(tf.wait_for_transaction(tx.transaction_hash).await);
}
