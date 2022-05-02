//!
//! Listen for blocks coming from WS RPC, stream them to the scheduler and run tasks appropriately.
//!

use crate::{
    channels::{ShutdownRx, ShutdownTx},
    consumers,
    env::Env,
    errors::Report,
    grpc,
    logging::info,
    tokio, ws,
};

///
/// Kick off the croncat agent!
///
pub async fn run(env: Env, shutdown_tx: ShutdownTx, shutdown_rx: ShutdownRx) -> Result<(), Report> {
    // Create a block stream channel
    // TODO (SeedyROM): Remove 128 hardcoded limit
    let (block_stream_tx, block_stream_rx) = consumers::create_block_stream(128);

    // Connect to GRPC
    let (_msg_client, _query_client) = grpc::connect(env.grpc_url.clone()).await?;

    // Stream new blocks from the WS RPC subscription
    let block_stream_shutdown_rx = shutdown_rx.clone();
    let block_stream_handle = tokio::task::spawn(async move {
        ws::stream_blocks(
            env.wsrpc_url.clone(),
            block_stream_tx,
            block_stream_shutdown_rx,
        )
        .await
        .expect("Failed stream blocks")
    });

    // Process blocks coming in from the blockchain
    let block_process_shutdown_rx = shutdown_rx.clone();
    let block_process_stream_handle = tokio::task::spawn(async move {
        consumers::consume_blocks(block_stream_rx, block_process_shutdown_rx)
            .await
            .expect("Failed to process streamed blocks")
    });

    // Handle SIGINT AKA Ctrl-C
    let ctrl_c_handle = tokio::task::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to wait for Ctrl-C");
        shutdown_tx
            .send(())
            .await
            .expect("Failed to send shutdown signal");
        println!("");
        info!("Shutting down croncatd...");
    });

    // TODO: Do something with the return values
    let _ = tokio::join!(
        ctrl_c_handle,
        block_stream_handle,
        block_process_stream_handle
    );

    Ok(())
}
