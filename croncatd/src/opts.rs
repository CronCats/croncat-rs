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

    #[structopt(long, default_value = "agent")]
    pub account_id: String,
}

#[derive(Debug, StructOpt)]
pub enum Command {
    /// Pound acorns into flour for cookie dough.
    RegisterAgent {
        payable_account_id: Option<String>,
    },
    GetAgentStatus {
        account_id: String,
    },
    GetAgentTasks {
        account_id: String,
    },
    UnregisterAgent(MessageInfo),
    UpdateAgent {
        payable_account_id: String,
    },
    Withdraw,
    Status,
    Tasks {
        from_index: Option<u64>,
        limit: Option<u64>,
    },
    Go {
        account_id: String,
    },
    Info,
    GenerateMnemonic,
    DepositUjunox {
        account_id: Option<String>,
    },
    GetAgent,
}
#[derive(Debug, StructOpt)]
pub struct MessageInfo {
    #[structopt(short)]
    pub sender: String,
}
