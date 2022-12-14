//!
//! Create a stream that can process incoming blocks and count how many its seen,
//! then use the count to check the account statuses of the agent.
//!

use std::sync::Arc;

use color_eyre::{eyre::eyre, Report};
use cw_croncat_core::types::AgentStatus;
use tokio::sync::Mutex;
use tracing::info;

use crate::{
    channels::{BlockStreamRx, ShutdownRx},
    grpc::GrpcClientService,
    utils::AtomicIntervalCounter,
};

///
/// Check every nth block with [`AtomicIntervalCounter`] for the current account
/// status of each account the agent watches.
///
pub async fn check_account_status_loop(
    mut block_stream_rx: BlockStreamRx,
    mut shutdown_rx: ShutdownRx,
    block_status: Arc<Mutex<AgentStatus>>,
    client: GrpcClientService,
) -> Result<(), Report> {
    let block_counter = AtomicIntervalCounter::new(10);
    let task_handle: tokio::task::JoinHandle<Result<(), Report>> = tokio::task::spawn(async move {
        while let Ok(block) = block_stream_rx.recv().await {
            block_counter.tick();
            if block_counter.is_at_interval() {
                info!(
                    "Checking agents statuses for block (height: {})",
                    block.header().height
                );
                let account_id = client.account_id();
                let agent = client
                    .execute(move |signer| {
                        let account_id = account_id.clone();
                        async move {
                            let agent = signer.get_agent(account_id.as_str()).await?;
                            Ok(agent)
                        }
                    })
                    .await?;
                let mut locked_status = block_status.lock().await;
                *locked_status = agent
                    .ok_or(eyre!("Agent unregistered during the loop"))?
                    .status;
                info!("Agent status: {:?}", *locked_status);
                if *locked_status == AgentStatus::Nominated {
                    info!(
                        "Checking in agent: {}",
                        client
                            .execute(|signer| async move {
                                signer.check_in_agent().await.map(|result| result.log)
                            })
                            .await?
                    );
                    let account_id = client.account_id();
                    let agent = client
                        .execute(|signer| {
                            let account_id = account_id.clone();
                            async move {
                                let agent = signer.get_agent(account_id.as_str()).await?;
                                Ok(agent)
                            }
                        })
                        .await?;
                    *locked_status = agent
                        .ok_or(eyre!("Agent unregistered during the loop"))?
                        .status;
                    info!("Agent status: {:?}", *locked_status);
                }
            }
        }
        Ok(())
    });

    tokio::select! {
        Ok(task) = task_handle => {task?}
        _ = shutdown_rx.recv() => {}
    }

    Ok(())
}
