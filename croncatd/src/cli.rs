//!
//! `croncatd` CLI functionality
//!

use cosm_orc::orchestrator::error;
use croncat::errors::Report;
use std::{collections::HashMap, result};
use structopt::StructOpt;
use reqwest::{self, Response};
use anyhow;
use crate::opts::Opts;

/// Load the banner ascii art as a `&'static str`.
const BANNER_STR: &'static str = include_str!("../banner.txt");

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

pub async fn deposit_junox(address:&str)->anyhow::Result<Response, Report> {
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
