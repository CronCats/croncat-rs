//!
//! Create a stream that can process incoming blocks and count how many its seen,
//! then use the count to check the account statuses of the agent.
//!

use std::sync::Arc;

use color_eyre::Report;
use cw_croncat_core::types::AgentStatus;
use tokio::sync::Mutex;
use tracing::info;

use crate::{
    channels::{BlockStreamRx, ShutdownRx},
    grpc::GrpcSigner,
    utils::AtomicIntervalCounter,
};

///
/// Check every nth block with [`AtomicIntervalCounter`] for the current account
/// status of each account the agent watches.
///
pub async fn check_account_status_loop(
    mut block_stream_rx: BlockStreamRx,
    mut shutdown_rx: ShutdownRx,
    signer: GrpcSigner,
) -> Result<(), Report> {
    let block_status = Arc::new(Mutex::new(AgentStatus::Nominated));
    let block_counter = AtomicIntervalCounter::new(10);
    let task_handle = tokio::task::spawn(async move {
        while let Ok(block) = block_stream_rx.recv().await {
            block_counter.tick();
            if block_counter.is_at_interval() {
                info!(
                    "Checking agents statuses for block (height: {})",
                    block.header.height
                );
                let signer = signer.clone();
                let account_id = signer.account_id().unwrap();
                let account_addr = account_id.as_ref();
                let agent = signer.get_agent(account_addr).await.unwrap();
                let mut locked_status = block_status.lock().await;
                *locked_status = agent.unwrap().status;
                println!("status:{:?}", *locked_status);
            }
        }
    });

    tokio::select! {
        _ = task_handle => {}
        _ = shutdown_rx.recv() => {}
    }

    Ok(())
}
