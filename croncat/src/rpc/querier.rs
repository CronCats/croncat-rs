//!
//! This module contains the code for querying the croncat contract via HTTP RPC.
//!

use cw_croncat_core::msg::{
    AgentResponse, AgentTaskResponse, GetConfigResponse, QueryMsg, TaskResponse,
};
use cw_croncat_core::types::AgentStatus;

use serde::de::DeserializeOwned;
use serde::Serialize;

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
            croncat_addr: cfg.manager,
        })
    }

    pub async fn query_croncat<S, T>(&self, msg: S) -> Result<T, Report>
    where
        S: Serialize,
        T: DeserializeOwned,
    {
        self.rpc_client.wasm_query(msg).await
    }

    pub async fn query_config(&self) -> Result<String, Report> {
        let config: GetConfigResponse = self.query_croncat(QueryMsg::GetConfig {}).await?;
        let json = serde_json::to_string_pretty(&config)?;
        Ok(json)
    }

    pub async fn get_agent_status(&self, account_id: String) -> Result<AgentStatus, Report> {
        let agent_info: Option<AgentResponse> = self
            .query_croncat(QueryMsg::GetAgent { account_id })
            .await?;

        if agent_info.is_none() {
            Err(eyre!("Agent not registered"))
        } else {
            Ok(agent_info.unwrap().status)
        }
    }

    pub async fn get_tasks(
        &self,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Result<String, Report> {
        let response: Vec<TaskResponse> = self
            .query_croncat(QueryMsg::GetTasks { from_index, limit })
            .await?;
        let json = serde_json::to_string_pretty(&response)?;
        Ok(json)
    }

    pub async fn get_agent_tasks(&self, account_id: String) -> Result<String, Report> {
        let response: Option<AgentTaskResponse> = self
            .query_croncat(QueryMsg::GetAgentTasks { account_id })
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
