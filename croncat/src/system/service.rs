use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use color_eyre::Report;
use indoc::indoc;
use tracing::log::info;

/// Name of the daemon service directory.
const DAEMON_SERVICES_DIR_NAME: &str = "system-services";

/// The croncat system service daemon.
pub struct ServiceDaemon;

impl ServiceDaemon {
    /// Create a new daemon service file at the given path with chain ID.
    pub fn create(path: Option<String>, chain_id: &String) -> Result<(), Report> {
        // If no path is given, use the default path
        let default_output = std::env::current_dir()
            .expect("Failed to get current directory")
            .to_str()
            .unwrap()
            .to_string();
        let path = PathBuf::from(path.unwrap_or(default_output)).join(DAEMON_SERVICES_DIR_NAME);

        // Create the daemon service directory at the given path if it doesn't exist.
        fs::create_dir_all(&path)?;

        // Create the service file based on the chain ID.
        Self::create_service_file(path, chain_id)?;

        Ok(())
    }

    fn create_service_file(path: PathBuf, chain_id: &String) -> Result<(), Report> {
        // Get the current user's name.
        let user = whoami::username();
        // Get the full path to the croncatd service directory.
        let full_service_dir_path = fs::canonicalize(&path)?;
        // File name for the croncatd service file.
        let service_file_path = full_service_dir_path
            .join(format!("croncatd-{}.service", chain_id))
            .to_str()
            .expect("Failed to get service file path.")
            .to_string();

        info!("Creating system service daemon at {}", &service_file_path);
        let mut file = File::create(&service_file_path)?;
        file.write_all(
            format!(
                indoc! {"
                    Description=croncatd {0} agent
                    After=multi-user.target

                    [Service]
                    Type=simple
                    User={1}
                    WorkingDirectory={2}
                    ExecStart={3} go
                    StandardOutput=append:/var/log/croncatd-{0}.log
                    StandardError=append:/var/log/croncatd-{0}-error.log
                    Restart=on-failure
                    RestartSec=60
                    KillSignal=SIGINT
                    TimeoutStopSec=45
                    KillMode=mixed

                    [Install]
                    WantedBy=multi-user.target
                "},
                chain_id,
                user,
                full_service_dir_path
                    .to_str()
                    .expect("Could not convert daemon service directory path to string."),
                std::env::current_exe()?
                    .to_str()
                    .expect("Could not convert daemon service executable path to string."),
            )
            .as_bytes(),
        )?;

        Ok(())
    }
}
