//!
//! This module contains the code for querying the croncat contract via HTTP RPC.
//!

use crate::config::ChainConfig;
use crate::errors::{eyre, Report};
use crate::utils::normalize_rpc_url;
use cosm_orc::orchestrator::Address;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;
use tokio::time::timeout;

use super::RpcClient;

pub struct Querier {
    pub rpc_client: RpcClient,
    pub contract_addr: Address,
}

impl std::fmt::Debug for Querier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Querier")
            .field("contract_addr", &self.contract_addr)
            .finish()
    }
}

impl Querier {
    pub async fn new(
        rpc_url: String,
        cfg: ChainConfig,
        contract_addr: Address,
    ) -> Result<Self, Report> {
        let rpc_url = normalize_rpc_url(&rpc_url);

        let rpc_client = RpcClient::new(&cfg, &rpc_url)?;

        Ok(Self {
            rpc_client,
            contract_addr,
        })
    }

    pub async fn query_croncat<S, T>(&self, msg: S, address: Option<Address>) -> Result<T, Report>
    where
        S: Serialize,
        T: DeserializeOwned,
    {
        let a = address.unwrap_or_else(|| self.contract_addr.clone());
        timeout(
            Duration::from_secs_f64(self.rpc_client.timeout_secs),
            self.rpc_client.wasm_query(msg, Some(a)),
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
