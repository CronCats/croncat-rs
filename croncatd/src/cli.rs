//!
//! `croncatd` CLI functionality
//!

use crate::opts::Opts;
use croncat::errors::Report;
use clap::Parser;

/// Load the banner ascii art as a `&'static str`.
const BANNER_STR: &str = include_str!("../banner.txt");

///
/// Print the cute croncat banner for fun.
///
pub fn print_banner() {
    println!("{BANNER_STR}");
}

///
/// Get the command line options.
///
pub fn get_opts() -> Result<Opts, Report> {
    Ok(Opts::try_parse()?)
}
