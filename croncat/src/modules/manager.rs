use crate::{errors::Report, rpc::RpcClientService};
use cosm_orc::orchestrator::{Address, ChainTxResponse};
use cosm_tome::modules::cosmwasm::model::ExecRequest;
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

    pub async fn proxy_call(&self, task_hash: Option<String>) -> Result<ChainTxResponse, Report> {
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

    // Generates batch of proxy_calls for executing a known batch without evented tasks
    pub async fn proxy_call_batch(&self, count: usize) -> Result<ChainTxResponse, Report> {
        self.client
            .execute(|signer| {
                let mut proxy_calls: Vec<Option<String>> = Vec::with_capacity(count);
                for _ in 0..count {
                    proxy_calls.push(None);
                }
                let reqs = vec![ExecRequest {
                    address: self.contract_addr.clone(),
                    msg: ManagerExecuteMsg::ProxyBatch(proxy_calls),
                    funds: vec![],
                }];
                async move { signer.execute_batch(reqs).await }
            })
            .await
    }

    // Generates batch of proxy_calls for executing a known batch without evented tasks
    pub async fn proxy_call_evented_batch(
        &self,
        tash_hashes: Vec<String>,
    ) -> Result<ChainTxResponse, Report> {
        self.client
            .execute(|signer| {
                let mut proxy_calls: Vec<Option<String>> = Vec::with_capacity(tash_hashes.len());
                for task_hash in tash_hashes.iter() {
                    proxy_calls.push(Some(task_hash.to_owned()));
                }
                let reqs = vec![ExecRequest {
                    address: self.contract_addr.clone(),
                    msg: ManagerExecuteMsg::ProxyBatch(proxy_calls),
                    funds: vec![],
                }];
                async move { signer.execute_batch(reqs).await }
            })
            .await
    }

    pub async fn withdraw_reward(&self) -> Result<ChainTxResponse, Report> {
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
