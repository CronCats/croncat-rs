use std::{
    fs::{self, File},
    io::Write,
};

use bip39::Mnemonic;
use color_eyre::{eyre::eyre, Report};
use cosm_orc::config::key::{Key, SigningKey};

pub const MNEMO_FILENAME: &str = "agent-mnemo";

// TODO: make interactive ask to continue if file already exist
pub fn generate_save_mnemonic() -> Result<(), Report> {
    let mnemo = Mnemonic::generate(24).unwrap();
    let mut mnemo_file = File::create(MNEMO_FILENAME)?;
    mnemo_file.write_all(mnemo.to_string().as_bytes())?;
    Ok(())
}

pub fn get_agent_signing_key() -> Result<SigningKey, Report> {
    let mnemo = fs::read_to_string(MNEMO_FILENAME)
        .map_err(|err| eyre!("Generate mnemonic first: {err}"))?;
    let key = SigningKey {
        name: "agent".to_string(),
        key: Key::Mnemonic(mnemo),
    };
    Ok(key)
}
