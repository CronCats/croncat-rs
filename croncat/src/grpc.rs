//!
//! Use the [cosmos_sdk_proto](https://crates.io/crates/cosmos-sdk-proto) library to create clients for GRPC node requests.
//!

use cosmos_sdk_proto::cosmwasm::wasm::v1::msg_client::MsgClient;
use cosmos_sdk_proto::cosmwasm::wasm::v1::query_client::QueryClient;
use tonic::transport::Channel;
use url::Url;

use crate::errors::Report;
use crate::logging::info;

///
/// Create message and query clients for interacting with the chain.
///
#[no_coverage]
pub async fn connect(url: String) -> Result<(MsgClient<Channel>, QueryClient<Channel>), Report> {
    // Parse url
    let url = Url::parse(&url)?;

    info!("Connecting to GRPC services @ {}", url);

    // Setup our GRPC clients
    let msg_client = MsgClient::connect(url.to_string()).await?;
    let query_client = QueryClient::connect(url.to_string()).await?;

    info!("Connected to GRPC services @ {}", url);

    Ok((msg_client, query_client))
}
