//!
//! How to store our agent data locally on disk,
//! this will be user later to verify on chain.
//!

use secp256k1::KeyPair;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};

use crate::{errors::Report, utils};

use super::LOCAL_STORAGE_DEFAULT_DIR;

/// Where our [`LocalAgentStorage`] will be stored.
const LOCAL_STORAGE_AGENTS_FILENAME: &'static str = "./agents.json";

/// Alias String as [`AccountId`] in this module only.
type AccountId = String;

/// The hashmap we intend to store on disk.
type LocalAgentStorageData = HashMap<AccountId, LocalAgentStorageEntry>;

/// Store the keypair and the payable account idea for a stored agent
#[derive(Debug, Serialize, Deserialize)]
pub struct LocalAgentStorageEntry {
    pub account_id: AccountId,
    #[serde(with = "secp256k1::serde_keypair")]
    pub keypair: KeyPair, // TODO (SeedyROM): This should probably point to a file, not store in memory
    pub payable_account_id: Option<AccountId>,
}

impl LocalAgentStorageEntry {
    /// Get the account to pay rewards to from a [`LocalAgentStorageEntry`]
    pub fn get_payable_account_id(&self) -> &AccountId {
        match &self.payable_account_id {
            Some(account_id) => account_id,
            None => &self.account_id,
        }
    }
}

/// Store key pairs on disk and allow access to the data.
// TODO (SeedyROM): This should be named different but I'm being insane and can't decide
pub struct LocalAgentStorage {
    pub path: PathBuf,
    data: LocalAgentStorageData,
}

impl LocalAgentStorage {
    /// Create a new [`LocalAgentStorage`] instance with the default directory.
    pub fn new() -> Self {
        Self::from_path(LOCAL_STORAGE_DEFAULT_DIR.into())
    }

    /// Create a [`LocalAgentStorage`] instance at a specified path,
    /// if the data already exists at the directory we load it.
    pub fn from_path(path: PathBuf) -> Self {
        let agent_data_file = path.join(LOCAL_STORAGE_AGENTS_FILENAME);

        // Load from the agent data file if it exists
        if agent_data_file.exists() {
            let json_data = fs::read_to_string(agent_data_file).unwrap();
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
        let agent_data_file = self.path.join(LOCAL_STORAGE_AGENTS_FILENAME);

        // Create the directory to store our data if it doesn't exist
        // TODO (SeedyROM): This can be moved to a helper probably...?
        if let Some(p) = agent_data_file.parent() {
            fs::create_dir_all(p)?
        };

        fs::write(agent_data_file, serde_json::to_string(&self.data).unwrap())?;

        Ok(())
    }

    /// Insert a new agent into the data map.
    fn insert(
        &mut self,
        account_id: AccountId,
        payable_account_id: Option<AccountId>,
    ) -> Option<LocalAgentStorageEntry> {
        if self.data.get(&account_id).is_some() {
            None
        } else {
            self.data.insert(
                account_id.clone(),
                LocalAgentStorageEntry {
                    account_id,
                    keypair: utils::generate_keypair(),
                    payable_account_id,
                },
            )
        }
    }

    /// Register a new account_id to the croncat agent.
    pub fn register(
        &mut self,
        account_id: AccountId,
        payable_account_id: Option<AccountId>,
    ) -> Result<(), Report> {
        match self.get(&account_id) {
            Some(_) => todo!(), // TODO (SeedyROM): Return a custom error
            None => {
                self.insert(account_id, payable_account_id);
                self.write_to_disk()?;
                Ok(())
            }
        }
    }

    /// Retrieve an agent based on the key
    pub fn get(&self, account_id: &AccountId) -> Option<&LocalAgentStorageEntry> {
        self.data.get(account_id)
    }
}
