//!
//! RPC client service that can be used to execute and query the croncat on chain.
//!

// use std::str::FromStr;
use std::time::Duration;

// use cosm_orc::orchestrator::Address;
use cosm_orc::orchestrator::ChainResponse;
use cosmrs::crypto::secp256k1::SigningKey;
use cosmrs::AccountId;
use serde::Serialize;
use tokio::time::timeout;

use crate::config::ChainConfig;
use crate::errors::{eyre, Report};
use crate::utils::normalize_rpc_url;

use super::RpcClient;

#[derive(Clone, Debug)]
pub struct Signer {
    pub rpc_client: RpcClient,
    pub contract_addr: String,
    pub account_id: AccountId,
}

impl Signer {
    pub async fn new(
        rpc_url: String,
        cfg: ChainConfig,
        contract_addr: String,
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

    // pub async fn query_croncat<R, S>(&self, msg: S) -> Result<R, Report>
    // where
    //     S: Serialize,
    //     R: DeserializeOwned,
    // {
    //     let out = timeout(
    //         Duration::from_secs_f64(self.rpc_client.timeout_secs),
    //         self.rpc_client.wasm_query(msg),
    //     )
    //     .await
    //     .map_err(|err| {
    //         eyre!(
    //             "Timeout ({}s) while querying contract: {}",
    //             self.rpc_client.timeout_secs,
    //             err
    //         )
    //     })??;

    //     Ok(out)
    // }

    pub async fn execute_croncat<S>(&self, msg: S) -> Result<ChainResponse, Report>
    where
        S: Serialize,
    {
        let res = timeout(
            Duration::from_secs_f64(self.rpc_client.timeout_secs),
            self.rpc_client.wasm_execute(msg),
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
