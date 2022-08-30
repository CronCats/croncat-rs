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
use cw_croncat_core::msg::AgentTaskResponse;
use cw_croncat_core::msg::TaskResponse;
use cw_croncat_core::msg::{ExecuteMsg, GetConfigResponse, QueryMsg};
use cw_croncat_core::types::AgentResponse;
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
const AGENT_PROXY_CALL: &str = "proxy_call";
const CRONCAT_CONFIG_QUERY: &str = "config";
const CRONCAT_AGENT_QUERY: &str = "query_get_agent";
const CRONCAT_QUERY_TASKS: &str = "query_get_tasks";
const CRONCAT_QUERY_AGENT_TASKS: &str = "query_get_agent_tasks";

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

    pub fn get_agent(&mut self, acount_addr: String) -> Result<Option<AgentResponse>, Report> {
        let res = self.cosm_orc.query(
            "croncat",
            CRONCAT_AGENT_QUERY,
            &QueryMsg::GetAgent {
                account_id: Addr::unchecked(acount_addr),
            },
        )?;
        let response: Option<AgentResponse> = res.data()?;
        Ok(response)
    }

    pub fn get_agent_tasks_raw(
        &mut self,
        account_addr: String,
    ) -> Result<Option<AgentTaskResponse>, Report> {
        let res = self.cosm_orc.query(
            "croncat",
            CRONCAT_QUERY_AGENT_TASKS,
            &QueryMsg::GetAgentTasks {
                account_id: Addr::unchecked(account_addr),
            },
        )?;
        let response: Option<AgentTaskResponse> = res.data()?;
        Ok(response)
    }

    pub fn proxy_call(&mut self) -> Result<ChainResponse, Report> {
        let res = self.cosm_orc.execute(
            "croncat",
            AGENT_PROXY_CALL,
            &ExecuteMsg::ProxyCall {},
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
    pub fn get_agent(&mut self, account_id: String) -> Result<String, Report> {
        let res = self.cosm_orc.query(
            "croncat",
            CRONCAT_AGENT_QUERY,
            &QueryMsg::GetAgent {
                account_id: Addr::unchecked(account_id),
            },
        )?;
        let response: AgentResponse = res.data()?;
        let json = serde_json::to_string_pretty(&response)?;
        Ok(json)
    }
    pub fn get_tasks(
        &mut self,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Result<String, Report> {
        let res = self.cosm_orc.query(
            "croncat",
            CRONCAT_QUERY_TASKS,
            &QueryMsg::GetTasks { from_index, limit },
        )?;
        let response: Vec<TaskResponse> = res.data()?;
        let json = serde_json::to_string_pretty(&response)?;
        Ok(json)
    }
    pub fn get_agent_tasks(&mut self, account_id: String) -> Result<String, Report> {
        let res = self.cosm_orc.query(
            "croncat",
            CRONCAT_QUERY_AGENT_TASKS,
            &QueryMsg::GetAgentTasks {
                account_id: Addr::unchecked(account_id),
            },
        )?;
        let response: AgentTaskResponse = res.data()?;
        let json = serde_json::to_string_pretty(&response)?;
        Ok(json)
    }
}
