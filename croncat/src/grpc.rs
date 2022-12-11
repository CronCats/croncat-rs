//!
//! Use the [cosmos_sdk_proto](https://crates.io/crates/cosmos-sdk-proto) library to create clients for GRPC node requests.
//!

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use cosmos_chain_registry::ChainInfo;
use cosmos_sdk_proto::cosmwasm::wasm::v1::msg_client::MsgClient;
use cosmos_sdk_proto::cosmwasm::wasm::v1::query_client::QueryClient;
use cosmrs::bip32;
use cosmrs::crypto::secp256k1::SigningKey;
use cosmrs::AccountId;
use cw_croncat_core::msg::AgentTaskResponse;
use cw_croncat_core::msg::CwCroncatResponse;
use cw_croncat_core::msg::TaskResponse;
use cw_croncat_core::msg::TaskWithRulesResponse;
use cw_croncat_core::msg::{ExecuteMsg, GetConfigResponse, QueryMsg};
use cw_croncat_core::types::AgentResponse;
use cw_rules_core::msg::QueryConstruct;
use cw_rules_core::types::Rule;
use futures_util::Future;
use lazy_static::__Deref;
use rand::seq::SliceRandom;
use serde::de::DeserializeOwned;
use tendermint_rpc::endpoint::broadcast::tx_commit::TxResult;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tonic::transport::Channel;
use tracing::debug;
use url::Url;

use crate::client::full_client::CosmosFullClient;
use crate::client::query_client::CosmosQueryClient;
use crate::config::ChainConfig;
use crate::config::ChainDataSource;
use crate::errors::{eyre, Report};
use crate::logging::info;

///
/// Create message and query clients for interacting with the chain.
///
pub async fn connect(url: String) -> Result<(MsgClient<Channel>, QueryClient<Channel>), Report> {
    // Parse url
    let url = Url::parse(&url)?;

    info!("Connecting to GRPC services @ {}", url);

    // Setup our GRPC clients
    let msg_client = MsgClient::connect(url.to_string()).await?;
    let query_client = QueryClient::connect(url.to_string()).await?;

    info!("Connected to GRPC services @ {}", url);

    Ok((msg_client, query_client))
}

#[derive(Clone)]
pub struct GrpcSigner {
    client: CosmosFullClient,
    pub manager: String,
    pub account_id: AccountId,
}

impl GrpcSigner {
    async fn new(
        rpc_url: String,
        grpc_url: String,
        chain_info: ChainInfo,
        manager: String,
        key: bip32::XPrv,
        gas_prices: f32,
        gas_adjustment: f32,
    ) -> Result<Self, Report> {
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
            CosmosFullClient::new(
                rpc_url,
                grpc_url,
                chain_info,
                key,
                gas_prices,
                gas_adjustment,
            ),
        )
        .await??;
        let account_id = client
            .key()
            .public_key()
            .account_id(&client.chain_info.bech32_prefix)?;

