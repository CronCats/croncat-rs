//!
//! Subscribe and stream blocks from the tendermint WS RPC client.
//!

use crate::channels::ShutdownTx;
use crate::utils::Block;
use async_stream::try_stream;
use color_eyre::{eyre::eyre, Report};
use futures_util::TryStream;
use std::{pin::Pin, time::Duration};
use tendermint_rpc::{Client, HttpClient, Url};
use tokio::{
    sync::mpsc,
    task::JoinHandle,
    time::{sleep, timeout},
};
use tracing::{debug, error};

///
/// Polls the chain using HTTP client calling latest_block
/// Then broadcasts (only) the height of that block.
///
pub async fn poll(
    duration: Duration,
    _timeout: Duration,
    block_stream_tx: &mpsc::UnboundedSender<Block>,
    shutdown_tx: &ShutdownTx,
    rpc_address: &str,
) -> Result<(), Report> {
    let mut shutdown_rx = shutdown_tx.subscribe();
    let node_address: Url = rpc_address.parse()?;

    debug!("Polling connecting to {}", node_address);

    let rpc_client = HttpClient::new(node_address.clone()).map_err(|err| {
        eyre!(
            "Could not get http client for RPC node for polling: {}",
            err.detail()
        )
    })?;

    debug!("Polling connected to {}", node_address);

    let block_stream_tx = block_stream_tx.clone();

    let polling_loop_handle: JoinHandle<Result<(), Report>> = tokio::task::spawn(async move {
        loop {
            let block = rpc_client
                .latest_block()
                .await
                .map_err(|err| eyre!("Timed out latest block: {}", err.detail()))?
                .block;
            debug!(
                "Polled block (height: {}) from {}",
                block.header.height, block.header.time
            );
            // Broadcast block height, will be received by …?
            // Currently getting:
            //   The application panicked (crashed).
            //   Message:  Failed to send block height from polling: SendError(..)
            // I think we need to have the block stream receiver (likely in )
            block_stream_tx
                .send(block.into())
                .map_err(|err| eyre!("Failed to send block: {}", err))?;
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

type BlockStream =
    Pin<Box<dyn TryStream<Item = Result<Block, Report>, Ok = Block, Error = Report> + Send>>;

///
/// Stream polled blocks from the given rpc endpoint.
///
pub fn poll_stream_blocks(http_rpc_host: String, poll_duration_secs: f64) -> BlockStream {
    Box::pin(try_stream! {
        let client = HttpClient::new(http_rpc_host.as_str()).map_err(|source| eyre!("Failed to connect to RPC: {}", source))?;

        // TODO: Double check this change - since block heights are ~6secs, don't want to have timeout 30 seconds for failures
        let poll_timeout_duration = Duration::from_secs_f64(poll_duration_secs);
        loop {
            match timeout(poll_timeout_duration, client.latest_block()).await {
                Ok(Ok(block)) => {
                    let block = block.block;
                    debug!("[{}] Polled block {}", block.header().chain_id, block.header().height);
                    yield block.into();
                }
                Ok(Err(err)) => {
                    debug!("Failed to get latest block: {}", err);
                }
                Err(err) => {
                    debug!("Timed out getting latest block: {}", err);
                }
            }
            sleep(Duration::from_secs_f64(poll_duration_secs)).await;
        }
    })
}
