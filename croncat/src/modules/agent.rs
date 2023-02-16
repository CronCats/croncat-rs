use color_eyre::{eyre::eyre, Report};
use croncat_sdk_agents::types::AgentStatus;
use std::process::exit;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::config::ChainConfig;
use crate::{
    channels::{BlockStreamRx, ShutdownRx},
    rpc::RpcClientService,
    utils::AtomicIntervalCounter,
};
use cosm_orc::orchestrator::{Address, ChainTxResponse, Coin};
use cosmrs::bip32;
use cosmrs::crypto::secp256k1::SigningKey;
use croncat_sdk_agents::msg::{
    AgentResponse, AgentTaskResponse, ExecuteMsg as AgentExecuteMsg, QueryMsg as AgentQueryMsg,
};

use super::manager::Manager;

pub struct Agent {
    pub client: RpcClientService,
    pub contract_addr: Address,
    pub account_id: String,
}

impl Agent {
    pub async fn new(
        cfg: ChainConfig,
        contract_addr: Address,
        key: bip32::XPrv,
        client: RpcClientService,
    ) -> Result<Self, Report> {
        let signing_key: SigningKey = key.into();
        let account_id = signing_key
            .public_key()
            .account_id(&cfg.info.bech32_prefix)?;

        Ok(Self {
            client,
            contract_addr,
            account_id: account_id.to_string(),
        })
    }

    pub fn account_id(&self) -> &String {
        &self.account_id
    }

    pub async fn register(
        &self,
        payable_account_id: &Option<String>,
    ) -> Result<ChainTxResponse, Report> {
        self.client
            .execute(|signer| {
                let payable_account_id = payable_account_id.clone();
                let contract_addr = self.contract_addr.clone();

                async move {
                    signer
                        .execute_croncat(
                            AgentExecuteMsg::RegisterAgent {
                                payable_account_id: payable_account_id.clone(),
                            },
                            Some(contract_addr),
                        )
                        .await
                }
            })
            .await
    }

    pub async fn check_in(&self) -> Result<ChainTxResponse, Report> {
        self.client
            .execute(|signer| {
                let contract_addr = self.contract_addr.clone();
                async move {
                    signer
                        .execute_croncat(AgentExecuteMsg::CheckInAgent {}, Some(contract_addr))
                        .await
                }
            })
            .await
    }

    pub async fn unregister(&self) -> Result<ChainTxResponse, Report> {
        self.client
            .execute(|signer| {
                let contract_addr = self.contract_addr.clone();
                async move {
                    signer
                        .execute_croncat(
                            AgentExecuteMsg::UnregisterAgent { from_behind: None },
                            Some(contract_addr),
                        )
                        .await
                }
            })
            .await
    }

    pub async fn update(&self, payable_account_id: String) -> Result<ChainTxResponse, Report> {
        self.client
            .execute(|signer| {
                let payable_account_id = payable_account_id.clone();
                let contract_addr = self.contract_addr.clone();

                async move {
                    signer
                        .execute_croncat(
                            AgentExecuteMsg::UpdateAgent {
                                payable_account_id: payable_account_id.clone(),
                            },
                            Some(contract_addr),
                        )
                        .await
                }
            })
            .await
    }

    pub async fn get(&self, account_id: &str) -> Result<Option<AgentResponse>, Report> {
        let res = self
            .client
            .query(move |querier| {
                let account_id = <&str>::clone(&account_id);
                let contract_addr = self.contract_addr.clone();

                async move {
                    querier
                        .query_croncat(
                            AgentQueryMsg::GetAgent {
                                account_id: account_id.to_string(),
                            },
                            Some(contract_addr),
                        )
                        .await
                }
            })
            .await?;
        Ok(res)
    }

    pub async fn get_status(&self, account_id: String) -> Result<AgentStatus, Report> {
        let agent_info: Option<AgentResponse> = self
            .client
            .query(move |querier| {
                let account_id = account_id.clone();
                let contract_addr = self.contract_addr.clone();

                async move {
                    querier
                        .query_croncat(
                            AgentQueryMsg::GetAgent {
                                account_id: account_id.to_string(),
                            },
                            Some(contract_addr),
                        )
                        .await
                }
            })
            .await?;

        if agent_info.is_none() {
            Err(eyre!("Agent not registered"))
        } else if let Some(agent) = agent_info.unwrap().agent {
            Ok(agent.status)
        } else {
            Err(eyre!("Agent not registered"))
        }
    }

