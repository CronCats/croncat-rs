use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use color_eyre::{eyre::eyre, Report};
use indoc::indoc;
use tracing::log::info;

use crate::store::LOCAL_STORAGE_DEFAULT_DIR;

/// Name of the daemon service directory.
const DAEMON_SERVICES_DIR_NAME: &str = "system-services";

/// The croncat system service daemon.
pub struct DaemonService;

impl DaemonService {
    /// Create a new daemon service file at the given path with chain ID.
    pub fn create(path: Option<String>, chain_id: &String, no_frills: bool) -> Result<(), Report> {
        // If no path is given, use the default storage path in the HOME directory.
        let mut default_output = std::env::var("HOME")?;
        default_output.push_str(LOCAL_STORAGE_DEFAULT_DIR);
        let path = PathBuf::from(path.unwrap_or(default_output)).join(DAEMON_SERVICES_DIR_NAME);

        // Create the daemon service directory at the given path if it doesn't exist.
        fs::create_dir_all(&path)?;

        // Create the service file based on the chain ID.
        let service_file_path = Self::create_service_file(path, chain_id)?;

        info!(
            "Created croncatd service file for chain ID {} at {}",
            chain_id, &service_file_path
        );

        // Link the service file to the systemd directory.
        Self::link_service_file(&service_file_path)?;

        info!("Linked croncatd service file to systemd directory");

        // Print a nice little message if we're not in no-frills mode.
        Self::print_next_steps(chain_id, no_frills);

        Ok(())
    }

    fn create_service_file(path: PathBuf, chain_id: &String) -> Result<String, Report> {
        // Get the current user's name.
        let user = whoami::username();
        // Get the full path to the croncatd service directory.
        let full_service_dir_path = fs::canonicalize(&path)?;
        // File path for the croncatd service file.
        let service_file_path = full_service_dir_path
            .join(format!("croncatd-{}.service", chain_id))
            .to_str()
            .ok_or_else(|| eyre!("Failed to get service file path."))?
            .to_string();

        info!("Creating system service daemon at {}", &service_file_path);
        let mut file = File::create(&service_file_path)?;
        file.write_all(
            format!(
                indoc! {"
                    Description=croncatd {chain_id} agent
                    After=multi-user.target

                    [Service]
                    Type=simple
                    User={user}
                    WorkingDirectory={service_dir}
                    ExecStart={exe_path} go
                    StandardOutput=append:/var/log/croncatd-{chain_id}.log
                    StandardError=append:/var/log/croncatd-{chain_id}-error.log
                    Restart=on-failure
                    RestartSec=60
                    KillSignal=SIGINT
                    TimeoutStopSec=45
                    KillMode=mixed

                    [Install]
                    WantedBy=multi-user.target
                "},
                chain_id = chain_id,
                user = user,
                service_dir = full_service_dir_path.to_str().ok_or_else(|| eyre!(
                    "Could not convert daemon service directory path to string",
                ))?,
                exe_path = std::env::current_exe()?.to_str().ok_or_else(|| eyre!(
                    "Could not convert daemon service executable path to string",
                ))?,
            )
            .as_bytes(),
        )?;

        Ok(service_file_path)
    }

    /// Create a symlink to the service file in the systemd directory.
    fn link_service_file(service_file_path: &String) -> Result<(), Report> {
        std::process::Command::new("sudo")
            .arg("systemctl")
            .arg("link")
            .arg(service_file_path)
            .status()
            .map_err(|err| eyre!("Failed to link service file to systemd: {}", err))?;

        Ok(())
    }

    fn print_next_steps(chain_id: &String, no_frills: bool) {
        if !no_frills {
            println!(
                indoc! {"\n
                    Next steps:
                    1. Enable the service: `sudo systemctl enable croncatd-{chain_id}`
                    2. Start the service: `sudo systemctl start croncatd-{chain_id}`
                "},
                chain_id = chain_id,
            );
        }
    }
}
