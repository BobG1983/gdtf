//! Root seed for a battle.

/// u64 seed that derives all labeled RNG streams.
#[derive(
    bevy::prelude::Resource, bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default,
)]
pub struct BattleSeed(u64);

impl BattleSeed {
    /// Wrap a seed value.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// Read the seed.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for BattleSeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
