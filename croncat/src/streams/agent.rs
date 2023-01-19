//!
//! Create a stream that can process incoming blocks and count how many its seen,
//! then use the count to check the account statuses of the agent.
//!

use color_eyre::{eyre::eyre, Report};
use cw_croncat_core::types::AgentStatus;
use std::process::exit;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::config::ChainConfig;
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
    chain_config: ChainConfig,
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
                    let agent = client
                        .execute(|signer| {
                            let account_id = client.account_id();
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

                if let Some(threshold) = chain_config.threshold {
                    // Check the agent's balance to make sure it's not falling below a threshold
                    let account_id = client.account_id();
                    let account_str = account_id.as_str();
                    let agent_balance = client
                        .execute(move |signer| async move {
                            let agent = signer.query_native_balance(account_str).await?;
                            Ok(agent)
                        })
                        .await?;
                    let agent_native_balance = agent_balance.amount;
                    let denom = agent_balance.denom;

                    // If agent balance is too low and the agent has some native coins in the manager contract
                    // call withdraw_reward
                    // If manager balance is zero, exit
                    if agent_native_balance < threshold as u128 {
                        let agent = client
                            .execute(move |signer| async move {
                                let agent = signer.get_agent(account_str).await?;
                                Ok(agent)
                            })
                            .await?;
                        let reward_balance = agent
                            .ok_or(eyre!("Agent unregistered during the loop"))?
                            .balance
                            .native
                            .into_iter()
                            .find(|c| c.denom == denom.to_string())
                            .unwrap_or_default()
                            .amount;
                        if !reward_balance.is_zero() {
                            info!("Automatically withdrawing agent reward");
                            let result = client
                                .execute(move |signer| async move {
                                    let agent = signer.withdraw_reward().await?;
                                    Ok(agent)
                                })
                                .await?;
                            let log = result.log;
                            info!("Log: {log}");

                            let native_balance_after_withdraw = client
                                .execute(move |signer| async move {
                                    let agent = signer.query_native_balance(account_str).await?;
                                    Ok(agent)
                                })
                                .await?
                                .amount;
                            if native_balance_after_withdraw < threshold as u128 {
                                error!("Not enough balance to continue, the agent in required to have {} {}, current balance: {} {}", threshold, denom, native_balance_after_withdraw, denom);
                                error!("Stopping the agent");
                                exit(1);
                            }
                        } else {
                            error!("Not enough balance to continue, the agent in required to have {} {}, current balance: {} {}", threshold, denom, agent_native_balance, denom);
                            error!("Stopping the agent");
                            exit(1);
                        }
                    }
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
