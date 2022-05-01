use crate::{
    channels::{BlockStreamRx, BlockStreamTx, ShutdownRx},
    errors::Report,
    logging::info,
};

pub async fn consume(
    block_stream_rx: BlockStreamRx,
    shutdown_rx: ShutdownRx,
) -> Result<(), Report> {
    while let Ok(block) = block_stream_rx.recv().await {
        if shutdown_rx.try_recv().is_ok() {
            break;
        }

        info!("{:#?}", block.header.time);
    }

    Ok(())
}

///
/// Create a block stream channel of a specified size, used to create back pressure.
///
pub fn create_stream(channel_size: usize) -> (BlockStreamTx, BlockStreamRx) {
    async_channel::bounded(channel_size)
}
