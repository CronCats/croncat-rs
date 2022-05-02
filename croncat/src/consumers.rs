//!
//! How to process and consume blocks from the chain.
//!

use crate::{
    channels::{BlockStreamRx, BlockStreamTx, ShutdownRx},
    errors::Report,
    logging::info,
};

///
/// Do work on blocks that are sent from the ws stream.
///
pub async fn consume_blocks(
    block_stream_rx: BlockStreamRx,
    shutdown_rx: ShutdownRx,
) -> Result<(), Report> {
    let block_consumer_stream = tokio::task::spawn(async move {
        while let Ok(block) = block_stream_rx.recv().await {
            info!("{:#?}", block.header);
        }
    });

    tokio::select! {
        _ = block_consumer_stream => {}
        _ = shutdown_rx.recv() => {}
    }

    Ok(())
}

///
/// Create a block stream channel of a specified size, used to create back pressure.
///
pub fn create_block_stream(channel_size: usize) -> (BlockStreamTx, BlockStreamRx) {
    async_channel::bounded(channel_size)
}
