use crate::{channels::BlockStreamRx, errors::Report, logging::info};

pub async fn consume(block_stream_rx: &mut BlockStreamRx) -> Result<(), Report> {
    while let Some(block) = block_stream_rx.recv().await {
        info!("{:#?}", block.header.time);
    }

    Ok(())
}
