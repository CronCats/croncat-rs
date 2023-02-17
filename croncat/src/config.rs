//! Agent configuration.

use std::collections::HashMap;

use color_eyre::Result;
use cosmos_chain_registry::{chain::Rpc, ChainInfo, ChainRegistry};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize)]
pub struct Config {
    pub chains: HashMap<String, ChainConfig>,
}

impl Config {
    pub fn from_pwd() -> Result<Self> {
        let pwd = std::env::current_dir()?;
        let config_path = pwd.join("config.yaml");
        let config = std::fs::read_to_string(config_path)?;
        let config = serde_yaml::from_str(&config)?;
        Ok(config)
    }
}

impl<'de> Deserialize<'de> for Config {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Deserialize the raw config entry so we can get info from the chain registry.
        let config_yaml =
            HashMap::<String, HashMap<String, RawChainConfigEntry>>::deserialize(deserializer)?;
        let chains = config_yaml
            .get("chains")
            .ok_or_else(|| serde::de::Error::custom("missing 'chains' key"))?;
        let registry =
            ChainRegistry::from_remote().map_err(|e| serde::de::Error::custom(e.to_string()))?;

        // Collect the chain configs from the registry.
        let mut chain_configs = HashMap::new();

        #[allow(clippy::unnecessary_to_owned)]
        for (chain_id, entry) in chains.to_owned() {
            let chain_info = registry.get_by_chain_id(&chain_id).map_err(|e| {
                serde::de::Error::custom(format!("Registry get_by_chain_id error: {e}"))
            })?;
            let chain_config = ChainConfig::from_entry(chain_info, entry);
            chain_configs.insert(chain_id.to_owned(), chain_config);
        }

        // Return the config.
        Ok(Self {
            chains: chain_configs,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawChainConfigEntry {
    pub factory: String,
    pub registry: Option<String>,
    pub block_polling_seconds: Option<f64>,
    pub block_polling_timeout_seconds: Option<f64>,
    pub websocket_timeout_seconds: Option<f64>,
    pub uptime_ping_url: Option<Url>,
    pub gas_prices: Option<f32>,
    pub gas_adjustment: Option<f32>,
    pub threshold: Option<u64>,
    pub include_evented_tasks: Option<bool>,
    pub custom_sources: Option<HashMap<String, ChainDataSource>>,
    pub rpc_timeout_seconds: Option<f64>,
    pub denom: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainDataSource {
    pub rpc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub info: ChainInfo,
    pub factory: String,
    pub registry: Option<String>,
    pub block_polling_seconds: f64,
    pub block_polling_timeout_seconds: f64,
    pub websocket_timeout_seconds: f64,
    pub uptime_ping_url: Option<Url>,
    pub gas_prices: f32,
    pub gas_adjustment: f32,
    pub threshold: Option<u64>,
    pub include_evented_tasks: Option<bool>,
    pub rpc_timeout_seconds: Option<f64>,
    pub denom: Option<String>,
}

impl ChainConfig {
    fn from_entry(mut info: ChainInfo, entry: RawChainConfigEntry) -> Self {
        let gas_prices = entry
            .gas_prices
            .unwrap_or(info.fees.fee_tokens[0].fixed_min_gas_price);
        let gas_adjustment = entry.gas_adjustment.unwrap_or(1.5);
        let block_polling_seconds = entry.block_polling_seconds.unwrap_or(5.0);
        let block_polling_timeout_seconds = entry.block_polling_timeout_seconds.unwrap_or(30.0);
        let websocket_timeout_seconds = entry.websocket_timeout_seconds.unwrap_or(30.0);

        // Add optional custom sources to the chain info.
        if let Some(custom_sources) = entry.custom_sources {
            for (provider, source) in custom_sources {
                // Add the custom RPC source.
                info.apis.rpc.push(Rpc {
                    provider: Some(provider.clone()),
                    address: source.rpc.clone(),
                });
            }
        }

        Self {
            info,
            factory: entry.factory,
            registry: entry.registry,
            block_polling_seconds,
            block_polling_timeout_seconds,
            websocket_timeout_seconds,
            uptime_ping_url: entry.uptime_ping_url,
            gas_prices,
            gas_adjustment,
            threshold: entry.threshold,
            include_evented_tasks: entry.include_evented_tasks,
            rpc_timeout_seconds: entry.rpc_timeout_seconds,
            denom: entry.denom,
        }
    }

    pub fn data_sources(&self) -> HashMap<String, ChainDataSource> {
        let mut data_sources = HashMap::new();

        for rpc_endpoint in self.info.apis.rpc.iter() {
            if rpc_endpoint.provider.is_some() {
                data_sources.insert(
                    rpc_endpoint.provider.clone().unwrap(),
                    ChainDataSource {
                        rpc: rpc_endpoint.address.clone(),
                    },
                );
            }
        }

        data_sources
    }
}
