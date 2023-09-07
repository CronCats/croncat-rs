//!
//! `croncatd` CLI option builder.
//!

use clap::Parser;
use croncat::utils::DEFAULT_AGENT_ID;
use enum_display::EnumDisplay;

#[derive(Debug, Parser, Clone)]
#[command(name = "croncatd", about = "The croncat agent daemon.")]
pub struct Opts {
    /// Debug mode
    #[clap(short, long)]
    pub debug: bool,

    /// Whether to print nice little things like the banner and a goodbye
    #[clap(long)]
    pub no_frills: bool,

    #[clap(subcommand)] // Note that we mark a field as a subcommand
    pub cmd: Command,

    /// Chain ID of the chain to connect to
    #[clap(long, global = true, env = "CRONCAT_CHAIN_ID")]
    pub chain_id: Option<String>,

    /// ID of the agent config to use
    #[clap(long, global = true, default_value = DEFAULT_AGENT_ID, env = "CRONCAT_AGENT")]
    pub agent: String,
}

#[derive(Debug, Parser, Clone, EnumDisplay)]
#[enum_display(case = "Kebab")]
pub enum Command {
    /// Useful for clearing local cached chain tasks
    ClearCache,

    /// Registers an agent, placing them in the pending queue unless it's the first agent.
    Register { payable_account_id: Option<String> },

    /// Get the agent's supported bech32 accounts
    ListAccounts,

    /// Get the agent's status (pending/active)
    Status,

    /// Get the agent's tasks they're assigned to fulfill
    GetTasks,

    /// Unregisters the agent from being in the queue with other agents
    Unregister,

    /// Update the agent's configuration
    Update { payable_account_id: String },

    /// Withdraw the agent's funds to the payable account ID
    Withdraw,

    /// Get contract's state
    // #[cfg(feature = "debug")]
    // GetState {
    //     from_index: Option<u64>,
    //     limit: Option<u64>,
    // },

    /// Show all task(s) information
    AllTasks {
        from_index: Option<u64>,
        limit: Option<u64>,
    },

    /// Starts the Croncat agent, allowing it to fulfill tasks
    Go {},

    /// Generates a new keypair and agent account (good first step)
    GenerateMnemonic {
        /// The agent's name
        new_name: String,

        /// Recover agent from mnemonic phrase. Please do not use your own account!
        #[structopt(long)]
        mnemonic: Option<String>,
    },

    /// [SENSITIVE!] Shows all details about agents on this machine
    GetAgentKeys {
        #[structopt(long, default_value = "agent", env = "CRONCAT_AGENT")]
        name: String,
    },

    /// Setup an agent as a system service (systemd)
    SetupService {
        #[structopt(long)]
        output: Option<String>,
    },

    /// Send funds from the agent account to another account (`cargo run send juno123abc... 1 ujuno`)
    #[structopt(name = "send")]
    SendFunds {
        /// The address to send funds to
        to: String,

        /// The amount of funds to send
        amount: String,

        /// The denom of the funds to send
        denom: Option<String>,
    },
}

impl Command {
    // Determine if this action happens on-chain
    pub fn on_chain(&self) -> bool {
        // It's reversed, because we have much less off-chain methods
        !matches!(
            self,
            Self::ListAccounts
                | Self::GenerateMnemonic { .. }
                | Self::GetAgentKeys { .. }
                | Self::SetupService { .. }
        )
    }
}
