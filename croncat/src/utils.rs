//!
//! Helpers for dealing with local agents.
//!

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use color_eyre::Report;
use delegate::delegate;
use tokio::task::JoinHandle;

use cw_croncat_core::msg::AgentTaskResponse;

pub const DEFAULT_AGENT_ID: &str = "agent";
pub const DERIVATION_PATH: &str = "m/44'/118'/0'/0/0";

// TODO: Change to request from deployed registries
pub const SUPPORTED_CHAIN_IDS: [&str; 7] = [
    "juno-1",
    "osmosis-1",
    "stargaze-1",
    "uni-5",
    "elfagar-1",
    "titus-1",
    "constantine-1",
];

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

pub fn sum_num_tasks(tasks: &AgentTaskResponse) -> u64 {
    (tasks.num_block_tasks
        + tasks.num_block_tasks_extra
        + tasks.num_cron_tasks
        + tasks.num_cron_tasks_extra)
        .into()
}

///
/// Flatten join handle results.
///
pub async fn flatten_join<T>(handle: JoinHandle<Result<T, Report>>) -> Result<T, Report> {
    match handle.await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(err)) => Err(err),
        Err(err) => Err(err.into()),
    }
}

///
/// Block wrapper
///
#[derive(Debug, Clone)]
pub struct Block {
    pub inner: tendermint::Block,
}

#[allow(dead_code)]
impl Block {
    delegate! {
        to self.inner {
            pub fn header(&self) -> &tendermint::block::Header;
            pub fn data(&self) -> &tendermint::abci::transaction::Data;
        }
    }
}

impl From<tendermint::Block> for Block {
    fn from(block: tendermint::Block) -> Self {
        Self { inner: block }
    }
}

impl Ord for Block {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.header().height.cmp(&other.header().height)
    }
}

impl PartialOrd for Block {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Block {
    fn eq(&self, other: &Self) -> bool {
        self.header().height == other.header().height
    }
}

impl Eq for Block {}
