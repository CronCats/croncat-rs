use futures_util::StreamExt;
use tendermint_rpc::query::EventType;
use tendermint_rpc::{SubscriptionClient, WebSocketClient};
use url::Url;

use crate::{logging::info, ShutdownRx};

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
    let inbound = tokio::task::spawn(async move {
        while let Some(msg) = subscriptions.next().await {
            info!("{:#?}", msg);
        }
    });

    // Handle shutdown
    tokio::select! {
      _ = inbound => {}
      _ = shutdown_rx.recv() => {
        info!("WS RPC shutting down");
      }
    }

    // Clean up
    client.close().unwrap();
    let _ = driver_handle.await.unwrap();

    Ok(())
}
