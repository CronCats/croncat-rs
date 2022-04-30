use structopt::StructOpt;

///
/// Command line options.
/// 
#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
pub struct Opts {
    /// Activate debug mode
    #[structopt(short, long)]
    pub debug: bool,

    /// Wether to show the banner or not
    #[structopt(short, long)]
    pub no_frills: bool,
}