    pub async fn get_tasks(&self, account_id: &str) -> Result<Option<AgentTaskResponse>, Report> {
        let res: Option<AgentTaskResponse> = self
            .client
            .query(move |querier| {
                let account_id = <&str>::clone(&account_id);
                let contract_addr = self.contract_addr.clone();

                async move {
                    querier
                        .query_croncat(
                            AgentQueryMsg::GetAgentTasks {
                                account_id: account_id.to_string(),
                            },
                            Some(contract_addr),
                        )
                        .await
                }
            })
            .await?;
        Ok(res)
    }

    pub async fn query_native_balance(&self, account: Option<String>) -> Result<Coin, Report> {
        let account_id: String = account
            .unwrap_or_else(|| self.account_id.clone())
            .to_string();
        self.client.query_balance(account_id.as_str()).await
    }

    pub async fn send_funds(
        &self,
        account_id: &str,
        to: &str,
        amount: u128,
        denom: &str,
    ) -> Result<ChainTxResponse, Report> {
        self.client
            .send_funds(account_id, to, denom, amount)
            .await
            .map_err(|err| {
                eyre!(
                    "Failed to send funds from {} to {}: {}",
                    account_id,
                    to,
                    err
                )
            })
    }
}

///
/// Check every nth block with [`AtomicIntervalCounter`] for the current account
/// status of each account the agent watches.
///
pub async fn check_status_loop(
    mut block_stream_rx: BlockStreamRx,
    mut shutdown_rx: ShutdownRx,
    block_status: Arc<Mutex<AgentStatus>>,
    chain_id: Arc<String>,
    chain_config: ChainConfig,
    agent_client: Arc<Agent>,
    manager_client: Arc<Manager>,
) -> Result<(), Report> {
    let block_counter = AtomicIntervalCounter::new(10);
    let task_handle: tokio::task::JoinHandle<Result<(), Report>> = tokio::task::spawn(async move {
        while let Ok(block) = block_stream_rx.recv().await {
            block_counter.tick();
            if block_counter.is_at_interval() {
                info!(
                    "Checking agents statuses for block (height: {})",
                    block.header().height
                );
                let account_id = agent_client.account_id();
                let agent = agent_client.get(account_id.as_str()).await?;
                let mut locked_status = block_status.lock().await;
                *locked_status = agent
                    .ok_or(eyre!("Agent unregistered during the loop"))?
                    .agent
                    .unwrap()
                    .status;
                info!("[{}] Agent status: {:?}", chain_id, *locked_status);
                if *locked_status == AgentStatus::Nominated {
                    info!(
                        "Checking in agent: {}",
                        agent_client.check_in().await.map(|result| result.res.log)?
                    );
                    let agent = agent_client.get(account_id.as_str()).await?;
                    *locked_status = agent
                        .ok_or(eyre!("Agent unregistered during the loop"))?
                        .agent
                        .unwrap()
                        .status;
                    info!("Agent status: {:?}", *locked_status);
                }

                if let Some(threshold) = chain_config.threshold {
                    // Check the agent's balance to make sure it's not falling below a threshold
                    let account_id = agent_client.account_id();
                    let agent_balance = agent_client
                        .query_native_balance(Some(account_id.clone()))
                        .await?;
                    let agent_native_balance = agent_balance.amount;
                    let denom = agent_balance.denom;

                    // If agent balance is too low and the agent has some native coins in the manager contract
                    // call withdraw_reward
                    // If manager balance is zero, exit
                    if agent_native_balance < threshold as u128 {
                        let agent = agent_client.get(account_id.as_str()).await?;
                        let reward_balance = agent
                            .ok_or(eyre!("Agent unregistered during the loop"))?
                            .agent
                            .unwrap()
                            .balance;

                        if !reward_balance.is_zero() {
                            info!("Automatically withdrawing agent reward");
                            let result = manager_client.withdraw_reward().await?;
                            let log = result.res.log;
                            info!("Log: {log}");

                            let native_balance_after_withdraw = agent_client
                                .query_native_balance(Some(account_id.clone()))
                                .await?
                                .amount;
                            if native_balance_after_withdraw < threshold as u128 {
                                error!("Not enough balance to continue, the agent in required to have {} {}, current balance: {} {}", threshold, denom, native_balance_after_withdraw, denom);
                                error!("Stopping the agent");
                                exit(1);
                            }
                        } else {
                            error!("Not enough balance to continue, the agent in required to have {} {}, current balance: {} {}", threshold, denom, agent_native_balance, denom);
                            error!("Stopping the agent");
                            exit(1);
                        }
                    }
                }
            }
        }
        Ok(())
    });

    tokio::select! {
        Ok(task) = task_handle => {task?}
        _ = shutdown_rx.recv() => {}
    }

    Ok(())
}
