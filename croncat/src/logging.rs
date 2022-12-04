//!
//! Setup tracing/logging/backtraces and re-export tracing log macros.
//!

use crate::{errors::Report, store::get_storage_path};
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

// Export the tracing function for use
pub use tracing::{debug, error, info, warn};

///
/// Setup logging for the go command
///
pub fn setup(chain_id: Option<String>) -> Result<Vec<WorkerGuard>, Report> {
    // Set RUST_LIB_BACKTRACE=1 to enable backtraces
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        std::env::set_var("RUST_LIB_BACKTRACE", "1");
    }

    // Set RUST_LOG to info by default
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    // Install color_eyre
    color_eyre::install()?;

    // Setup multi file logging.
    let mut file_appender_guards = vec![];

    // No chain id, so just log to the default file.
    if chain_id.is_none() {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_writer(std::io::stderr)
            .init();
    } else {
        // Log file for errors.
        let error_file_appender = tracing_appender::rolling::daily(
            format!("{}/logs", get_storage_path().to_str().unwrap()),
            format!("{}.error.log", chain_id.as_ref().unwrap()),
        );
        let (error_file_writer, guard) = tracing_appender::non_blocking(error_file_appender);
        file_appender_guards.push(guard);

        // Log file for info.
        let file_appender = tracing_appender::rolling::daily(
            format!("{}/logs", get_storage_path().to_str().unwrap()),
            format!("{}.log", chain_id.as_ref().unwrap()),
        );
        let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
        file_appender_guards.push(guard);

        // Create the tracing subscriber with the file appender layers.
        let subscriber = tracing_subscriber::registry().with(
            fmt::Layer::new()
                .with_writer(
                    file_writer
                        .with_max_level(Level::INFO)
                        .with_min_level(Level::WARN),
                )
                .and_then(
                    fmt::Layer::new().with_writer(error_file_writer.with_max_level(Level::ERROR)),
                )
                .and_then(
                    fmt::Layer::new().with_writer(std::io::stderr.with_max_level(Level::INFO)),
                ),
        );
        // Set the subscriber as the global default.
        tracing::subscriber::set_global_default(subscriber)?;
    }

    // Return back teh file appender guards.
    Ok(file_appender_guards)
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
