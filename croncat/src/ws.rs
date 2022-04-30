use std::collections::HashMap;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;
use tungstenite::Message;

use crate::{
    logging::{error, info},
    ShutdownRx,
};

const RPC_SUBSCRIPTION_MSG: &'static str = "{ \"jsonrpc\": \"2.0\", \"method\": \"subscribe\", \"params\": [\"tm.event='NewBlock'\"], \"id\": 1 }";

///
/// Connect to the RPC websocket endpoint and subscribe for incoming blocks.
///
pub async fn stream_blocks(url: String, shutdown_rx: &mut ShutdownRx) -> tungstenite::Result<()> {
    // Connect to the WS RPC url
    let (mut socket, _) = connect_async(url).await?;

    info!("Subscribing to NewBlock event");
    socket.send(RPC_SUBSCRIPTION_MSG.into()).await?;
    info!("Sucessfully subscribed to NewBlock event");

    // Ignore the first message, this is the response.
    // Probably should handle errors here, but not now.
    let _ = socket.next().await;

    // Handle inbound blocks
    let inbound = tokio::task::spawn(async move {
        while let Some(msg) = socket.next().await {
            let msg = msg.expect("Failed to parse WS message");

            match msg {
                Message::Text(block_string) => {
                    let block: HashMap<String, serde_json::Value> =
                        serde_json::from_str(&block_string).expect("Failed to parse block JSON");
                    info!(
                        "Received block: {:#?}",
                        block["result"]["data"]["value"]["block"]
                    );
                }
                _ => {
                    error!("Invalid message type received")
                }
            }
        }
    });

    // Handle shutdown
    tokio::select! {
      _ = inbound => {}
      _ = shutdown_rx.recv() => {
        info!("WS RPC shutting down");
      }
    }

    Ok(())
}
