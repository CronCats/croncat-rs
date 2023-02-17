//!
//! RPC client service that can be used to execute and query the croncat on chain.
//!

use crate::config::ChainConfig;
use crate::errors::{eyre, Report};
use crate::utils::normalize_rpc_url;
use cosm_orc::orchestrator::{Address, ChainTxResponse};
use cosmrs::bip32;
use cosmrs::crypto::secp256k1::SigningKey;
use cosmrs::AccountId;
use serde::Serialize;
use std::time::Duration;
use tokio::time::timeout;
use super::client::{BatchMsg, RpcClient};

#[derive(Clone, Debug)]
pub struct Signer {
    pub rpc_client: RpcClient,
    pub contract_addr: Address,
    pub account_id: AccountId,
}

impl Signer {
    pub async fn new(
        rpc_url: String,
        cfg: ChainConfig,
        contract_addr: Address,
        key: bip32::XPrv,
    ) -> Result<Self, Report> {
        let rpc_url = normalize_rpc_url(&rpc_url);

        // Get the account id from the key.
        let key_bytes = key.to_bytes().to_vec();
        let signing_key: SigningKey = key.into();
        let account_id = signing_key
            .public_key()
            .account_id(&cfg.info.bech32_prefix)?;

        // Create a new RPC client
        let mut rpc_client = RpcClient::new(&cfg, rpc_url.as_str())?;
        rpc_client.set_key(key_bytes);
        rpc_client.set_denom(
            cfg.denom
                .unwrap_or_else(|| cfg.info.fees.fee_tokens[0].denom.clone())
                .as_str(),
        );

        Ok(Self {
            account_id,
            contract_addr,
            rpc_client,
        })
    }

    pub async fn execute_croncat<S>(
        &self,
        msg: S,
        address: Option<Address>,
    ) -> Result<ChainTxResponse, Report>
    where
        S: Serialize,
    {
        let a = address.unwrap_or_else(|| self.contract_addr.clone());
        let res = timeout(
            Duration::from_secs_f64(self.rpc_client.timeout_secs),
            self.rpc_client.wasm_execute(msg, Some(a)),
        )
        .await
        .map_err(|err| {
            eyre!(
                "Timeout ({}s) while executing wasm: {}",
                self.rpc_client.timeout_secs,
                err
            )
        })??;

        Ok(res)
    }

    pub async fn execute_batch(
        &self,
        msgs: Vec<BatchMsg>,
    ) -> Result<ChainTxResponse, Report> {
        let res = timeout(
            Duration::from_secs_f64(self.rpc_client.timeout_secs),
            self.rpc_client.wasm_execute_batch(msgs),
        )
        .await
        .map_err(|err| {
            eyre!(
                "Timeout ({}s) while executing wasm: {}",
                self.rpc_client.timeout_secs,
                err
            )
        })??;

        Ok(res)
    }
}
