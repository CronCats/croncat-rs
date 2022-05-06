//!
//! The croncat system daemon.
//!

use crate::{
    channels::{self, ShutdownRx, ShutdownTx},
    env::Env,
    errors::Report,
    grpc,
    logging::info,
    streams::{agent, tasks, ws},
    tokio,
};

///
/// Kick off the croncat daemon
///
pub async fn run(env: Env, shutdown_tx: ShutdownTx, shutdown_rx: ShutdownRx) -> Result<(), Report> {
    // Create a block stream channel
    // TODO (SeedyROM): Remove 128 hardcoded limit
    let (block_stream_tx, block_stream_rx) = channels::create_block_stream(128);

    // Connect to GRPC
    let (_msg_client, _query_client) = grpc::connect(env.grpc_url.clone()).await?;

    // Stream new blocks from the WS RPC subscription
    let block_stream_shutdown_rx = shutdown_rx.clone();
    let block_stream_handle = tokio::task::spawn(async move {
        ws::stream_blocks_loop(
            env.wsrpc_url.clone(),
            block_stream_tx,
            block_stream_shutdown_rx,
        )
        .await
        .expect("Failed to stream blocks")
    });

    // TODO (SeedyROM): For each agent check the status before beginning the loop.

    // Account status checks
    let account_status_check_shutdown_rx = shutdown_rx.clone();
    let account_status_check_block_stream_rx = block_stream_rx.clone();
    let account_status_check_handle = tokio::task::spawn(async move {
        agent::check_account_status_loop(
            account_status_check_block_stream_rx,
            account_status_check_shutdown_rx,
        )
        .await
        .expect("Failed to check account statuses")
    });

    // Process blocks coming in from the blockchain
    let task_runner_shutdown_rx = shutdown_rx.clone();
    let task_runner_block_stream_rx = block_stream_rx.clone();
    let task_runner_handle = tokio::task::spawn(async move {
        tasks::tasks_loop(task_runner_block_stream_rx, task_runner_shutdown_rx)
            .await
            .expect("Failed to process streamed blocks")
    });

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
        println!("");
        info!("Shutting down croncatd...");
    });

    // TODO (SeedyROM): Maybe do something with the return values?
    let _ = tokio::join!(
        ctrl_c_handle,
        block_stream_handle,
        task_runner_handle,
        account_status_check_handle,
    );

    Ok(())
}
