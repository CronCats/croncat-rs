//!
//! Various channel types for croncat.
//!

/// Shutdown channel Sender.
pub type ShutdownTx = async_channel::Sender<()>;

/// Shutdown channel Receiver.
pub type ShutdownRx = async_channel::Receiver<()>;

/// Block stream channel Sender.
pub type BlockStreamTx = async_channel::Sender<tendermint::Block>;

/// Block stream channel Receiver.
pub type BlockStreamRx = async_channel::Receiver<tendermint::Block>;
