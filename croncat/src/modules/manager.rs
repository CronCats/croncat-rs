use cosm_orc::orchestrator::ChainResponse;
use croncat_sdk_manager::msg::ManagerExecuteMsg;
use crate::{
    errors::Report,
    rpc::RpcClientService,
};

pub struct Manager {
    pub client: RpcClientService,
    pub contract_addr: String,
}

impl Manager {
    pub async fn new(
        contract_addr: String,
        client: RpcClientService,
    ) -> Result<Self, Report> {
        Ok(Self {
            client,
            contract_addr,
        })
    }

    pub async fn proxy_call(&self, task_hash: Option<String>) -> Result<ChainResponse, Report> {
        self.client.execute(|signer| {
            let task_hash = task_hash.clone();
            async move {
                signer
                .execute_croncat(ManagerExecuteMsg::ProxyCall { task_hash })
                .await
            }
        })
        .await
    }

    pub async fn withdraw_reward(&self) -> Result<ChainResponse, Report> {
        self.client.execute(|signer| {
            async move {
                signer
                .execute_croncat(ManagerExecuteMsg::AgentWithdraw(None))
                .await
            }
        })
        .await
    }
}
