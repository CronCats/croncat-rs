use lazy_static::lazy_static;
use tracing::{trace, warn};

lazy_static! {
    static ref UPTIME_MONITOR_PING_URL: Option<String> =
        std::env::var("UPTIME_MONITOR_PING_URL").ok();
}

pub async fn ping_uptime_monitor() {
    if UPTIME_MONITOR_PING_URL.as_ref().is_some() {
        trace!("Pinging uptime monitor...");
        let _ = reqwest::get(UPTIME_MONITOR_PING_URL.as_ref().unwrap())
            .await
            .map_err(|err| {
                warn!("Failed to ping uptime monitor: {}", err);
            });
    }
}
