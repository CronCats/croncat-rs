//!
//! This module contains the code for querying the croncat contract via gRPC.
//!

use cosm_tome::modules::auth::model::Address;
use cw_croncat_core::msg::AgentTaskResponse;
use cw_croncat_core::msg::TaskResponse;
use cw_croncat_core::msg::{GetConfigResponse, QueryMsg};
use cw_croncat_core::types::AgentStatus;
use serde::de::DeserializeOwned;
use std::time::Duration;
use tokio::time::timeout;

use crate::client::query_client::CosmosQueryClient;
use crate::client::GetWasmQueryClient;
use crate::config::ChainConfig;
use crate::errors::{eyre, Report};

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
    pub async fn new(cfg: ChainConfig, grpc_url: String, rpc_url: String) -> Result<Self, Report> {
        // TODO: How should we handle this? Is the hack okay?
        // Quick hack to add https:// to the url if it is missing
        let grpc_url = if !grpc_url.starts_with("https://") {
            format!("https://{}", grpc_url)
        } else {
            grpc_url
        };
        let rpc_url = if !rpc_url.starts_with("https://") {
            format!("https://{}", rpc_url)
        } else {
            rpc_url
        };

        let client = timeout(
            Duration::from_secs(10),
            CosmosQueryClient::new(grpc_url, rpc_url, &cfg.info.fees.fee_tokens[0].denom),
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

    pub async fn get_agent_status(
        &self,
        account_id: String,
    ) -> Result<Option<AgentStatus>, Report> {
        let croncat_address: Address = self.croncat_addr.parse()?;
        let client = self.client.tm_wasm_query_client().client;

        // TODO: remove this funsies block
        'funsies_remove_me_haha: {
            // For funsies, try it with Trevor's agent
            let funsies = client
                .wasm_query(
                    croncat_address.clone(),
                    &QueryMsg::GetAgent {
                        account_id: "juno1rez0cc8zx8u75wqaz04xzcr83f79lw4hk62z7t".to_string(),
                    },
                )
                .await?;
            println!("(remove this demo) funsies {:?}", funsies);
        }

        let agent = client
            .wasm_query(croncat_address, &QueryMsg::GetAgent { account_id })
            .await;
        let agent_status_decoded = String::from_utf8(agent.unwrap().res.data.clone().unwrap());
        let agent_status_readable = match agent_status_decoded {
            Ok(status) => status,
            Err(e) => return Err(eyre!("Could not turn agent status into string. {:?}", e)),
        };
        let status: Option<AgentStatus> = match agent_status_readable.to_lowercase().as_str() {
            "active" => Some(AgentStatus::Active),
            "pending" => Some(AgentStatus::Pending),
            "nominated" => Some(AgentStatus::Nominated),
            "null" => None,
            _ => return Err(eyre!("Unknown agent status")),
        };

        Ok(status)
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
