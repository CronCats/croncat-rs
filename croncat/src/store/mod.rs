//! Helpers to handle storing and retrieving data from the local filesystem.

use std::path::PathBuf;

/// The default directory where we'll store the agent data.
pub const LOCAL_STORAGE_DEFAULT_DIR: &str = "/.croncatd";

pub mod agent;
pub mod factory;
pub mod logs;
pub mod tasks;

pub fn get_storage_path() -> PathBuf {
    let mut home = std::env::var("HOME").unwrap();
    home.push_str(LOCAL_STORAGE_DEFAULT_DIR);
    PathBuf::from(home)
}
