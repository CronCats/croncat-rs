//!
//! This module contains the code for querying the croncat contract via HTTP RPC.
//!

use std::time::Duration;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::time::timeout;

use crate::config::ChainConfig;
use crate::errors::{eyre, Report};
use crate::utils::normalize_rpc_url;

use super::RpcClient;

pub struct Querier {
    pub rpc_client: RpcClient,
    pub croncat_addr: String,
}

impl std::fmt::Debug for Querier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Querier")
            .field("croncat_addr", &self.croncat_addr)
            .finish()
    }
}

impl Querier {
    pub async fn new(cfg: ChainConfig, rpc_url: String) -> Result<Self, Report> {
        let rpc_url = normalize_rpc_url(&rpc_url);

        let rpc_client = RpcClient::new(&cfg, &rpc_url)?;

        Ok(Self {
            rpc_client,
            croncat_addr: cfg.factory,
        })
    }

    pub async fn query_croncat<S, T>(&self, msg: S) -> Result<T, Report>
    where
        S: Serialize,
        T: DeserializeOwned,
    {
        timeout(
            Duration::from_secs_f64(self.rpc_client.timeout_secs),
            self.rpc_client.wasm_query(msg),
        )
        .await
        .map_err(|err| {
            eyre!(
                "Timeout ({}s) while querying contract: {}",
                self.rpc_client.timeout_secs,
                err
            )
        })?
    }
}

impl std::fmt::Debug for RpcClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcClient")
            .field("contract_addr", &self.contract_addr)
            .field("client", &self.client)
            .finish()
    }
}
