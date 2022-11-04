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

use crate::{
    config::{ChainConfig, Config},
    errors::Report,
    utils::{DERIVATION_PATH, SUPPORTED_CHAIN_IDS},
};

use super::get_storage_path;

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
    keypair: KeyPair,
    pub mnemonic: String,
    pub payable_account_id: Option<String>,
}

/// Hide the user mnemonic from the user when debug printing.
impl std::fmt::Debug for LocalAgentStorageEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalAgentStorageEntry")
            .field("keypair", &self.keypair)
            .field("payable_account_id", &self.payable_account_id)
            .finish()
    }
}

/// Store key pairs on disk and allow access to the data.
pub struct LocalAgentStorage {
    pub path: PathBuf,
    data: LocalAgentStorageData,
}

impl LocalAgentStorage {
    /// Create a new [`LocalAgentStorage`] instance with the default directory.
    pub fn new() -> Self {
        Self::from_path(get_storage_path())
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
        if let Some(p) = agent_data_file.parent() {
            fs::create_dir_all(p)?
        };

        fs::write(agent_data_file, serde_json::to_string_pretty(&self.data)?)?;

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
                &DERIVATION_PATH.parse()?,
            )?;
            let public_key = key.public_key().to_string(cosmrs::bip32::Prefix::XPRV);
            let private_key = key.to_string(cosmrs::bip32::Prefix::XPRV).to_string();
            let keypair = KeyPair {
                public_key,
                private_key,
            };

            let mnemonic = mnemonic.to_string();

            let new_key = LocalAgentStorageEntry {
                keypair,
                mnemonic,
                payable_account_id: None,
            };
            self.data.insert(account_id, new_key.clone());
            Ok(Some(new_key))
        }
    }

    /// Generate a new account_id to the local storage.
    pub async fn generate_account(
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
                self.write_to_disk()?;
                self.display_addrs(&account_id).await?;
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

    pub async fn display_addrs(&self, account_id: &str) -> Result<(), Report> {
        // println!("Account Addresses for: {account_id}");
        // // Loop and print supported accounts for a keypair
        // for chain_id in SUPPORTED_CHAIN_IDS.iter() {
        //     let config = Config::from_pwd()?;

        //     let prefix = config.prefix;
        //     let account_addr =
        //         self.get_agent_signing_account_addr(&account_id.to_string(), prefix)?;

        //     println!("{}: {}", chain_id, account_addr);
        // }

        // println!("\n\nPlease fund the above accounts with their native token!\nYou will need enough funds to cover several transactions before rewards will start covering costs.\nYou only need to fund the address for the network you plan to run an agent on.\n\n");

        Ok(())
    }

    pub fn get_agent_signing_key(&self, account_id: &AccountId) -> Result<bip32::XPrv, Report> {
        let entry = if let Some(entry) = self.get(account_id) {
            entry
        } else {
            return Err(eyre!("Agent not found: {}", account_id));
        };
        let mnemonic: Mnemonic = entry.mnemonic.parse()?;
        let key =
            cosmrs::bip32::XPrv::derive_from_path(mnemonic.to_seed(""), &DERIVATION_PATH.parse()?)?;
        Ok(key)
    }

    pub fn get_agent_signing_account_addr(
        &self,
        account_id: &AccountId,
        prefix: String,
    ) -> Result<String, Report> {
        let key = self.get_agent_signing_key(account_id)?;
        let signing_key: SigningKey = key.try_into()?;

        Ok(signing_key
            .public_key()
            .account_id(prefix.as_str())?
            .to_string())
    }

    /// Retrieve an agent based on the key
    fn get(&self, account_id: &str) -> Option<&LocalAgentStorageEntry> {
        info!("Getting agent by id: {}", account_id);

        let found = self.data.get(account_id);

        found
    }
}

impl Default for LocalAgentStorage {
    fn default() -> Self {
        Self::new()
    }
}
