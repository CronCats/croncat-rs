//!
//! How to store our agent data locally on disk,
//! this will be user later to verify on chain.
//!

use bip39::Mnemonic;
use color_eyre::eyre::eyre;
use cosm_orc::config::key::{Key, SigningKey};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};

use crate::{errors::Report, utils};

use super::LOCAL_STORAGE_DEFAULT_DIR;

/// Where our [`LocalAgentStorage`] will be stored.
const LOCAL_STORAGE_AGENTS_FILENAME: &str = "./agents.json";

/// Alias String as [`AccountId`] in this module only.
type AccountId = String;

/// The hashmap we intend to store on disk.
type LocalAgentStorageData = HashMap<AccountId, LocalAgentStorageEntry>;

#[derive(Debug, Serialize, Deserialize)]
struct KeyPair {
    private_key: String,
    public_key: String,
}

/// Store the keypair and the payable account idea for a stored agent
#[derive(Debug, Serialize, Deserialize)]
pub struct LocalAgentStorageEntry {
    pub account_addr: String,
    keypair: KeyPair, // TODO (SeedyROM): This should probably point to a file, not store in memory
    pub mnemonic: String,
    pub payable_account_id: Option<String>,
}

impl LocalAgentStorageEntry {
    /// Get the account to pay rewards to from a [`LocalAgentStorageEntry`]
    pub fn get_payable_account_id(&self) -> &str {
        match &self.payable_account_id {
            Some(account_id) => account_id.as_str(),
            None => self.account_addr.as_str(),
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
        let mut home = std::env::var("HOME").unwrap();
        home.push_str(LOCAL_STORAGE_DEFAULT_DIR);
        Self::from_path(home.into())
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

        fs::write(
            agent_data_file,
            serde_json::to_string_pretty(&self.data).unwrap(),
        )?;

        Ok(())
    }

    /// Insert a new agent into the data map.
    fn insert(
        &mut self,
        account_id: AccountId,
        mnemonic: Mnemonic,
    ) -> Option<LocalAgentStorageEntry> {
        if self.data.get(&account_id).is_some() {
            None
        } else {
            let signing_key = cosmrs::bip32::XPrv::derive_from_path(
                mnemonic.to_seed(""),
                &utils::DERVIATION_PATH.parse().unwrap(),
            )
            .unwrap();
            let public_key = signing_key
                .public_key()
                .to_string(cosmrs::bip32::Prefix::XPRV);
            let private_key = signing_key
                .to_string(cosmrs::bip32::Prefix::XPRV)
                .to_string();
            let keypair = KeyPair {
                public_key,
                private_key,
            };

            let mnemonic = mnemonic.to_string();
            let signing_key = SigningKey {
                name: account_id.clone(),
                key: Key::Mnemonic(mnemonic.clone()),
            };
            let account_addr = signing_key.to_account("juno").unwrap().to_string();

            self.data.insert(
                account_id,
                LocalAgentStorageEntry {
                    account_addr,
                    keypair,
                    mnemonic,
                    payable_account_id: None,
                },
            )
        }
    }

    /// Generate a new account_id to the local storage.
    pub fn generate_account(&mut self, account_id: AccountId) -> Result<(), Report> {
        match self.get(&account_id) {
            Some(_) => Err(eyre!(r#"Agent "{account_id}" already created"#)),
            None => {
                self.insert(account_id.clone(), Mnemonic::generate(24).unwrap());
                let new_account = self.get(&account_id);
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({ account_id: new_account }))
                        .unwrap()
                );
                self.write_to_disk()?;
                Ok(())
            }
        }
    }

    /// Retrieve an agent based on the key
    pub fn get(&self, account_id: &AccountId) -> Option<&LocalAgentStorageEntry> {
        self.data.get(account_id)
    }

    pub fn get_agent_signing_key(&self, account_id: &AccountId) -> Result<SigningKey, Report> {
        let entry = if let Some(entry) = self.get(account_id) {
            entry
        } else {
            return Err(eyre!("No agent key by this id"));
        };
        let key = SigningKey {
            name: account_id.clone(),
            key: Key::Mnemonic(entry.mnemonic.clone()),
        };
        Ok(key)
    }
}

impl Default for LocalAgentStorage {
    fn default() -> Self {
        Self::new()
    }
}
