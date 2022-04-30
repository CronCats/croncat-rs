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

    /// Wether to print nice little things like the banner and a goodbye
    #[structopt(short, long)]
    pub no_frills: bool,
}
