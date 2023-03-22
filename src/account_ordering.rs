use {
    crate::stop_handle::StopHandle,
    log::{ info, warn },
    neon_cli_lib::types::TracerDb,
    tokio::{ self, sync::mpsc::Receiver },
};

async fn run_account_ordering(
    tracer_db: TracerDb,
    mut stop_rcv: Receiver<()>,
) {
    info!("Starting account ordering...");
    let sleep_int = std::time::Duration::from_secs(1);
    let mut interval = tokio::time::interval(sleep_int);
    let stmt = tracer_db.client.prepare("CALL public.order_accounts()")
        .await.expect("Failed to prepare account ordering DB query");

    interval.tick().await;
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let res = tracer_db.client.query(&stmt, &[]).await;
                if let Err(res) = res {
                    warn!("Failed to run order_accounts: {:?}", res);
                }
            }
            _ = stop_rcv.recv() => {
                break;
            }
        }
    }

    info!("Monitoring stopped.");
}

pub fn start_account_ordering(tracer_db: TracerDb) -> StopHandle {
    let (stop_snd, stop_rcv) = tokio::sync::mpsc::channel::<()>(1);
    StopHandle::new(
        tokio::spawn( run_account_ordering(tracer_db, stop_rcv)),
        stop_snd,
    )
}