//!
//! The `croncatd` agent.
//!

use croncat::{
    channels::create_shutdown_channel,
    //    client::{BankQueryClient, QueryBank},
    config::Config,
    errors::{eyre, Report},
    logging::{self, error, info},
    rpc::RpcClientService,
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
        opts::Command::Register { payable_account_id } => {
            // Make sure we have a chain id to run on
            if opts.chain_id.is_none() {
                return Err(eyre!("chain-id is required for this command"));
            }
            let chain_id = opts.chain_id.unwrap();

            // Get the chain config for the chain we're going to run on
            let chain_config = config
                .chains
                .get(&chain_id)
                .ok_or_else(|| eyre!("Chain not found in configuration: {}", chain_id))?;

            // Get the key and create a signer
            let key = storage.get_agent_signing_key(&opts.agent)?;
            let mnemonic = storage.get_agent_mnemonic(&opts.agent)?;

            // Get an rpc client
            let client =
                RpcClientService::new(chain_config.clone(), mnemonic.to_string(), key).await;

            client
                .execute(|signer| {
                    let payable_account_id = payable_account_id.clone();

                    async move {
                        // Print info about the agent about to be registered
                        info!("Account ID: {}", signer.account_id());
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
                })
                .await?;
        }
        opts::Command::Unregister => {
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
            let mnemonic = storage.get_agent_mnemonic(&opts.agent)?;

            // Get an rpc client
            let client =
                RpcClientService::new(chain_config.clone(), mnemonic.to_string(), key).await;

            client
                .execute(|signer| async move {
                    // Print info about the agent about to be registered
                    info!("Account ID: {}", signer.account_id());

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
                            Err(eyre!("Agent not registered"))?;
                        }
                        Err(err) => Err(eyre!("Failed to register agent: {}", err))?,
                    }

                    Ok(())
                })
                .await?;
        }
        opts::Command::Withdraw => {
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
            let mnemonic = storage.get_agent_mnemonic(&opts.agent)?;

            // Get an rpc client
            let client =
                RpcClientService::new(chain_config.clone(), mnemonic.to_string(), key).await;

            client
                .execute(|signer| async move {
                    // Print info about the agent about to be registered
                    info!("Account ID: {}", signer.account_id());

                    // Unregister the agent
                    let query = signer.withdraw_reward().await;

                    // Handle the result of the query
                    match query {
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
                })
                .await?;
        }
        opts::Command::ListAccounts => {
            println!("Account addresses for agent: {}\n", &opts.agent);
            // Get the chain config for the chain we're going to run on
            for (chain_id, chain_config) in config.chains {
                let account_addr = storage
                    .get_agent_signing_account_addr(&opts.agent, chain_config.info.bech32_prefix)?;
                println!("{}: {}", chain_id, account_addr);
            }
        }
        opts::Command::Status => {
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
            let mnemonic = storage.get_agent_mnemonic(&opts.agent)?;

            // Get an rpc client
            let client =
                RpcClientService::new(chain_config.clone(), mnemonic.to_string(), key).await;

            // Get the account id
            let account_addr = storage.get_agent_signing_account_addr(
                &opts.agent,
                chain_config.info.bech32_prefix.clone(),
            )?;

            client
                .query(|querier| {
                    let account_addr = account_addr.clone();
                    async move {
                        // Print info about the agent about to be registered
                        info!("Account ID: {}", account_addr);

                        // Get the agent status
                        let query = querier.get_agent_status(account_addr).await;

                        // Handle the result of the query
                        match query {
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
                })
                .await?;
        }
        opts::Command::AllTasks { from_index, limit } => {
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
            let mnemonic = storage.get_agent_mnemonic(&opts.agent)?;

            // Get an rpc client
            let client =
                RpcClientService::new(chain_config.clone(), mnemonic.to_string(), key).await;

            // Get the account id
            let account_addr = storage.get_agent_signing_account_addr(
                &opts.agent,
                chain_config.info.bech32_prefix.clone(),
            )?;

            client
                .query(|querier| {
                    let account_addr = account_addr.clone();
                    async move {
                        // Print info about the agent about to be registered
                        info!("Account ID: {}", account_addr);

                        // Get the agent status
                        let query = querier.get_tasks(from_index, limit).await;

                        // Handle the result of the query
                        match query {
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
                })
                .await?;
        }
        opts::Command::GetTasks => {
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
            let mnemonic = storage.get_agent_mnemonic(&opts.agent)?;

            // Get an rpc client
            let client =
                RpcClientService::new(chain_config.clone(), mnemonic.to_string(), key).await;

            // Get the account id
            let account_addr = storage.get_agent_signing_account_addr(
                &opts.agent,
                chain_config.info.bech32_prefix.clone(),
            )?;

            client
                .query(|querier| {
                    let account_addr = account_addr.clone();

                    async move {
                        // Print info about the agent about to be registered
                        info!("Account ID: {}", account_addr);

                        // Get the agent status
                        let query = querier.get_agent_tasks(account_addr).await;

                        // Handle the result of the query
                        match query {
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
                })
                .await?;
        }
        opts::Command::GenerateMnemonic { new_name, mnemonic } => {
            storage.generate_account(new_name.clone(), mnemonic).await?;
            println!("Generated agent for {}", new_name);
        }
        opts::Command::Update => {
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
            let mnemonic = storage.get_agent_mnemonic(&opts.agent)?;

            // Get an rpc client
            let client =
                RpcClientService::new(chain_config.clone(), mnemonic.to_string(), key).await;

            client
                .execute(|signer| async move {
                    // Print info about the agent about to be registered
                    info!("Account ID: {}", signer.account_id());

                    // Unregister the agent
                    let query = signer.update_agent(signer.account_id().to_string()).await;

                    // Handle the result of the query
                    match query {
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
                })
                .await?;
        }
        opts::Command::GetAgent { name } => storage.display_account(&name),
        opts::Command::Go { with_queries } => {
            // Make sure we have a chain id to run on
            if opts.chain_id.is_none() {
                return Err(eyre!("chain-id is required for go command"));
            }
            let chain_id = opts.chain_id.unwrap();

            // Get the key for the agent signing account
            let key = storage.get_agent_signing_key(&opts.agent)?;
            let mnemonic = storage.get_agent_mnemonic(&opts.agent)?.to_string();

            // Get the chain config for the chain we're going to run on
            let chain_config = config
                .chains
                .get(&chain_id)
                .ok_or_else(|| eyre!("Chain not found in configuration: {}", chain_id))?;

            // Create the global shutdown channel
            let (shutdown_tx, _shutdown_rx) = create_shutdown_channel();

            // Run the agent on the chain
            system::run_retry(
                &chain_id,
                &shutdown_tx,
                chain_config,
                &key,
                &mnemonic,
                with_queries,
            )
            .await?;
        }
        opts::Command::SetupService { output } => {
            for (chain_id, _) in config.chains {
                system::DaemonService::create(output.clone(), &chain_id, opts.no_frills)?;
            }
        }
        opts::Command::SendFunds { to, denom, amount } => {
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
            let mnemonic = storage.get_agent_mnemonic(&opts.agent)?;

            // Get an rpc client
            let client =
                RpcClientService::new(chain_config.clone(), mnemonic.to_string(), key).await;

            // Parse the amount
            let amount = amount
                .parse::<u128>()
                .map_err(|e| eyre!("Invalid send amount: {}", e))?;

            client
                .execute(|signer| {
                    let to = to.clone();
                    let denom = denom.clone();

                    async move {
                        // Print info about the agent sending funds
                        info!("Account ID: {}", signer.account_id());

                        // Send funds to the given address.
                        let query = signer
                            .send_funds(
                                signer.account_id().as_ref(),
                                to.as_str(),
                                amount,
                                denom.as_str(),
                            )
                            .await;

                        // Handle the result of the transaction
                        match query {
                            Ok(_) => {
                                info!("Funds sent successfully");
                            }
                            Err(err) => Err(err)?,
                        }

                        Ok(())
                    }
                })
                .await?;
        }
    }

    Ok(())
}
