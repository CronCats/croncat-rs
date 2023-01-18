//!
//! This module contains the code for querying the croncat contract via gRPC.
//!

use std::collections::HashMap;

use cosm_orc::config::cfg::Config as CosmOrcConfig;
use cosm_orc::config::ChainConfig as CosmOrcChainConfig;
use cosm_orc::orchestrator::cosm_orc::CosmOrc;
use cosm_orc::orchestrator::deploy::DeployInfo;
use cosm_orc::orchestrator::TendermintRPC;
use cosm_tome::modules::auth::model::Address;

use cw_croncat_core::msg::AgentTaskResponse;
use cw_croncat_core::msg::TaskResponse;
use cw_croncat_core::msg::{GetConfigResponse, QueryMsg};
use cw_croncat_core::types::AgentStatus;

use serde::de::DeserializeOwned;

use crate::config::ChainConfig;
use crate::errors::{eyre, Report};

/// An RPC client for querying the croncat contract.
pub struct RpcClient {
    client: CosmOrc<TendermintRPC>,
    contract_addr: Address,
}

impl RpcClient {
    /// Create a new [`RpcClient`].
    pub fn new(cfg: &ChainConfig, rpc_url: &str) -> Result<Self, Report> {
        // Build the contract info map.
        let mut contract_deploy_info = HashMap::new();
        contract_deploy_info.insert(
            "croncat-manager".to_string(),
            DeployInfo {
                code_id: None,
                address: Some(cfg.manager.clone()),
            },
        );

        // Convert our config into a CosmOrc config with the specified rpc url.
        let config = CosmOrcConfig {
            chain_cfg: CosmOrcChainConfig {
                denom: cfg.info.fees.fee_tokens[0].denom.clone(),
                prefix: cfg.info.bech32_prefix.clone(),
                chain_id: cfg.info.chain_id.clone(),
                rpc_endpoint: Some(rpc_url.to_string()),
                grpc_endpoint: None,
                gas_prices: cfg.gas_prices as f64,
                gas_adjustment: cfg.gas_adjustment as f64,
            },
            contract_deploy_info,
        };
        let contract_addr = cfg.manager.parse::<Address>()?;

        Ok(Self {
            client: CosmOrc::new_tendermint_rpc(config, true)?,
            contract_addr,
        })
    }

    /// Query the contract via RPC.
    pub async fn wasm_query<S, R>(&self, msg: S) -> Result<R, Report>
    where
        S: Into<QueryMsg>,
        R: DeserializeOwned,
    {
        // Query the chain
        let response = self
            .client
            .client
            .wasm_query(self.contract_addr.clone(), &msg.into())
            .await?;

        // Deserialize the response
        let data = response
            .data()
            .map_err(|e| eyre!("Failed to deserialize response data: {}", e))?;

        Ok(data)
    }
}

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
        S: Into<QueryMsg>,
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
