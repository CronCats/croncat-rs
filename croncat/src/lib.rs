//!
//! The building blocks for a service that needs to interact with croncat.
//!
//! ## Example Agent:
//!
//! Taken from [croncatd/src/main.rs](../croncatd/fn.main.html).
//!
//! There are bunch of other agent daemon specific things that need to be setup first,
//! look at the source code for how it's done ATM.
//!
//! ```
//! use std::process::exit;
//!
//! use croncat::{
//!     errors::Report,
//!     grpc,
//!     logging::{self, info},
//!     tokio, ws,
//! };
//!
//! mod cli;
//! mod env;
//! mod opts;
//!
//! ///
//! /// Start the `croncatd` agent.
//! ///
//! #[tokio::main]
//! async fn main() -> Result<(), Report> {
//!     // Get environment variables
//!     let env = env::load()?;
//!
//!     // Setup tracing and error reporting
//!     logging::setup()?;
//!
//!     // Get the CLI options, handle argument errors nicely
//!     let opts = cli::get_opts()
//!         .map_err(|e| {
//!             println!("{}", e);
//!             exit(1);
//!         })
//!         .unwrap();
//!
//!     // If there ain't no no-frills...
//!     if !opts.no_frills {
//!         cli::print_banner();
//!     }
//!
//!     info!("Starting croncatd...");
//!
//!     // Create a channel to handle graceful shutdown
//!     let (shutdown_tx, mut shutdown_rx) = cli::create_shutdown_channel();
//!
//!     // Connect to GRPC
//!     let (_msg_client, _query_client) = grpc::connect(env.grpc_url.clone()).await?;
//!
//!     // Stream new blocks from the WS RPC subscription
//!     let block_stream = tokio::task::spawn(async move {
//!         ws::stream_blocks(env.wsrpc_url.clone(), &mut shutdown_rx)
//!             .await
//!             .expect("Failed");
//!     });
//!
//!     // Handle SIGINT AKA Ctrl-C
//!     let ctrl_c = tokio::task::spawn(async move {
//!         tokio::signal::ctrl_c()
//!             .await
//!             .expect("Failed to wait for Ctrl-C");
//!         shutdown_tx
//!             .send(())
//!             .await
//!             .expect("Failed to send shutdown signal");
//!         println!("");
//!         info!("Shutting down croncatd...");
//!     });
//!
//!     // TODO: Do something with the second return value
//!     let (_, _) = tokio::join!(ctrl_c, block_stream);
//!
//!     // Say goodbye if no no-frills
//!     if !opts.no_frills {
//!         println!("\n🐱 Cron Cat says: Goodbye / さようなら\n");
//!     }
//!
//!     Ok(())
//! }
//! ```
//!

// Features
#![feature(no_coverage)]

// Export tokio for convenience and version management
pub use tokio;

// Our modules
pub mod channels;
pub mod consumers;
pub mod errors;
pub mod grpc;
pub mod logging;
pub mod ws;
