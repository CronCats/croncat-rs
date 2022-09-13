use color_eyre::Report;
use cosmos_sdk_proto::cosmos::tx::v1beta1::service_client::ServiceClient;

use cosmrs::bip32;
use cosmrs::crypto::secp256k1::SigningKey;
use cosmrs::rpc::HttpClient;

use serde::Serialize;
use tendermint_rpc::endpoint::broadcast::tx_commit::Response;
use tonic::transport::Channel;

use crate::config::ChainConfig;

use super::auth_query::QueryBaseAccount;
use super::query_client::CosmosQueryClient;
use super::wasm_execute::{
    generate_wasm_body, prepare_send, prepare_simulate_tx, send_tx, simulate_gas_fee,
};

#[derive(Clone)]
pub struct CosmosFullClient {
    http_client: HttpClient,
    key: bip32::XPrv,
    service_client: ServiceClient<Channel>,
    pub query_client: CosmosQueryClient,
    pub cfg: ChainConfig,
}

impl CosmosFullClient {
    pub async fn new(cfg: ChainConfig, key: bip32::XPrv) -> Result<Self, Report> {
        Ok(Self {
            http_client: HttpClient::new(cfg.rpc_endpoint.as_ref())?,
            key,
            service_client: ServiceClient::connect(cfg.grpc_endpoint.clone()).await?,
            query_client: CosmosQueryClient::new(&cfg.grpc_endpoint, &cfg.denom).await?,
            cfg,
        })
    }

    pub async fn execute_wasm(
        &self,
        msg: &impl Serialize,
        contract_name: &str,
    ) -> Result<Response, Report> {
        let sender = self.key().public_key().account_id(&self.cfg.prefix)?;
        let tx_body = generate_wasm_body(sender.as_ref(), contract_name, msg)?;
        let base_account = self
            .query_client
            .query_base_account(sender.as_ref().to_owned())
            .await?;
        let simulate_tx_raw = prepare_simulate_tx(&tx_body, &self.cfg, &self.key(), &base_account)?;
        let fee = simulate_gas_fee(self.service_client.clone(), simulate_tx_raw, &self.cfg).await?;
        let raw = prepare_send(&tx_body, &self.cfg, &self.key(), &base_account, fee)?;
        let tx_result = send_tx(&self.http_client, raw).await?;
        Ok(tx_result)
    }

    pub fn key(&self) -> SigningKey {
        (&self.key).try_into().unwrap()
    }
}
