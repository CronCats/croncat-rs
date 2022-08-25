//!
//! `croncatd` CLI option builder.
//!

use structopt::StructOpt;

///
/// Command line options.
///
#[derive(Debug, StructOpt)]
#[structopt(name = "croncatd", about = "The croncat agent daemon.")]
pub struct Opts {
    /// Debug mode
    #[structopt(short, long)]
    pub debug: bool,

    /// Whether to print nice little things like the banner and a goodbye
    #[structopt(long)]
    pub no_frills: bool,

    #[structopt(subcommand)] // Note that we mark a field as a subcommand
    pub cmd: Command,
}

#[derive(Debug, StructOpt)]
pub enum Command {
    /// Pound acorns into flour for cookie dough.
    RegisterAgent {
        payable_account_id: Option<String>,
    },
    UnregisterAgent(MessageInfo),
    UpdateAgent {
        payable_account_id: String,
    },
    Withdraw,
    Status,
    Tasks,
    Go,
    GenerateMnemonic,
}
#[derive(Debug, StructOpt)]
pub struct MessageInfo {
    #[structopt(short)]
    pub sender: String,
}
