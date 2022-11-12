//!
//! The croncat system daemon.
//!

use std::sync::Arc;
use std::time::Duration;

use cosmrs::bip32::{secp256k1::ecdsa::SigningKey, ExtendedPrivateKey};
use croncat_pipeline::{try_flat_join, Dispatcher, Sequencer};
use futures_util::{stream::FuturesUnordered, StreamExt};
use tokio::{
    sync::{broadcast, mpsc, Mutex},
    task::JoinHandle,
};
use tokio_retry::{strategy::FixedInterval, Retry};
use tracing::log::error;

use crate::{
    channels::ShutdownTx,
    config::ChainConfig,
    errors::{eyre, Report},
    grpc::GrpcClientService,
    logging::info,
    streams::{agent, polling, tasks},
    tokio,
};

pub mod service;

pub use service::DaemonService;

///
/// Kick off the croncat daemon
///
pub async fn run(
    chain_id: &String,
    shutdown_tx: &ShutdownTx,
    config: &ChainConfig,
    key: &ExtendedPrivateKey<SigningKey>,
    with_rules: bool,
) -> Result<(), Report> {
    // Create a channel for block sources
    let (block_source_tx, block_source_rx) = mpsc::unbounded_channel();
    // Create a FuturesUnordered to handle all the block sources
    let mut block_stream_tasks = FuturesUnordered::new();

    // Setup the chain client.
    let client = GrpcClientService::new(config.clone(), key.clone());

    // Get the status of the agent
    let account_id = client.account_id();
    let status = client
        .execute(|signer| async move {
            signer
                .get_agent(&account_id)
                .await
                .map_err(|err| eyre!("Failed to get agent status: {}", err))?
                .ok_or_else(|| eyre!("Agent account {} is not registered", account_id))
                .map(|agent| agent.status)
        })
        .await?;

    let account_id = client.account_id();
    info!("[{}] Agent account id: {}", chain_id, account_id);
    info!("[{}] Initial agent status: {:?}", chain_id, status);
    let status = Arc::new(Mutex::new(status));

    // For each RPC endpoint, spawn a task to stream blocks from it
    for rpc_polling_url in &config.info.apis.rpc {
        info!(
            "[{}] Starting polling task for {}",
            chain_id, &rpc_polling_url.address
        );

        let polling_block_source_tx = block_source_tx.clone();
        let rpc_polling_url = rpc_polling_url.clone();
        let polling_shutdown_tx = shutdown_tx.clone();
        let polling_config = config.clone();
        let polling_retry_strategy = FixedInterval::from_millis(5000);
        let polling_chain_id = chain_id.clone();

        let block_stream_task = tokio::task::spawn(async move {
            Retry::spawn(polling_retry_strategy, || async {
                polling::poll(
                    Duration::from_secs_f64(polling_config.block_polling_seconds),
                    Duration::from_secs_f64(polling_config.block_polling_timeout_seconds),
                    &polling_block_source_tx,
                    &polling_shutdown_tx,
                    &rpc_polling_url.address,
                )
                .await
                .map_err(|err| {
                    error!("[{}] Error polling blocks: {}", polling_chain_id, err);
                    err
                })
            })
            .await
        });
        block_stream_tasks.push(block_stream_task);
    }

    // TODO: Try websocket for each polling addr.

    // Sequence the blocks we receive from the block stream. This is necessary because we may receive
    // blocks from multiple sources, and we need to ensure that we process them in order.
    let (sequencer_tx, sequencer_rx) = mpsc::unbounded_channel();
    let mut sequencer = Sequencer::new(block_source_rx, sequencer_tx, 512)?;
    let _sequencer_handle = tokio::task::spawn(async move { sequencer.consume().await });

    // Dispatch blocks to anybody who is listening.
    let (dispatcher_tx, _dispatcher_rx) = broadcast::channel(32);
    let mut dispatcher = Dispatcher::new(sequencer_rx, dispatcher_tx.clone());
    let _dispatcher_handle = tokio::task::spawn(async move { dispatcher.fanout().await });

    // Task to show blocks from the block stream
    let mut block_stream_info_rx = dispatcher_tx.subscribe();
    let block_stream_chain_id = chain_id.clone();
    let _block_stream_info_handle = tokio::task::spawn(async move {
        while let Ok(block) = block_stream_info_rx.recv().await {
            info!(
                "[{}] Processing block (height: {})",
                block_stream_chain_id,
                block.header().height,
            );
        }
    });

    // Account status checks
    let account_status_check_shutdown_rx = shutdown_tx.subscribe();
    let account_status_check_block_stream_rx = dispatcher_tx.subscribe();
    let block_status = status.clone();
    let block_status_accounts_loop = block_status.clone();
    let block_status_client = client.clone();
    let account_status_check_handle = tokio::task::spawn(agent::check_account_status_loop(
        account_status_check_block_stream_rx,
        account_status_check_shutdown_rx,
        block_status_accounts_loop,
        block_status_client,
    ));

    // Process blocks coming in from the blockchain
    let task_runner_shutdown_rx = shutdown_tx.subscribe();
    let task_runner_block_stream_rx = dispatcher_tx.subscribe();
    let tasks_client = client.clone();
    let block_status_tasks = block_status.clone();
    let task_runner_handle = tokio::task::spawn(tasks::tasks_loop(
        task_runner_block_stream_rx,
        task_runner_shutdown_rx,
        tasks_client,
        block_status_tasks,
    ));

    // Check rules if enabled
    let rules_runner_handle = if with_rules {
        tokio::task::spawn(tasks::rules_loop(
            dispatcher_tx.subscribe(),
            shutdown_tx.subscribe(),
            client,
            block_status,
        ))
    } else {
        tokio::task::spawn(async { Ok(()) })
    };

    // Ctrl-C handler
    let ctrl_c_shutdown_tx = shutdown_tx.clone();
    let ctrl_c_chain_id = chain_id.clone();
    let ctrl_c_handle: JoinHandle<Result<(), Report>> = tokio::task::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .map_err(|err| eyre!("[{}] Failed to wait for Ctrl-C: {}", ctrl_c_chain_id, err))?;
        ctrl_c_shutdown_tx.send(()).map_err(|err| {
            eyre!(
                "[{}] Failed to send shutdown signal: {}",
                ctrl_c_chain_id,
                err
            )
        })?;
        println!();
        info!("[{}] Shutting down...", ctrl_c_chain_id);

        Ok(())
    });

    // Handle all the block streams
    let block_stream_tasks_handler_chain_id = chain_id.clone();
    let block_stream_tasks_handler = tokio::task::spawn(async move {
        // Wait for each block stream task to finish. If any of them fail, we need to propagate the error.
        while let Some(block_stream_task) = block_stream_tasks.next().await {
            match block_stream_task {
                Ok(Ok(())) => (),
                Ok(Err(err)) => {
                    error!(
                        "[{}] Block stream task failed: {}",
                        block_stream_tasks_handler_chain_id, err
                    );
                    return Err(err);
                }
                Err(err) => {
                    error!(
                        "[{}] Block stream task failed: {}",
                        block_stream_tasks_handler_chain_id, err
                    );
                    return Err(err.into());
                }
            }
        }

        Ok::<(), Report>(())
    });

    // Try to join all the system tasks.
    let system_status = try_flat_join!(
        ctrl_c_handle,
        account_status_check_handle,
        task_runner_handle,
        rules_runner_handle,
        block_stream_tasks_handler
    );

    // If any of the tasks failed, we need to propagate the error.
    match system_status {
        Ok(_) => Ok(()),
        Err(err) => {
            error!("croncatd shutdown with error");
            Err(err)
        }
    }
}

pub async fn run_retry(
    chain_id: &String,
    shutdown_tx: &ShutdownTx,
    config: &ChainConfig,
    key: &ExtendedPrivateKey<SigningKey>,
    with_rules: bool,
) -> Result<(), Report> {
    // TODO: Rethink this retry logic
    // let retry_strategy = FixedInterval::from_millis(5000).take(1200);

    // Retry::spawn(retry_strategy, || async {
    run(chain_id, shutdown_tx, config, key, with_rules).await?;
    // .map_err(|err| {
    //     error!("[{}] System crashed: {}", &chain_id, err);
    //     error!("[{}] Retrying...", &chain_id);
    //     err
    // })?;
    // })
    // .await?;

    Ok(())
}
