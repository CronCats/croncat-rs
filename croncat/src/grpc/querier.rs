//!
//! This module contains the code for querying the croncat contract via gRPC.
//!

use std::time::Duration;

use cw_croncat_core::msg::AgentResponse;
use cw_croncat_core::msg::AgentTaskResponse;
use cw_croncat_core::msg::TaskResponse;
use cw_croncat_core::msg::{GetConfigResponse, QueryMsg};
use serde::de::DeserializeOwned;
use tokio::time::timeout;

use crate::client::query_client::CosmosQueryClient;
use crate::config::ChainConfig;
use crate::errors::Report;

pub struct GrpcQuerier {
    client: CosmosQueryClient,
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
    pub async fn new(cfg: ChainConfig, grpc_url: String) -> Result<Self, Report> {
        // TODO: How should we handle this? Is the hack okay?
        // Quick hack to add https:// to the url if it is missing
        let grpc_url = if !grpc_url.starts_with("https://") {
            format!("https://{}", grpc_url)
        } else {
            grpc_url
        };

        let client = timeout(
            Duration::from_secs(10),
            CosmosQueryClient::new(grpc_url, &cfg.info.fees.fee_tokens[0].denom),
        )
        .await??;

        Ok(Self {
            client,
            croncat_addr: cfg.manager,
        })
    }

    pub async fn query_croncat<T>(&self, msg: &QueryMsg) -> Result<T, Report>
    where
        T: DeserializeOwned,
    {
        let out = self.client.query_contract(&self.croncat_addr, msg).await?;
        Ok(out)
    }

    pub async fn query_config(&self) -> Result<String, Report> {
        let config: GetConfigResponse = self.query_croncat(&QueryMsg::GetConfig {}).await?;
        let json = serde_json::to_string_pretty(&config)?;
        Ok(json)
    }

    pub async fn get_agent_status(&self, account_id: String) -> Result<String, Report> {
        let agent: Option<AgentResponse> = self
            .query_croncat(&QueryMsg::GetAgent { account_id })
            .await?;
        let json = serde_json::to_string_pretty(&agent)?;
        Ok(json)
    }

    pub async fn get_tasks(
        &self,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Result<String, Report> {
        let response: Vec<TaskResponse> = self
            .query_croncat(&QueryMsg::GetTasks { from_index, limit })
            .await?;
        let json = serde_json::to_string_pretty(&response)?;
        Ok(json)
    }

    pub async fn get_agent_tasks(&self, account_id: String) -> Result<String, Report> {
        let response: Option<AgentTaskResponse> = self
            .query_croncat(&QueryMsg::GetAgentTasks { account_id })
            .await?;
        let json = serde_json::to_string_pretty(&response)?;
        Ok(json)
    }
}
