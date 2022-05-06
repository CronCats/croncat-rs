//!
//! Helpers for dealing with local agents.
//!

use ed25519_dalek::Keypair;
use rand::rngs::OsRng;

pub fn generate_keypair() -> Keypair {
    let mut csprng = OsRng {};
    Keypair::generate(&mut csprng)
}
