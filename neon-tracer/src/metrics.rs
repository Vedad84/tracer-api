use {
    crate::stop_handle::StopHandle,
    lazy_static::lazy_static,
    neon_cli_lib::types::TracerDb,
    prometheus::{
        gather, Encoder, Histogram, HistogramOpts, HistogramVec, IntCounterVec, Opts, Registry,
        TextEncoder,
    },
    std::{net::Ipv4Addr, sync::Arc},
    tokio::{self, sync::mpsc::Receiver, time::Instant},
    tracing::{info, warn},
    warp::{Filter, Rejection, Reply},
};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref INCOMING_REQUESTS: IntCounterVec = IntCounterVec::new(
        Opts::new("neon_tracer_incoming_requests", "Incoming Requests"),
        &["req_type"]
    )
    .expect("Failed create metric: neon_tracer_incoming_requests");
    pub static ref FAILED_REQUESTS: IntCounterVec = IntCounterVec::new(
        Opts::new("neon_tracer_failed_requests", "Failed Requests"),
        &["req_types"]
    )
    .expect("Failed create metric: neon_tracer_failed_requests");
    pub static ref RESPONSE_TIME_COLLECTOR: HistogramVec = HistogramVec::new(
        HistogramOpts::new("neon_tracer_response_time", "Response Times"),
        &["req_type"]
    )
    .expect("Failed create metric: neon_tracer_response_time");
    pub static ref SLOT_DIFFERENCE: Histogram = Histogram::with_opts(HistogramOpts::new(
        "neon_tracer_slot_difference",
        "Difference between DB and Web3 slot number"
    ),)
    .expect("Failed create metric: neon_tracer_slot_difference");
}

async fn metrics_handler() -> Result<impl Reply, Rejection> {
    let encoder = TextEncoder::new();

    // gather custom metrics
    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&REGISTRY.gather(), &mut buffer) {
        warn!("could not encode custom metrics: {:?}", e);
    };
    let res_custom = match String::from_utf8(buffer.clone()) {
        Ok(v) => v,
        Err(e) => {
            warn!("custom metrics could not be from_utf8'd: {:?}", e);
            String::default()
        }
    };
    buffer.clear();

    // gather system metrics
    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&gather(), &mut buffer) {
        warn!("could not encode prometheus metrics: {:?}", e);
    };
    let mut res = match String::from_utf8(buffer.clone()) {
        Ok(v) => v,
        Err(e) => {
            warn!("prometheus metrics could not be from_utf8'd: {:?}", e);
            String::default()
        }
    };
    buffer.clear();

    res.push_str(&res_custom);
    Ok(res)
}

async fn start_metrics_server(ip: Ipv4Addr, port: u16) {
    let metrics_route = warp::path!("metrics").and_then(metrics_handler);
    info!("Metrics server started on port {}", port);
    warp::serve(metrics_route).run((ip, port)).await;
}

static MONITORING_INTERVAL_SEC: &str = "MONITORING_INTERVAL_SEC";
static MONITORING_INTERVAL_SEC_DEFAULT: &str = "60";

fn register_metrics() {
    info!("Registering metrics...");

    REGISTRY
        .register(Box::new(INCOMING_REQUESTS.clone()))
        .expect("neon_tracer_incoming_request metric not registered");

    REGISTRY
        .register(Box::new(FAILED_REQUESTS.clone()))
        .expect("neon_tracer_failed_request metric not registered");

    REGISTRY
        .register(Box::new(RESPONSE_TIME_COLLECTOR.clone()))
        .expect("neon_tracer_response_time metric not registered");

    REGISTRY
        .register(Box::new(SLOT_DIFFERENCE.clone()))
        .expect("neon_tracer_slot_difference metric not registered");
}

pub async fn run_monitoring(
    tracer_db: TracerDb,
    proxy: Arc<web3::Web3<web3::transports::Http>>,
    metrics_ip: Ipv4Addr,
    metrics_port: u16,
    mut stop_rcv: Receiver<()>,
) {
    info!("Starting monitoring...");
    let monitoring_interval_sec = std::env::var(MONITORING_INTERVAL_SEC)
        .unwrap_or_else(|_| MONITORING_INTERVAL_SEC_DEFAULT.to_string());

    let monitoring_interval_sec = monitoring_interval_sec
        .parse::<u64>()
        .map_err(|err| warn!("Failed to parse MONITORING_INTERVAL_SEC = {:?}", err))
        .unwrap();

    register_metrics();
    tokio::spawn(start_metrics_server(metrics_ip, metrics_port));

    let sleep_int = std::time::Duration::from_secs(monitoring_interval_sec);
    let mut interval = tokio::time::interval(sleep_int);
    interval.tick().await;
    loop {
        tokio::select! {
            _ = interval.tick() =>
                match proxy.eth().block_number().await {
                    Ok(proxy_block_number) => {
                        tracer_db.get_latest_block().await.map(|db_slot| {
                            SLOT_DIFFERENCE.observe((proxy_block_number.as_u64() - db_slot) as f64);
                        })
                            .map_err(|err| warn!("Failed to submit neon_tracer_slot_difference: {:?}", err))
                            .ok();
                    }
                    Err(err) => warn!("Failed to submit neon_tracer_slot_difference: {:?}", err),
                },

            _ = stop_rcv.recv() => {
                break;
            }
        }
    }

    info!("Monitoring stopped.");
}

pub fn start_monitoring(
    tracer_db: TracerDb,
    proxy: Arc<web3::Web3<web3::transports::Http>>,
    metrics_ip: Ipv4Addr,
    metrics_port: u16,
) -> StopHandle {
    let (stop_snd, stop_rcv) = tokio::sync::mpsc::channel::<()>(1);
    StopHandle::new(
        tokio::spawn(run_monitoring(
            tracer_db,
            proxy,
            metrics_ip,
            metrics_port,
            stop_rcv,
        )),
        stop_snd,
    )
}

pub fn report_incoming_request(req_tag: &str) -> Instant {
    INCOMING_REQUESTS.with_label_values(&[req_tag]).inc();
    Instant::now()
}

pub fn report_request_finished(started: Instant, req_tag: &str, success: bool) {
    if !success {
        FAILED_REQUESTS.with_label_values(&[req_tag]).inc();
    }

    let elapsed = started.elapsed();
    let elapsed = elapsed.as_secs() as f64 + f64::from(elapsed.subsec_micros()) / 1_000_000.0;
    RESPONSE_TIME_COLLECTOR
        .with_label_values(&[req_tag])
        .observe(elapsed);
}
