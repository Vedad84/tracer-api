use {
    crate::db::DbClient,
    lazy_static::lazy_static,
    prometheus::{ Encoder, gather, IntCounter, Histogram, HistogramVec, HistogramOpts, TextEncoder,
                  Registry, IntCounterVec, Opts },
    std::{ net::Ipv4Addr, sync::Arc },
    tokio::{ self, time::Instant },
    tracing::{info, warn},
    warp::{ Filter, Reply, Rejection },
    web3,
};

lazy_static!(
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

    pub static ref SLOT_DIFFERENCE: Histogram = Histogram::with_opts(
        HistogramOpts::new("neon_tracer_slot_difference", "Difference between DB and Web3 slot number"),
    )
    .expect("Failed create metric: neon_tracer_slot_difference");
);

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
    warp::serve(metrics_route)
        .run((ip, port))
        .await;
}

static MONITORING_INTERVAL_SEC: &str = "MONITORING_INTERVAL_SEC";
static MONITORING_INTERVAL_SEC_DEFAULT: &str = "60";

fn register_metrics() {
    info!("Registering metrics...");

    REGISTRY.register(Box::new(INCOMING_REQUESTS.clone()))
        .expect("neon_tracer_incoming_request metric not registered");

    REGISTRY.register(Box::new(FAILED_REQUESTS.clone()))
        .expect("neon_tracer_failed_request metric not registered");

    REGISTRY.register(Box::new(RESPONSE_TIME_COLLECTOR.clone()))
        .expect("neon_tracer_response_time metric not registered");

    REGISTRY.register(Box::new(SLOT_DIFFERENCE.clone()))
        .expect("neon_tracer_slot_difference metric not registered");
}

pub async fn start_monitoring(
    db: Arc<DbClient>,
    proxy: Arc<web3::Web3<web3::transports::Http>>,
    metrics_ip: Ipv4Addr,
    metrics_port: u16,
) {
    let monitoring_interval_sec = std::env::var(MONITORING_INTERVAL_SEC)
        .unwrap_or(MONITORING_INTERVAL_SEC_DEFAULT.to_string());

    let monitoring_interval_sec = monitoring_interval_sec.parse::<u64>()
        .map_err(|err| warn!("Failed to parse MONITORING_INTERVAL_SEC = {:?}", err)).unwrap();

    register_metrics();
    start_metrics_server(metrics_ip, metrics_port).await;

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(monitoring_interval_sec)).await;
        proxy.eth().block_number().await
            .map(|proxy_block_number|{
                db.get_slot().map(|db_slot| {
                    SLOT_DIFFERENCE.observe((proxy_block_number.as_u64() - db_slot) as f64);
                    ()
                })
            })
            .map_err(|err| warn!("Failed to submit neon_tracer_slot_difference: {:?}", err));
    }
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
    let elapsed = elapsed.as_secs() as f64 + elapsed.subsec_micros() as f64 / 1000000.0;
    RESPONSE_TIME_COLLECTOR
        .with_label_values(&[req_tag])
        .observe(elapsed);
}