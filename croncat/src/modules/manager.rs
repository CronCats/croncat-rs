use cosm_orc::orchestrator::ChainResponse;
use croncat_sdk_manager::msg::ManagerExecuteMsg;

use crate::config::ChainConfig;
use crate::{
    errors::Report,
    rpc::{Querier, Signer},
};

pub struct Manager {
    querier: Querier,
    signer: Signer,
    pub contract_addr: String,
}

impl Manager {
    pub async fn new(
        cfg: ChainConfig,
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

    pub async fn proxy_call(&self, task_hash: Option<String>) -> Result<ChainResponse, Report> {
        self.signer
            .execute_croncat(ManagerExecuteMsg::ProxyCall { task_hash })
            .await
    }
}
