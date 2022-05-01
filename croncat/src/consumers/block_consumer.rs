use crate::{
    channels::{BlockStreamRx, BlockStreamTx, ShutdownRx},
    errors::Report,
    logging::info,
};

pub async fn consume(
    block_stream_rx: BlockStreamRx,
    shutdown_rx: ShutdownRx,
) -> Result<(), Report> {
    let block_consumer_stream = tokio::task::spawn(async move {
        while let Ok(block) = block_stream_rx.recv().await {
            info!("{:#?}", block.header.time);
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
pub fn create_stream(channel_size: usize) -> (BlockStreamTx, BlockStreamRx) {
    async_channel::bounded(channel_size)
}
