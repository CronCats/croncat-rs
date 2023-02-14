use color_eyre::eyre::eyre;
use cosmrs::{bip32, crypto::secp256k1::SigningKey};
use serde::{Deserialize, Serialize};
use chrono::Utc;
use std::{collections::HashMap, fs, path::PathBuf};
use crate::{errors::Report, utils::DERIVATION_PATH};
use croncat_sdk_factory::msg::ContractMetadataInfo;

use super::get_storage_path;

/// Where our [`LocalCacheStorage`] will be stored.
const LOCAL_STORAGE_FILENAME: &str = "./cache.json";

/// The hashmap we intend to store on disk.
type LocalCacheStorageData = HashMap<String, LocalCacheStorageEntry>;

/// Store the factory data cache
#[derive(Serialize, Deserialize, Clone)]
pub struct LocalCacheStorageEntry {
    pub expires: u64,
    pub latest: HashMap<&str, [u8; 2]>,
    pub versions: HashMap<(&str, &[u8]), ContractMetadataInfo>,
}

impl std::fmt::Debug for LocalCacheStorageEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalCacheStorageEntry")
            .field("expires", &self.expires.to_string())
            .field("latest", &self.latest)
            .field("versions", &self.versions)
            .finish()
    }
}

/// Store key pairs on disk and allow access to the data.
pub struct LocalCacheStorage {
    pub path: PathBuf,
    data: LocalCacheStorageData,
}

impl LocalCacheStorage {
    /// Create a new [`LocalCacheStorage`] instance with the default directory.
    pub fn new() -> Self {
        Self::from_path(get_storage_path())
    }

    /// Create a [`LocalCacheStorage`] instance at a specified path,
    /// if the data already exists at the directory we load it.
    pub fn from_path(path: PathBuf) -> Self {
        let data_file = path.join(LOCAL_STORAGE_FILENAME);

        // Load from the agent data file if it exists
        if data_file.exists() {
            let json_data = fs::read_to_string(data_file).unwrap();
            let data =
                serde_json::from_str(json_data.as_str()).expect("Failed to parse agent JSON data");
            Self { path, data }
        } else {
            // Otherwise create a new hashmap
            Self {
                path,
                data: HashMap::new(),
            }
        }
    }

    /// Write our data to disk at the specified location.
    fn write_to_disk(&self) -> Result<(), Report> {
        let data_file = self.path.join(LOCAL_STORAGE_FILENAME);

        // Create the directory to store our data if it doesn't exist
        if let Some(p) = data_file.parent() {
            fs::create_dir_all(p)?
        };

        fs::write(data_file, serde_json::to_string_pretty(&self.data)?)?;

        Ok(())
    }

    /// Insert a item into the data map.
    fn insert(
        &mut self,
        key: String,
        metadata: ContractMetadataInfo,
    ) -> Result<Option<LocalCacheStorageEntry>, Report> {
        if self.data.get(&key).is_some() {
            Ok(None)
        } else {
            let dt = Utc::now();
            let timestamp: u64 = dt.timestamp();
            let new_data = LocalCacheStorageEntry {
                timestamp,
                metadata,
            };
            self.data.insert(key, new_data.clone());
            Ok(Some(new_data))
        }
    }

    /// Retrieve data based on the key
    fn get(&self, key: &str) -> Option<&LocalCacheStorageEntry> {
        self.data.get(key)
    }
}

impl Default for LocalCacheStorage {
    fn default() -> Self {
        Self::new()
    }
}
