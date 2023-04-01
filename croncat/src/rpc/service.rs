//!
//! RPC client service that can be used to execute and query the croncat chain.
//! This uses multiple approaches to ensure that the service is always available.
//!

use crate::config::{ChainConfig, ChainDataSource};
use crate::errors::{eyre, Report};
use crate::logging::info;
use cosm_orc::orchestrator::{Address, ChainTxResponse};
use cosm_tome::chain::coin::Coin;
use cosmrs::bip32;
use cosmrs::crypto::secp256k1::SigningKey;
use futures_util::Future;
use rand::seq::SliceRandom;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::debug;

use super::Querier;
use super::Signer;

type RpcSources = Arc<Mutex<HashMap<String, (ChainDataSource, bool)>>>;

lazy_static::lazy_static! {
    pub(crate) static ref RPC_SOURCES: RpcSources = Arc::new(Mutex::new(HashMap::new()));
}

#[derive(Debug)]
pub enum ServiceFailure {
    Timeout(Report),
    Transport(Report),
    Other(Report),
}

#[derive(Clone, PartialEq, Hash, Eq, Debug)]
pub enum RpcCallType {
    Execute,
    Query,
}

#[derive(Debug)]
pub enum RpcClientType {
    Execute(Box<Signer>),
    Query(Box<Querier>),
}

#[derive(Clone, Debug)]
pub struct RpcClientService {
    chain_config: ChainConfig,
    contract_addr: Address,
    key: bip32::XPrv,
    source_info: RpcSources,
}

