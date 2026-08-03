use crate::{
    rng::{BattleSeed, FightRng, InjuryRng, ReactionRng, SeverityRng, ShotRng},
    slab::SlabLedger,
};

#[must_use]
pub fn shot_rng(seed: u64) -> ShotRng {
    ShotRng::from_root(BattleSeed::new(seed))
}

#[must_use]
pub fn severity_rng(seed: u64) -> SeverityRng {
    SeverityRng::from_root(BattleSeed::new(seed))
}

#[must_use]
pub fn injury_rng(seed: u64) -> InjuryRng {
    InjuryRng::from_root(BattleSeed::new(seed))
}

#[must_use]
pub fn fight_rng(seed: u64) -> FightRng {
    FightRng::from_root(BattleSeed::new(seed))
}

#[must_use]
pub fn reaction_rng(seed: u64) -> ReactionRng {
    ReactionRng::from_root(BattleSeed::new(seed))
}

#[must_use]
pub fn empty_slab_ledger() -> SlabLedger {
    SlabLedger::new()
}
