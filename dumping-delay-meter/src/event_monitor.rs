use {
    crate::stop_handle::StopHandle,
    arrayref::array_ref,
    std::{str::FromStr, sync::Arc},
    tokio_postgres::Client as PgClient,
    tracing::{info, warn},
    web3::{
        contract::Contract,
        transports::Http,
        types::{Address, FilterBuilder, H256, U256, U64},
        Web3,
    },
};

pub struct EventMonitor {
    eth: web3::api::Eth<Http>,
    factory: Address,
    test_contract_abi: web3::ethabi::Contract,
    topic: H256,
    pg_client: Arc<PgClient>,
}

const NUM_RETRIES: u32 = 30;
const NEW_CONTRACT_EVENT_SIG: &str =
    "fcf9a0c9dedbfcd1a047374855fc36baaf605bd4f4837802a0cc938ba1b5f302";

async fn read_latest_block(eth: &web3::api::Eth<Http>) -> web3::error::Result<U64> {
    let mut counter = NUM_RETRIES;
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
    interval.tick().await; // first tick completes immediately

    loop {
        let result = eth.block_number().await;
        if result.is_ok() {
            return result;
        }

        counter -= 1;
        if counter == 0 {
            return result;
        }

        interval.tick().await;
    }
}

async fn read_events(
    eth: &web3::api::Eth<Http>,
    from_block: U64,
    to_block: U64,
    contract: Address,
    topic: H256,
) -> web3::error::Result<Vec<web3::types::Log>> {
    info!("Reading logs from {} to {}", from_block, to_block);
    let mut counter = NUM_RETRIES;
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
    interval.tick().await; // first tick completes immediately

    loop {
        let result = eth
            .logs(
                FilterBuilder::default()
                    .from_block(from_block.into())
                    .to_block(to_block.into())
                    .address(vec![contract])
                    .topics(Some(vec![topic]), None, None, None)
                    .build(),
            )
            .await;

        if result.is_ok() {
            return result;
        }

        counter -= 1;
        if counter == 0 {
            return result;
        }

        interval.tick().await;
    }
}

async fn monitor_iteration(
    monitor: &EventMonitor,
    latest_block: U64,
    read_delay_spread_slots: u64,
) -> U64 {
    let new_latest_block = read_latest_block(&monitor.eth)
        .await
        .expect("Unable to read latest block number");
    if new_latest_block != latest_block {
        if let Ok(events) = read_events(
            &monitor.eth,
            latest_block + 1,
            new_latest_block,
            monitor.factory,
            monitor.topic,
        )
        .await
        {
            for log in events {
                if let Some(log_block_number) = log.block_number {
                    let test_contract_addr =
                        web3::types::Address::from(array_ref!(log.data.0, 12, 20));
                    info!(
                        "New Test contract {:?} created at block {:?}",
                        test_contract_addr, log_block_number
                    );
                    let eth = monitor.eth.clone();
                    let test_contract_abi = monitor.test_contract_abi.clone();
                    let pg_client = monitor.pg_client.clone();
                    tokio::spawn(async move {
                        let rel_delay_slots = rand::random::<u64>() % read_delay_spread_slots;
                        let abs_delay_slots =
                            new_latest_block.as_u64() - log_block_number.as_u64() + rel_delay_slots;
                        let delay_ms = rel_delay_slots * 400;
                        info!("Delay read operation for {} ms", delay_ms);
                        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                        info!("Creating contract object...");
                        let test_contract: Contract<Http> =
                            Contract::new(eth, test_contract_addr, test_contract_abi);
                        let log_block_number = web3::types::BlockId::Number(
                            web3::types::BlockNumber::Number(log_block_number),
                        );
                        let result: web3::contract::Result<U256> = test_contract
                            .query(
                                "getCreationBlock",
                                (),
                                Address::default(),
                                web3::contract::Options::default(),
                                log_block_number,
                            )
                            .await;
                        info!("getCreationBlock result: {:?}", result);

                        match result {
                            Ok(_) => {
                                if let Err(pg_err) = pg_client.execute(
                                    "INSERT INTO delay_stat(delay_slots, success, err) VALUES($1, True, NULL);",
                                    &[&(abs_delay_slots as i64)],
                                ).await {
                                    warn!("Failed to store stats: {:?}", pg_err);
                                }
                            }
                            Err(err) => {
                                if let Err(pg_err) = pg_client.execute(
                                    "INSERT INTO delay_stat(delay_slots, success, err) VALUES($1, False, $2);",
                                    &[&(abs_delay_slots as i64), &err.to_string()],
                                ).await {
                                    warn!("Failed to store stats: {:?}", pg_err);
                                }
                            }
                        }
                    });
                }
            }

            new_latest_block
        } else {
            latest_block
        }
    } else {
        latest_block
    }
}

impl EventMonitor {
    pub fn new(
        web3_client: &Web3<Http>,
        factory: Address,
        test_contract_abi: web3::ethabi::Contract,
        pg_client: Arc<PgClient>,
    ) -> Self {
        Self {
            eth: web3_client.eth(),
            factory,
            test_contract_abi,
            topic: H256::from_str(NEW_CONTRACT_EVENT_SIG)
                .unwrap_or_else(|_| panic!("Failed to parse topic: {NEW_CONTRACT_EVENT_SIG}")),
            pg_client,
        }
    }

    pub fn run(self, monitoring_period_ms: u64, read_delay_spread_slots: u64) -> StopHandle {
        info!("Starting Event Monitor...");
        let (stop_snd, mut stop_rcv) = tokio::sync::mpsc::channel::<()>(1);
        StopHandle::new(
            tokio::spawn(async move {
                let sleep_int = std::time::Duration::from_millis(monitoring_period_ms);
                let mut interval = tokio::time::interval(sleep_int);
                interval.tick().await;
                let mut latest_block = read_latest_block(&self.eth)
                    .await
                    .expect("Unable to read latest block number");

                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            latest_block = monitor_iteration(&self, latest_block, read_delay_spread_slots).await;
                        }
                        _ = stop_rcv.recv() => {
                            break;
                        }
                    }
                }

                info!("Event Monitor stopped");
            }),
            stop_snd,
        )
    }
}
