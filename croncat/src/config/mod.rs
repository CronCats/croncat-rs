/// TODO: Move to chain registry
/// Right now juno testnet missing grpc's, so we keeping it like `cosm-orc`'s chain config
use color_eyre::Report;
use config::Config;
use serde::{Deserialize, Serialize};
use std::path::Path;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChainConfig {
    pub denom: String,
    pub prefix: String,
    pub chain_id: String,
    pub rpc_endpoint: String,
    pub grpc_endpoint: String,
    pub wsrpc_endpoint: String,
    pub contract_address: String,
    pub gas_prices: f64,
    pub gas_adjustment: f64,
}

impl ChainConfig {
    pub fn is_chain_registry_enabled() -> bool {
        true
    }
    pub async fn new(chain_id: Option<&str>) -> Result<Self, Report> {
        let mut ch_id=chain_id.unwrap();
        if !chain_id.is_some() {
            ch_id = "uni-3";
        }
        let config_file = &format!("config.{}.yaml", ch_id);
        let config_override_file = &format!("config.{}.override.yaml", ch_id);
        if Path::new(config_override_file).is_file() {
            return Self::from_file(config_override_file);
        }
        let config = Self::from_file(config_file)?;
        // if Self::is_chain_registry_enabled() {
        //     return Ok(Self::from_chain_registry(config).await?);
        // }
        return Ok(config);
    }
    pub fn from_file(file_name: &str) -> Result<Self, Report> {
        let settings = Config::builder()
            .add_source(config::File::with_name(file_name))
            .build()?;

        let config = settings.try_deserialize::<ChainConfig>()?;
        Ok(config)
    }
    pub async fn from_chain_registry(fallback: ChainConfig) -> Result<Self, Report> {
        let result = chain_registry::get::get_chain("juno").await?;
        let apis = result.unwrap().apis;

        let config = ChainConfig {
            rpc_endpoint: apis.rpc[0].address.clone(),
            grpc_endpoint: apis.grpc[0].address.clone(),
            ..fallback
        };

        Ok(config)
    }
}
