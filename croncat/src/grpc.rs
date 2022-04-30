use cosmos_sdk_proto::cosmwasm::wasm::v1::msg_client::MsgClient;
use cosmos_sdk_proto::cosmwasm::wasm::v1::query_client::QueryClient;
use tonic::transport::Channel;

use crate::logging::info;
use crate::errors::Report;

///
/// Create message and query clients for interacting with the chain.
/// 
#[no_coverage]
pub async fn connect(url: String) -> Result<(MsgClient<Channel>, QueryClient<Channel>), Report> {
  let msg_client = MsgClient::connect(url.clone()).await?;
  let query_client = QueryClient::connect(url.clone()).await?;

  info!("Connected to GRPC services @ {}", url);
  
  Ok((msg_client, query_client))
}