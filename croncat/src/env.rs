//!
//! `croncatd` environment variable loading and parsing.
//!

use crate::errors::Report;
use serde::Deserialize;

///
/// Default url for GRPC locally.
///
fn default_grpc_url() -> String {
    String::from("http://localhost:9090")
}

///
/// Default url for WS RPC locally.
///
fn default_wsrpc_url() -> String {
    String::from("ws://localhost:26657/websocket")
}

///
/// The environment variables struct.
///
#[derive(Debug, Deserialize)]
pub struct Env {
    #[serde(default = "default_grpc_url")]
    pub grpc_url: String,
    #[serde(default = "default_wsrpc_url")]
    pub wsrpc_url: String,
}


///
/// Load our environment variables from a .env and chuck em in an `Env`.
///
pub fn load() -> Result<Env, Report> {
    dotenv::dotenv()?;
    Ok(envy::from_env::<Env>()?)
}
