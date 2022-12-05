//!
//! The `croncatd` agent.
//!

use croncat::{
    channels,
    //    client::{BankQueryClient, QueryBank},
    config::ChainConfig,
    errors::{eyre, Report},
    grpc::{GrpcQuerier, GrpcSigner},
    logging::{self, error, info},
    store::{agent::LocalAgentStorage, logs::ErrorLogStorage},
    system,
    tokio,
    utils::SUPPORTED_CHAIN_IDS,
};
use once_cell::sync::OnceCell;
use opts::Opts;
use std::process::exit;

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
            return Err(err);
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
            let _guards = logging::setup_go(chain_id.to_string())?;
            CHAIN_ID.set(chain_id.clone()).unwrap();

            let key = storage.get_agent_signing_key(&sender_name)?;
            let signer = GrpcSigner::new(ChainConfig::new(&chain_id).await?, key).await?;

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
            let _guards = logging::setup_go(chain_id.to_string())?;
            CHAIN_ID.set(chain_id.clone()).unwrap();

            let key = storage.get_agent_signing_key(&sender_name)?;
            let signer = GrpcSigner::new(ChainConfig::new(&chain_id).await?, key).await?;
            let result = signer.unregister_agent().await?;
            let log = result.log;
            info!("Log: {log}");
        }
        opts::Command::Withdraw {
            sender_name,
            chain_id,
        } => {
            let _guards = logging::setup_go(chain_id.to_string())?;
            CHAIN_ID.set(chain_id.clone()).unwrap();

            let key = storage.get_agent_signing_key(&sender_name)?;
            let signer = GrpcSigner::new(ChainConfig::new(&chain_id).await?, key).await?;
            let result = signer.withdraw_reward().await?;
            let log = result.log;
            info!("Log: {log}");
        }
        opts::Command::Info { chain_id } => {
            let _guards = logging::setup_go(chain_id.to_string())?;
            CHAIN_ID.set(chain_id.clone()).unwrap();

            let querier = GrpcQuerier::new(ChainConfig::new(&chain_id).await?).await?;
            let config = querier.query_config().await?;
            info!("Config: {config}")
        }
        opts::Command::GetAgentAccounts {
            sender_name,
            chain_id,
        } => {
            // IF chain ID, then print the prefix derived account address from chain-id
            if chain_id != "local" {
                let config = ChainConfig::new(&chain_id.clone().to_string()).await?;
                let prefix = config.prefix;
                let account_addr = storage.get_agent_signing_account_addr(&sender_name, prefix)?;

                println!("{}: {}", chain_id, account_addr);
            } else {
                info!("Account Addresses for: {sender_name}");
                // Loop and print supported accounts for a keypair
                for chain_id in SUPPORTED_CHAIN_IDS.iter() {
                    let config = ChainConfig::new(&chain_id.to_string()).await?;
                    let prefix = config.prefix;
                    let account_addr =
                        storage.get_agent_signing_account_addr(&sender_name, prefix)?;

                    println!("{}: {}", chain_id, account_addr);
                }
            }
        }
        opts::Command::GetAgentStatus {
            account_id,
            chain_id,
        } => {
            let _guards = logging::setup_go(chain_id.to_string())?;
            CHAIN_ID.set(chain_id.clone()).unwrap();

            let querier = GrpcQuerier::new(ChainConfig::new(&chain_id).await?).await?;
            let status = querier.get_agent(account_id).await?;
            info!("Agent Status: {status}")
        }
        opts::Command::Tasks {
            from_index,
            limit,
            chain_id,
        } => {
            let _guards = logging::setup_go(chain_id.to_string())?;
            CHAIN_ID.set(chain_id.clone()).unwrap();

            let querier = GrpcQuerier::new(ChainConfig::new(&chain_id).await?).await?;
            let tasks = querier.get_tasks(from_index, limit).await?;
            info!("Tasks: {tasks}")
        }
        opts::Command::GetAgentTasks {
            account_addr,
            chain_id,
        } => {
            let _guards = logging::setup_go(chain_id.to_string())?;
            CHAIN_ID.set(chain_id.clone()).unwrap();

            let querier = GrpcQuerier::new(ChainConfig::new(&chain_id).await?).await?;
            let agent_tasks = querier.get_agent_tasks(account_addr).await?;
            info!("Agent Tasks: {agent_tasks}")
        }
        opts::Command::GenerateMnemonic { new_name, mnemonic } => {
            storage.generate_account(new_name, mnemonic).await?
        }
        opts::Command::UpdateAgent {
            payable_account_id,
            sender_name,
            chain_id,
        } => {
            let _guards = logging::setup_go(chain_id.to_string())?;
            CHAIN_ID.set(chain_id.clone()).unwrap();

            let key = storage.get_agent_signing_key(&sender_name)?;
            let signer = GrpcSigner::new(ChainConfig::new(&chain_id).await?, key).await?;
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
            let _guards = logging::setup_go(chain_id.to_string())?;
            CHAIN_ID.set(chain_id.clone()).unwrap();

            let key = storage.get_agent_signing_key(&sender_name)?;
            let cfg = ChainConfig::new(&chain_id).await?;
            let ChainConfig {
                polling_duration_secs,
                ..
            } = cfg;
            let signer = GrpcSigner::new(cfg, key)
                .await
                .map_err(|err| eyre!("Failed to setup GRPC: {}", err))?;
            let initial_status = signer
                .get_agent(signer.account_id.as_ref())
                .await?
                .ok_or(eyre!("Agent must be registered to start the loop"))?
                .status;
            // Create a channel to handle graceful shutdown and wrap receiver for cloning
            let (shutdown_tx, shutdown_rx) = channels::create_shutdown_channel();
            // Start the agent
            system::run_retry(
                &shutdown_tx,
                &shutdown_rx,
                &signer,
                &initial_status,
                with_rules,
                polling_duration_secs,
            )
            .await?;
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
