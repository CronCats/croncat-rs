use async_trait::async_trait;
use color_eyre::Report;
use cosm_orc::orchestrator::cosm_orc::CosmOrc;
use cosm_tome::clients::tendermint_rpc::TendermintRPC;
use cosmos_sdk_proto::cosmwasm::wasm::v1::query_client::QueryClient;
use cosmos_sdk_proto::cosmwasm::wasm::v1::QuerySmartContractStateRequest;
use serde::de::DeserializeOwned;
use tonic::transport::Channel;

pub trait GetWasmQueryClient {
    fn wasm_query_client(&self) -> QueryClient<Channel>;
    fn tm_wasm_query_client(&self) -> CosmOrc<TendermintRPC>;
}

#[async_trait]
pub trait QuerySmartContract<T> {
    async fn query_wasm_smart(&self, croncat_addr: String, msg: Vec<u8>) -> Result<T, Report>;
}

#[async_trait]
impl<C, T> QuerySmartContract<T> for C
where
    C: GetWasmQueryClient + std::marker::Sync,
    T: DeserializeOwned,
{
    async fn query_wasm_smart(&self, croncat_addr: String, msg: Vec<u8>) -> Result<T, Report> {
        let mut client = self.wasm_query_client();
        let request = QuerySmartContractStateRequest {
            address: croncat_addr,
            query_data: msg,
        };
        let res = client.smart_contract_state(request).await?;
        let data = serde_json::from_slice(&res.into_inner().data)?;
        Ok(data)
    }
}
