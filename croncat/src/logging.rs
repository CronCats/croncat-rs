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
/// Setup logging with `color_eyre` and `tracing_subscriber`.
///
pub fn setup() -> Result<(), Report> {
    // Set RUST_LIB_BACKTRACE=1 to enable backtraces
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        std::env::set_var("RUST_LIB_BACKTRACE", "1");
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

///
/// Setup logging for the go command
///
pub fn setup_go(chain_id: String) -> Result<Vec<WorkerGuard>, Report> {
    let mut guards = vec![];

    let error_file_appender = tracing_appender::rolling::daily(
        format!("{}/logs", get_storage_path().to_str().unwrap()),
        format!("{}.error.log", chain_id),
    );
    let (error_file_writer, guard) = tracing_appender::non_blocking(error_file_appender);
    guards.push(guard);

    let file_appender = tracing_appender::rolling::daily(
        format!("{}/logs", get_storage_path().to_str().unwrap()),
        format!("{}.log", chain_id),
    );
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
    guards.push(guard);

    let subscriber = tracing_subscriber::registry()
        .with(
            fmt::Layer::new().with_writer(
                file_writer
                    .with_max_level(Level::INFO)
                    .with_min_level(Level::WARN),
            ),
        )
        .with(fmt::Layer::new().with_writer(error_file_writer.with_max_level(Level::ERROR)))
        .with(fmt::Layer::new().with_writer(std::io::stdout.with_max_level(Level::INFO)));

    tracing::subscriber::set_global_default(subscriber)?;

    Ok(guards)
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
