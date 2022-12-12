//!
//! Create a stream that can process incoming blocks and count how many its seen,
//! then use the count to check the account statuses of the agent.
//!

use std::sync::Arc;
use cosmos_sdk_proto::cosmos::bank::v1beta1::{QueryAllBalancesRequest, QueryAllBalancesResponse};
use prost::{DecodeError, Message};
use tendermint::abci::Path;
use tendermint_rpc::endpoint::abci_query::AbciQuery;
use tendermint_rpc::{Client, HttpClient, Url};
use color_eyre::{eyre::eyre, Report};
use cw_croncat_core::types::AgentStatus;
use tokio::sync::Mutex;
use tracing::info;

use crate::{
    channels::{BlockStreamRx, ShutdownRx},
    grpc::GrpcSigner,
    utils::AtomicIntervalCounter,
};

///
/// Check every nth block with [`AtomicIntervalCounter`] for the current account
/// status of each account the agent watches.
///
pub async fn check_account_status_loop(
    mut block_stream_rx: BlockStreamRx,
    mut shutdown_rx: ShutdownRx,
    block_status: Arc<Mutex<AgentStatus>>,
    signer: GrpcSigner,
) -> Result<(), Report> {
    let block_counter = AtomicIntervalCounter::new(10);
    let task_handle: tokio::task::JoinHandle<Result<(), Report>> = tokio::task::spawn(async move {
        while let Ok(block) = block_stream_rx.recv().await {
            block_counter.tick();
            if block_counter.is_at_interval() {
                info!(
                    "Checking agents statuses for block (height: {})",
                    block.header.height
                );
                let account_addr = signer.account_id().as_ref();
                let agent = signer.get_agent(account_addr).await?;
                let mut locked_status = block_status.lock().await;
                *locked_status = agent
                    .ok_or(eyre!("Agent unregistered during the loop"))?
                    .status;
                info!("Agent status: {:?}", *locked_status);
                if *locked_status == AgentStatus::Nominated {
                    info!("Checking in agent: {}", signer.check_in_agent().await?.log);
                    let agent = signer.get_agent(account_addr).await?;
                    *locked_status = agent
                        .ok_or(eyre!("Agent unregistered during the loop"))?
                        .status;
                    info!("Agent status: {:?}", *locked_status);
                }
                
                // Check the agent's balance to make sure it's not falling below a threshold
                let msg_request = QueryAllBalancesRequest {
                    address: account_addr.to_string(),
                    pagination: None
                };
                println!("aloha msg_request {}", msg_request.address);
                let encoded_msg_request = Message::encode_to_vec(&msg_request);

                let rpc_address = signer.client.cfg.rpc_endpoint.clone();
                let node_address: Url = rpc_address.parse()?;
                let rpc_client = HttpClient::new(node_address).map_err(|err| {
                    eyre!(
                        "Could not get http client for RPC node for polling: {}",
                        err.detail()
                    )
                })?;
                let agent_balance: AbciQuery = rpc_client.abci_query(Some(Path::from("/cosmos.bank.v1beta1.Query/AllBalances".parse()?)), encoded_msg_request, None, false).await?;
                let msg_response: Result<QueryAllBalancesResponse, DecodeError> =  Message::decode(&*agent_balance.value);
                if msg_response.is_err() {
                    // Eventually pipe a good error to whatever APM we choose
                    println!("Error: unexpected result when querying the balance of the agent. Moving on…");
                    continue;
                }

                let denom = signer.client.cfg.denom.clone();
                println!("aloha denom {}", denom);
                let agent_native_balance = msg_response
                  .unwrap()
                  .balances
                  .into_iter()
                  .find(|c| c.denom == denom)
                  .unwrap()
                  .amount
                  .parse::<u128>()
                  .unwrap();

                println!("aloha agent_native_balance {}", agent_native_balance);
                // TODO: Here's where we can use the balance to determine
                //   if we should call withdraw_reward in grpc.rs
            }
        }
        Ok(())
    });

    tokio::select! {
        Ok(task) = task_handle => {task?}
        _ = shutdown_rx.recv() => {}
    }

    Ok(())
}
