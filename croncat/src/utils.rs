//!
//! Helpers for dealing with local agents.
//!

use ed25519_dalek::Keypair;
use rand::rngs::OsRng;

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

/// Generate a ed25519 keypair from a random input
pub fn generate_keypair() -> Keypair {
    let mut csprng = OsRng {};
    Keypair::generate(&mut csprng)
}

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
