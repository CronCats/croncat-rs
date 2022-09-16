//!
//! How to process and consume blocks from the chain.
//!

use std::sync::Arc;

use cosmos_sdk_proto::tendermint::google::protobuf::Timestamp;
use cw_croncat_core::types::{AgentStatus, Boundary};
use tokio::sync::Mutex;

use crate::{
    channels::{BlockStreamRx, ShutdownRx},
    errors::Report,
    grpc::GrpcSigner,
    logging::{info, warn},
    utils::{sum_num_tasks, AtomicIntervalCounter},
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
            let is_active = *locked_status == AgentStatus::Active;
            // unlocking it ASAP
            std::mem::drop(locked_status);
            if is_active {
                let account_addr = signer.account_id().as_ref();
                let tasks = signer.get_agent_tasks(account_addr).await.unwrap();
                if let Some(tasks) = tasks {
                    println!("{:?}", tasks);
                    for _ in 0..sum_num_tasks(&tasks) {
                        if let Ok(proxy_call_res) = signer.proxy_call(None).await {
                            info!("Finished task: {}", proxy_call_res.log);
                        } else {
                            warn!("Something went wrong during proxy_call");
                        }
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

pub async fn rules_loop(
    mut block_stream_rx: BlockStreamRx,
    mut shutdown_rx: ShutdownRx,
    signer: GrpcSigner,
    block_status: Arc<Mutex<AgentStatus>>,
) -> Result<(), Report> {
    let mut task_with_rules = signer.fetch_rules().await?;
    let block_counter = AtomicIntervalCounter::new(10);

    let block_consumer_stream = tokio::task::spawn(async move {
        while let Ok(block) = block_stream_rx.recv().await {
            block_counter.tick();
            if block_counter.is_at_interval() {
                task_with_rules = signer.fetch_rules().await.expect("Failed to fetch rules");
            }
            let locked_status = block_status.lock().await;
            let is_active = *locked_status == AgentStatus::Active;
            // unlocking it ASAP
            std::mem::drop(locked_status);
            if is_active {
                let time: Timestamp = block.header.time.into();
                let time_nanos = time.seconds as u64 * 1_000_000_000 + time.nanos as u64;

                let mut finished_tasks = vec![];
                for (task_hash, task) in task_with_rules.iter() {
                    let in_boundary = match task.boundary {
                        Some(Boundary::Height { start, end }) => {
                            let height = block.header.height.value();
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
                            .check_rules(task.rules.clone().unwrap())
                            .await
                            .expect("Failed to query rules");
                        if rules_ready {
                            let res = signer.proxy_call(Some(task.task_hash.clone())).await;
                            if let Ok(proxy_call_res) = res {
                                finished_tasks.push(task_hash.clone());
                                info!("Finished task: {}", proxy_call_res.log);
                            } else {
                                warn!("Something went wrong during proxy_call");
                            }
                        }
                    }
                }

                for finished in finished_tasks {
                    task_with_rules.remove(&finished);
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
        while (block_stream_rx.recv().await).is_ok() {
            let account_addr = signer.account_id().as_ref();
            let agent_active = signer
                .get_agent(account_addr)
                .await
                .unwrap()
                .map_or(false, |ag| ag.status == AgentStatus::Active);
            if agent_active {
                let tasks = signer.get_agent_tasks(account_addr).await.unwrap();
                if tasks.is_some() {
                    if let Ok(proxy_call_res) = signer.proxy_call(None).await {
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
