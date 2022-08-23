//!
//! Use the [cosmos_sdk_proto](https://crates.io/crates/cosmos-sdk-proto) library to create clients for GRPC node requests.
//!
use cosm_orc::client::error::ClientError;
use cosm_orc::client::ChainResponse;
use cosm_orc::config::cfg::Config;
use cosm_orc::config::key::SigningKey;
use cosm_orc::orchestrator::cosm_orc::CosmOrc;
use cosm_orc::orchestrator::error::ProcessError;
use cosm_orc::profilers::gas_profiler::GasProfiler;
use cosmos_sdk_proto::cosmos::auth::v1beta1::query_client::QueryClient as AuthQueryClient;
use cosmos_sdk_proto::cosmos::auth::v1beta1::{BaseAccount, QueryAccountRequest};
use cosmos_sdk_proto::cosmwasm::wasm::v1::msg_client::MsgClient;
use cosmos_sdk_proto::cosmwasm::wasm::v1::query_client::QueryClient;
use cosmwasm_std::Addr;
use cw_croncat_core::msg::ExecuteMsg;
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

const AGENT_REGISTER_OPERTATION: &str = "register_agent";
const AGENT_UNREGISTER_OPERTATION: &str = "unregister_agent";
const CONFIG_FILE: &str = "config.yaml";

pub async fn register_agent(
    address: String,
    payable_account_id: String,
    key: &SigningKey,
) -> Result<ChainResponse, ProcessError> {
    let mut cosm_orc = CosmOrc::new(Config::from_yaml(CONFIG_FILE).unwrap())
        .unwrap()
        .add_profiler(Box::new(GasProfiler::new()));

    let result = cosm_orc.execute::<String, ExecuteMsg>(
        address,
        AGENT_REGISTER_OPERTATION.to_string(),
        &ExecuteMsg::RegisterAgent {
            payable_account_id: Some(Addr::unchecked(payable_account_id)),
        },
        key,
    );
    result
}

pub async fn unregister_agent(
    address: String,
    key: &SigningKey,
) -> Result<ChainResponse, ProcessError> {
    let mut cosm_orc = CosmOrc::new(Config::from_yaml(CONFIG_FILE).unwrap())
        .unwrap()
        .add_profiler(Box::new(GasProfiler::new()));

    let result = cosm_orc.execute::<String, ExecuteMsg>(
        address,
        AGENT_UNREGISTER_OPERTATION.to_string(),
        &ExecuteMsg::UnregisterAgent {},
        key,
    );
    result
}
