//! Agent configuration.

use std::collections::HashMap;

use color_eyre::Result;
use cosmos_chain_registry::{ChainInfo, ChainRegistry};
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
        #[allow(clippy::unnecessary_to_owned)]
        let chain_configs = chains
            .to_owned()
            .into_iter()
            .map(|(chain_id, entry)| {
                let chain_info = registry
                    .get_by_chain_id(&chain_id)
                    .map_err(|e| serde::de::Error::custom(e.to_string()))?;
                let chain_config = ChainConfig::from_entry(chain_info, entry);
                Ok((chain_id, chain_config))
            })
            .collect::<Result<Vec<_>, D::Error>>()?;
        let chain_configs: HashMap<String, ChainConfig> = chain_configs.into_iter().collect();

        Ok(Self {
            chains: chain_configs,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawChainConfigEntry {
    pub manager: String,
    pub registry: Option<String>,
    pub block_polling_seconds: Option<f64>,
    pub block_polling_timeout_seconds: Option<f64>,
    pub websocket_timeout_seconds: Option<f64>,
    pub uptime_ping_url: Option<Url>,
    pub gas_prices: Option<f32>,
    pub gas_adjustment: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub info: ChainInfo,
    pub manager: String,
    pub registry: Option<String>,
    pub block_polling_seconds: f64,
    pub block_polling_timeout_seconds: f64,
    pub websocket_timeout_seconds: f64,
    pub uptime_ping_url: Option<Url>,
    pub gas_prices: f32,
    pub gas_adjustment: f32,
}

impl ChainConfig {
    fn from_entry(info: ChainInfo, entry: RawChainConfigEntry) -> Self {
        let gas_prices = entry
            .gas_prices
            .unwrap_or(info.fees.fee_tokens[0].fixed_min_gas_price);
        let gas_adjustment = entry.gas_adjustment.unwrap_or(1.5);
        let block_polling_seconds = entry.block_polling_seconds.unwrap_or(5.0);
        let block_polling_timeout_seconds = entry.block_polling_timeout_seconds.unwrap_or(30.0);
        let websocket_timeout_seconds = entry.websocket_timeout_seconds.unwrap_or(30.0);

        Self {
            info,
            manager: entry.manager,
            registry: entry.registry,
            block_polling_seconds,
            block_polling_timeout_seconds,
            websocket_timeout_seconds,
            uptime_ping_url: entry.uptime_ping_url,
            gas_prices,
            gas_adjustment,
        }
    }
}