        Ok(Self {
            client,
            account_id,
            manager,
        })
    }

    pub fn from_chain_config(
        chain_config: &ChainConfig,
        key: bip32::XPrv,
    ) -> impl Future<Output = Result<Self, Report>> {
        GrpcSigner::new(
            chain_config.info.apis.rpc[0].address.clone(),
            chain_config.info.apis.grpc[0].address.clone(),
            chain_config.info.clone(),
            chain_config.manager.clone(),
            key,
            chain_config.gas_prices,
            chain_config.gas_adjustment,
        )
    }

    pub async fn query_croncat<T>(&self, msg: &QueryMsg) -> Result<T, Report>
    where
        T: DeserializeOwned,
    {
        let out = timeout(
            Duration::from_secs(30),
            self.client
                .query_client
                .query_contract(&self.manager.to_string(), msg),
        )
        .await
        .map_err(|err| eyre!("Timeout (30s) while querying contract: {}", err))??;

        Ok(out)
    }

    pub async fn execute_croncat(&self, msg: &ExecuteMsg) -> Result<TxResult, Report> {
        let res = timeout(
            Duration::from_secs(30),
            self.client.execute_wasm(msg, &self.manager.to_string()),
        )
        .await
        .map_err(|err| eyre!("Timeout (30s) while executing wasm: {}", err))??;

        Ok(res.deliver_tx)
    }

    pub async fn register_agent(
        &self,
        payable_account_id: &Option<String>,
    ) -> Result<TxResult, Report> {
        self.execute_croncat(&ExecuteMsg::RegisterAgent {
            payable_account_id: payable_account_id.clone(),
        })
        .await
    }

    pub async fn unregister_agent(&self) -> Result<TxResult, Report> {
        self.execute_croncat(&ExecuteMsg::UnregisterAgent {}).await
    }

    pub async fn update_agent(&self, payable_account_id: String) -> Result<TxResult, Report> {
        self.execute_croncat(&ExecuteMsg::UpdateAgent { payable_account_id })
            .await
    }

    pub async fn withdraw_reward(&self) -> Result<TxResult, Report> {
        self.execute_croncat(&ExecuteMsg::WithdrawReward {}).await
    }

    pub async fn proxy_call(&self, task_hash: Option<String>) -> Result<TxResult, Report> {
        self.execute_croncat(&ExecuteMsg::ProxyCall { task_hash })
            .await
    }

    pub async fn get_agent(&self, account_id: &str) -> Result<Option<AgentResponse>, Report> {
        let res = self
            .query_croncat(&QueryMsg::GetAgent {
                account_id: account_id.to_string(),
            })
            .await?;
        Ok(res)
    }

    pub async fn check_in_agent(&self) -> Result<TxResult, Report> {
        self.execute_croncat(&ExecuteMsg::CheckInAgent {}).await
    }

    pub fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    pub async fn get_agent_tasks(
        &self,
        account_id: &str,
    ) -> Result<Option<AgentTaskResponse>, Report> {
        let res: Option<AgentTaskResponse> = self
            .query_croncat(&QueryMsg::GetAgentTasks {
                account_id: account_id.to_string(),
            })
            .await?;
        Ok(res)
    }

    pub async fn query_get_tasks_with_rules(
        &self,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Result<Vec<TaskWithRulesResponse>, Report> {
        let res: Vec<TaskWithRulesResponse> = self
            .query_croncat(&QueryMsg::GetTasksWithRules {
                // TODO: find optimal pagination
                from_index,
                limit,
            })
            .await?;
        Ok(res)
    }

    pub async fn fetch_rules(&self) -> Result<Vec<TaskWithRulesResponse>, Report> {
        let mut tasks_with_rules = Vec::new();
        let mut start_index = 0;
        let limit = 20;
        loop {
            let current_iteration = self
                .query_get_tasks_with_rules(Some(start_index), Some(limit))
                .await?;
            let last_iteration = current_iteration.len() < limit as usize;
            tasks_with_rules.extend(current_iteration);
            if last_iteration {
                break;
            }
            start_index += limit;
        }
        Ok(tasks_with_rules)
    }

    pub async fn check_rules(&self, rules: Vec<Rule>) -> Result<(bool, Option<u64>), Report> {
        let cw_rules_addr = {
            let cfg: GetConfigResponse = self.query_croncat(&QueryMsg::GetConfig {}).await?;
            cfg.cw_rules_addr
        };
        let res = self
            .client
            .query_client
            .query_contract(
                &cw_rules_addr,
                cw_rules_core::msg::QueryMsg::QueryConstruct(QueryConstruct { rules }),
            )
            .await?;
        Ok(res)
    }

    pub fn key(&self) -> SigningKey {
        self.client.key()
    }

    pub fn chain_info(&self) -> &ChainInfo {
        &self.client.chain_info
    }
}

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

    pub async fn get_contract_state(
        &self,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Result<String, Report> {
        let response: CwCroncatResponse = self
            .query_croncat(&QueryMsg::GetState { from_index, limit })
            .await?;
        let json = serde_json::to_string_pretty(&response)?;
        Ok(json)
    }
}

#[derive(Debug)]
pub enum ServiceFailure {
    Timeout(Report),
    Transport(Report),
    Other(Report),
}

#[derive(Debug, Default)]
#[allow(dead_code)]
struct GrpcClientStatus {
    bad: bool,
    last_success_timestamp: Option<u64>,
    last_failure_timestamp: Option<u64>,
    last_failure: Option<ServiceFailure>,
}

#[derive(Clone, PartialEq, Hash, Eq, Debug)]
pub enum GrpcCallType {
    Execute,
    Query,
}

pub enum GrpcClient {
    Execute(Box<GrpcSigner>),
    Query(Box<GrpcQuerier>),
}

#[derive(Clone, Debug)]
pub struct GrpcClientService {
    chain_config: ChainConfig,
    key: bip32::XPrv,
    source_info: Arc<Mutex<HashMap<String, (ChainDataSource, bool)>>>,
}

impl GrpcClientService {
    pub async fn new(chain_config: ChainConfig, key: bip32::XPrv) -> Self {
        let data_sources =
            Self::pick_best_sources(&chain_config, &chain_config.data_sources()).await;

        Self {
            key,
            chain_config,
            source_info: Arc::new(Mutex::new(data_sources)),
        }
    }

