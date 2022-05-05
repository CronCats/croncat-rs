use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use color_eyre::Report;
use tracing::info;

use crate::channels::{BlockStreamRx, ShutdownRx};

pub struct BlockCounter {
    count: Arc<AtomicU64>,
    check_interval: u64,
}

impl BlockCounter {
    pub fn new() -> Self {
        Self {
            count: Arc::new(AtomicU64::default()),
            check_interval: 10,
        }
    }

    pub fn tick(&self) {
        self.count.fetch_add(1, Ordering::SeqCst);
    }

    pub fn is_at_interval(&self) -> bool {
        let current_count = self.count.load(Ordering::Relaxed);

        current_count == 0 || current_count % self.check_interval == 0
    }
}

pub async fn check_account_status_loop(
    mut block_stream_rx: BlockStreamRx,
    mut shutdown_rx: ShutdownRx,
) -> Result<(), Report> {
    let block_counter = BlockCounter::new();
    let task_handle = tokio::task::spawn(async move {
        while let Ok(block) = block_stream_rx.recv().await {
            if block_counter.is_at_interval() {
                info!(
                    "Checking agent status for block (height: {})",
                    block.header.height
                );
            }

            block_counter.tick()
        }
    });

    tokio::select! {
        _ = task_handle => {}
        _ = shutdown_rx.recv() => {}
    }

    Ok(())
}
