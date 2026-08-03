//! Derive per-stream seeds from the battle root via FNV-1a.

use bevy::prelude::Deref;

use super::seeded::BattleSeed;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Seed for one labeled stream.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct StreamSeed(u64);

impl StreamSeed {
    #[must_use]
    pub(super) const fn new(seed: u64) -> Self {
        Self(seed)
    }

    #[must_use]
    pub(super) const fn get(self) -> u64 {
        self.0
    }
}

/// Mix root seed and stream label into a stream seed.
pub(super) const fn fnv1a64(root: BattleSeed, label: &[u8]) -> StreamSeed {
    let root_bytes = root.get().to_le_bytes();
    let mut hash = FNV_OFFSET;
    let mut i = 0usize;
    while i < root_bytes.len() {
        hash ^= root_bytes[i] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    let mut j = 0usize;
    while j < label.len() {
        hash ^= label[j] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        j += 1;
    }
    StreamSeed::new(hash)
}

/// Stable byte label for a stream family.
pub struct StreamLabel(&'static [u8]);

impl StreamLabel {
    /// Wrap a static label.
    #[must_use]
    pub const fn new(b: &'static [u8]) -> Self {
        Self(b)
    }

    pub(super) const fn as_bytes(&self) -> &[u8] {
        self.0
    }
}
