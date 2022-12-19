//!
//! Various channel types for croncat.
//!

use tokio::sync::broadcast;

use crate::utils;

/// Shutdown channel Sender.
pub type ShutdownTx = broadcast::Sender<()>;

/// Shutdown channel Receiver.
pub type ShutdownRx = broadcast::Receiver<()>;

/// Block stream channel Sender.
pub type BlockStreamTx = broadcast::Sender<utils::Block>;

/// Block stream channel Receiver.
pub type BlockStreamRx = broadcast::Receiver<utils::Block>;

///
/// Create a shutdown channel.
///
pub fn create_shutdown_channel() -> (ShutdownTx, ShutdownRx) {
    broadcast::channel(1)
}
