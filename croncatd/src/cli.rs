//!
//! `croncatd` CLI functionality
//!

use crate::opts::Opts;
use croncat::errors::Report;
use reqwest::{self, Response};
use std::collections::HashMap;
use structopt::StructOpt;

/// Load the banner ascii art as a `&'static str`.
const BANNER_STR: &str = include_str!("../banner.txt");

///
/// Print the cute croncat banner for fun.
///
pub fn print_banner() {
    println!("{}", BANNER_STR);
}

///
/// Get the command line options.
///
pub fn get_opts() -> Result<Opts, Report> {
    Ok(Opts::from_args_safe()?)
}

pub async fn deposit_junox(address: &str) -> Result<Response, Report> {
    let mut map = HashMap::new();
    map.insert("denom", "ujunox");
    map.insert("address", address);

    let client = reqwest::Client::new();
    let res = client
        .post("https://faucet.uni.juno.deuslabs.fi/credit")
        .json(&map)
        .send()
        .await?;
    Ok(res)
}
