//!
//! The `croncatd` agent.
//!

use std::process::exit;

use croncat::{
    channels, env,
    errors::Report,
    grpc::update_agent,
    logging::{self, info},
    seed_generator::{generate_save_mnemonic, get_agent_signing_key},
    system, tokio,
};

mod cli;
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

    match opts.cmd {
        opts::Command::RegisterAgent { .. } => {
            info!("Register agent...");
        }
        opts::Command::UnregisterAgent { .. } => {
            info!("Unregister agent...");
        }
        opts::Command::GenerateMnemonic => generate_save_mnemonic()?,
        opts::Command::UpdateAgent { payable_account_id } => {
            let res = update_agent(
                env.croncat_addr,
                get_agent_signing_key()?,
                payable_account_id,
            ).await?;
            println!("{res:?}");
        }
        _ => {
            // Create a channel to handle graceful shutdown and wrap receiver for cloning
            let (shutdown_tx, shutdown_rx) = channels::create_shutdown_channel();

            // Start the agent
            system::run(env, shutdown_tx, shutdown_rx).await?;
        }
    }

    // Say goodbye if no no-frills
    if !opts.no_frills {
        println!("\n🐱 Cron Cat says: Goodbye / さようなら\n");
    }

    Ok(())
}
