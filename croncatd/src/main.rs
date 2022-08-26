//!
//! The `croncatd` agent.
//!

use std::process::exit;

use croncat::{
    channels, env,
    errors::Report,
    grpc::update_agent,
    logging::{self, info},
    store::agent::LocalAgentStorage,
    system, tokio,
    utils::setup_cosm_orc,
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
    let cosm_orc = setup_cosm_orc(&env.croncat_addr)?;
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
        opts::Command::RegisterAgent {
            mut payable_account_id,
        } => {
            let key = storage.get_agent_signing_key(&opts.account_id)?;
            if payable_account_id.is_none() {
                payable_account_id = Some(key.to_account("juno").unwrap().to_string());
            }
            println!("key {:?}", key);

            println!("Account Id {}", payable_account_id.clone().unwrap());
            let result = croncat::grpc::register_agent(
                cosm_orc,
                payable_account_id.expect("Invalid payable_account_id!"),
                key,
            )
            .await?;
            println!("{result:?}");
        }
        opts::Command::UnregisterAgent { .. } => {
            info!("Unregister agent...");
        }
        opts::Command::GenerateMnemonic => storage.register(opts.account_id)?,
        opts::Command::UpdateAgent { payable_account_id } => {
            let res = update_agent(
                cosm_orc,
                storage.get_agent_signing_key(&opts.account_id)?,
                payable_account_id,
            )
            .await?;
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
