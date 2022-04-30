use croncat::errors::Report;
use serde::Deserialize;

///
/// Default url for GRPC locally.
/// 
fn default_grpc_url() -> String {
  String::from("http://[::1]:9090")
}

///
/// The environment variables struct.
/// 
#[derive(Debug, Deserialize)]
pub struct Env {
  #[serde(default="default_grpc_url")]
  pub grpc_url: String,
}

///
/// Load our environment variables from a .env and chuck em in an `Env`. 
/// 
pub fn load() -> Result<Env, Report> {
  dotenv::dotenv()?;
  Ok(envy::from_env::<Env>()?)
}