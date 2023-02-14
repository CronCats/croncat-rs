use croncat_sdk_factory::msg::{Config, FactoryQueryMsg};
use crate::config::ChainConfig;
use crate::{
    errors::{Report},
    rpc::{Querier, Signer},
};


pub struct Factory {
    querier: Querier,
    signer: Signer,
    pub contract_addr: String,
}

impl Factory {
    pub async fn new(cfg: ChainConfig, contract_addr: String, signer: Signer, querier: Querier) -> Result<Self, Report> {
        Ok(Self {
            querier,
            signer,
            contract_addr,
        })
    }

    pub async fn query_config(&self) -> Result<String, Report> {
        let config: Config = self.querier.query_croncat(FactoryQueryMsg::Config {}).await?;
        let json = serde_json::to_string_pretty(&config)?;
        Ok(json)
    }
}