impl RpcClientService {
    pub async fn new(
        chain_config: ChainConfig,
        key: bip32::XPrv,
        contract_addr: Option<Address>,
    ) -> Self {
        let contract_addr = contract_addr
            .unwrap_or_else(|| Address::from_str(chain_config.clone().factory.as_str()).unwrap());

        let mut global_sources = RPC_SOURCES.lock().await;

        let data_sources = if global_sources.is_empty() {
            let data_sources =
                Self::pick_best_sources(&chain_config, &chain_config.data_sources()).await;
            for (provider, data_source) in data_sources.iter() {
                global_sources.insert(provider.clone(), data_source.clone());
            }
            data_sources
        } else {
            global_sources.clone()
        };

        Self {
            key,
            chain_config,
            contract_addr,
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
        let mut race_track = RaceTrack::disqualify_after(Duration::from_secs(5));

        // Seen sources
        let mut seen_sources = HashSet::new();

        // Race all the sources and check that they connect to RPC.
        for (name, source) in sources {
            let source = source.clone();

            // If we've seen the before then skip it.
            if source.rpc.is_empty() || seen_sources.contains(&source.rpc) {
                continue;
            }
            // Add the source to the seen sources.
            seen_sources.insert(source.rpc.clone());

            let chain_config = chain_config.clone();
            let factory_addr = chain_config.clone().factory;
            race_track.add_racer(name, async move {
                let rpc_client = Querier::new(
                    source.rpc.clone(),
                    chain_config,
                    Address::from_str(&factory_addr)?,
                )
                .await?;
                // get block height from the nodes directly
                let _ = rpc_client
                    .rpc_client
                    .client
                    .client
                    .tendermint_query_latest_block()
                    .await?;

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
        let list: Vec<String> = data_sources
            .iter()
            .filter(|(_, (_, disqualified))| !disqualified)
            .map(|(i, _)| i.to_owned())
            .collect();
        let plural = if list.len() == 1 { "source" } else { "sources" };
        info!(
            "[{}] {} {} available! {:?}",
            chain_config.info.chain_id,
            list.len(),
            plural,
            list,
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

    async fn call<T, Fut, F>(&self, kind: RpcCallType, f: F) -> Result<T, Report>
    where
        Fut: Future<Output = Result<T, Report>>,
        F: Fn(RpcClientType) -> Fut,
    {
        let f = Box::new(f);
        let mut last_error = None;

        loop {
            let source_keys = {
                let source_info = self.source_info.lock().await;

                source_info
                    .keys()
                    .cloned()
                    .filter(|k| !source_info.get(k).unwrap().1)
                    .collect::<Vec<_>>()
            };

            if source_keys.is_empty() {
                if last_error.is_some() {
                    return Err(last_error.unwrap());
                }

                return Err(eyre!("No valid rpc sources available"));
            }

            let source_key = source_keys
                .choose(&mut rand::thread_rng())
                .unwrap()
                .to_string();
            let (source, _) = {
                self.source_info
                    .lock()
                    .await
                    .get_mut(&source_key)
                    .unwrap()
                    .clone()
            };

            // TODO: Change to contract_addr
            let rpc_client = match kind {
                RpcCallType::Execute => RpcClientType::Execute(Box::new(
                    match Signer::new(
                        source.rpc.to_string(),
                        self.chain_config.clone(),
                        self.contract_addr.clone(),
                        self.key.clone(),
                    )
                    .await
                    {
                        Ok(client) => client,
                        Err(e) => {
                            debug!("Failed to create RpcClient for {}: {}", source_key, e);
                            {
                                let mut source_info = self.source_info.lock().await;
                                let (_, bad) = source_info.get_mut(&source_key).unwrap();
                                *bad = true;
                            }
                            last_error = Some(e);
                            continue;
                        }
                    },
                )),
                RpcCallType::Query => RpcClientType::Query(Box::new(
                    match Querier::new(
                        source.clone().rpc.to_string(),
                        self.chain_config.clone(),
                        self.contract_addr.clone(),
                    )
                    .await
                    {
                        Ok(client) => client,
                        Err(e) => {
                            debug!("Failed to create RpcClient for {}: {}", source_key, e);
                            {
                                let mut source_info = self.source_info.lock().await;
                                let (_, bad) = source_info.get_mut(&source_key).unwrap();
                                *bad = true;
                            }
                            last_error = Some(e);
                            continue;
                        }
                    },
                )),
            };

            // ONLY mark as bad IF the /status endpoint doesnt return, otherwise provider is not considered bad.
            match f(rpc_client).await {
                Ok(result) => {
                    return Ok(result);
                }
                Err(e) if break_loop_errors(&e) => {
                    debug!("Error calling chain for {}: {}", source_key, e);
                    break Err(e);
                }
                Err(e) => {
                    debug!("Error calling chain for {}: {}", source_key, e);
                    // Handle cases for when query against a non-existant contract could stop this node
                    if e.to_string().to_lowercase().contains("contract: not found") {
                        continue;
                    }
                    // This will remove invalid providers if they have errors we dont know how to handle.
                    {
                        let mut source_info = self.source_info.lock().await;
                        let (_, bad) = source_info.get_mut(&source_key).unwrap();
                        *bad = true;
                    }
                    last_error = Some(e);
                    continue;
                }
            }
        }
    }

    pub async fn execute<T, Fut, F>(&self, f: F) -> Result<T, Report>
    where
        Fut: Future<Output = Result<T, Report>>,
        F: Fn(Box<Signer>) -> Fut,
    {
        self.call(RpcCallType::Execute, |client| async {
            if let RpcClientType::Execute(client) = client {
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
        F: Fn(Box<Querier>) -> Fut,
    {
        self.call(RpcCallType::Query, |client| async {
            if let RpcClientType::Query(client) = client {
                f(client).await
            } else {
                unreachable!()
            }
        })
        .await
    }

    /// Query the balance of an address.
    /// Returns the balance in the denom set for this client.
    pub async fn query_balance(&self, address: &str) -> Result<Coin, Report> {
        let balance = self
            .query(move |querier| {
                let address = address.parse::<Address>().unwrap();
                async move {
                    querier
                        .rpc_client
                        .query_balance(address.to_string().as_str())
                        .await
                }
            })
            .await?;

        Ok(balance)
    }

    /// Send funds to an address.
    pub async fn send_funds(
        &self,
        to: &str,
        from: &str,
        denom: &str,
        amount: u128,
    ) -> Result<ChainTxResponse, Report> {
        let response = self
            .execute(|signer| {
                let to = to;
                let from = from;
                let denom = denom;
                let amount = amount;
                async move { signer.rpc_client.send_funds(to, from, denom, amount).await }
            })
            .await?;

        Ok(response)
    }
}

fn break_loop_errors(e: &Report) -> bool {
    let msg = e.to_string().to_lowercase();
    msg.contains("agent not registered")
        || msg.contains("agent already registered")
        || msg.contains("agent not found")
        || msg.contains("account not found")
}
