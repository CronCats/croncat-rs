use color_eyre::{eyre::eyre, Report};
use cosmos_chain_registry::ChainInfo;
use cosmos_sdk_proto::cosmos::tx::v1beta1::service_client::ServiceClient;

use cosmrs::bip32;
use cosmrs::crypto::secp256k1::SigningKey;
use cosmrs::rpc::HttpClient;

use serde::Serialize;
use tendermint_rpc::endpoint::broadcast::tx_commit::Response;
use tonic::transport::Channel;

use super::auth_query::QueryBaseAccount;
use super::query_client::CosmosQueryClient;
use super::wasm_execute::{
    generate_wasm_body, prepare_send, prepare_simulate_tx, send_tx, simulate_gas_fee,
};

#[derive(Clone)]
pub struct CosmosFullClient {
    pub(crate) chain_info: ChainInfo,
    pub(crate) key: bip32::XPrv,
    pub(crate) native_denom: String,
    pub(crate) gas_prices: f32,
    pub(crate) gas_adjustment: f32,
    http_client: HttpClient,
    service_client: ServiceClient<Channel>,
    pub(crate) query_client: CosmosQueryClient,
}

impl CosmosFullClient {
    pub async fn new(
        rpc_url: String,
        grpc_url: String,
        chain_info: ChainInfo,
        key: bip32::XPrv,
        gas_prices: f32,
        gas_adjustment: f32,
    ) -> Result<Self, Report> {
        let native_denom = chain_info.fees.fee_tokens[0].denom.clone();
        let http_client = HttpClient::new(rpc_url.as_str())
            .map_err(|err| eyre!("Failed to create http client: {}", err))?;
        let service_client = ServiceClient::connect(grpc_url.clone())
            .await
            .map_err(|err| eyre!("Failed to create GRPC service client: {}", err))?;
        let query_client = CosmosQueryClient::new(&grpc_url, &native_denom)
            .await
            .map_err(|err| eyre!("Failed to create GRPC query client: {}", err))?;

        Ok(Self {
            chain_info,
            key,
            native_denom,
            http_client,
            service_client,
            query_client,
            gas_prices,
            gas_adjustment,
        })
    }

    pub async fn execute_wasm(
        &self,
        msg: &impl Serialize,
        contract_name: &str,
    ) -> Result<Response, Report> {
        let sender = self
            .key()
            .public_key()
            .account_id(&self.chain_info.bech32_prefix)?;
        let tx_body = generate_wasm_body(sender.as_ref(), contract_name, msg)?;
        let base_account = self
            .query_client
            .query_base_account(sender.as_ref().to_owned())
            .await?;
        let simulate_tx_raw =
            prepare_simulate_tx(&tx_body, &self.chain_info, &self.key(), &base_account)?;
        let fee = simulate_gas_fee(
            self.service_client.clone(),
            simulate_tx_raw,
            &self.native_denom,
            self.gas_prices,
            self.gas_adjustment,
        )
        .await?;
        let raw = prepare_send(&tx_body, &self.chain_info, &self.key(), &base_account, fee)?;
        let tx_result = send_tx(&self.http_client, raw).await?;
        Ok(tx_result)
    }

    pub fn key(&self) -> SigningKey {
        (&self.key).try_into().unwrap()
    }
}
