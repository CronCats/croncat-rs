//!
//! Setup tracing/logging/backtraces and re-export tracing log macros.
//!

use crate::errors::Report;
use tracing_subscriber::EnvFilter;

// Export the tracing function for use
pub use tracing::{debug, error, info, warn};

///
/// Setup logging with `color_eyre` and `tracing_subscriber`.
///
pub fn setup() -> Result<(), Report> {
    // Get / set backtrace
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        std::env::set_var("RUST_LIB_BACKTRACE", "1")
    }
    // Install color_eyre
    color_eyre::install()?;

    // Get/set the log level
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    // Setup tracing and tracing-subscriber
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    Ok(())
}

#[cfg(test)]
mod tests {
    use tracing::info;

    #[test]
    #[tracing_test::traced_test]
    fn can_pretty_log() {
        info!("We're cool for cats");

        assert!(logs_contain("We're cool for cats"));
    }
}
