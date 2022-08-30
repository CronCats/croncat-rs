//!
//! How to process and consume blocks from the chain.
//!

use crate::{
    channels::{BlockStreamRx, ShutdownRx},
    errors::Report,
    grpc::OrcSigner,
    store::agent::LocalAgentStorage,
};

use crate::logging::{info, warn};

use cosmos_sdk_proto::cosmwasm::wasm::v1::query_client::QueryClient;
use cosmos_sdk_proto::cosmwasm::wasm::v1::{msg_client::MsgClient, QuerySmartContractStateRequest};
use cosmrs::AccountId;
use cw_croncat_core::types::AgentStatus;
use tonic::transport::Channel;
///
/// Do work on blocks that are sent from the ws stream.
///
pub async fn tasks_loop(
    mut block_stream_rx: BlockStreamRx,
    mut shutdown_rx: ShutdownRx,
) -> Result<(), Report> {
    let block_consumer_stream =
        tokio::task::spawn(async move { while let Ok(_block) = block_stream_rx.recv().await {} });

    tokio::select! {
        _ = block_consumer_stream => {}
        _ = shutdown_rx.recv() => {}
    }

    Ok(())
}

pub async fn do_task_if_any(
    mut block_stream_rx: BlockStreamRx,
    mut shutdown_rx: ShutdownRx,
    croncat_addr: String,
    account_id: String,
    storage: &LocalAgentStorage,
) -> Result<(), Report> {
    let key = storage.get_agent_signing_key(&account_id).unwrap();
    let account_id = key.to_account("juno").unwrap();
    let block_consumer_stream = tokio::task::spawn(async move {
        while let Ok(_) = block_stream_rx.recv().await {
            let croncat_addr = croncat_addr.clone();
            let key = key.clone();
            let mut signer = OrcSigner::new(&croncat_addr, key).unwrap();

            let account_addr = account_id.to_string();
            tokio::task::spawn_blocking(move || {
                let agent_active = signer
                    .get_agent(account_addr.clone())
                    .unwrap()
                    .map_or(false, |ag| ag.status == AgentStatus::Active);
                if agent_active {
                    let tasks = signer.get_agent_tasks_raw(account_addr.clone()).unwrap();
                    if tasks.is_some() {
                        if let Ok(proxy_call_res) = signer.proxy_call() {
                            info!("Finished task: {}", proxy_call_res.log);
                        } else {
                            warn!("Refill agent's balance: agent_addr: {account_addr}");
                        }
                    } else {
                        info!("no tasks for this block");
                    }
                } else {
                    warn!("agent is not registered");
                }
            });
        }
    });

    tokio::select! {
        _ = block_consumer_stream => {}
        _ = shutdown_rx.recv() => {}
    }

    Ok(())
}
