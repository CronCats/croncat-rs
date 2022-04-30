
use cosmos_sdk_proto::cosmwasm::wasm::v1::msg_client::MsgClient;
use tonic::transport::Channel;

use crate::logging::info;
use crate::errors::Report;

pub async fn connect() -> Result<MsgClient<Channel>, Report> {
  let url = "http://[::1]:50051";
  let client = MsgClient::connect(url).await?;
  info!("Connected to GRPC server @ {}", url);
  Ok(client)
}