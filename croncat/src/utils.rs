//!
//! Helpers for dealing with local agents.
//!

use cosm_orc::{
    config::cfg::Config, orchestrator::cosm_orc::CosmOrc, profilers::gas_profiler::GasProfiler,
};
use secp256k1::{rand, KeyPair, Secp256k1};

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

/// Generate a secp256k1 keypair from a random OS rng input
pub fn generate_keypair() -> KeyPair {
    let secp = Secp256k1::new();
    KeyPair::new(&secp, &mut rand::thread_rng())
}

pub const CONFIG_FILE: &str = "config.yaml";
pub const DEFAULT_AGENT_ID: &str = "agent";
pub const DERVIATION_PATH: &str = "m/44'/118'/0'/0/0";

///
/// Count block received from the stream.
///
pub struct AtomicIntervalCounter {
    count: Arc<AtomicU64>,
    check_interval: u64,
}

impl AtomicIntervalCounter {
    /// Create a new [`AtomicIntervalCounter`] and check every 10 samples.
    pub fn new(interval: u64) -> Self {
        Self {
            count: Arc::new(AtomicU64::default()),
            check_interval: interval,
        }
    }

    /// Increase the current offset into the sample.
    pub fn tick(&self) {
        self.count.fetch_add(1, Ordering::SeqCst);
    }

    /// Determine if the count is a multiple of the integer interval.
    pub fn is_at_interval(&self) -> bool {
        let current_count = self.count.load(Ordering::Relaxed);

        current_count > 0 && current_count % self.check_interval == 0
    }
}

pub fn setup_cosm_orc(croncat_addr: &str) -> Result<CosmOrc, color_eyre::Report> {
    let mut cosm_orc = CosmOrc::new(Config::from_yaml(CONFIG_FILE).unwrap())
        .unwrap()
        .add_profiler(Box::new(GasProfiler::new()));
    cosm_orc.contract_map.add_address("croncat", croncat_addr)?;
    Ok(cosm_orc)
}
