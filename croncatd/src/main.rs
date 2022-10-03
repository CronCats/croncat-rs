//!
//! The `croncatd` agent.
//!

use std::process::exit;

use croncat::{
    channels,
    //    client::{BankQueryClient, QueryBank},
    config::ChainConfig,
    errors::{eyre, Report},
    grpc::{GrpcQuerier, GrpcSigner},
    logging::{self, error, info},
    store::agent::LocalAgentStorage,
    system,
    tokio,
};
use opts::Opts;

mod cli;
mod opts;

///
/// Start the `croncatd` agent.
///
#[tokio::main]
async fn main() -> Result<(), Report> {
    // Get environment variables
    let storage = LocalAgentStorage::new();

    // Setup tracing and error reporting
    logging::setup()?;

    // Get the CLI options, handle argument errors nicely
    let opts = cli::get_opts()
        .map_err(|err| {
            eprintln!("{}", err);
            exit(1);
        })
        .unwrap();

    // If there ain't no no-frills...
    if !opts.no_frills {
        cli::print_banner();
    }

    info!("Starting croncatd...");

    // Run a command
    run_command(opts.clone(), storage).await.map_err(|err| {
        error!("Error from {} command...", opts.cmd);
        error!("{}", err);
        err
    })?;

    // Say goodbye if no no-frills
    if !opts.no_frills {
        println!("\n🐱 Cron Cat says: Goodbye / さようなら\n");
    }

    Ok(())
}

async fn run_command(opts: Opts, mut storage: LocalAgentStorage) -> Result<(), Report> {
    match opts.cmd {
        opts::Command::RegisterAgent {
            payable_account_id,
            sender_name,
            chain_id,
        } => {
            let key = storage.get_agent_signing_key(&sender_name)?;
            let signer = GrpcSigner::new(ChainConfig::new(&chain_id).await?, key).await?;

            println!("Key: {}", signer.key().public_key().to_json());
            println!(
                "Payable account Id: {}",
                serde_json::to_string_pretty(&payable_account_id)?
            );

            let result = signer.register_agent(payable_account_id).await?;
            let log = result.log;
            println!("{log}");
        }
        opts::Command::UnregisterAgent {
            sender_name,
            chain_id,
        } => {
            let key = storage.get_agent_signing_key(&sender_name)?;
            let signer = GrpcSigner::new(ChainConfig::new(&chain_id).await?, key).await?;
            let result = signer.unregister_agent().await?;
            let log = result.log;
            println!("{log}");
        }
        opts::Command::Withdraw {
            sender_name,
            chain_id,
        } => {
            let key = storage.get_agent_signing_key(&sender_name)?;
            let signer = GrpcSigner::new(ChainConfig::new(&chain_id).await?, key).await?;
            let result = signer.withdraw_reward().await?;
            let log = result.log;
            println!("{log}");
        }
        opts::Command::Info { chain_id } => {
            let querier = GrpcQuerier::new(ChainConfig::new(&chain_id).await?).await?;
            let config = querier.query_config().await?;
            println!("{config}")
        }
        opts::Command::GetAgentStatus {
            account_id,
            chain_id,
        } => {
            let querier = GrpcQuerier::new(ChainConfig::new(&chain_id).await?).await?;
            let status = querier.get_agent(account_id).await?;
            println!("status: {status}")
        }
        opts::Command::Tasks {
            from_index,
            limit,
            chain_id,
        } => {
            let querier = GrpcQuerier::new(ChainConfig::new(&chain_id).await?).await?;
            let tasks = querier.get_tasks(from_index, limit).await?;
            println!("{tasks}")
        }
        opts::Command::GetAgentTasks {
            account_addr,
            chain_id,
        } => {
            let querier = GrpcQuerier::new(ChainConfig::new(&chain_id).await?).await?;
            let agent_tasks = querier.get_agent_tasks(account_addr).await?;
            println!("{agent_tasks}")
        }
        opts::Command::GenerateMnemonic { new_name, mnemonic } => {
            storage.generate_account(new_name, mnemonic)?
        }
        opts::Command::UpdateAgent {
            payable_account_id,
            sender_name,
            chain_id,
        } => {
            let key = storage.get_agent_signing_key(&sender_name)?;
            let signer = GrpcSigner::new(ChainConfig::new(&chain_id).await?, key).await?;
            let result = signer.update_agent(payable_account_id).await?;
            let log = result.log;
            println!("{log}");
        }
        //@TODO: remember to finish this command, since it's only querying
        opts::Command::DepositUjunox {
            account_id: _,
            chain_id: _,
        } => {
            todo!("Credit webservice is not working for now!");
            // //let result = deposit_junox(&account_id).await?;
            // let cfg = ChainConfig::new(&chain_id).await?;
            // let bank_q_client =
            //     BankQueryClient::new(cfg.grpc_endpoint, "ujunox".to_string()).await?;
            // println!(
            //     "new balance: {:?}",
            //     bank_q_client.query_native_balance(&account_id).await?
            // );
        }
        opts::Command::GetAgent { name } => storage.display_account(&name),
        opts::Command::Go {
            sender_name,
            with_rules,
            chain_id,
        } => {
            let key = storage.get_agent_signing_key(&sender_name)?;
            let signer = GrpcSigner::new(ChainConfig::new(&chain_id).await?, key)
                .await
                .map_err(|err| eyre!("Failed to setup GRPC: {}", err))?;
            let initial_status = signer
                .get_agent(signer.account_id().as_ref())
                .await?
                .ok_or(eyre!("Agent must be registered to start the loop"))?
                .status;
            // Create a channel to handle graceful shutdown and wrap receiver for cloning
            let (shutdown_tx, shutdown_rx) = channels::create_shutdown_channel();
            // Start the agent
            system::run(shutdown_tx, shutdown_rx, signer, initial_status, with_rules).await?;
        }
        opts::Command::SetupService { chain_id, output } => {
            system::DaemonService::create(output, &chain_id, opts.no_frills)?;
        }
        _ => {}
    }

    Ok(())
}
