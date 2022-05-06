//!
//! `croncatd` CLI functionality
//!

use croncat::errors::Report;
use structopt::StructOpt;

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
