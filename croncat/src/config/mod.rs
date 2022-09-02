/// TODO: Move to chain registry
/// Right now juno testnet missing grpc's, so we keeping it like `cosm-orc`'s chain config
use color_eyre::Report;
use config::Config;
use serde::{Deserialize, Serialize};

const CONFIG_FILE: &str = "config.yaml";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChainConfig {
    pub denom: String,
    pub prefix: String,
    pub chain_id: String,
    pub rpc_endpoint: String,
    pub grpc_endpoint: String,
    pub gas_prices: f64,
    pub gas_adjustment: f64,
}

impl ChainConfig {
    pub fn new() -> Result<Self, Report> {
        let settings = Config::builder()
            .add_source(config::File::with_name(CONFIG_FILE))
            .build()?;

        Ok(settings.try_deserialize::<ChainConfig>()?)
    }
}
