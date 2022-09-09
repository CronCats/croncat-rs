use chain_registry::{cache::RegistryCache, paths::Tag};
/// TODO: Move to chain registry
/// Right now juno testnet missing grpc's, so we keeping it like `cosm-orc`'s chain config
use color_eyre::Report;
use config::Config;
use cosmos_sdk_proto::tendermint::p2p::NetAddress;
use serde::{Deserialize, Serialize};
const CONFIG_FILE: &str = "config.yaml";
const CONFIG_FILE_OVERRIDE: &str = "config.override.yaml";
const CONFIG_FILE_MAINNET: &str = "config.mainnet.yaml";
const CONFIG_FILE_MAINNET_OVERRIDE: &str = "config.mainnet.override.yaml";
use std::path::Path;

pub enum NetworkType {
    Local,
    Testnet,
    Mainnet,
}
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
    pub fn is_chain_registry_enabled()-> bool{
        true
    }
    pub async fn new(network_type: Option<NetworkType>) -> Result<Self, Report> {
        let mut network=network_type;
        if !network.is_some(){
            network=Some(NetworkType::Testnet);
        }
        match network.unwrap() {
            NetworkType::Testnet => {
                if Path::new(CONFIG_FILE_OVERRIDE).is_file() {
                    return Self::from_file(CONFIG_FILE_OVERRIDE);
                }
                let config = Self::from_file(CONFIG_FILE)?;
                return Ok(config);
            }
            NetworkType::Local => todo!(),
            NetworkType::Mainnet => {
                if Path::new(CONFIG_FILE_OVERRIDE).is_file() {
                    return Self::from_file(CONFIG_FILE_MAINNET_OVERRIDE);
                }
                let config = Self::from_file(CONFIG_FILE_MAINNET)?;
                if Self::is_chain_registry_enabled(){
                    return Ok(Self::from_chain_registry(config).await?);
                }
                return Ok(config);
            }
        }
    }
    pub fn from_file(file_name: &str) -> Result<Self, Report> {
        let settings = Config::builder()
            .add_source(config::File::with_name(file_name))
            .build()?;

        let config = settings.try_deserialize::<ChainConfig>()?;
        Ok(config)
    }
    pub async fn from_chain_registry(fallback:ChainConfig) -> Result<Self, Report> {
        let result = chain_registry::get::get_chain("juno").await?;
        let apis=result.unwrap().apis;

        let config=ChainConfig{
            rpc_endpoint:apis.rpc[0].address.clone(),
            grpc_endpoint:apis.grpc[0].address.clone(),
            ..fallback
        };
       
        Ok(config)
    }
}
