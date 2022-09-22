//!
//! Subscribe and stream blocks from the tendermint WS RPC client.
//!

use std::time::Duration;
use tendermint_rpc::{Client, HttpClient, Url};
use tokio::time::sleep;

use crate::channels::{BlockStreamTx, ShutdownRx};
use crate::logging::info;

///
/// Polls the chain using HTTP client calling latest_block
/// Then broadcasts (only) the height of that block.
///
pub async fn poll(duration: Duration, block_stream_tx: BlockStreamTx, mut shutdown_rx: ShutdownRx, rpc_address: String) {
    info!("rpc_address {}", rpc_address);

    let node_address: Url = rpc_address.parse().unwrap();
    info!("node_address {}", node_address);

    let rpc_client =
        HttpClient::new(node_address).expect("Could not get http client for RPC node for polling");

    let polling_loop_handle = tokio::task::spawn(async move {
        loop {
            let block_response = rpc_client
                .latest_block()
                .await
                .expect("Could not fetch latest block");
            let block_height = block_response.block.header.height.value();
            info!("block_height {}", block_height);

            // Broadcast block height, will be received by …?
            // Currently getting:
            //   The application panicked (crashed).
            //   Message:  Failed to send block height from polling: SendError(..)
            // I think we need to have the block stream receiver (likely in )
            block_stream_tx
                .broadcast(block_response.block)
                .await
                .expect("Failed to send block height from polling");

            // Wait
            sleep(duration).await;
        }
    });

    // Allow this task to get shut down when a person types Ctrl+C
    let mut continue_loop = true;
    tokio::select! {
        _ = polling_loop_handle => {}
        _ = shutdown_rx.recv() => {}
    }
}
