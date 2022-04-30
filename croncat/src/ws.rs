//!
//! Subscribe and stream blocks from the tendermint WS RPC client.
//!

use futures_util::StreamExt;
use tendermint_rpc::event::EventData;
use tendermint_rpc::query::EventType;
use tendermint_rpc::{SubscriptionClient, WebSocketClient};
use url::Url;

use crate::{
    channels::ShutdownRx,
    logging::{info, warn},
};

///
/// Connect to the RPC websocket endpoint and subscribe for incoming blocks.
///
pub async fn stream_blocks(
    url: String,
    shutdown_rx: &mut ShutdownRx,
) -> Result<(), Box<dyn std::error::Error>> {
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
    let block_stream_handle = tokio::task::spawn(async move {
        while let Some(msg) = subscriptions.next().await {
            let msg = msg.expect("Failed to receive event");
            match msg.data {
                // Handle blocks
                EventData::NewBlock {
                    block: Some(block), ..
                } => {
                    info!(
                        "Received block (height: {}) from {}",
                        block.header.height, block.header.time
                    );
                }
                // Warn about all events for now
                message => {
                    warn!("Unexpected message type: {:?}", message);
                }
            }
        }
    });

    // Handle shutdown
    tokio::select! {
      _ = block_stream_handle => {}
      _ = shutdown_rx.recv() => {}
    }

    // Clean up
    client.close().unwrap();
    let _ = driver_handle.await.unwrap();

    Ok(())
}
