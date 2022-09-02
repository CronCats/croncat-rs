use color_eyre::Report;
use cosmos_sdk_proto::cosmos::auth::v1beta1::query_client::QueryClient as AuthQueryClient;
use cosmos_sdk_proto::cosmwasm::wasm::v1::query_client::QueryClient as WasmQueryClient;
use serde::{de::DeserializeOwned, Serialize};
use tonic::transport::Channel;

use super::{
    auth_query::GetAuthQueryClient,
    bank_query::{BankQueryClient, GetBankQueryClient},
    wasm_query::{GetWasmQueryClient, QuerySmartContract},
};

#[derive(Clone)]
pub struct CosmosQueryClient {
    wasm_query_client: WasmQueryClient<Channel>,
    auth_query_client: AuthQueryClient<Channel>,
    bank_query_client: BankQueryClient,
}

impl CosmosQueryClient {
    pub async fn new(
        grpc: impl Into<String>,
        native_denom: impl Into<String>,
    ) -> Result<Self, Report> {
        let grpc = grpc.into();
        Ok(Self {
            wasm_query_client: WasmQueryClient::connect(grpc.clone()).await?,
            auth_query_client: AuthQueryClient::connect(grpc.clone()).await?,
            bank_query_client: BankQueryClient::new(grpc, native_denom.into()).await?,
        })
    }

    pub async fn query_contract<T>(
        &self,
        contract_addr: impl Into<String>,
        request: impl Serialize,
    ) -> Result<T, Report>
    where
        T: DeserializeOwned,
    {
        let msg = serde_json::to_vec(&request)?;
        let res = self.query_wasm_smart(contract_addr.into(), msg).await?;
        Ok(res)
    }
}

impl GetWasmQueryClient for CosmosQueryClient {
    fn wasm_query_client(&self) -> WasmQueryClient<Channel> {
        self.wasm_query_client.clone()
    }
}

impl GetBankQueryClient for CosmosQueryClient {
    fn bank_query_client(&self) -> BankQueryClient {
        self.bank_query_client.clone()
    }
}

impl GetAuthQueryClient for CosmosQueryClient {
    fn auth_query_client(&self) -> AuthQueryClient<Channel> {
        self.auth_query_client.clone()
    }
}
