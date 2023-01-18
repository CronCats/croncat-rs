use color_eyre::Report;
use cosm_orc::config::cfg::Config as CosmOrcConfig;
use cosm_orc::config::ChainConfig;
use cosm_orc::orchestrator::cosm_orc::CosmOrc;
use cosm_orc::orchestrator::deploy::DeployInfo;
use cosm_tome::clients::tendermint_rpc::TendermintRPC;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use tonic::transport::Channel;

use super::{
    auth_query::GetAuthQueryClient,
    bank_query::{BankQueryClient, GetBankQueryClient},
    wasm_query::{GetWasmQueryClient, QuerySmartContract},
    AuthQueryClient, WasmQueryClient,
};

#[derive(Clone, Debug)]
pub struct CosmosQueryClient {
    wasm_query_client: WasmQueryClient<Channel>,
    auth_query_client: AuthQueryClient<Channel>,
    bank_query_client: BankQueryClient,
    tm_query_client: CosmOrc<TendermintRPC>,
}

impl CosmosQueryClient {
    pub async fn new(
        grpc: impl Into<String>,
        rpc: impl Into<String>,
        native_denom: impl Into<String>,
    ) -> Result<Self, Report> {
        let grpc = grpc.into();
        let rpc = rpc.into();
        let native_denom = native_denom.into();
        let mut contract_deploy_info = HashMap::new();
        // TODO: remove super-hardcore-hardcoding
        contract_deploy_info.insert(
            "croncat-manager".to_string(),
            DeployInfo {
                code_id: Some(4239),
                address: Some(
                    "juno1gqkv06dxrccavckw8ydwaxm353pvlrtx0cgxfehvn0gjvlwjfscq58nn8w".to_string(),
                ),
            },
        );
        let config = CosmOrcConfig {
            chain_cfg: ChainConfig {
                denom: native_denom.clone(),
                prefix: "juno".to_string(),
                chain_id: "uni-5".to_string(),
                rpc_endpoint: Some(rpc.clone()),
                grpc_endpoint: Some(grpc.clone()),
                gas_prices: 0.025, // TODO: what is this, can turn to auto?
                gas_adjustment: 1.3,
            },
            contract_deploy_info,
        };
        Ok(Self {
            wasm_query_client: WasmQueryClient::connect(grpc.clone()).await?,
            auth_query_client: AuthQueryClient::connect(grpc.clone()).await?,
            bank_query_client: BankQueryClient::new(grpc, native_denom).await?,
            tm_query_client: CosmOrc::new_tendermint_rpc(config, true)?,
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

    fn tm_wasm_query_client(&self) -> CosmOrc<TendermintRPC> {
        self.tm_query_client.clone()
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
