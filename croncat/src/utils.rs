//!
//! Helpers for dealing with local agents.
//!

use secp256k1::{rand, KeyPair, Secp256k1};

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

/// Generate a secp256k1 keypair from a random OS rng input
pub fn generate_keypair() -> KeyPair {
    let secp = Secp256k1::new();
    KeyPair::new(&secp, &mut rand::thread_rng())
}
use std::{
    fs::{self, File},
    io::Write,
};

use bip39::Mnemonic;
use color_eyre::{eyre::eyre, Report};
use cosm_orc::config::key::{Key, SigningKey};



///
/// Count block received from the stream.
///
pub struct AtomicIntervalCounter {
    count: Arc<AtomicU64>,
    check_interval: u64,
}

impl AtomicIntervalCounter {
    /// Create a new [`AtomicIntervalCounter`] and check every 10 samples.
    pub fn new(interval: u64) -> Self {
        Self {
            count: Arc::new(AtomicU64::default()),
            check_interval: interval,
        }
    }

    /// Increase the current offset into the sample.
    pub fn tick(&self) {
        self.count.fetch_add(1, Ordering::SeqCst);
    }

    /// Determine if the count is a multiple of the integer interval.
    pub fn is_at_interval(&self) -> bool {
        let current_count = self.count.load(Ordering::Relaxed);

        current_count > 0 && current_count % self.check_interval == 0
    }
}


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
