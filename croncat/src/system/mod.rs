//!
//! The croncat system daemon.
//!

use std::sync::Arc;
use std::time::Duration;

use cosmrs::bip32::{secp256k1::ecdsa::SigningKey, ExtendedPrivateKey};
use croncat_pipeline::{try_flat_join, Dispatcher, Sequencer};
use cw_croncat_core::types::AgentStatus;
use futures_util::{future::join_all, stream::FuturesUnordered, StreamExt};
use tokio::{
    sync::{broadcast, mpsc, Mutex},
    task::JoinHandle,
};
use tokio_retry::{
    strategy::{jitter, FibonacciBackoff, FixedInterval},
    Retry,
};
use tracing::log::error;

use crate::{
    channels::{self, ShutdownRx, ShutdownTx},
    config::{ChainConfig, Config},
    errors::{eyre, Report},
    grpc::GrpcSigner,
    logging::info,
    streams::{agent, polling, tasks, ws},
    tokio,
    utils::flatten_join,
};

pub mod service;

pub use service::DaemonService;

// ///
// /// Kick off the croncat daemon
// ///
// pub async fn run(
//     shutdown_tx: &ShutdownTx,
//     shutdown_rx: &ShutdownRx,
//     signer: GrpcSigner,
//     initial_status: AgentStatus,
//     with_rules: bool,
//     polling_duration_secs: u64,
// ) -> Result<(), Report> {
//     // Create a block stream channel
//     let (block_stream_tx, block_stream_rx) = channels::create_block_stream(32);

//     // Connect to GRPC  Stream new blocks from the WS RPC subscription
//     let block_stream_shutdown_rx = shutdown_rx.clone();
//     let wsrpc = signer.wsrpc().to_owned();
//     let ws_block_stream_tx = block_stream_tx.clone();

//     // Generic retry strategy
//     let retry_strategy = FixedInterval::from_millis(3000).map(jitter).take(1200);

//     // Handle retries if the websocket task fails.
//     //
//     // NOTE:
//     // This was super tricky, bascially we need to pass all non Copyable values
//     // in by reference otherwise our closure will be FnOnce and we can't call it.
//     //
//     // This should be repeated for all the other tasks probably?
//     //
//     let retry_block_stream_strategy = retry_strategy.clone();
//     let retry_block_stream_handle = tokio::task::spawn(async move {
//         Retry::spawn(retry_block_stream_strategy, || async {
//             // Stream blocks
//             ws::stream_blocks_loop(&wsrpc, &ws_block_stream_tx, &block_stream_shutdown_rx)
//                 .await
//                 .map_err(|err| {
//                     error!("Error streaming blocks: {}", err);
//                     err
//                 })
//         })
//         .await
//     });

//     // Set up polling
//     let block_polling_shutdown_rx = shutdown_rx.clone();
//     let rpc_addr = signer.rpc().to_owned();
//     let http_block_stream_tx = block_stream_tx.clone();

//     // Handle retries if the polling task fails.
//     let retry_polling_strategy = retry_strategy.clone();
//     let retry_polling_handle = tokio::task::spawn(async move {
//         Retry::spawn(retry_polling_strategy, || async {
//             // Poll for new blocks
//             polling::poll(
//                 Duration::from_secs(polling_duration_secs),
//                 &http_block_stream_tx,
//                 &block_polling_shutdown_rx,
//                 &rpc_addr,
//             )
//             .await
//             .map_err(|err| {
//                 error!("Error polling blocks: {}", err);
//                 err
//             })
//         })
//         .await
//     });

//     // Account status checks
//     let account_status_check_shutdown_rx = shutdown_rx.clone();
//     let account_status_check_block_stream_rx = block_stream_rx.clone();
//     let block_status = Arc::new(Mutex::new(initial_status));
//     let block_status_accounts_loop = block_status.clone();
//     let signer_status = signer.clone();
//     let account_status_check_handle = tokio::task::spawn(agent::check_account_status_loop(
//         account_status_check_block_stream_rx,
//         account_status_check_shutdown_rx,
//         block_status_accounts_loop,
//         signer_status,
//     ));

