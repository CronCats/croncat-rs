//!
//! The `croncatd` agent.
//!

use croncat::{
    channels::create_shutdown_channel,
    //    client::{BankQueryClient, QueryBank},
    config::Config,
    errors::{eyre, Report},
    grpc::GrpcSigner,
    logging::{self, error, info},
    store::agent::LocalAgentStorage,
    system,
    tokio,
};
use opts::Opts;
use std::process::exit;

mod cli;
mod opts;

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

    // Setup tracing and logging.
    let _logging_guards = logging::setup(opts.chain_id.clone())?;

    // If there ain't no no-frills...
    if !opts.no_frills {
        cli::print_banner();
    }

    // Run a command and handle errors
    if let Err(err) = run_command(opts.clone(), storage).await {
        error!("Command failed: {}", opts.cmd);
        error!("{}", err);

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
    // Get the key for the agent signing account
    let config = Config::from_pwd()?;

    match opts.cmd {
        opts::Command::Register {
            payable_account_id,
            agent,
        } => {
            // Make sure we have a chain id to run on
            if opts.chain_id.is_none() {
                return Err(eyre!("chain-id is required for go command"));
            }
            let chain_id = opts.chain_id.unwrap();

            // Get the chain config for the chain we're going to run on
            let chain_config = config
                .chains
                .get(&chain_id)
                .ok_or_else(|| eyre!("Chain not found in configuration: {}", chain_id))?;

            // Get the key and create a signer
            let key = storage.get_agent_signing_key(&agent)?;
            let signer = GrpcSigner::from_chain_config(chain_config, key)
                .await
                .map_err(|err| eyre!("Failed to create GrpcSigner: {}", err))?;

            // Print info about the agent about to be registered
            info!("Account ID: {}", signer.account_id());
            info!("Key: {}", signer.key().public_key().to_json());
            if payable_account_id.is_some() {
                info!(
                    "Payable account Id: {}",
                    serde_json::to_string_pretty(&payable_account_id)?
                );
            }

            // Register the agent
            let query = signer.register_agent(&payable_account_id).await;

            // Handle the result of the query
            match query {
                Ok(result) => {
                    info!("Agent registered successfully");
                    let log = result.log;
                    info!("Result: {}", log);
                }
                Err(err) if err.to_string().contains("Agent already exists") => {
                    Err(eyre!("Agent already registered"))?;
                }
                Err(err) => Err(eyre!("Failed to register agent: {}", err))?,
            }
        }
        opts::Command::Unregister { agent } => {
            // Make sure we have a chain id to run on
            if opts.chain_id.is_none() {
                return Err(eyre!("chain-id is required for go command"));
            }
            let chain_id = opts.chain_id.unwrap();

            // Get the chain config for the chain we're going to run on
            let chain_config = config
                .chains
                .get(&chain_id)
                .ok_or_else(|| eyre!("Chain not found in configuration: {}", chain_id))?;

            // Get the key and create a signer
            let key = storage.get_agent_signing_key(&agent)?;
            let signer = GrpcSigner::from_chain_config(chain_config, key)
                .await
                .map_err(|err| eyre!("Failed to create GrpcSigner: {}", err))?;

            // Print info about the agent about to be registered
            info!("Account ID: {}", signer.account_id());
            info!("Key: {}", signer.key().public_key().to_json());

            // Unregister the agent
            let query = signer.unregister_agent().await;

            // Handle the result of the query
            match query {
                Ok(result) => {
                    info!("Agent unregistered successfully");
                    let log = result.log;
                    info!("Result: {}", log);
                }
                Err(err) if err.to_string().contains("Agent not registered") => {
                    Err(eyre!("Agent not already registered"))?;
                }
                Err(err) => Err(eyre!("Failed to register agent: {}", err))?,
            }
        }
        // opts::Command::Withdraw {
        //     sender_name,
        //     chain_id,
        // } => {
        //     let _guards = logging::setup_go(chain_id.to_string())?;
        //     CHAIN_ID.set(chain_id.clone()).unwrap();

        //     let key = storage.get_agent_signing_key(&sender_name)?;
        //     let signer = GrpcSigner::new(ChainConfig::new(&chain_id).await?, key).await?;
        //     let result = signer.withdraw_reward().await?;
        //     let log = result.log;
        //     info!("Log: {log}");
        // }
        // opts::Command::Info { chain_id } => {
        //     let _guards = logging::setup_go(chain_id.to_string())?;
        //     CHAIN_ID.set(chain_id.clone()).unwrap();

        //     let querier = GrpcQuerier::new(ChainConfig::new(&chain_id).await?).await?;
        //     let config = querier.query_config().await?;
        //     info!("Config: {config}")
        // }
        opts::Command::ListAccounts { agent } => {
            println!("Account addresses for agent: {agent}\n");
            // Get the chain config for the chain we're going to run on
            for (chain_id, chain_config) in config.chains {
                let account_addr = storage
                    .get_agent_signing_account_addr(&agent, chain_config.info.bech32_prefix)?;
                println!("{}: {}", chain_id, account_addr);
            }
        }
        // opts::Command::GetAgentStatus {
        //     account_id,
        //     chain_id,
        // } => {
        //     let _guards = logging::setup_go(chain_id.to_string())?;
        //     CHAIN_ID.set(chain_id.clone()).unwrap();

        //     let querier = GrpcQuerier::new(ChainConfig::new(&chain_id).await?).await?;
        //     let status = querier.get_agent(account_id).await?;
        //     info!("Agent Status: {status}")
        // }
        // opts::Command::Tasks {
        //     from_index,
        //     limit,
        //     chain_id,
        // } => {
        //     let _guards = logging::setup_go(chain_id.to_string())?;
        //     CHAIN_ID.set(chain_id.clone()).unwrap();

        //     let querier = GrpcQuerier::new(ChainConfig::new(&chain_id).await?).await?;
        //     let tasks = querier.get_tasks(from_index, limit).await?;
        //     info!("Tasks: {tasks}")
        // }
        // opts::Command::GetAgentTasks {
        //     account_addr,
        //     chain_id,
        // } => {
        //     let _guards = logging::setup_go(chain_id.to_string())?;
        //     CHAIN_ID.set(chain_id.clone()).unwrap();

        //     let querier = GrpcQuerier::new(ChainConfig::new(&chain_id).await?).await?;
        //     let agent_tasks = querier.get_agent_tasks(account_addr).await?;
        //     info!("Agent Tasks: {agent_tasks}")
        // }
        opts::Command::GenerateMnemonic { agent, mnemonic } => {
            storage.generate_account(agent, mnemonic).await?
        }
        // opts::Command::UpdateAgent {
        //     payable_account_id,
        //     sender_name,
        //     chain_id,
        // } => {
        //     let _guards = logging::setup_go(chain_id.to_string())?;
        //     CHAIN_ID.set(chain_id.clone()).unwrap();

        //     let key = storage.get_agent_signing_key(&sender_name)?;
        //     let signer = GrpcSigner::new(ChainConfig::new(&chain_id).await?, key).await?;
        //     let result = signer.update_agent(payable_account_id).await?;
        //     let log = result.log;
        //     info!("Log: {log}");
        // }
        // //@TODO: remember to finish this command, since it's only querying
        // opts::Command::DepositUjunox {
        //     account_id: _,
        //     chain_id: _,
        // } => {
        //     // CHAIN_ID.set(chain_id.clone()).unwrap();
        //     todo!("Credit webservice is not working for now!");
        //     // //let result = deposit_junox(&account_id).await?;
        //     // let cfg = ChainConfig::new(&chain_id).await?;
        //     // let bank_q_client =
        //     //     BankQueryClient::new(cfg.grpc_endpoint, "ujunox".to_string()).await?;
        //     // println!(
        //     //     "new balance: {:?}",
        //     //     bank_q_client.query_native_balance(&account_id).await?
        //     // );
        // }
        opts::Command::GetAgent { name } => storage.display_account(&name),
        opts::Command::Go { agent, with_rules } => {
            // Make sure we have a chain id to run on
            if opts.chain_id.is_none() {
                return Err(eyre!("chain-id is required for go command"));
            }
            let chain_id = opts.chain_id.unwrap();

            // Get the key for the agent signing account
            let key = storage.get_agent_signing_key(&agent)?;

            // Get the chain config for the chain we're going to run on
            let chain_config = config
                .chains
                .get(&chain_id)
                .ok_or_else(|| eyre!("Chain not found in configuration: {}", chain_id))?;

            // Create the global shutdown channel
            let (shutdown_tx, _shutdown_rx) = create_shutdown_channel();

            // Run the agent on the chain
            system::run_retry(&chain_id, &shutdown_tx, chain_config, &key, with_rules).await?;
        }
        opts::Command::SetupService { output } => {
            for (chain_id, _) in config.chains {
                system::DaemonService::create(output.clone(), &chain_id, opts.no_frills)?;
            }
        }
        _ => unreachable!(), // TODO: Remove this when done debugging
    }

    Ok(())
}
