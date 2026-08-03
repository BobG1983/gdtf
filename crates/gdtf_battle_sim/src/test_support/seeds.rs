//! Deterministic RNG and ledger seeds for tests.

use crate::{
    rng::{BattleSeed, FightRng, InjuryRng, ReactionRng, SeverityRng, ShotRng},
    slab::SlabLedger,
};

/// Shot stream from a fixed seed.
#[must_use]
pub fn shot_rng(seed: u64) -> ShotRng {
    ShotRng::from_root(BattleSeed::new(seed))
}

/// Severity stream from a fixed seed.
#[must_use]
pub fn severity_rng(seed: u64) -> SeverityRng {
    SeverityRng::from_root(BattleSeed::new(seed))
}

/// Injury stream from a fixed seed.
#[must_use]
pub fn injury_rng(seed: u64) -> InjuryRng {
    InjuryRng::from_root(BattleSeed::new(seed))
}

/// Fight stream from a fixed seed.
#[must_use]
pub fn fight_rng(seed: u64) -> FightRng {
    FightRng::from_root(BattleSeed::new(seed))
}

/// Reaction stream from a fixed seed.
#[must_use]
pub fn reaction_rng(seed: u64) -> ReactionRng {
    ReactionRng::from_root(BattleSeed::new(seed))
}

/// Empty slab ledger.
#[must_use]
pub fn empty_slab_ledger() -> SlabLedger {
    SlabLedger::new()
}