//     // Process blocks coming in from the blockchain
//     let task_runner_shutdown_rx = shutdown_rx.clone();
//     let task_runner_block_stream_rx = block_stream_rx.clone();
//     let tasks_signer = signer.clone();
//     let block_status_tasks = block_status.clone();
//     let task_runner_handle = tokio::task::spawn(tasks::tasks_loop(
//         task_runner_block_stream_rx,
//         task_runner_shutdown_rx,
//         tasks_signer,
//         block_status_tasks,
//     ));

//     // Check rules if enabled
//     let rules_runner_handle = if with_rules {
//         tokio::task::spawn(tasks::rules_loop(
//             block_stream_rx,
//             shutdown_rx.to_owned(),
//             signer,
//             block_status,
//         ))
//     } else {
//         tokio::task::spawn(async { Ok(()) })
//     };

//     // Handle SIGINT AKA Ctrl-C
//     let ctrl_c_shutdown_tx = shutdown_tx.clone();
//     let ctrl_c_handle: JoinHandle<Result<(), Report>> = tokio::task::spawn(async move {
//         tokio::signal::ctrl_c()
//             .await
//             .map_err(|err| eyre!("Failed to wait for Ctrl-C: {}", err))?;
//         ctrl_c_shutdown_tx
//             .broadcast(())
//             .await
//             .map_err(|err| eyre!("Failed to send shutdown signal: {}", err))?;
//         println!();
//         info!("Shutting down croncatd...");

//         Ok(())
//     });

//     // Try to join all the system tasks.
//     let system_status = tokio::try_join!(
//         flatten_join(ctrl_c_handle),
//         flatten_join(account_status_check_handle),
//         flatten_join(retry_block_stream_handle),
//         flatten_join(retry_polling_handle),
//         flatten_join(task_runner_handle),
//         flatten_join(rules_runner_handle),
//     );

//     // If any of the tasks failed, we need to propagate the error.
//     match system_status {
//         Ok(_) => Ok(()),
//         Err(err) => {
//             error!("croncatd shutdown with error");
//             Err(err)
//         }
//     }
// }

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
    let (dispatcher_tx, dispatcher_rx) = broadcast::channel(32);
    let mut dispatcher = Dispatcher::new(sequencer_rx, dispatcher_tx.clone());
    let _dispatcher_handle = tokio::task::spawn(async move { dispatcher.fanout().await });

    // Test block subscriber
    let test_block_subscriber_dispatcher_tx = dispatcher_tx.clone();
    let test_block_subscriber_chain_id = chain_id.clone();
    let _test_block_subscriber_handle = tokio::task::spawn(async move {
        let mut block_subscriber = test_block_subscriber_dispatcher_tx.subscribe();
        while let Ok(block) = block_subscriber.recv().await {
            info!(
                "[{}] Received block: {}",
                test_block_subscriber_chain_id,
                block.header().height
            );
        }
    });

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

    // Wait for each block stream task to finish. If any of them fail, we need to propagate the error.
    while let Some(block_stream_task) = block_stream_tasks.next().await {
        match block_stream_task {
            Ok(Ok(())) => (),
            Ok(Err(err)) => {
                error!("[{}] Block stream task failed: {}", chain_id, err);
                return Err(err);
            }
            Err(err) => {
                error!("[{}] Block stream task failed: {}", chain_id, err);
                return Err(err.into());
            }
        }
    }

    try_flat_join!(ctrl_c_handle)?;

    Ok(())
}

pub async fn run_retry(
    chain_id: &String,
    shutdown_tx: &ShutdownTx,
    config: &ChainConfig,
    key: &ExtendedPrivateKey<SigningKey>,
    with_rules: bool,
) -> Result<(), Report> {
    let retry_strategy = FixedInterval::from_millis(3000).map(jitter).take(1200);

    Retry::spawn(retry_strategy, || async {
        run(chain_id, shutdown_tx, config, key, with_rules)
            .await
            .map_err(|err| {
                error!("System crashed: {}", err);
                error!("Retrying...");
                err
            })
    })
    .await?;

    Ok(())
}
