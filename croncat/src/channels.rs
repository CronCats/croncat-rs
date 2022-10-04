//!
//! Various channel types for croncat.
//!

/// Shutdown channel Sender.
pub type ShutdownTx = async_broadcast::Sender<()>;

/// Shutdown channel Receiver.
pub type ShutdownRx = async_broadcast::Receiver<()>;

/// Block stream channel Sender.
pub type BlockStreamTx = async_broadcast::Sender<tendermint::Block>;

/// Block stream channel Receiver.
pub type BlockStreamRx = async_broadcast::Receiver<tendermint::Block>;

///
/// Create a block stream channel of a specified size, used to create back pressure.
///
pub fn create_block_stream(channel_size: usize) -> (BlockStreamTx, BlockStreamRx) {
    let mut channel = async_broadcast::broadcast(channel_size);
    // TODO: THIS IS A HACK. Something is not flushing the channel properly.
    // For now all our tasks should be received fine.
    channel.0.set_overflow(true);
    channel
}

///
/// Create a shutdown channel.
///
pub fn create_shutdown_channel() -> (ShutdownTx, ShutdownRx) {
    async_broadcast::broadcast(1)
}
