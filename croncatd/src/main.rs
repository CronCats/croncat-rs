//!
//! The `croncatd` agent.
//!

use std::process::exit;

use croncat::{
    channels,
    //    client::{BankQueryClient, QueryBank},
    config::ChainConfigFile,
    errors::Report,
    grpc::{GrpcQuerier, GrpcSigner},
    logging::{self, error, info},
    store::{agent::LocalAgentStorage, logs::ErrorLogStorage},
    system,
    tokio,
};
use once_cell::sync::OnceCell;
use opts::Opts;

mod cli;
mod opts;

static CHAIN_ID: OnceCell<String> = OnceCell::new();

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
    if let Err(err) = run_command(opts.clone(), storage).await {
        error!("Command failed: {}", opts.cmd);
        error!("{}", err);

        ErrorLogStorage::write(CHAIN_ID.get().unwrap(), &err)?;

        if opts.debug {
            Err(err)?;
        }

        exit(1);
    }

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
            CHAIN_ID.set(chain_id.clone()).unwrap();
            let key = storage.get_agent_signing_key(&sender_name)?;
            let signer =
                GrpcSigner::new(ChainConfigFile::new(&chain_id).await?.first(), key).await?;

            info!("Key: {}", signer.key().public_key().to_json());
            info!(
                "Payable account Id: {}",
                serde_json::to_string_pretty(&payable_account_id)?
            );

            let result = signer.register_agent(payable_account_id).await?;
            let log = result.log;
            info!("Log: {log}");
        }
        opts::Command::UnregisterAgent {
            sender_name,
            chain_id,
        } => {
            CHAIN_ID.set(chain_id.clone()).unwrap();
            let key = storage.get_agent_signing_key(&sender_name)?;
            let signer =
                GrpcSigner::new(ChainConfigFile::new(&chain_id).await?.first(), key).await?;
            let result = signer.unregister_agent().await?;
            let log = result.log;
            info!("Log: {log}");
        }
        opts::Command::Withdraw {
            sender_name,
            chain_id,
        } => {
            CHAIN_ID.set(chain_id.clone()).unwrap();
            let key = storage.get_agent_signing_key(&sender_name)?;
            let signer =
                GrpcSigner::new(ChainConfigFile::new(&chain_id).await?.first(), key).await?;
            let result = signer.withdraw_reward().await?;
            let log = result.log;
            info!("Log: {log}");
        }
        opts::Command::Info { chain_id } => {
            CHAIN_ID.set(chain_id.clone()).unwrap();
            let querier = GrpcQuerier::new(ChainConfigFile::new(&chain_id).await?.first()).await?;
            let config = querier.query_config().await?;
            info!("Config: {config}")
        }
        opts::Command::GetAgentStatus {
            account_id,
            chain_id,
        } => {
            CHAIN_ID.set(chain_id.clone()).unwrap();
            let querier = GrpcQuerier::new(ChainConfigFile::new(&chain_id).await?.first()).await?;
            let status = querier.get_agent(account_id).await?;
            info!("Agent Status: {status}")
        }
        opts::Command::Tasks {
            from_index,
            limit,
            chain_id,
        } => {
            CHAIN_ID.set(chain_id.clone()).unwrap();
            let querier = GrpcQuerier::new(ChainConfigFile::new(&chain_id).await?.first()).await?;
            let tasks = querier.get_tasks(from_index, limit).await?;
            info!("Tasks: {tasks}")
        }
        opts::Command::GetAgentTasks {
            account_addr,
            chain_id,
        } => {
            CHAIN_ID.set(chain_id.clone()).unwrap();
            let querier = GrpcQuerier::new(ChainConfigFile::new(&chain_id).await?.first()).await?;
            let agent_tasks = querier.get_agent_tasks(account_addr).await?;
            info!("Agent Tasks: {agent_tasks}")
        }
        opts::Command::GenerateMnemonic { new_name, mnemonic } => {
            storage.generate_account(new_name, mnemonic)?
        }
        opts::Command::UpdateAgent {
            payable_account_id,
            sender_name,
            chain_id,
        } => {
            CHAIN_ID.set(chain_id.clone()).unwrap();
            let key = storage.get_agent_signing_key(&sender_name)?;
            let signer =
                GrpcSigner::new(ChainConfigFile::new(&chain_id).await?.first(), key).await?;
            let result = signer.update_agent(payable_account_id).await?;
            let log = result.log;
            info!("Log: {log}");
        }
        //@TODO: remember to finish this command, since it's only querying
        opts::Command::DepositUjunox {
            account_id: _,
            chain_id: _,
        } => {
            // CHAIN_ID.set(chain_id.clone()).unwrap();
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
            CHAIN_ID.set(chain_id.clone()).unwrap();
            let key = storage.get_agent_signing_key(&sender_name)?;
            let config_file = ChainConfigFile::new(&chain_id).await?;
            // Create a channel to handle graceful shutdown and wrap receiver for cloning
            let (shutdown_tx, shutdown_rx) = channels::create_shutdown_channel();
            // Start the agent
            system::run(shutdown_tx, shutdown_rx, key, &config_file, with_rules).await?;
        }
        opts::Command::SetupService { chain_id, output } => {
            system::DaemonService::create(output, &chain_id, opts.no_frills)?;
        }
        #[cfg(feature = "debug")]
        opts::Command::GetState { .. } => {
            // let querier = GrpcQuerier::new(_cfg).await?;

            // let state = querier.get_contract_state(from_index, limit).await?;
            // println!("{state}");
        }
    }

    Ok(())
}
