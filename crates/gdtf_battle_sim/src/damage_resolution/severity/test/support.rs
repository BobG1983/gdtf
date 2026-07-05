//! Shared seeded-RNG + input-bundle fixtures for the severity tests — reached by
//! each concern file via `use super::support::*;`.

use super::super::{SeverityInputs, part_severity_mod};
use crate::{
    armor::BodyPart,
    ganger::{Luck, Toughness},
    resolve_hit::PenetratingDamage,
    rng::{BattleSeed, SeverityRng},
    weapon::FatalBias,
};

/// A fixed seed for the per-test RNG streams — determinism is the property, so
/// the same seed must reproduce the same draws (an arbitrary value, not tuned).
pub(super) const SEED: u64 = 0xC0FF_EE15;

/// Build a [`SeverityRng`] from the shared fixed seed (a fresh stream per call).
pub(super) fn rng() -> SeverityRng {
    SeverityRng::from_root(BattleSeed::new(SEED))
}

/// Build a severity-input bundle from arbitrary inputs — a helper so each test
/// varies only the field it exercises. Magnitudes are mechanism inputs, never
/// asserted as values.
pub(super) fn inputs(
    pen: i32,
    toughness: f32,
    part: BodyPart,
    luck_shooter: f32,
    luck_defender: f32,
) -> SeverityInputs {
    SeverityInputs::new(
        PenetratingDamage::new(pen),
        Toughness::new(toughness),
        part_severity_mod(part),
        FatalBias::new(0.0),
        Luck::new(luck_shooter),
        Luck::new(luck_defender),
    )
}
