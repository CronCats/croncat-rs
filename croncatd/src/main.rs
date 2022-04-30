use std::time::Duration;

use croncat::{logging::{self, info}, errors::Report, tokio, grpc};

mod cli;
mod opts;

#[tokio::main]
async fn main() -> Result<(), Report> {
    // Setup tracing and error reporting
    logging::setup()?;
    
    // Get the CLI options
    let opts = cli::get_opts()?;

    // If there ain't no no-banner...
    if !opts.no_banner {
        cli::print_banner();
    }

    info!("Starting croncatd...");

    // Create a channel to handle graceful shutdown
    let (shutdown_tx, mut shutdown_rx) = cli::create_shutdown_channel();
    
    // Connect to GRPC
    let (_msg_client, _query_client) = grpc::connect().await?;

    // Handle SIGINT AKA Ctrl-C
    let ctrl_c = tokio::task::spawn( async move {
        tokio::signal::ctrl_c().await.expect("Failed to wait for Ctrl-C");
        shutdown_tx.send(()).await.expect("Failed to send shutdown signal");
        println!("");
        info!("Shutting down croncatd...");
    });

    // TODO: Implement actual work
    let main_loop = tokio::task::spawn(async move {
        // Main test loop for now...
        let task = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(1000)).await;
                info!("🔄 Looping... 🔁");
            }
        });
        let handle_shutdown = tokio::spawn(async move { shutdown_rx.recv().await });

        tokio::select! {
            _ = task => {},
            _ = handle_shutdown => {
                info!("Task loop shutdown...");
                // TODO: Handle shutdown
            }
        }
    });

    // TODO: Do something with the main_loop return value
    let (_, _main_result) = tokio::join!(ctrl_c, main_loop);

    println!("\n🐱 Cron Cat says: Goodbye / さようなら\n");

    Ok(())
}

