use crate::{errors::Report, rpc::RpcClientService};
use cosm_orc::orchestrator::{Address, ChainResponse};
use croncat_sdk_manager::msg::ManagerExecuteMsg;

pub struct Manager {
    pub client: RpcClientService,
    pub contract_addr: Address,
}

impl Manager {
    pub async fn new(contract_addr: Address, client: RpcClientService) -> Result<Self, Report> {
        Ok(Self {
            client,
            contract_addr,
        })
    }

    pub async fn proxy_call(&self, task_hash: Option<String>) -> Result<ChainResponse, Report> {
        self.client
            .execute(|signer| {
                let task_hash = task_hash.clone();
                let contract_addr = self.contract_addr.clone();
                async move {
                    signer
                        .execute_croncat(
                            ManagerExecuteMsg::ProxyCall { task_hash },
                            Some(contract_addr),
                        )
                        .await
                }
            })
            .await
    }

    pub async fn withdraw_reward(&self) -> Result<ChainResponse, Report> {
        self.client
            .execute(|signer| {
                let contract_addr = self.contract_addr.clone();
                async move {
                    signer
                        .execute_croncat(
                            ManagerExecuteMsg::AgentWithdraw(None),
                            Some(contract_addr),
                        )
                        .await
                }
            })
            .await
    }
}
