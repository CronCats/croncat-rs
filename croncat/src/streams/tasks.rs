//!
//! How to process and consume blocks from the chain.
//!

use std::sync::Arc;

use crate::system::Block;
use cosmos_sdk_proto::tendermint::google::protobuf::Timestamp;
use cw_croncat_core::types::{AgentStatus, Boundary};
use tokio::{
    sync::{broadcast, Mutex},
    task::JoinHandle,
};

use crate::{
    channels::ShutdownRx,
    errors::{eyre, Report},
    grpc::GrpcSigner,
    logging::{info, warn},
    monitor::ping_uptime_monitor,
    utils::sum_num_tasks,
};

///
/// Do work on blocks that are sent from the ws stream.
///
pub async fn tasks_loop(
    mut block_stream_rx: broadcast::Receiver<Block>,
    mut shutdown_rx: ShutdownRx,
    signer: GrpcSigner,
    block_status: Arc<Mutex<AgentStatus>>,
) -> Result<(), Report> {
    let block_consumer_stream: JoinHandle<Result<(), Report>> = tokio::task::spawn(async move {
        while let Ok(block) = block_stream_rx.recv().await {
            let locked_status = block_status.lock().await;
            let is_active = *locked_status == AgentStatus::Active;
            // unlocking it ASAP
            std::mem::drop(locked_status);
            if is_active {
                let account_addr = signer.account_id().as_ref();
                let tasks = signer
                    .get_agent_tasks(account_addr)
                    .await
                    .map_err(|err| eyre!("Failed to get agent tasks: {}", err))?;

                ping_uptime_monitor().await;

                if let Some(tasks) = tasks {
                    println!("{:?}", tasks);
                    for _ in 0..sum_num_tasks(&tasks) {
                        match signer.proxy_call(None).await {
                            Ok(proxy_call_res) => {
                                info!("Finished task: {}", proxy_call_res.log);
                            }
                            Err(err) => {
                                warn!("Something went wrong during proxy_call: {}", err);
                            }
                        }
                    }
                } else {
                    info!("No tasks for block (height: {})", block.header().height);
                }
            }
        }

        Ok(())
    });

    tokio::select! {
        _ = block_consumer_stream => {}
        _ = shutdown_rx.recv() => {}
    }

    Ok(())
}

pub async fn rules_loop(
    mut block_stream_rx: broadcast::Receiver<Block>,
    mut shutdown_rx: ShutdownRx,
    signer: GrpcSigner,
    block_status: Arc<Mutex<AgentStatus>>,
) -> Result<(), Report> {
    let block_consumer_stream: JoinHandle<Result<(), Report>> = tokio::task::spawn(async move {
        while let Ok(block) = block_stream_rx.recv().await {
            let tasks_with_rules = signer
                .fetch_rules()
                .await
                .map_err(|err| eyre!("Failed to fetch rules: {}", err))?;

            let locked_status = block_status.lock().await;
            let is_active = *locked_status == AgentStatus::Active;
            // unlocking it ASAP
            std::mem::drop(locked_status);
            if is_active {
                let time: Timestamp = block.header().time.into();
                let time_nanos = time.seconds as u64 * 1_000_000_000 + time.nanos as u64;

                for task in tasks_with_rules.iter() {
                    let in_boundary = match task.boundary {
                        Some(Boundary::Height { start, end }) => {
                            let height = block.header().height.value();
                            start.map_or(true, |s| s.u64() >= height)
                                && end.map_or(true, |e| e.u64() <= height)
                        }
                        Some(Boundary::Time { start, end }) => {
                            start.map_or(true, |s| s.nanos() >= time_nanos)
                                && end.map_or(true, |e| e.nanos() >= time_nanos)
                        }
                        None => true,
                    };
                    if in_boundary {
                        let (rules_ready, _) = signer
                            .check_rules(task.rules.clone().ok_or_else(|| eyre!("No rules"))?)
                            .await
                            .map_err(|err| eyre!("Failed to query rules: {}", err))?;
                        if rules_ready {
                            let res = signer.proxy_call(Some(task.task_hash.clone())).await;
                            if let Ok(proxy_call_res) = res {
                                info!("Finished task: {}", proxy_call_res.log);
                            } else {
                                warn!("Something went wrong during proxy_call");
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    });
    tokio::select! {
        _ = block_consumer_stream => {}
        _ = shutdown_rx.recv() => {}
    }

    Ok(())
}
