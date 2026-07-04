//! Value-level seeding knobs (GTW-576) — fresh per-subsystem RNG streams and ledger
//! values for `World`/`SystemState` tests that pin a seed or ledger state WITHOUT an
//! app harness, so no test re-spells the `from_root`/`::new()` litany inline.

use crate::{
    rng::{BattleSeed, FightRng, InjuryRng, ReactionRng, SeverityRng, ShotRng},
    slab::SlabLedger,
};

/// A fresh [`ShotRng`] stream from a raw seed — the value-level knob a
/// `World`/`SystemState` test pins its cone/body-part draws through instead of
/// re-spelling the `from_root` litany inline.
#[must_use]
pub fn shot_rng(seed: u64) -> ShotRng {
    ShotRng::from_root(BattleSeed::new(seed))
}

/// A fresh [`SeverityRng`] stream from a raw seed (the wound-severity draw).
#[must_use]
pub fn severity_rng(seed: u64) -> SeverityRng {
    SeverityRng::from_root(BattleSeed::new(seed))
}

/// A fresh [`InjuryRng`] stream from a raw seed (the injury-roll draw).
#[must_use]
pub fn injury_rng(seed: u64) -> InjuryRng {
    InjuryRng::from_root(BattleSeed::new(seed))
}

/// A fresh [`FightRng`] stream from a raw seed (the melee opposed-Fight draw).
#[must_use]
pub fn fight_rng(seed: u64) -> FightRng {
    FightRng::from_root(BattleSeed::new(seed))
}

/// A fresh [`ReactionRng`] stream from a raw seed (the reaction-fire trigger draw).
#[must_use]
pub fn reaction_rng(seed: u64) -> ReactionRng {
    ReactionRng::from_root(BattleSeed::new(seed))
}

/// An EMPTY [`SlabLedger`] — the seeding knob for a test that needs a fresh ledger
/// value (a `SystemState` `fire()` call) or pins ledger state before a run.
#[must_use]
pub fn empty_slab_ledger() -> SlabLedger {
    SlabLedger::new()
}
