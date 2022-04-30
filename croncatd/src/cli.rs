use croncat::{errors::Report, tokio::sync::mpsc::{self}};
use structopt::StructOpt;

use crate::opts::Opts;

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
/// Shutdown channel Sender.
/// 
pub type ShutdownTx = mpsc::Sender<()>;

///
/// Shutdown channel Receiver.
/// 
pub type ShutdownRx = mpsc::Receiver<()>;

///
/// Create a shutdown channel.
///
pub fn create_shutdown_channel() -> (ShutdownTx, ShutdownRx) {
  mpsc::channel(1)
}