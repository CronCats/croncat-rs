use crate::config::ChainConfig;
use crate::{
    errors::Report,
    rpc::{Querier, Signer},
};
use croncat_sdk_factory::msg::{
    Config, ContractMetadataInfo, ContractMetadataResponse, EntryResponse, FactoryQueryMsg,
};

pub struct Factory {
    pub querier: Querier,
    pub signer: Signer,
    pub contract_addr: String,
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
//   expires: 168242309209090,
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
        cfg: ChainConfig,
        // This MUST exist or the whole app goes booommmmmm. 
        contract_addr: String,
        signer: Signer,
        querier: Querier,
    ) -> Result<Self, Report> {
        Ok(Self {
            querier,
            signer,
            contract_addr,
        })
    }

    // TODO: load versions: get latest & all versions, put into storage
    pub async fn load(&self) -> Result<bool, Report> {
      Ok(true)
    }

    // TODO: get contract addr for contract_name, by version or default latest
    pub async fn get_contract_addr(&self, contract_name: String) -> Result<bool, Report> {
      Ok(true)
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
