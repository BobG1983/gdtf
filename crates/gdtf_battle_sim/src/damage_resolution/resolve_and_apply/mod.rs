//! authoritative-model role this crate plays in the model/view split (ADR-0001,
mod fold;
mod kinds;
mod report;
mod wound_core;

#[cfg(test)]
mod test;

pub use fold::resolve_and_apply;
pub(crate) use kinds::cover::{cover_armor_piece, cover_damage_from_hp};
pub use kinds::{
    cover::CoverVerdict, ganger::AppliedDamage, ganger::GangerVerdict, ground::GroundAccrual,
    slab::SlabVerdict,
};
pub use report::{HitReport, HitVerdict, Protecting, StruckPiece, StruckSurfaces, TargetGanger};
pub(crate) use wound_core::{WoundBlow, WoundCoreInputs, synthesize_wound};
