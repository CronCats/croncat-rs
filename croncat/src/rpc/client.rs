use std::collections::HashMap;
use std::str::FromStr;

use color_eyre::{eyre::eyre, Report};
use cosm_orc::config::cfg::Config as CosmOrcConfig;
use cosm_orc::config::ChainConfig as CosmOrcChainConfig;
use cosm_orc::orchestrator::{
    cosm_orc::CosmOrc, deploy::DeployInfo, Address, Denom, SigningKey, TendermintRPC,
};
use cosm_orc::orchestrator::{ChainResponse, Coin, Key};
use cosm_tome::chain::request::TxOptions;
use cosm_tome::modules::cosmwasm::model::ExecRequest;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::config::ChainConfig;

/// An RPC client for querying the croncat contract.
#[derive(Clone)]
pub struct RpcClient {
    pub(crate) client: CosmOrc<TendermintRPC>,
    pub(crate) contract_addr: Address,
    key: Option<SigningKey>,
    denom: Option<Denom>,
}

impl RpcClient {
    /// Create a new [`RpcClient`].
    pub fn new(cfg: &ChainConfig, rpc_url: &str) -> Result<Self, Report> {
        // Build the contract info map.
        let mut contract_deploy_info = HashMap::new();
        contract_deploy_info.insert(
            "croncat-manager".to_string(),
            DeployInfo {
                code_id: None,
                address: Some(cfg.manager.clone()),
            },
        );

        // Convert our config into a CosmOrc config with the specified rpc url.
        let config = CosmOrcConfig {
            chain_cfg: CosmOrcChainConfig {
                denom: cfg.info.fees.fee_tokens[0].denom.clone(),
                prefix: cfg.info.bech32_prefix.clone(),
                chain_id: cfg.info.chain_id.clone(),
                rpc_endpoint: Some(rpc_url.to_string()),
                grpc_endpoint: None,
                gas_prices: cfg.gas_prices as f64,
                gas_adjustment: cfg.gas_adjustment as f64,
            },
            contract_deploy_info,
        };
        let contract_addr = cfg.manager.parse::<Address>()?;

        Ok(Self {
            client: CosmOrc::new_tendermint_rpc(config, true)?,
            contract_addr,
            key: None,
            denom: None,
        })
    }

    /// Set the signing key for this client.
    pub fn set_mnemonic(&mut self, mnemonic: String) {
        self.key = Some(SigningKey {
            name: "".to_string(),
            key: Key::Mnemonic(mnemonic),
        });
    }

    /// Set the denom for this client.
    pub fn set_denom(&mut self, denom: &str) {
        self.denom = Some(Denom::from_str(denom).unwrap());
    }

    /// Query the contract via RPC at a specific address.
    pub async fn call_wasm_query<S, R>(&self, address: Address, msg: S) -> Result<R, Report>
    where
        S: Serialize,
        R: DeserializeOwned,
    {
        // Query the chain
        let response = self.client.client.wasm_query(address, &msg).await?;

        // Deserialize the response
        let data = response
            .data()
            .map_err(|e| eyre!("Failed to deserialize response data: {}", e))?;

        Ok(data)
    }

    /// Query the contract at the manager address.
    pub async fn wasm_query<S, R>(&self, msg: S) -> Result<R, Report>
    where
        S: Serialize,
        R: DeserializeOwned,
    {
        self.call_wasm_query(self.contract_addr.clone(), msg).await
    }

    /// Execute a contract via RPC.
    pub async fn wasm_execute<S>(&self, msg: S) -> Result<ChainResponse, Report>
    where
        S: Serialize,
    {
        if self.key.is_none() {
            return Err(eyre!("No signing key set"));
        }

        // Query the chain
        let response = self
            .client
            .client
            .wasm_execute(
                ExecRequest {
                    address: self.contract_addr.clone(),
                    msg: &msg,
                    funds: vec![],
                },
                self.key.as_ref().unwrap(),
                &TxOptions::default(),
            )
            .await?;

        // Get the response data
        let data = response.res;

        Ok(data.res)
    }

    /// Query the balance of an address.
    /// Returns the balance in the denom set for this client.
    pub async fn query_balance(&self, address: &str) -> Result<Coin, Report> {
        if self.denom.is_none() {
            return Err(eyre!("No denom set"));
        }

        let address = address.parse::<Address>()?;
        let balance = self
            .client
            .client
            .bank_query_balance(address, self.denom.as_ref().unwrap().clone())
            .await?;

        Ok(balance.balance)
    }
}
