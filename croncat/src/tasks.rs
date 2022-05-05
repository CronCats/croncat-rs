//!
//! How to process and consume blocks from the chain.
//!

use crate::{
    channels::{BlockStreamRx, ShutdownRx},
    errors::Report,
};

///
/// Do work on blocks that are sent from the ws stream.
///
pub async fn run_tasks(
    mut block_stream_rx: BlockStreamRx,
    mut shutdown_rx: ShutdownRx,
) -> Result<(), Report> {
    let block_consumer_stream =
        tokio::task::spawn(async move { while let Ok(_block) = block_stream_rx.recv().await {} });

    tokio::select! {
        _ = block_consumer_stream => {}
        _ = shutdown_rx.recv() => {}
    }

    Ok(())
}
