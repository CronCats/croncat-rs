//!
//! Various channel types for croncat.
//!

use tokio::sync::watch;

///
/// Shutdown channel Sender.
///
pub type ShutdownTx = watch::Sender<()>;

///
/// Shutdown channel Receiver.
///
pub type ShutdownRx = watch::Receiver<()>;
