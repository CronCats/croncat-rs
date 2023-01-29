//!
//! RPC client service that can be used to execute and query the croncat on chain.
//!

use std::str::FromStr;
use std::time::Duration;

use cosm_orc::orchestrator::Address;
use cosm_orc::orchestrator::ChainResponse;
use cosm_orc::orchestrator::ChainTxResponse;
use cosm_orc::orchestrator::Coin;
use cosmrs::bip32;
use cosmrs::crypto::secp256k1::SigningKey;
use cosmrs::AccountId;
use cw_croncat_core::msg::AgentResponse;
use cw_croncat_core::msg::AgentTaskResponse;
use cw_croncat_core::msg::TaskWithQueriesResponse;
use cw_croncat_core::msg::{ExecuteMsg, GetConfigResponse, QueryMsg};
use cw_rules_core::msg::QueryConstruct;
use cw_rules_core::types::CroncatQuery;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::time::timeout;

use crate::config::ChainConfig;
use crate::errors::{eyre, Report};
use crate::utils::normalize_rpc_url;

use super::RpcClient;

#[derive(Clone, Debug)]
pub struct Signer {
    rpc_client: RpcClient,
    pub manager: String,
    pub account_id: AccountId,
}

impl Signer {
    pub async fn new(
        rpc_url: String,
        cfg: ChainConfig,
        manager: String,
        key: bip32::XPrv,
        mnemonic: String,
    ) -> Result<Self, Report> {
        let rpc_url = normalize_rpc_url(&rpc_url);

        // Get the account id from the key.
        let signing_key: SigningKey = key.into();
        let account_id = signing_key
            .public_key()
            .account_id(&cfg.info.bech32_prefix)?;

        // Create a new RPC client
        let mut rpc_client = RpcClient::new(&cfg, rpc_url.as_str())?;
        // TODO: This is a hack to get around the fact that cosm-tome doesn't
        // let us pass an xprv.
        rpc_client.set_mnemonic(mnemonic);
        rpc_client.set_denom(
            cfg.denom
                .unwrap_or_else(|| cfg.info.fees.fee_tokens[0].denom.clone())
                .as_str(),
        );

        Ok(Self {
            account_id,
            manager,
            rpc_client,
        })
    }

    pub async fn query_croncat<R, S>(&self, msg: S) -> Result<R, Report>
    where
        S: Serialize,
        R: DeserializeOwned,
    {
        let out = timeout(
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
        })??;

        Ok(out)
    }

    pub async fn execute_croncat<S>(&self, msg: S) -> Result<ChainResponse, Report>
    where
        S: Serialize,
    {
        let res = timeout(
            Duration::from_secs_f64(self.rpc_client.timeout_secs),
            self.rpc_client.wasm_execute(msg),
        )
        .await
        .map_err(|err| {
            eyre!(
                "Timeout ({}s) while executing wasm: {}",
                self.rpc_client.timeout_secs,
                err
            )
        })??;

        Ok(res)
    }

    pub async fn register_agent(
        &self,
        payable_account_id: &Option<String>,
    ) -> Result<ChainResponse, Report> {
        self.execute_croncat(ExecuteMsg::RegisterAgent {
            payable_account_id: payable_account_id.clone(),
        })
        .await
    }

    pub async fn unregister_agent(&self) -> Result<ChainResponse, Report> {
        self.execute_croncat(ExecuteMsg::UnregisterAgent { from_behind: None })
            .await
    }

    pub async fn update_agent(&self, payable_account_id: String) -> Result<ChainResponse, Report> {
        self.execute_croncat(ExecuteMsg::UpdateAgent { payable_account_id })
            .await
    }

    pub async fn withdraw_reward(&self) -> Result<ChainResponse, Report> {
        self.execute_croncat(ExecuteMsg::WithdrawReward {}).await
    }

    pub async fn proxy_call(&self, task_hash: Option<String>) -> Result<ChainResponse, Report> {
        self.execute_croncat(ExecuteMsg::ProxyCall { task_hash })
            .await
    }

    pub async fn get_agent(&self, account_id: &str) -> Result<Option<AgentResponse>, Report> {
        let res = self
            .query_croncat(QueryMsg::GetAgent {
                account_id: account_id.to_string(),
            })
            .await?;
        Ok(res)
    }

    pub async fn check_in_agent(&self) -> Result<ChainResponse, Report> {
        self.execute_croncat(ExecuteMsg::CheckInAgent {}).await
    }

    pub fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    pub async fn get_agent_tasks(
        &self,
        account_id: &str,
    ) -> Result<Option<AgentTaskResponse>, Report> {
        let res: Option<AgentTaskResponse> = self
            .query_croncat(QueryMsg::GetAgentTasks {
                account_id: account_id.to_string(),
            })
            .await?;
        Ok(res)
    }

    pub async fn query_get_tasks_with_queries(
        &self,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Result<Vec<TaskWithQueriesResponse>, Report> {
        let res: Vec<TaskWithQueriesResponse> = self
            .query_croncat(QueryMsg::GetTasksWithQueries {
                // TODO: find optimal pagination
                from_index,
                limit,
            })
            .await?;
        Ok(res)
    }

    pub async fn fetch_queries(&self) -> Result<Vec<TaskWithQueriesResponse>, Report> {
        let mut tasks_with_queries = Vec::new();
        let mut start_index = 0;
        let limit = 20;
        loop {
            let current_iteration = self
                .query_get_tasks_with_queries(Some(start_index), Some(limit))
                .await?;
            let last_iteration = current_iteration.len() < limit as usize;
            tasks_with_queries.extend(current_iteration);
            if last_iteration {
                break;
            }
            start_index += limit;
        }
        Ok(tasks_with_queries)
    }

    pub async fn check_queries(
        &self,
        queries: Vec<CroncatQuery>,
    ) -> Result<(bool, Option<u64>), Report> {
        let cw_rules_addr = {
            let cfg: GetConfigResponse = self.query_croncat(QueryMsg::GetConfig {}).await?;
            cfg.cw_rules_addr
        }
        .to_string();
        let res = self
            .rpc_client
            .call_wasm_query(
                Address::from_str(cw_rules_addr.as_str()).unwrap(),
                cw_rules_core::msg::QueryMsg::QueryConstruct(QueryConstruct { queries }),
            )
            .await?;
        Ok(res)
    }

    pub async fn query_native_balance(&self, account_id: &str) -> Result<Coin, Report> {
        self.rpc_client.query_balance(account_id).await
    }

    pub async fn send_funds(
        &self,
        account_id: &str,
        to: &str,
        amount: u128,
        denom: &str,
    ) -> Result<ChainTxResponse, Report> {
        self.rpc_client
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
