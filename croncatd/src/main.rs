//!
//! The `croncatd` agent.
//!

use std::process::exit;

use croncat::{
    consumers::block_consumer,
    errors::Report,
    grpc,
    logging::{self, info},
    tokio, ws,
};

mod cli;
mod env;
mod opts;

///
/// Start the `croncatd` agent.
///
#[tokio::main]
async fn main() -> Result<(), Report> {
    // Get environment variables
    let env = env::load()?;

    // Setup tracing and error reporting
    logging::setup()?;

    // Get the CLI options, handle argument errors nicely
    let opts = cli::get_opts()
        .map_err(|e| {
            println!("{}", e);
            exit(1);
        })
        .unwrap();

    // If there ain't no no-frills...
    if !opts.no_frills {
        cli::print_banner();
    }

    info!("Starting croncatd...");

    // Create a channel to handle graceful shutdown and wrap receiver for cloning
    let (shutdown_tx, shutdown_rx) = cli::create_shutdown_channel();

    // Create a block stream channel and wrap receiver for cloning
    // TODO (SeedyROM): Remove 128 hardcoded limit
    let (block_stream_tx, block_stream_rx) = block_consumer::create_stream(128);

    // Connect to GRPC
    let (_msg_client, _query_client) = grpc::connect(env.grpc_url.clone()).await?;

    // Stream new blocks from the WS RPC subscription
    let block_stream_shutdown_rx = shutdown_rx.clone();
    let block_stream_handle = tokio::task::spawn(async move {
        ws::stream_blocks(
            env.wsrpc_url.clone(),
            block_stream_tx.clone(),
            block_stream_shutdown_rx,
        )
        .await
        .expect("Failed stream blocks")
    });

    // Process blocks coming in from the blockchain
    let block_process_shutdown_rx = shutdown_rx.clone();
    let block_process_stream_handle = tokio::task::spawn(async move {
        block_consumer::consume(block_stream_rx.clone(), block_process_shutdown_rx)
            .await
            .expect("Failed to process streamed blocks")
    });

    // Handle SIGINT AKA Ctrl-C
    let ctrl_c_handle = tokio::task::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to wait for Ctrl-C");
        shutdown_tx
            .send(())
            .await
            .expect("Failed to send shutdown signal");
        println!("");
        info!("Shutting down croncatd...");
    });

    // TODO: Do something with the return values
    let _ = tokio::join!(
        ctrl_c_handle,
        block_stream_handle,
        block_process_stream_handle
    );

    // Say goodbye if no no-frills
    if !opts.no_frills {
        println!("\n🐱 Cron Cat says: Goodbye / さようなら\n");
    }

    Ok(())
}
