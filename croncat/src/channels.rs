//!
//! Various channel types for croncat.
//!

use tokio::sync::broadcast;

/// Shutdown channel Sender.
pub type ShutdownTx = broadcast::Sender<()>;

/// Shutdown channel Receiver.
pub type ShutdownRx = broadcast::Receiver<()>;

/// Block stream channel Sender.
pub type BlockStreamTx = broadcast::Sender<tendermint::Block>;

/// Block stream channel Receiver.
pub type BlockStreamRx = broadcast::Receiver<tendermint::Block>;

///
/// Create a shutdown channel.
///
pub fn create_shutdown_channel() -> (ShutdownTx, ShutdownRx) {
    broadcast::channel(1)
}
