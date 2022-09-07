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

        #[structopt(long, default_value = "agent")]
        sender_name: String,
    },
    GetAgentStatus {
        account_id: String,
    },
    GetAgentTasks {
        account_addr: String,
    },
    UnregisterAgent {
        #[structopt(long, default_value = "agent")]
        sender_name: String,
    },
    UpdateAgent {
        payable_account_id: String,
        #[structopt(long, default_value = "agent")]
        sender_name: String,
    },
    Withdraw {
        #[structopt(long, default_value = "agent")]
        sender_name: String,
    },
    Status,
    Tasks {
        from_index: Option<u64>,
        limit: Option<u64>,
    },
    Go {
        #[structopt(long, default_value = "agent")]
        sender_name: String,
    },
    Info,
    GenerateMnemonic {
        #[structopt(long, default_value = "agent")]
        new_name: String,
    },
    DepositUjunox {
        account_id: String,
    },
    GetAgent {
        #[structopt(long, default_value = "agent")]
        name: String,
    },
    Daemon {
        #[structopt(long, default_value = "agent")]
        sender_name: String,
    },
}
