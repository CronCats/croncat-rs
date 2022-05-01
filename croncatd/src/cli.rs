//!
//! `croncatd` CLI functionality
//!

use croncat::{
    channels::{BlockStreamRx, BlockStreamTx, ShutdownRx, ShutdownTx},
    errors::Report,
    tokio::sync::{mpsc, watch},
};
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

///
/// Create a shutdown channel.
///
pub fn create_shutdown_channel() -> (ShutdownTx, ShutdownRx) {
    watch::channel(())
}

///
/// Create a block stream channel of a specified size, used to create back pressure.
///
pub fn create_block_stream_channel(channel_size: usize) -> (BlockStreamTx, BlockStreamRx) {
    mpsc::channel(channel_size)
}
