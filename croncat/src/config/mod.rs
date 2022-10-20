//! Agent configuration.

/// TODO: Move to chain registry
/// Right now juno testnet missing grpc's, so we keeping it like `cosm-orc`'s chain config
use color_eyre::{eyre::eyre, Report};
use config::Config;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChainConfigSource {
    grpc_endpoint: String,
    rpc_endpoint: String,
    wsrpc_endpoint: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChainConfigFile {
    pub denom: String,
    pub prefix: String,
    pub chain_id: String,
    pub sources: Vec<ChainConfigSource>,
    pub contract_address: Option<String>,
    pub gas_prices: f64,
    pub gas_adjustment: f64,
    pub polling_duration_secs: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChainConfig {
    pub denom: String,
    pub prefix: String,
    pub chain_id: String,
    pub grpc_endpoint: String,
    pub rpc_endpoint: String,
    pub wsrpc_endpoint: String,
    pub contract_address: Option<String>,
    pub gas_prices: f64,
    pub gas_adjustment: f64,
    pub polling_duration_secs: u64,
}

impl ChainConfig {
    pub fn from_chain_config_file(chain_config_file: &ChainConfigFile, source_index: u64) -> Self {
        let source = &chain_config_file
            .sources
            .get(source_index as usize)
            .ok_or_else(|| eyre!("No source found for index: {}", source_index))
            .unwrap();
        ChainConfig {
            denom: chain_config_file.denom.clone(),
            prefix: chain_config_file.prefix.clone(),
            chain_id: chain_config_file.chain_id.clone(),
            grpc_endpoint: source.grpc_endpoint.clone(),
            rpc_endpoint: source.rpc_endpoint.clone(),
            wsrpc_endpoint: source.wsrpc_endpoint.clone(),
            contract_address: chain_config_file.contract_address.to_owned(),
            gas_prices: chain_config_file.gas_prices,
            gas_adjustment: chain_config_file.gas_adjustment,
            polling_duration_secs: chain_config_file.polling_duration_secs,
        }
    }
}

impl ChainConfigFile {
    pub fn is_chain_registry_enabled() -> bool {
        true
    }
    pub async fn new(chain_id: &String) -> Result<Self, Report> {
        let config_file = &format!("config.{}.yaml", chain_id);
        let config_override_file = &format!("config.{}.override.yaml", chain_id);
        if Path::new(config_override_file).is_file() {
            return Self::from_file(config_override_file);
        }
        let config = Self::from_file(config_file)?;
        // if Self::is_chain_registry_enabled() {
        //     return Ok(Self::from_chain_registry(config).await?);
        // }
        Ok(config)
    }

    pub fn from_file(file_name: &str) -> Result<Self, Report> {
        let settings = Config::builder()
            .add_source(config::File::with_name(file_name))
            .build()?;

        let mut config = settings.try_deserialize::<ChainConfigFile>()?;

        // Override config contract address if env var is set
        if let Ok(contract_address) = std::env::var("CRONCAT_CONTRACT_ADDRESS") {
            config.contract_address = Some(contract_address);
        } else if config.contract_address.is_none() {
            Err(eyre!(
                "Contract address is not set via config or environment variable"
            ))?;
        }

        Ok(config)
    }

    pub fn first(&self) -> ChainConfig {
        ChainConfig::from_chain_config_file(self, 0)
    }

    // pub async fn from_chain_registry(fallback: ChainConfig) -> Result<Self, Report> {
    //     let result = chain_registry::get::get_chain("juno").await?;
    //     let apis = result.ok_or_else(|| eyre!("No chain info found"))?.apis;

    //     let config = ChainConfig {
    //         rpc_endpoint: apis.rpc[0].address.clone(),
    //         grpc_endpoint: apis.grpc[0].address.clone(),
    //         ..fallback
    //     };

    //     Ok(config)
    // }
}
