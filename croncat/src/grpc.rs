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
use cw_croncat_core::msg::{ExecuteMsg, GetConfigResponse, QueryMsg};
use tonic::transport::Channel;
use url::Url;

use crate::errors::Report;
use crate::logging::info;
use crate::utils::setup_cosm_orc;

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
const AGENT_UPDATE_AGENT_OPERATION: &str = "update_agent";
const AGENT_WITHDRAW_OPERATION: &str = "withdraw";
const CRONCAT_CONFIG_QUERY: &str = "config";

pub struct OrcSigner {
    cosm_orc: CosmOrc,
    key: SigningKey,
}

impl OrcSigner {
    pub fn new(croncat_addr: &str, key: SigningKey) -> Result<Self, Report> {
        let cosm_orc = setup_cosm_orc(croncat_addr)?;
        Ok(Self { cosm_orc, key })
    }
    pub fn register_agent(
        &mut self,
        payable_account_id: Option<String>,
    ) -> Result<ChainResponse, Report> {
        let res = self.cosm_orc.execute::<String, ExecuteMsg>(
            "croncat".to_string(),
            AGENT_REGISTER_OPERTATION.to_string(),
            &ExecuteMsg::RegisterAgent {
                payable_account_id: payable_account_id.map(|id| Addr::unchecked(id)),
            },
            &self.key,
        )?;

        Ok(res)
    }

    pub fn unregister_agent(&mut self) -> Result<ChainResponse, Report> {
        let res = self.cosm_orc.execute::<String, ExecuteMsg>(
            "croncat".to_string(),
            AGENT_UNREGISTER_OPERTATION.to_string(),
            &ExecuteMsg::UnregisterAgent {},
            &self.key,
        )?;
        Ok(res)
    }

    pub fn update_agent(&mut self, payable_account_id: String) -> Result<ChainResponse, Report> {
        let payable_account_id = Addr::unchecked(payable_account_id);
        let res = self.cosm_orc.execute(
            "croncat".to_string(),
            AGENT_UPDATE_AGENT_OPERATION.to_string(),
            &ExecuteMsg::UpdateAgent { payable_account_id },
            &self.key,
        )?;
        Ok(res)
    }

    pub fn withdraw_reward(&mut self) -> Result<ChainResponse, Report> {
        let res = self.cosm_orc.execute(
            "croncat".to_string(),
            AGENT_WITHDRAW_OPERATION.to_string(),
            &ExecuteMsg::WithdrawReward {},
            &self.key,
        )?;
        Ok(res)
    }
}

pub struct OrcQuerier {
    cosm_orc: CosmOrc,
}
impl OrcQuerier {
    pub fn new(croncat_addr: &str) -> Result<Self, Report> {
        let cosm_orc = setup_cosm_orc(croncat_addr)?;
        Ok(Self { cosm_orc })
    }

    pub fn query_config(&mut self) -> Result<String, Report> {
        let res = self
            .cosm_orc
            .query("croncat", CRONCAT_CONFIG_QUERY, &QueryMsg::GetConfig {})?;
        let config: GetConfigResponse = res.data()?;
        let config_json = serde_json::to_string_pretty(&config)?;
        Ok(config_json)
    }
}
