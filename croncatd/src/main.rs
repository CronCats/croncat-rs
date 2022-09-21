//!
//! The `croncatd` agent.
//!

use std::process::exit;

use croncat::{
    channels,
    client::{BankQueryClient, QueryBank},
    config::ChainConfig,
    errors::Report,
    grpc::{GrpcQuerier, GrpcSigner},
    logging::{self, info},
    store::agent::LocalAgentStorage,
    system, tokio,
};

use crate::cli::deposit_junox;

mod cli;
mod opts;

///
/// Start the `croncatd` agent.
///
#[tokio::main]
async fn main() -> Result<(), Report> {
    // Get environment variables
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

    let chain_id = opts.chain_id.clone();
    let cfg = ChainConfig::new(&chain_id).await?;
    info!("Starting croncatd...");
    match opts.cmd {
        opts::Command::RegisterAgent {
            payable_account_id,
            sender_name,
        } => {
            let key = storage.get_agent_signing_key(&sender_name)?;
            let signer = GrpcSigner::new(cfg, key).await?;

            println!("Key: {}", signer.key().public_key().to_json());
            println!(
                "Payable account Id: {}",
                serde_json::to_string_pretty(&payable_account_id)?
            );

            let result = signer.register_agent(payable_account_id).await?;
            let log = result.log;
            println!("{log}");
        }
        opts::Command::UnregisterAgent { sender_name } => {
            let key = storage.get_agent_signing_key(&sender_name)?;
            let signer = GrpcSigner::new(cfg, key).await?;
            let result = signer.unregister_agent().await?;
            let log = result.log;
            println!("{log}");
        }
        opts::Command::Withdraw { sender_name } => {
            let key = storage.get_agent_signing_key(&sender_name)?;
            let signer = GrpcSigner::new(cfg, key).await?;
            let result = signer.withdraw_reward().await?;
            let log = result.log;
            println!("{log}");
        }
        opts::Command::Info => {
            let querier = GrpcQuerier::new(cfg).await?;
            let config = querier.query_config().await?;
            println!("{config}")
        }
        opts::Command::GetAgentStatus { account_id } => {
            let querier = GrpcQuerier::new(cfg).await?;
            let status = querier.get_agent(account_id).await?;
            println!("status: {status}")
        }
        opts::Command::Tasks { from_index, limit } => {
            let querier = GrpcQuerier::new(cfg).await?;
            let tasks = querier.get_tasks(from_index, limit).await?;
            println!("{tasks}")
        }
        opts::Command::GetAgentTasks { account_addr } => {
            let querier = GrpcQuerier::new(cfg).await?;
            let agent_tasks = querier.get_agent_tasks(account_addr).await?;
            println!("{agent_tasks}")
        }
        opts::Command::GenerateMnemonic { new_name, mnemonic } => {
            storage.generate_account(new_name, mnemonic)?
        }
        opts::Command::UpdateAgent {
            payable_account_id,
            sender_name,
        } => {
            let key = storage.get_agent_signing_key(&sender_name)?;
            let signer = GrpcSigner::new(cfg, key).await?;
            let result = signer.update_agent(payable_account_id).await?;
            let log = result.log;
            println!("{log}");
        }
        //@TODO: remember to finish this command, since it's only querying
        opts::Command::DepositUjunox { account_id } => {
            let result = deposit_junox(&account_id).await?;
            println!("{:?}", result);
            let bank_q_client =
                BankQueryClient::new(cfg.grpc_endpoint, "ujunox".to_string()).await?;
            println!(
                "new balance: {:?}",
                bank_q_client.query_native_balance(&account_id).await?
            );
        }
        opts::Command::GetAgent { name } => storage.display_account(&name),
        opts::Command::Go { sender_name } => {
            let key = storage.get_agent_signing_key(&sender_name)?;
            let signer = GrpcSigner::new(cfg, key).await?;
            let (shutdown_tx, shutdown_rx) = channels::create_shutdown_channel();
            system::go(shutdown_tx, shutdown_rx, signer).await?;
        }
        opts::Command::SetupService { output } => {
            system::ServiceDaemon::create(output, &chain_id)?;
        }
        _ => {}
    }

    // Say goodbye if no no-frills
    if !opts.no_frills {
        println!("\n🐱 Cron Cat says: Goodbye / さようなら\n");
    }

    Ok(())
}
