//!
//! The croncat system daemon.
//!

use std::sync::Arc;
use std::time::Duration;

use cw_croncat_core::types::AgentStatus;
use tokio::sync::Mutex;
use tokio_retry::{
    strategy::{jitter, ExponentialBackoff},
    Retry,
};
use tracing::log::error;

use crate::{
    channels::{self, ShutdownRx, ShutdownTx},
    errors::Report,
    grpc::GrpcSigner,
    logging::info,
    streams::{agent, polling, tasks, ws},
    tokio,
};

pub mod service;

pub use service::DaemonService;

///
/// Kick off the croncat daemon
///
pub async fn run(
    shutdown_tx: ShutdownTx,
    shutdown_rx: ShutdownRx,
    signer: GrpcSigner,
    initial_status: AgentStatus,
    with_rules: bool,
) -> Result<(), Report> {
    // Create a block stream channel
    // TODO (SeedyROM): Remove 128 hardcoded limit
    let (block_stream_tx, block_stream_rx) = channels::create_block_stream(128);

    // Connect to GRPC  Stream new blocks from the WS RPC subscription
    let block_stream_shutdown_rx = shutdown_rx.clone();
    let wsrpc = signer.wsrpc().to_owned();
    let ws_block_stream_tx = block_stream_tx.clone();
    let block_stream_handle = tokio::task::spawn(ws::stream_blocks_loop(
        wsrpc,
        ws_block_stream_tx,
        block_stream_shutdown_rx,
    ));

    // Set up polling
    let block_polling_shutdown_rx = shutdown_rx.clone();
    let rpc_addr = signer.rpc().to_owned();
    let http_block_stream_tx = block_stream_tx.clone();

    let retry_strategy = ExponentialBackoff::from_millis(10).map(jitter).take(8);
    let retry_polling_handle = tokio::task::spawn(async move {
        Retry::spawn(retry_strategy.clone(), || async {
            polling::poll(
                // TODO (mikedotexe) let's have the duration be in config. lfg Cosmoverse first
                Duration::from_secs(2),
                &http_block_stream_tx,
                &block_polling_shutdown_rx,
                &rpc_addr,
            )
            .await
        })
        .await
    });

    // Account status checks
    let account_status_check_shutdown_rx = shutdown_rx.clone();
    let account_status_check_block_stream_rx = block_stream_rx.clone();
    let block_status = Arc::new(Mutex::new(initial_status));
    let block_status_accounts_loop = block_status.clone();
    let signer_status = signer.clone();
    let account_status_check_handle = tokio::task::spawn(agent::check_account_status_loop(
        account_status_check_block_stream_rx,
        account_status_check_shutdown_rx,
        block_status_accounts_loop,
        signer_status,
    ));

    // Process blocks coming in from the blockchain
    let task_runner_shutdown_rx = shutdown_rx.clone();
    let task_runner_block_stream_rx = block_stream_rx.clone();
    let tasks_signer = signer.clone();
    let block_status_tasks = block_status.clone();
    let task_runner_handle = tokio::task::spawn(tasks::tasks_loop(
        task_runner_block_stream_rx,
        task_runner_shutdown_rx,
        tasks_signer,
        block_status_tasks,
    ));

    // Check rules if enabled
    let rules_runner_handle = if with_rules {
        tokio::task::spawn(tasks::rules_loop(
            block_stream_rx,
            shutdown_rx,
            signer,
            block_status,
        ))
    } else {
        tokio::task::spawn(async move { Ok(()) })
    };

    // Handle SIGINT AKA Ctrl-C
    let ctrl_c_shutdown_tx = shutdown_tx.clone();
    let ctrl_c_handle = tokio::task::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to wait for Ctrl-C");
        ctrl_c_shutdown_tx
            .broadcast(())
            .await
            .expect("Failed to send shutdown signal");
        println!();
        info!("Shutting down croncatd...");
    });

    // Try to join all the system tasks.
    let system_status = tokio::try_join!(
        ctrl_c_handle,
        block_stream_handle,
        task_runner_handle,
        account_status_check_handle,
        rules_runner_handle,
        retry_polling_handle,
    );

    // If any of the tasks failed, we need to propagate the error.
    match system_status {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("croncatd shutdown with error: {}", e);
            Err(e.into())
        }
    }
}
