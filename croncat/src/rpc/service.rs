//!
//! GRPC client service that can be used to execute and query the croncat chain.
//! This uses multiple approaches to ensure that the service is always available.
//!

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use cosmrs::bip32;
use cosmrs::crypto::secp256k1::SigningKey;
use futures_util::Future;
use lazy_static::__Deref;
use rand::seq::SliceRandom;
use tokio::sync::Mutex;
use tracing::debug;

use crate::config::ChainConfig;
use crate::config::ChainDataSource;
use crate::errors::{eyre, Report};
use crate::logging::info;

use super::Querier;
use super::Signer;

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

#[derive(Debug)]
pub enum GrpcClient {
    Execute(Box<Signer>),
    Query(Box<Querier>),
}

#[derive(Clone, Debug)]
pub struct GrpcClientService {
    chain_config: ChainConfig,
    key: bip32::XPrv,
    mnemonic: String,
    source_info: Arc<Mutex<HashMap<String, (ChainDataSource, bool)>>>,
}

impl GrpcClientService {
    pub async fn new(chain_config: ChainConfig, mnemonic: String, key: bip32::XPrv) -> Self {
        let data_sources =
            Self::pick_best_sources(&chain_config, &chain_config.data_sources()).await;

        Self {
            key,
            mnemonic,
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
        let mut race_track = RaceTrack::disqualify_after(Duration::from_secs(5));

        // Race all the sources and check that they connect to GRPC.
        for (name, source) in sources {
            let source = source.clone();
            let chain_config = chain_config.clone();
            race_track.add_racer(name, async move {
                let rpc_client = Querier::new(chain_config, source.rpc.clone()).await?;
                let _ = rpc_client.query_config().await?;

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
                    match Signer::new(
                        source.rpc.to_string(),
                        self.chain_config.clone(),
                        self.chain_config.manager.clone(),
                        self.key.clone(),
                        self.mnemonic.clone(),
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
                    match Querier::new(self.chain_config.clone(), source.rpc.to_string()).await {
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
                Err(e) if break_loop_errors(&e) => {
                    debug!("Agent not registered for {}: {}", source_key, e);
                    break Err(e);
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
        F: Fn(Box<Signer>) -> Fut,
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
        F: Fn(Box<Querier>) -> Fut,
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

fn break_loop_errors(e: &Report) -> bool {
    let msg = e.to_string().to_lowercase();
    msg.contains("agent not registered")
        || msg.contains("account not found")
        || msg.contains("already registered")
}
