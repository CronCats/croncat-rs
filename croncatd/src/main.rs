//!
//! The `croncatd` agent.
//!

use croncat::{
    channels::create_shutdown_channel,
    config::Config,
    errors::{eyre, Report},
    logging::{self, error, info},
    modules::{agent::Agent, factory::Factory, manager::Manager, tasks::Tasks},
    rpc::RpcClientService,
    store::agent::LocalAgentStorage,
    system,
    tokio::{self, sync::Mutex},
};
use opts::Opts;
use std::{process::exit, sync::Arc};
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
    if let Err(err) = run_command(opts.clone(), storage).await {
        error!("{}", err);

        if opts.debug {
            error!("Command failed: {}", opts.cmd);
            return Err(err);
        }

        exit(1);
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
    let fee_token = chain_config.clone().info.fees.fee_tokens.pop();
    let chain_denom = if let Some(token) = fee_token {
        token.denom
    } else {
        chain_config.clone().denom.unwrap_or_default()
    };

    // Get the key and create a signer
    let key = storage.get_agent_signing_key(&opts.agent)?;

    // Get an rpc client
    let factory_client = RpcClientService::new(chain_config.clone(), key.clone(), None).await;

    // Bootstrap all the factory stuffz
    let factory = Arc::new(Mutex::new(
        Factory::new(chain_config.clone(), factory_client).await?,
    ));

    // Get that factory info before moving on
    if factory.lock().await.load().await? {
        info!("[{}] Factory Cache Reloaded", chain_id);
    }

    // Init that agent client lyfe
    let agent_contract_addr = factory
        .lock()
        .await
        .get_contract_addr("agents".to_string())
        .await?;
    let agent_client = RpcClientService::new(
        chain_config.clone(),
        key.clone(),
        Some(agent_contract_addr.clone()),
    )
    .await;
    // Get the account id
    let account_addr = agent_client.account_id();
    let agent = Arc::new(
        Agent::new(
            chain_config.clone(),
            agent_contract_addr,
            key.clone(),
            agent_client,
        )
        .await?,
    );

    // Init that manager client lyfe
    let manager_contract_addr = factory
        .lock()
        .await
        .get_contract_addr("manager".to_string())
        .await?;
    let manager_client = RpcClientService::new(
        chain_config.clone(),
        key.clone(),
        Some(manager_contract_addr.clone()),
    )
    .await;
    let manager =
        Arc::new(Manager::new(manager_contract_addr.clone(), manager_client.clone()).await?);

    // Init that tasks client lyfe
    let tasks_contract_addr = factory
        .lock()
        .await
        .get_contract_addr("tasks".to_string())
        .await?;
    let generic_querier_addr = factory
        .lock()
        .await
        .get_contract_addr("mod_generic".to_string())
        .await?;
    let tasks_client = RpcClientService::new(
        chain_config.clone(),
        key.clone(),
        Some(tasks_contract_addr.clone()),
    )
    .await;
    let tasks = Arc::new(Mutex::new(
        Tasks::new(
            chain_config.clone(),
            tasks_contract_addr.clone(),
            tasks_client,
            generic_querier_addr,
        )
        .await?,
    ));

    match opts.cmd {
        opts::Command::Register { payable_account_id } => {
            // Register the agent
            let res = agent.register(&payable_account_id).await;

            // Handle the result
            match res {
                Ok(result) => {
                    let account_addr = account_addr.clone();
                    info!("Agent {} registered successfully! 😻", account_addr);

                    let status =
                        result.find_event_tags("wasm".to_string(), "agent_status".to_string());
                    for s in status {
                        info!("Agent is {}", s.value);
                        info!("Now run the command: `cargo run go`");
                        if s.value != *"active" {
                            info!("Make sure to keep your agent running, it will automatically become active when enough tasks exist.");
                        }
                    }

                    if opts.debug {
                        let log = result.res.log;
                        info!("Result: {}", log);
                    }

                    // // Get the agent status
                    // let res = agent.get_status(account_addr).await?;
                    // info!("Agent is {:?}", res);
                    // info!("Now run the command: `cargo run go`");
                    // if res != AgentStatus::Active {
                    //     info!("Make sure to keep your agent running, it will automatically become active when enough tasks exist.");
                    // }
                }
                Err(err) if err.to_string().contains("Agent already registered") => {
                    let account_addr = account_addr.clone();
                    Err(eyre!("Agent {} already registered", account_addr))?;
                }
                Err(err)
                    if err
                        .to_string()
                        .contains("Agent registration currently operates on a whitelist") =>
                {
                    let account_addr = account_addr.clone();
                    Err(eyre!(
                        "Agent {} needs whitelist approval, please submit request to CronCat DAO",
                        account_addr
                    ))?;
                }
                Err(err)
                    if err.to_string().contains("account")
                        && err.to_string().contains("not found") =>
                {
                    let account_addr = account_addr.clone();
                    Err(eyre!("\n\nAgent account not found on chain\nPlease add enough funds to execute a few transactions on your account then try to register again.\nYour account: {}", account_addr))?;
                }
                Err(err) => Err(eyre!("Failed to register agent: {}", err))?,
            }
        }
        opts::Command::Unregister => {
            let res = agent.unregister().await;

            // Handle the result
            match res {
                Ok(result) => {
                    let account_addr = account_addr.clone();
                    info!("Agent {} unregistered successfully! 👋", account_addr);
                    // Unwrap all the logs, to show funds received, if any
                    let rewards = result.find_event_tags("wasm".to_string(), "rewards".to_string());
                    for r in rewards {
                        info!("Rewards received: {} {}", r.value, chain_denom);
                    }

                    if opts.debug {
                        let log = result.res.log;
                        info!("\nResult: {}", log);
                    }
                }
                Err(err) if err.to_string().contains("Agent not registered") => {
                    Err(eyre!(
                        "Agent doesnt exist, must first register and do tasks."
                    ))?;
                }
                Err(err) => Err(eyre!("Failed to unregister agent: {}", err))?,
            }
        }
        opts::Command::Withdraw => {
            let res = manager.withdraw_reward().await;

            // Handle the result
            match res {
                Ok(result) => {
                    info!("Agent reward withdrawn successfully");
                    // Parse logs and show how much funds were sent
                    let rewards = result.find_event_tags("wasm".to_string(), "rewards".to_string());
                    for r in rewards {
                        info!("Rewards received: {} {}", r.value, chain_denom);
                    }
                    if opts.debug {
                        let log = result.res.log;
                        info!("\nResult: {}", log);
                    }
                }
                Err(err) if err.to_string().contains("Agent not registered") => {
                    Err(eyre!(
                        "Agent doesnt exist, must first register and do tasks."
                    ))?;
                }
                Err(err) if err.to_string().contains("No rewards available for withdraw") => {
                    info!(
                        "No rewards available for withdraw, please wait until your agent is active and has processed tasks before next withdraw."
                    );
                }
                Err(err) => Err(eyre!("Failed to withdraw reward: {}", err))?,
            }
        }
        opts::Command::ListAccounts => {
            println!("Account addresses for agent: {}\n", &opts.agent);
            // Get the chain config for the chain we're going to run on
            for (chain_id, chain_config) in config.chains {
                let account_addr = storage
                    .get_agent_signing_account_addr(&opts.agent, chain_config.info.bech32_prefix)?;
                println!("{chain_id}: {account_addr}");
            }
        }
        opts::Command::Status => {
            // Print info about the agent
            let account_addr = account_addr.clone();
            let err_helper = eyre!("Agent not registered, please make sure your account '{}' has funds then run the command: `cargo run register`", account_addr);

            // Get the agent status
            let res = agent.get(account_addr.as_str()).await?;

            if let Some(result) = res {
                if let Some(info) = result.agent {
                    let c = agent
                        .query_native_balance(Some(account_addr.clone()))
                        .await?;
                    let b = format!("{:?} {}", c.amount, c.denom);
                    info!("\n\nStatus: {:?}\nAddress: {}\nReward Address: {}\nEarned Rewards: {:?} {}\nCurrent Balance: {}\n\n", info.status, account_addr, info.payable_account_id.to_string(), u128::from(info.balance), chain_denom, b);
                    return Ok(());
                } else {
                    Err(err_helper)?
                }
            } else {
                Err(err_helper)?
            }
        }
        opts::Command::AllTasks { from_index, limit } => {
            let res = tasks.lock().await.get_all(from_index, limit).await;

            // Handle the result
            match res {
                Ok(result) => {
                    // TODO: Parse and represent results better
                    info!("{}", result);
                }
                Err(err) if err.to_string().contains("Agent not registered") => {
                    Err(eyre!("Agent not registered"))?;
                }
                Err(err) => Err(eyre!("Failed to get contract tasks: {}", err))?,
            }
        }
        opts::Command::GetTasks => {
            let result = agent.get_tasks(account_addr.as_str()).await?;

            if let Some(res) = result {
                info!(
                    "Block Tasks: {}, Cron Tasks: {}",
                    res.stats.num_block_tasks, res.stats.num_cron_tasks
                );
                return Ok(());
            } else {
                Err(eyre!("Failed to get agent tasks"))?
            }
        }
        opts::Command::GenerateMnemonic { new_name, mnemonic } => {
            storage.generate_account(new_name.clone(), mnemonic).await?;
            println!("Generated agent keys for '{new_name}'");
            println!("Start using it by doing the command: `export CRONCAT_AGENT={new_name}`");
            println!("View the account addresses with command: `cargo run list-accounts`");
        }
        opts::Command::Update { payable_account_id } => {
            let res = agent.update(payable_account_id).await;

            // Handle the result
            match res {
                Ok(result) => {
                    info!("Agent configuration updated successfully! 😸");
                    if opts.debug {
                        let log = result.res.log;
                        info!("\nResult: {}", log);
                    }
                }
                Err(err) if err.to_string().contains("Agent not registered") => {
                    Err(eyre!("Agent not registered"))?;
                }
                Err(err) => Err(eyre!(
                    "Failed to update agent configuration on chain: {}",
                    err
                ))?,
            }
        }
        opts::Command::GetAgentKeys { name } => storage.display_account(&name),
        opts::Command::Go {} => {
            // Create the global shutdown channel
            let (shutdown_tx, _shutdown_rx) = create_shutdown_channel();

            // Run the agent on the chain
            system::run_retry(
                &chain_id,
                &shutdown_tx,
                chain_config,
                factory,
                agent,
                manager,
                tasks,
            )
            .await?
        }
        opts::Command::SetupService { output } => {
            for (chain_id, _) in config.chains {
                system::DaemonService::create(output.clone(), &chain_id, opts.no_frills)?;
            }
        }
        opts::Command::SendFunds { to, amount, denom } => {
            let amount = amount.parse::<u128>()?;
            let account_addr = account_addr.clone();
            let d = denom.unwrap_or(chain_denom);

            // Send funds to the given address.
            let res = agent
                .send_funds(&account_addr, to.as_str(), amount, d.as_str())
                .await;

            // Handle the result of the transaction
            match res {
                Ok(tx) => {
                    info!("Funds sent successfully");
                    // TODO: Would be TIGHT to link to explorer here using the chain registry config
                    info!("TxHash: {}", tx.tx_hash);
                }
                Err(err) => Err(err)?,
            }
        }
    }

    Ok(())
}
