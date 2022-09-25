//!
//! Subscribe and stream blocks from the tendermint WS RPC client.
//!

use color_eyre::Report;
use std::time::Duration;
use tendermint_rpc::{Client, HttpClient, Url};
use tokio::task::JoinHandle;
use tokio::time::sleep;

use crate::channels::{BlockStreamTx, ShutdownRx};
use crate::logging::info;

///
/// Polls the chain using HTTP client calling latest_block
/// Then broadcasts (only) the height of that block.
///
pub async fn poll(
    duration: Duration,
    block_stream_tx: &BlockStreamTx,
    shutdown_rx: &ShutdownRx,
    rpc_address: &String,
) -> Result<(), Report> {

    info!("rpc_address {}", rpc_address);

    let node_address: Url = rpc_address.parse().unwrap();
    info!("node_address {}", node_address);

    let rpc_client =
        HttpClient::new(node_address).expect("Could not get http client for RPC node for polling");

    let block_stream_tx = block_stream_tx.clone();
    let mut shutdown_rx = shutdown_rx.clone();

    let polling_loop_handle: JoinHandle<Result<(), Report>> = tokio::task::spawn(async move {
        loop {
            let block_response = rpc_client.latest_block().await?;
            let block_height = block_response.block.header.height.value();
            info!("block_height {}", block_height);

            // Broadcast block height, will be received by …?
            // Currently getting:
            //   The application panicked (crashed).
            //   Message:  Failed to send block height from polling: SendError(..)
            // I think we need to have the block stream receiver (likely in )
            block_stream_tx.broadcast(block_response.block).await?;
            // Wait
            sleep(duration).await;
        }
    });

    // Allow this task to get shut down when a person types Ctrl+C
    tokio::select! {
        _ = polling_loop_handle => {}
        _ = shutdown_rx.recv() => {}
    }

    Ok(())
}
