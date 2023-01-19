//!
//! This module contains the code for querying the croncat contract via gRPC.
//!

use cw_croncat_core::msg::AgentTaskResponse;
use cw_croncat_core::msg::TaskResponse;
use cw_croncat_core::msg::{GetConfigResponse, QueryMsg};
use cw_croncat_core::types::AgentStatus;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::config::ChainConfig;
use crate::errors::{eyre, Report};

use super::RpcClient;

pub struct GrpcQuerier {
    rpc_client: RpcClient,
    croncat_addr: String,
}

impl std::fmt::Debug for GrpcQuerier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrpcQuerier")
            .field("croncat_addr", &self.croncat_addr)
            .finish()
    }
}

impl GrpcQuerier {
    pub async fn new(cfg: ChainConfig, rpc_url: String) -> Result<Self, Report> {
        let rpc_url = if !rpc_url.starts_with("https://") {
            format!("https://{}", rpc_url)
        } else {
            rpc_url
        };

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
        let status: Option<AgentStatus> = self
            .query_croncat(QueryMsg::GetAgent { account_id })
            .await?;

        status.ok_or_else(|| eyre!("Agent not registered"))
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
