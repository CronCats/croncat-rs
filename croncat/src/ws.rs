use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;
use tungstenite::Message;

use crate::{logging::{error, info}, ShutdownRx};

const RPC_SUBSCRIPTION_MSG: &'static str = "{ \"jsonrpc\": \"2.0\", \"method\": \"subscribe\", \"params\": [\"tm.event='NewBlock'\"], \"id\": 1 }";

pub async fn stream_blocks(url: String, shutdown_rx: &mut ShutdownRx) -> tungstenite::Result<()> {
  let (mut socket, _) = connect_async(url).await?;

  // TODO: Replace me
  info!("Subscribing to NewBlock event");
  socket.send(RPC_SUBSCRIPTION_MSG.into()).await?;

  // Ignore the first message, this is the response.
  // Probably should handle errors here, but not now.
  let _ = socket.next().await;

  // Handle inbound blocks
  let inbound = tokio::task::spawn(async move {
    while let Some(msg) = socket.next().await {
      let msg = msg.expect("Failed to parse WS message");

      match msg {
        Message::Text(block_string) => {
          info!("{}", block_string);
        }
        _ => {
          error!("Invalid message type received")
        }
      }
    }
  });

  tokio::select! {
    _ = inbound => {}
    _ = shutdown_rx.recv() => {
      info!("WS RPC shutting down");
    }
  }

  Ok(())
} 
