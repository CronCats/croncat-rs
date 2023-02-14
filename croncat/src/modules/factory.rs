use std::collections::HashMap;
use crate::config::ChainConfig;
use color_eyre::{eyre::eyre, Report};
use crate::{
    rpc::{Querier, Signer},
    store::factory::LocalCacheStorage,
};
use croncat_sdk_factory::msg::{
    ContractMetadataInfo, ContractMetadataResponse, EntryResponse, FactoryQueryMsg,
};

pub struct Factory {
    pub querier: Querier,
    pub signer: Signer,
    pub contract_addr: String,
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
        signer: Signer,
        querier: Querier,
    ) -> Result<Self, Report> {
        Ok(Self {
            querier,
            signer,
            contract_addr: cfg.factory,
            store: LocalCacheStorage::default(),
        })
    }

    // load versions: get latest & all versions, put into storage
    pub async fn load(&mut self) -> Result<bool, Report> {
      let b = if let Some(data) = self.store.get() {
        // Have the unexpired cache data, wooooot!
        true
      } else {
        // Go get latest version data
        let mut latest = HashMap::new();
        let mut versions = HashMap::new();
        let entries = self.get_latest_contracts().await?;

        // NOTE: This doesnt handle ALL versions, just latest
        for entry in entries {
            latest.insert(entry.contract_name.clone(), entry.metadata.version);
            versions.insert((entry.contract_name, entry.metadata.version), entry.metadata);
        }

        // update storage
        self.store.insert(Some(latest), Some(versions));

        true
      };

      // only need to make sure we loaded y'all
      Ok(b)
    }

    // get contract addr for contract_name, by version or default latest
    pub async fn get_contract_addr(&self, contract_name: String) -> Result<String, Report> {
      let err = "No version found for {contract_name}";
      if let Some(data) = self.store.get() {
        let version = data.latest.get(&contract_name).expect(err);
        let metadata = data.versions.get(&(contract_name, *version)).expect(err);
        return Ok(metadata.contract_addr.to_string());
      }
      Err(eyre!(err))
    }

    pub async fn get_latest_contracts(&self) -> Result<Vec<EntryResponse>, Report> {
        let entries: Vec<EntryResponse> = self
            .querier
            .query_croncat(FactoryQueryMsg::LatestContracts {})
            .await?;
        Ok(entries)
    }

    pub async fn get_latest_contract_by_name(
        &self,
        contract_name: String,
    ) -> Result<ContractMetadataResponse, Report> {
        let data: ContractMetadataResponse = self
            .querier
            .query_croncat(FactoryQueryMsg::LatestContract { contract_name })
            .await?;
        Ok(data)
    }

    pub async fn get_versions_by_contract_name(
        &self,
        contract_name: String,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Result<Vec<ContractMetadataInfo>, Report> {
        let from_index = Some(from_index.unwrap_or(0));
        let limit = Some(limit.unwrap_or(100));
        let entries: Vec<ContractMetadataInfo> = self
            .querier
            .query_croncat(FactoryQueryMsg::VersionsByContractName {
                contract_name,
                from_index,
                limit,
            })
            .await?;
        Ok(entries)
    }

    pub async fn get_contract_names(
        &self,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Result<Vec<String>, Report> {
        let from_index = Some(from_index.unwrap_or(0));
        let limit = Some(limit.unwrap_or(100));
        let entries: Vec<String> = self
            .querier
            .query_croncat(FactoryQueryMsg::ContractNames { from_index, limit })
            .await?;
        Ok(entries)
    }

    pub async fn get_all_versions(
        &self,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Result<Vec<EntryResponse>, Report> {
        let from_index = Some(from_index.unwrap_or(0));
        let limit = Some(limit.unwrap_or(100));
        let entries: Vec<EntryResponse> = self
            .querier
            .query_croncat(FactoryQueryMsg::AllEntries { from_index, limit })
            .await?;
        Ok(entries)
    }
}
