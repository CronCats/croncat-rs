//!
//! The `croncatd` agent.
//!

use croncat::{
    channels::create_shutdown_channel,
    //    client::{BankQueryClient, QueryBank},
    config::Config,
    errors::{eyre, Report},
    logging::{self, error, info},
    modules::{agent::Agent, factory::Factory, manager::Manager, tasks::Tasks},
    rpc::{Querier, RpcClientService, Signer},
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
            eprintln!("{err}");
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
    // TODO: for opts::Command::Go need to handle errors & reboot if possible
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
    let key = storage.get_agent_signing_key(&opts.agent)?;

    // Get an rpc client
    let factory_client = RpcClientService::new(chain_config.clone(), key, None).await;

    // Bootstrap all the factory stuffz
    let factory = Factory::new(chain_config.clone(), factory_client).await?;

    // Get that factory info before moving on
    if !factory.load().await {
        return Err(eyre!("Failed to load factory contracts!"));
    }

    // Init that agent client lyfe
    let agent_contract_addr = factory.get_contract_addr("agents".to_string()).await?;
    let agent_client =
        RpcClientService::new(chain_config.clone(), key, Some(agent_contract_addr)).await;
    let agent = Agent::new(chain_config.clone(), agent_contract_addr, key, agent_client).await?;

    // Init that manager client lyfe
    let manager_contract_addr = factory.get_contract_addr("manager".to_string()).await?;
    let manager_client =
        RpcClientService::new(chain_config.clone(), key, Some(manager_contract_addr)).await;
    let manager = Manager::new(manager_contract_addr, manager_client).await?;

    // Init that tasks client lyfe
    let tasks_contract_addr = factory.get_contract_addr("manager".to_string()).await?;
    let tasks_client =
        RpcClientService::new(chain_config.clone(), key, Some(manager_contract_addr)).await;
    let tasks = Tasks::new(manager_contract_addr, manager_client).await?;

    // Get the account id
    let account_addr = agent_client.account_id();

    match opts.cmd {
        opts::Command::Register { payable_account_id } => {
            // Register the agent
            let res = agent.register(&payable_account_id).await;

            // Handle the result
            match res {
                Ok(result) => {
                    info!("Agent registered successfully");
                    let log = result.log;
                    info!("Result: {}", log);
                }
                Err(err) if err.to_string().contains("Agent already exists") => {
                    Err(eyre!("Agent already registered"))?;
                }
                Err(err)
                    if err.to_string().contains("account")
                        && err.to_string().contains("not found") =>
                {
                    Err(eyre!("Agent account not found on chain"))?;
                }
                Err(err) => Err(eyre!("Failed to register agent: {}", err))?,
            }
            Ok(())
        }
        opts::Command::Unregister => {
            let res = agent.unregister().await;

            // Handle the result
            match res {
                Ok(result) => {
                    info!("Agent unregistered successfully");
                    let log = result.log;
                    info!("Result: {}", log);
                }
                Err(err) if err.to_string().contains("Agent not registered") => {
                    Err(eyre!("Agent not registered"))?;
                }
                Err(err) => Err(eyre!("Failed to register agent: {}", err))?,
            }

            Ok(())
        }
        opts::Command::Withdraw => {
            let res = manager.withdraw_reward().await;

            // Handle the result
            match res {
                Ok(result) => {
                    info!("Agent reward withdrawn successfully");
                    let log = result.log;
                    info!("Result: {}", log);
                }
                Err(err) if err.to_string().contains("Agent not registered") => {
                    Err(eyre!("Agent not registered"))?;
                }
                Err(err) => Err(eyre!("Failed to withdraw reward: {}", err))?,
            }

            Ok(())
        }
        opts::Command::ListAccounts => {
            println!("Account addresses for agent: {}\n", &opts.agent);
            // Get the chain config for the chain we're going to run on
            for (chain_id, chain_config) in config.chains {
                let account_addr = storage
                    .get_agent_signing_account_addr(&opts.agent, chain_config.info.bech32_prefix)?;
                println!("{chain_id}: {account_addr}");
            }

            Ok(())
        }
        opts::Command::Status => {
            // Print info about the agent
            let account_addr = account_addr.clone();
            info!("Account ID: {}", account_addr);

            // Get the agent status
            let res = agent.get_status(account_addr).await?;

            // Handle the result of the query
            match res {
                Ok(result) => {
                    info!("Result: {:?}", result);
                }
                Err(err) if err.to_string().contains("Agent not registered") => {
                    Err(eyre!("Agent not registered"))?;
                }
                Err(err) => Err(eyre!("Failed to get agent status: {}", err))?,
            }

            Ok(())
        }
        opts::Command::AllTasks { from_index, limit } => {
            let res = tasks.get_all(from_index, limit).await;

            // Handle the result
            match res {
                Ok(result) => {
                    info!("Result: {}", result);
                }
                Err(err) if err.to_string().contains("Agent not registered") => {
                    Err(eyre!("Agent not registered"))?;
                }
                Err(err) => Err(eyre!("Failed to get contract tasks: {}", err))?,
            }

            Ok(())
        }
        opts::Command::GetTasks => {
            let res = agent.get_tasks(account_addr.as_str()).await;

            // Handle the result
            match res {
                Ok(result) => {
                    info!("Result: {}", result);
                }
                Err(err) if err.to_string().contains("Agent not registered") => {
                    Err(eyre!("Agent not registered"))?;
                }
                Err(err) => Err(eyre!("Failed to get contract tasks: {}", err))?,
            }

            Ok(())
        }
        opts::Command::GenerateMnemonic { new_name, mnemonic } => {
            storage.generate_account(new_name.clone(), mnemonic).await?;
            println!("Generated agent for {new_name}");
            Ok(())
        }
        opts::Command::Update => {
            let res = agent.update(signer.account_id().to_string()).await;

            // Handle the result
            match res {
                Ok(result) => {
                    info!("Agent configuration updated successfully");
                    let log = result.log;
                    info!("Result: {}", log);
                }
                Err(err) if err.to_string().contains("Agent not registered") => {
                    Err(eyre!("Agent not registered"))?;
                }
                Err(err) => Err(eyre!(
                    "Failed to update agent configuration on chain: {}",
                    err
                ))?,
            }

            Ok(())
        }
        opts::Command::GetAgent { name } => Ok(storage.display_account(&name)),
        // TODO: Move "with_queries" to just be config.yaml
        opts::Command::Go { with_queries } => {
            // Create the global shutdown channel
            let (shutdown_tx, _shutdown_rx) = create_shutdown_channel();

            // Run the agent on the chain
            system::run_retry(&chain_id, &shutdown_tx, chain_config, &key, with_queries).await
        }
        opts::Command::SetupService { output } => {
            for (chain_id, _) in config.chains {
                system::DaemonService::create(output.clone(), &chain_id, opts.no_frills)?;
            }
            Ok(())
        }
        opts::Command::SendFunds { to, denom, amount } => {
            // Send funds to the given address.
            let res = agent
                .send_funds(
                    signer.account_id().as_ref(),
                    to.as_str(),
                    amount,
                    denom.as_str(),
                )
                .await;

            // Handle the result of the transaction
            match res {
                Ok(tx) => {
                    info!("Funds sent successfully");
                    info!("TxHash: {}", tx.tx_hash);
                }
                Err(err) => Err(err)?,
            }

            Ok(())
        }
    }

    Ok(())
}
