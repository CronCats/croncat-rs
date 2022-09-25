//!
//! Subscribe and stream blocks from the tendermint WS RPC client.
//!

use color_eyre::Report;
use futures_util::StreamExt;
use tendermint_rpc::event::EventData;
use tendermint_rpc::query::EventType;
use tendermint_rpc::{SubscriptionClient, WebSocketClient};
use tokio::task::JoinHandle;
use url::Url;

use crate::channels::BlockStreamTx;
use crate::{
    channels::ShutdownRx,
    logging::{info, warn},
};

///
/// Connect to the RPC websocket endpoint and subscribe for incoming blocks.
///
pub async fn stream_blocks_loop(
    url: String,
    block_stream_tx: BlockStreamTx,
    mut shutdown_rx: ShutdownRx,
) -> Result<(), Report> {
    // Parse url
    let url = Url::parse(&url).unwrap();

    info!("Connecting to WS RPC server @ {}", url);

    // Connect to the WS RPC url
    let (client, driver) = WebSocketClient::new(url.as_str()).await?;
    let driver_handle = tokio::task::spawn(driver.run());

    info!("Connected to WS RPC server @ {}", url);

    info!("Subscribing to NewBlock event");

    // Subscribe to the NewBlock event stream
    let mut subscriptions = client.subscribe(EventType::NewBlock.into()).await?;

    info!("Successfully subscribed to NewBlock event");

    // Handle inbound blocks
    let block_stream_handle: JoinHandle<Result<(), Report>> = tokio::task::spawn(async move {
        while let Some(msg) = subscriptions.next().await {
            let msg = msg?;
            match msg.data {
                // Handle blocks
                EventData::NewBlock {
                    block: Some(block), ..
                } => {
                    info!(
                        "Received block (height: {}) from {}",
                        block.header.height, block.header.time
                    );
                    block_stream_tx.broadcast(block).await?;
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
        _ = block_stream_handle => {}
        _ = shutdown_rx.recv() => {}
    }

    // Clean up
    client.close().unwrap();
    let _ = driver_handle.await.unwrap();

    Ok(())
}
