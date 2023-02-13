//!
//! This module contains the code for querying the croncat contract via HTTP RPC.
//!

use std::time::Duration;

use croncat_sdk_agents::msg::{AgentResponse, AgentTaskResponse, QueryMsg as AgentQueryMsg};
use croncat_sdk_agents::types::AgentStatus;
use croncat_sdk_factory::msg::{Config, FactoryQueryMsg};
use croncat_sdk_tasks::msg::TasksQueryMsg;
use croncat_sdk_tasks::types::TaskResponse;

use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::time::timeout;

use crate::config::ChainConfig;
use crate::errors::{eyre, Report};
use crate::utils::normalize_rpc_url;

use super::RpcClient;

pub struct Querier {
    rpc_client: RpcClient,
    croncat_addr: String,
}

impl std::fmt::Debug for Querier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Querier")
            .field("croncat_addr", &self.croncat_addr)
            .finish()
    }
}

impl Querier {
    pub async fn new(cfg: ChainConfig, rpc_url: String) -> Result<Self, Report> {
        let rpc_url = normalize_rpc_url(&rpc_url);

        let rpc_client = RpcClient::new(&cfg, &rpc_url)?;

        Ok(Self {
            rpc_client,
            croncat_addr: cfg.factory,
        })
    }

    pub async fn query_croncat<S, T>(&self, msg: S) -> Result<T, Report>
    where
        S: Serialize,
        T: DeserializeOwned,
    {
        timeout(
            Duration::from_secs_f64(self.rpc_client.timeout_secs),
            self.rpc_client.wasm_query(msg),
        )
        .await
        .map_err(|err| {
            eyre!(
                "Timeout ({}s) while querying contract: {}",
                self.rpc_client.timeout_secs,
                err
            )
        })?
    }

    pub async fn query_config(&self) -> Result<String, Report> {
        let config: Config = self.query_croncat(FactoryQueryMsg::Config {}).await?;
        let json = serde_json::to_string_pretty(&config)?;
        Ok(json)
    }

    pub async fn get_agent_status(&self, account_id: String) -> Result<AgentStatus, Report> {
        let agent_info: Option<AgentResponse> = self
            .query_croncat(AgentQueryMsg::GetAgent { account_id })
            .await?;

        if agent_info.is_none() {
            Err(eyre!("Agent not registered"))
        } else {
            if let Some(agent) = agent_info.unwrap().agent {
                Ok(agent.status)
            } else {
                Err(eyre!("Agent not registered"))
            }
        }
    }

    pub async fn get_tasks(
        &self,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Result<String, Report> {
        let response: Vec<TaskResponse> = self
            .query_croncat(TasksQueryMsg::Tasks { from_index, limit })
            .await?;
        let json = serde_json::to_string_pretty(&response)?;
        Ok(json)
    }

    pub async fn get_evented_tasks(
        &self,
        start: Option<u64>,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Result<String, Report> {
        let response: Vec<TaskResponse> = self
            .query_croncat(TasksQueryMsg::EventedTasks {
                start,
                from_index,
                limit,
            })
            .await?;
        let json = serde_json::to_string_pretty(&response)?;
        Ok(json)
    }

    pub async fn get_agent_tasks(&self, account_id: String) -> Result<String, Report> {
        let response: Option<AgentTaskResponse> = self
            .query_croncat(AgentQueryMsg::GetAgentTasks { account_id })
            .await?;
        let json = serde_json::to_string_pretty(&response)?;
        Ok(json)
    }
}

impl std::fmt::Debug for RpcClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcClient")
            .field("contract_addr", &self.contract_addr)
            .field("client", &self.client)
            .finish()
    }
}
