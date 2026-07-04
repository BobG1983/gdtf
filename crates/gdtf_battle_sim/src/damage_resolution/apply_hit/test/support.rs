//! Shared fixtures for the E3.6 wound-application tests — a throwaway entity, a
//! uniform worn suit, and a per-test [`HitResult`] builder — plus the `pub(super)`
//! re-export of the symbols the AC test files exercise. Relocated verbatim from the
//! flat module's `tests` submodule.

pub(super) use bevy::prelude::{App, Entity, MinimalPlugins, Update, World};

pub(super) use super::super::{GangerHitTarget, apply_hit, fold::wound_cost};
pub(super) use crate::{
    armor::{ArmorIntegrity, BodyPart},
    armor_wear::{ArmorBroken, ArmorDamaged, ArmorWearOutcome},
    ganger::{Hp, LifeState, Wounds},
    inflicted_wound::{InflictedWound, InflictedWounds},
    resolve_hit::{HitResult, HpDamage, IntegrityWear, PenetratingDamage},
    severity::Severity,
    tuning::{CombatTuning, WoundCost, WoundCosts},
};

/// A real, valid [`Entity`] id to stand in for the owning ganger — spawned from
/// a throwaway [`World`] so the tests never hand-craft a raw id (0.18's
/// `from_raw_u32` is fallible; spawning yields a guaranteed-valid handle without
/// any `unwrap`). Mirrors `armor_wear::tests::a_ganger`.
pub(super) fn a_ganger() -> Entity {
    World::new().spawn_empty().id()
}

/// The struck piece's [`ArmorIntegrity`] component at `integrity` — an arbitrary
/// (NOT-shipped-tuning) magnitude; since GTW-323 (ADR-0004) `apply_hit` wears the
/// struck piece ENTITY's integrity component directly (via `GangerHitTarget.integrity`),
/// so the tests drive that component by `&mut` rather than a `WornArmor` array slot. A
/// hit's wear lands on it without first being soaked (the other armor stats are
/// irrelevant to these wear-path tests).
pub(super) fn worn_piece_integrity(integrity: i32) -> ArmorIntegrity {
    ArmorIntegrity::new(integrity)
}

/// Build a [`HitResult`] from arbitrary per-test magnitudes — mechanism inputs,
/// never asserted as values. `hp_damage` drives the HP loss, `wear` the armor
/// wear; `penetrating` is carried for completeness (`apply_hit` does not read it —
/// the severity it would have gated is passed in directly).
pub(super) fn hit(hp_damage: i32, wear: i32) -> HitResult {
    HitResult {
        penetrating: PenetratingDamage::new(hp_damage.max(0)),
        hp_damage:   HpDamage::new(hp_damage),
        wear:        IntegrityWear::new(wear),
    }
}
