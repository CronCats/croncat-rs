//!
//! How to store our agent data locally on disk,
//! this will be user later to verify on chain.
//!

use bip39::Mnemonic;
use color_eyre::eyre::eyre;
use cosmrs::{bip32, crypto::secp256k1::SigningKey};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};
use tracing::log::info;

use crate::{errors::Report, utils};

use super::LOCAL_STORAGE_DEFAULT_DIR;

/// Where our [`LocalAgentStorage`] will be stored.
const LOCAL_STORAGE_AGENTS_FILENAME: &str = "./agents.json";

/// Alias String as [`AccountId`] in this module only.
type AccountId = String;

/// The hashmap we intend to store on disk.
type LocalAgentStorageData = HashMap<AccountId, LocalAgentStorageEntry>;

#[derive(Serialize, Deserialize, Clone)]
struct KeyPair {
    private_key: String,
    public_key: String,
}

/// Hide the private key from the user when debug printing.
impl std::fmt::Debug for KeyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyPair")
            .field("public_key", &self.public_key)
            .finish()
    }
}

/// Store the keypair and the payable account idea for a stored agent
#[derive(Serialize, Deserialize, Clone)]
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

/// Hide the user mnemonic from the user when debug printing.
impl std::fmt::Debug for LocalAgentStorageEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalAgentStorageEntry")
            .field("account_addr", &self.account_addr)
            .field("keypair", &self.keypair)
            .field("payable_account_id", &self.payable_account_id)
            .finish()
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
    ) -> Result<Option<LocalAgentStorageEntry>, Report> {
        if self.data.get(&account_id).is_some() {
            Ok(None)
        } else {
            let key = cosmrs::bip32::XPrv::derive_from_path(
                mnemonic.to_seed(""),
                &utils::DERIVATION_PATH.parse().unwrap(),
            )?;
            let public_key = key.public_key().to_string(cosmrs::bip32::Prefix::XPRV);
            let private_key = key.to_string(cosmrs::bip32::Prefix::XPRV).to_string();
            let keypair = KeyPair {
                public_key,
                private_key,
            };
            let signing_key: SigningKey = key.try_into()?;

            let mnemonic = mnemonic.to_string();

            let account_addr = signing_key
                .public_key()
                .account_id("juno")
                .unwrap()
                .to_string();
            let new_key = LocalAgentStorageEntry {
                account_addr,
                keypair,
                mnemonic,
                payable_account_id: None,
            };
            self.data.insert(account_id, new_key.clone());
            Ok(Some(new_key))
        }
    }

    /// Generate a new account_id to the local storage.
    pub fn generate_account(
        &mut self,
        account_id: AccountId,
        mnemonic: Option<String>,
    ) -> Result<(), Report> {
        match self.get(&account_id) {
            Some(_) => Err(eyre!(r#"Agent "{account_id}" already created"#)),
            None => {
                let validated_mnemonic = if let Some(phrase) = mnemonic {
                    Mnemonic::parse_normalized(&phrase)
                } else {
                    Mnemonic::generate(24)
                }?;
                self.insert(account_id.clone(), validated_mnemonic)?;
                self.display_account(&account_id);
                self.write_to_disk()?;
                Ok(())
            }
        }
    }

    pub fn display_account(&self, account_id: &str) {
        let new_account = self.get(account_id);
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({ account_id: new_account })).unwrap()
        );
    }

    pub fn get_agent_signing_key(&self, account_id: &AccountId) -> Result<bip32::XPrv, Report> {
        let entry = if let Some(entry) = self.get(account_id) {
            entry
        } else {
            return Err(eyre!("No agent key by this id"));
        };
        let mnemonic: Mnemonic = entry.mnemonic.parse()?;
        let key = cosmrs::bip32::XPrv::derive_from_path(
            mnemonic.to_seed(""),
            &utils::DERIVATION_PATH.parse().unwrap(),
        )?;
        Ok(key)
    }

    /// Retrieve an agent based on the key
    fn get(&self, account_id: &str) -> Option<&LocalAgentStorageEntry> {
        info!("Getting agent by id: {}", account_id);

        let found = self.data.get(account_id);

        if let Some(entry) = found {
            info!("Found agent: {:#?}", entry);
        }

        found
    }
}

impl Default for LocalAgentStorage {
    fn default() -> Self {
        Self::new()
    }
}
