//!
//! The `croncatd` agent.
//!

use std::process::exit;

use croncat::{
    channels, env,
    errors::Report,
    grpc::OrcSigner,
    logging::{self, info},
    store::agent::LocalAgentStorage,
    system,
    tokio::runtime::Runtime,
};

mod cli;
mod opts;

///
/// Start the `croncatd` agent.
///
fn main() -> Result<(), Report> {
    // Get environment variables
    let env = env::load()?;
    let mut storage = LocalAgentStorage::new();

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
        opts::Command::RegisterAgent { payable_account_id } => {
            let key = storage.get_agent_signing_key(&opts.account_id)?;
            println!("key {:?}", key);

            println!("Account Id {:?}", payable_account_id);
            let mut signer = OrcSigner::new(&env.croncat_addr, key)?;
            let result = signer.register_agent(payable_account_id)?;
            let log = result.log;
            println!("{log}");
        }
        opts::Command::UnregisterAgent { .. } => {
            info!("Unregister agent...");
        }
        opts::Command::GenerateMnemonic => storage.generate_account(opts.account_id)?,
        opts::Command::UpdateAgent { payable_account_id } => {
            let key = storage.get_agent_signing_key(&opts.account_id)?;
            let mut signer = OrcSigner::new(&env.croncat_addr, key)?;
            let result = signer.update_agent(payable_account_id)?;
            let log = result.log;
            println!("{log}");
        }
        _ => {
            // Create a channel to handle graceful shutdown and wrap receiver for cloning
            let (shutdown_tx, shutdown_rx) = channels::create_shutdown_channel();

            // Start the agent
            Runtime::new()
                .unwrap()
                .block_on(async { system::run(env, shutdown_tx, shutdown_rx).await })?;
        }
    }

    // Say goodbye if no no-frills
    if !opts.no_frills {
        println!("\n🐱 Cron Cat says: Goodbye / さようなら\n");
    }

    Ok(())
}
