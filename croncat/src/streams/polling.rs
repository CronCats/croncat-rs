//!
//! Subscribe and stream blocks from the tendermint WS RPC client.
//!

use crate::utils::Block;
use color_eyre::{eyre::eyre, Report};
use std::time::Duration;
use tendermint_rpc::{Client, HttpClient, Url};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::log::error;
use tracing::trace;

use crate::channels::ShutdownTx;
use crate::logging::info;

///
/// Polls the chain using HTTP client calling latest_block
/// Then broadcasts (only) the height of that block.
///
pub async fn poll(
    duration: Duration,
    timeout: Duration,
    block_stream_tx: &mpsc::UnboundedSender<Block>,
    shutdown_tx: &ShutdownTx,
    rpc_address: &str,
) -> Result<(), Report> {
    let mut shutdown_rx = shutdown_tx.subscribe();
    let node_address: Url = rpc_address.parse()?;

    trace!("Polling connecting to {}", node_address);

    let rpc_client = HttpClient::new(node_address.clone()).map_err(|err| {
        eyre!(
            "Could not get http client for RPC node for polling: {}",
            err.detail()
        )
    })?;

    trace!("Polling connected to {}", node_address);

    let block_stream_tx = block_stream_tx.clone();

    let polling_loop_handle: JoinHandle<Result<(), Report>> = tokio::task::spawn(async move {
        loop {
            let block = tokio::time::timeout(timeout, rpc_client.latest_block())
                .await?
                .map_err(|err| eyre!("Timed out latest block: {}", err.detail()))?
                .block;
            trace!(
                "Polled block (height: {}) from {}",
                block.header.height,
                block.header.time
            );
            // Broadcast block height, will be received by …?
            // Currently getting:
            //   The application panicked (crashed).
            //   Message:  Failed to send block height from polling: SendError(..)
            // I think we need to have the block stream receiver (likely in )
            block_stream_tx.send(block.into())?;
            // Wait
            sleep(duration).await;
        }
    });

    tokio::select! {
        res = polling_loop_handle => {
            res?.map_err(|err| {
                error!("Block polling loop failed: {}", err);
                err
            })?
        }
        _ = shutdown_rx.recv() => {}
    }

    Ok(())
}
