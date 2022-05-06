//!
//! Create a stream that can process incoming blocks and count how many its seen,
//! then use the count to check the account statuses of the agent.
//!

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use color_eyre::Report;
use tracing::info;

use crate::channels::{BlockStreamRx, ShutdownRx};

///
/// Count block received from the stream.
///
pub struct BlockCounter {
    count: Arc<AtomicU64>,
    check_interval: u64,
}

impl BlockCounter {
    /// Create a new [`BlockCounter`] and check every 10 samples.
    pub fn new() -> Self {
        Self {
            count: Arc::new(AtomicU64::default()),
            check_interval: 10,
        }
    }

    /// Increase the current offset into the sample.
    pub fn tick(&self) {
        self.count.fetch_add(1, Ordering::SeqCst);
    }

    /// Determine if the count is a multiple of the integer interval.
    pub fn is_at_interval(&self) -> bool {
        let current_count = self.count.load(Ordering::Relaxed);

        current_count > 0 && current_count % (self.check_interval - 1) == 0
    }
}

///
/// Check every nth block for the current account
/// status of each account the agent watches.
///
pub async fn check_account_status_loop(
    mut block_stream_rx: BlockStreamRx,
    mut shutdown_rx: ShutdownRx,
) -> Result<(), Report> {
    let block_counter = BlockCounter::new();
    let task_handle = tokio::task::spawn(async move {
        while let Ok(block) = block_stream_rx.recv().await {
            block_counter.tick();
            if block_counter.is_at_interval() {
                info!(
                    "Checking agents statuses for block (height: {})",
                    block.header.height
                );
            }
        }
    });

    tokio::select! {
        _ = task_handle => {}
        _ = shutdown_rx.recv() => {}
    }

    Ok(())
}
