//!
//! Subscribe and stream blocks from the tendermint WS RPC client.
//!

use std::time::Duration;

use crate::channels::ShutdownTx;
use crate::utils::Block;
use color_eyre::{eyre::eyre, Report};
use futures_util::StreamExt;
use tendermint_rpc::event::EventData;
use tendermint_rpc::query::EventType;
use tendermint_rpc::{SubscriptionClient, WebSocketClient};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::log::error;
use tracing::trace;
use url::Url;

use crate::{
    channels::ShutdownRx,
    logging::{info, warn},
};

///
/// Connect to the RPC websocket endpoint and subscribe for incoming blocks.
///
pub async fn stream_blocks_loop(
    url: &str,
    block_stream_tx: &mpsc::UnboundedSender<Block>,
    shutdown_tx: &ShutdownTx,
) -> Result<(), Report> {
    let mut shutdown_rx = shutdown_tx.subscribe();
    let block_stream_tx = block_stream_tx.clone();

    // Parse url
    let url = Url::parse(url)?;

    trace!("Connecting to WS RPC server @ {}", url);

    // Connect to the WS RPC url
    let (client, driver) =
        tokio::time::timeout(Duration::from_secs(5), WebSocketClient::new(url.as_str())).await??;

    let driver_handle = tokio::task::spawn(driver.run());

    trace!("Connected to WS RPC server @ {}", url);

    trace!("Subscribing to NewBlock event");

    // Subscribe to the NewBlock event stream
    let mut subscriptions = client
        .subscribe(EventType::NewBlock.into())
        .await
        .map_err(|err| eyre!("Failed to subscribe to the block stream: {}", err.detail()))?;

    info!("Successfully subscribed to NewBlock event");

    // Handle inbound blocks
    let block_stream_handle: JoinHandle<Result<(), Report>> = tokio::task::spawn(async move {
        while let Ok(msg) =
            tokio::time::timeout(Duration::from_secs(30), subscriptions.next()).await
        {
            let msg = msg.ok_or_else(|| eyre!("Block stream next timeout"))??;
            match msg.data {
                // Handle blocks
                EventData::NewBlock {
                    block: Some(block), ..
                } => {
                    trace!(
                        "Received block (height: {}) from {}",
                        block.header.height,
                        block.header.time
                    );
                    block_stream_tx.send(block.into())?;
                }
                // Warn about all events for now
                message => {
                    warn!("Unexpected message type: {:?}", message);
                }
            }
        }

        Ok(())
    });

    tokio::select! {
        res = block_stream_handle => {
            res?.map_err(|err| {
                error!("Block stream failed: {}", err);
                err
            })?
        }
        _ = shutdown_rx.recv() => {}
    };

    // Clean up
    client.close()?;
    let _ = driver_handle.await?;

    Ok(())
}
