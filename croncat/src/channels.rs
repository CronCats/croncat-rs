//!
//! Various channel types for croncat.
//!

use tokio::sync::mpsc;

///
/// Shutdown channel Sender.
///
pub type ShutdownTx = mpsc::Sender<()>;

///
/// Shutdown channel Receiver.
///
pub type ShutdownRx = mpsc::Receiver<()>;
