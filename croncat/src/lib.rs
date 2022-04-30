// Features
#![feature(no_coverage)]

// Export tokio for convenience
pub use tokio;
use tokio::sync::mpsc;

// Our modules
pub mod errors;
pub mod grpc;
pub mod logging;
pub mod ws;

///
/// Shutdown channel Sender.
///
pub type ShutdownTx = mpsc::Sender<()>;

///
/// Shutdown channel Receiver.
///
pub type ShutdownRx = mpsc::Receiver<()>;
