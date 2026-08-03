//! Deterministic battle RNG: root seed and labeled streams.

mod derivation;
mod safe_draw;
mod seeded;
pub(super) mod streams;
#[cfg(test)]
mod test;

pub use seeded::BattleSeed;
pub use streams::{
    DeploymentRng, FightRng, InjuryRng, LootRng, ProcgenRng, ReactionRng, SeverityRng, ShotRng,
};