    async fn pick_best_sources(
        chain_config: &ChainConfig,
        sources: &HashMap<String, ChainDataSource>,
    ) -> HashMap<String, (ChainDataSource, bool)> {
        use speedracer::RaceTrack;

        info!(
            "[{}] Picking best source for chain...",
            chain_config.info.chain_id
        );

        // Create a racetrack for testing sources.
        let mut race_track = RaceTrack::disqualify_after(Duration::from_secs(2));

        // Race all the sources and check that they connect to GRPC.
        for (name, source) in sources {
            let source = source.clone();
            let chain_config = chain_config.clone();
            race_track.add_racer(name, async move {
                let grpc_client = GrpcQuerier::new(chain_config, source.grpc.clone()).await?;
                let _ = grpc_client.query_config().await?;

                Ok(source)
            });
        }

        // Run our racers.
        race_track.run().await;

        // Get the rankings
        let rankings = race_track.rankings();
        // Get the data sources
        let data_sources = chain_config.data_sources();

        // Create a map of data sources with their rankings and disqualified status
        let data_sources: HashMap<String, (ChainDataSource, bool)> = rankings
            .into_iter()
            .map(|result| {
                let source = data_sources.get(&result.name).unwrap();
                (result.name, (source.clone(), result.disqualified))
            })
            .collect();

        // Log how many available sources we have
        info!(
            "[{}] {} source(s) available!",
            chain_config.info.chain_id,
            data_sources
                .iter()
                .filter(|(_, (_, disqualified))| !disqualified)
                .count()
        );

        data_sources
    }

    pub fn key(&self) -> SigningKey {
        (&self.key).try_into().unwrap()
    }

    pub fn account_id(&self) -> String {
        self.key()
            .public_key()
            .account_id(&self.chain_config.info.bech32_prefix.clone())
            .unwrap()
            .to_string()
    }

    async fn call<T, Fut, F>(&self, kind: GrpcCallType, f: F) -> Result<T, Report>
    where
        Fut: Future<Output = Result<T, Report>>,
        F: Fn(GrpcClient) -> Fut,
    {
        let f = Box::new(f);
        let mut last_error = None;

        loop {
            let mut source_info = self.source_info.lock().await;
            let source_keys = source_info
                .keys()
                .filter(|k| !source_info.get(*k).unwrap().1)
                .collect::<Vec<_>>();

            if source_keys.is_empty() {
                if last_error.is_some() {
                    return Err(last_error.unwrap());
                }

                // TODO: This should be a more specific error
                return Err(eyre!("No valid data sources available"));
            }

            let source_key = source_keys
                .choose(&mut rand::thread_rng())
                .unwrap()
                .deref()
                .clone();
            let (source, _) = source_info.get_mut(&source_key).unwrap().clone();

            let grpc_client = match kind {
                GrpcCallType::Execute => GrpcClient::Execute(Box::new(
                    match GrpcSigner::new(
                        source.rpc.to_string(),
                        source.grpc.to_string(),
                        self.chain_config.info.clone(),
                        self.chain_config.manager.clone(),
                        self.key.clone(),
                        self.chain_config.gas_prices,
                        self.chain_config.gas_adjustment,
                    )
                    .await
                    {
                        Ok(client) => client,
                        Err(e) => {
                            debug!("Failed to create grpc client for {}: {}", source_key, e);
                            let (_, bad) = source_info.get_mut(&source_key).unwrap();
                            *bad = true;
                            last_error = Some(e);
                            continue;
                        }
                    },
                )),
                GrpcCallType::Query => GrpcClient::Query(Box::new(
                    match GrpcQuerier::new(self.chain_config.clone(), source.grpc.to_string()).await
                    {
                        Ok(client) => client,
                        Err(e) => {
                            debug!("Failed to create grpc client for {}: {}", source_key, e);
                            let (_, bad) = source_info.get_mut(&source_key).unwrap();
                            *bad = true;
                            last_error = Some(e);
                            continue;
                        }
                    },
                )),
            };

            match f(grpc_client).await {
                Ok(result) => {
                    return Ok(result);
                }
                Err(e) => {
                    debug!("Error calling chain for {}: {}", source_key, e);
                    let (_, bad) = source_info.get_mut(&source_key).unwrap();
                    *bad = true;
                    last_error = Some(e);
                    continue;
                }
            }
        }
    }

    pub async fn execute<T, Fut, F>(&self, f: F) -> Result<T, Report>
    where
        Fut: Future<Output = Result<T, Report>>,
        F: Fn(Box<GrpcSigner>) -> Fut,
    {
        self.call(GrpcCallType::Execute, |client| async {
            if let GrpcClient::Execute(client) = client {
                f(client).await
            } else {
                unreachable!()
            }
        })
        .await
    }

    pub async fn query<T, Fut, F>(&self, f: F) -> Result<T, Report>
    where
        Fut: Future<Output = Result<T, Report>>,
        F: Fn(Box<GrpcQuerier>) -> Fut,
    {
        self.call(GrpcCallType::Query, |client| async {
            if let GrpcClient::Query(client) = client {
                f(client).await
            } else {
                unreachable!()
            }
        })
        .await
    }
}
