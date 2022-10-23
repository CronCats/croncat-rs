use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use color_eyre::Report;
use tracing::error;

use super::get_storage_path;

pub struct ErrorLogStorage;

impl ErrorLogStorage {
    /// Get the path to the error log file.
    fn get_path(agent_id: &String) -> PathBuf {
        let mut path = get_storage_path();
        path.push("logs");
        path.push(format!(
            "{}.error.log.{}",
            agent_id,
            chrono::Local::now().format("%Y-%m-%d")
        ));
        path
    }

    /// Write the given error to the error log file.
    pub fn write(agent_name: &String, err: &Report) -> Result<(), Report> {
        let path = Self::get_path(agent_name);
        fs::create_dir_all(path.parent().unwrap())?;
        error!("Writing error to log file at {}", path.to_str().unwrap());
        let mut file = File::create(path)?;
        file.write_all(format!("{:?}", err).as_bytes())?;
        Ok(())
    }
}
