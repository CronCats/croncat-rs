use crate::channels::{ShutdownRx, StatusStreamRx};
use crate::config::ChainConfig;
use crate::utils::AtomicIntervalCounter;
use crate::{rpc::RpcClientService, store::factory::LocalCacheStorage};
use color_eyre::{eyre::eyre, Report};
use cosm_orc::orchestrator::Address;
use croncat_sdk_factory::msg::{
    ContractMetadataInfo, ContractMetadataResponse, EntryResponse, FactoryQueryMsg,
};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tracing::info;

pub fn to_version_key(name: String, version: [u8; 2]) -> String {
    format!("{}_{}.{}", name, version[0], version[1])
}

pub struct Factory {
    pub client: RpcClientService,
    pub contract_addr: Address,
    pub store: LocalCacheStorage,
}

// FLOW:
// - check if local cache has versions ready, if at all
//   - if no versions, go get from chain - using current chain context
//   - if no chain versions, panic
//   - if versions, load into local cache & storage
// - return known versions
//
// Example Data:
// {
//   // when this cache should get removed/updated
//   expires: 1696407069536,
//   // Which versions to default to
//   latest: {
//     manager: "0.1"
//   },
//   // the entirety of the versions, in case we want the agent to override and use this version instead
//   // Great for when there are multiple versions, maybe needing to switch & decommission safely/slowly
//   versions: {
//     (manager, 0.1): {
//       ...metadata
//     }
//   }
// }

impl Factory {
    pub async fn new(
        // This MUST exist or the whole app goes booommmmmm.
        cfg: ChainConfig,
        client: RpcClientService,
    ) -> Result<Self, Report> {
        Ok(Self {
            client,
            contract_addr: Address::from_str(&cfg.factory)?,
            store: LocalCacheStorage::default(),
        })
    }

    // load versions: get latest & all versions, put into storage
    // NOTE: Result returns if it was reloaded or not
    pub async fn load(&mut self) -> Result<bool, Report> {
        let b = if self.store.get().is_some() {
            // Have the unexpired cache data, wooooot!
            false
        } else {
            // Go get latest version data
            let mut latest = HashMap::new();
            let mut versions = HashMap::new();
            let entries = self.get_latest_contracts().await?;

            // NOTE: This doesnt handle ALL versions, just latest
            for entry in entries {
                latest.insert(entry.contract_name.clone(), entry.metadata.version);
                versions.insert(
                    to_version_key(entry.contract_name, entry.metadata.version),
                    entry.metadata,
                );
            }

            // update storage
            self.store.insert(Some(latest), Some(versions)).is_ok()
        };

        // only need to make sure we loaded y'all
        Ok(b)
    }

    pub async fn get_expiry(&mut self) -> Result<i64, Report> {
        let ts = if let Some(data) = self.store.get() {
            data.expires
        } else {
            0
        };

        Ok(ts)
    }

    // get contract addr for contract_name, by version or default latest
    pub async fn get_contract_addr(&self, contract_name: String) -> Result<Address, Report> {
        let err = "No version found for {contract_name}";
        if let Some(data) = self.store.get() {
            let version = data.latest.get(&contract_name).expect(err);
            let metadata = data
                .versions
                .get(&to_version_key(contract_name, *version))
                .expect(err);
            return Ok(Address::from_str(metadata.contract_addr.as_ref())?);
        }
        Err(eyre!(err))
    }

    pub async fn get_latest_contracts(&self) -> Result<Vec<EntryResponse>, Report> {
        let entries: Vec<EntryResponse> = self
            .client
            .query(move |querier| {
                let contract_addr = self.contract_addr.clone();
                async move {
                    querier
                        .query_croncat(FactoryQueryMsg::LatestContracts {}, Some(contract_addr))
                        .await
                }
            })
            .await?;
        Ok(entries)
    }

    pub async fn get_latest_contract_by_name(
        &self,
        contract_name: String,
    ) -> Result<ContractMetadataResponse, Report> {
        let data: ContractMetadataResponse = self
            .client
            .query(move |querier| {
                let contract_addr = self.contract_addr.clone();
                let contract_name = contract_name.clone();
                async move {
                    querier
                        .query_croncat(
                            FactoryQueryMsg::LatestContract { contract_name },
                            Some(contract_addr),
                        )
                        .await
                }
            })
            .await?;
        Ok(data)
    }

    pub async fn get_versions_by_contract_name(
        &self,
        contract_name: String,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Result<Vec<ContractMetadataInfo>, Report> {
        let entries: Vec<ContractMetadataInfo> = self
            .client
            .query(move |querier| {
                let contract_addr = self.contract_addr.clone();
                let contract_name = contract_name.clone();
                let from_index = Some(from_index.unwrap_or(0));
                let limit = Some(limit.unwrap_or(100));
                async move {
                    querier
                        .query_croncat(
                            FactoryQueryMsg::VersionsByContractName {
                                contract_name,
                                from_index,
                                limit,
                            },
                            Some(contract_addr),
                        )
                        .await
                }
            })
            .await?;
        Ok(entries)
    }

    pub async fn get_contract_names(
        &self,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Result<Vec<String>, Report> {
        let entries: Vec<String> = self
            .client
            .query(move |querier| {
                let contract_addr = self.contract_addr.clone();
                let from_index = Some(from_index.unwrap_or(0));
                let limit = Some(limit.unwrap_or(100));
                async move {
                    querier
                        .query_croncat(
                            FactoryQueryMsg::ContractNames { from_index, limit },
                            Some(contract_addr),
                        )
                        .await
                }
            })
            .await?;
        Ok(entries)
    }

    pub async fn get_all_versions(
        &self,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Result<Vec<EntryResponse>, Report> {
        let entries: Vec<EntryResponse> = self
            .client
            .query(move |querier| {
                let contract_addr = self.contract_addr.clone();
                let from_index = Some(from_index.unwrap_or(0));
                let limit = Some(limit.unwrap_or(100));
                async move {
                    querier
                        .query_croncat(
                            FactoryQueryMsg::AllEntries { from_index, limit },
                            Some(contract_addr),
                        )
                        .await
                }
            })
            .await?;
        Ok(entries)
    }
}

///
/// Check every nth block with [`AtomicIntervalCounter`] if factory cache needs refresh
///
pub async fn refresh_factory_loop(
    mut block_stream_rx: StatusStreamRx,
    mut shutdown_rx: ShutdownRx,
    chain_id: Arc<String>,
    mut factory_client: Factory,
) -> Result<(), Report> {
    let block_counter = AtomicIntervalCounter::new(200);
    let task_handle: tokio::task::JoinHandle<Result<(), Report>> = tokio::task::spawn(async move {
        while let Ok(block) = block_stream_rx.recv().await {
            println!("Current block {:?}", block.inner.sync_info.latest_block_height);
            block_counter.tick();
            if block_counter.is_at_interval() && factory_client.load().await? {
                info!("[{}] Factory Cache Reloaded", chain_id);
            }
        }
        Ok(())
    });

    tokio::select! {
        Ok(task) = task_handle => {task?}
        _ = shutdown_rx.recv() => {}
    }

    Ok(())
}
