//!
//! The `croncatd` agent.
//!

use std::process::exit;
use tokio_compat::prelude::*;

use croncat::{
    channels, env,
    errors::Report,
    grpc::{OrcQuerier, OrcSigner},
    logging::{self, info},
    store::agent::LocalAgentStorage,
    system,
    tokio::runtime::Runtime,
};

use crate::cli::deposit_junox;

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
        opts::Command::Withdraw => {
            let key = storage.get_agent_signing_key(&opts.account_id)?;
            let mut signer = OrcSigner::new(&env.croncat_addr, key)?;
            let result = signer.withdraw_reward()?;
            let log = result.log;
            println!("{log}");
        }
        opts::Command::Info => {
            let mut querier = OrcQuerier::new(&env.croncat_addr)?;
            let config = querier.query_config()?;
            println!("{config}")
        }
        opts::Command::GetAgentStatus { account_id } => {
            let mut querier = OrcQuerier::new(&env.croncat_addr)?;
            let status = querier.get_agent(account_id)?;
            println!("{status}")
        }
        opts::Command::Tasks {from_index, limit} => {
            let mut querier = OrcQuerier::new(&env.croncat_addr)?;
            let tasks = querier.get_tasks(from_index,limit)?;
            println!("{tasks}")
        }
          opts::Command::GetAgentTasks {account_id } => {
            let mut querier = OrcQuerier::new(&env.croncat_addr)?;
            let agent_tasks = querier.get_agent_tasks(account_id)?;
            println!("{agent_tasks}")
        }
        opts::Command::GenerateMnemonic => storage.generate_account(opts.account_id)?,
        opts::Command::UpdateAgent { payable_account_id } => {
            let key = storage.get_agent_signing_key(&opts.account_id)?;
            let mut signer = OrcSigner::new(&env.croncat_addr, key)?;
            let result = signer.update_agent(payable_account_id)?;
            let log = result.log;
            println!("{log}");
        }
         opts::Command::DepositUjunox { account_id } =>{
            //let result=task.await;
            tokio_compat::run_std(async move {
                let result=deposit_junox(account_id.as_ref().unwrap()).await;
                println!(" {:?}", result);

            });
            //let result= futures::executor::block_on(Compat::new(task));
        }
        opts::Command::GetAgent => storage.display_account(&opts.account_id),
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
