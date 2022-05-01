//!
//! Various channel types for croncat.
//!

use tokio::sync::{mpsc, watch};

/// Shutdown channel Sender.
pub type ShutdownTx = watch::Sender<()>;

/// Shutdown channel Receiver.
pub type ShutdownRx = watch::Receiver<()>;

/// Block stream channel Sender.
pub type BlockStreamTx = mpsc::Sender<tendermint::Block>;

/// Block stream channel Receiver.
pub type BlockStreamRx = mpsc::Receiver<tendermint::Block>;
