//!
//! How to process and consume blocks from the chain.
//!

use std::sync::Arc;

use cw_croncat_core::types::AgentStatus;
use tokio::sync::Mutex;

use crate::{
    channels::{BlockStreamRx, ShutdownRx},
    errors::Report,
    grpc::GrpcSigner,
    logging::{info, warn},
};

///
/// Do work on blocks that are sent from the ws stream.
///
pub async fn tasks_loop(
    mut block_stream_rx: BlockStreamRx,
    mut shutdown_rx: ShutdownRx,
    signer: GrpcSigner,
    block_status: Arc<Mutex<AgentStatus>>,
) -> Result<(), Report> {
    let block_consumer_stream = tokio::task::spawn(async move {
        while let Ok(_block) = block_stream_rx.recv().await {
            let locked_status = block_status.lock().await;
            if *locked_status == AgentStatus::Active {
                let account_addr = signer.account_id().as_ref();
                let tasks = signer.get_agent_tasks(account_addr).await.unwrap();
                if tasks.is_some() {
                    if let Ok(proxy_call_res) = signer.proxy_call().await {
                        info!("Finished task: {}", proxy_call_res.log);
                    } else {
                        warn!("Something went wrong during proxy_call");
                    }
                } else {
                    info!("no tasks for this block");
                }
            }
        }
    });

    tokio::select! {
        _ = block_consumer_stream => {}
        _ = shutdown_rx.recv() => {}
    }

    Ok(())
}

pub async fn do_task_if_any(
    mut block_stream_rx: BlockStreamRx,
    mut shutdown_rx: ShutdownRx,
    signer: GrpcSigner,
) -> Result<(), Report> {
    let block_consumer_stream = tokio::task::spawn(async move {
        'grab_blocks: while let Ok(block) = block_stream_rx.recv().await {
            info!("Received block {:?}", block);
            let account_addr = signer.account_id().as_ref();
            let agent_active = signer
                .get_agent(account_addr)
                .await
                .unwrap()
                .map_or(false, |ag| ag.status == AgentStatus::Active);
            if agent_active {
                let tasks = signer.get_agent_tasks(account_addr).await.unwrap();
                if tasks.is_some() {
                    if let Ok(proxy_call_res) = signer.proxy_call().await {
                        info!("Finished task: {}", proxy_call_res.log);
                    } else {
                        warn!("Something went wrong during proxy_call");
                    }
                } else {
                    info!("no tasks for this block");
                }
            } else {
                warn!("agent is not registered");
            }
        }
    });

    tokio::select! {
        _ = block_consumer_stream => {}
        _ = shutdown_rx.recv() => {}
    }

    Ok(())
}
