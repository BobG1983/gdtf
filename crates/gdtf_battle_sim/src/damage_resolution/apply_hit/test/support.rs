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

pub(super) fn a_ganger() -> Entity {
    World::new().spawn_empty().id()
}

pub(super) fn worn_piece_integrity(integrity: i32) -> ArmorIntegrity {
    ArmorIntegrity::new(integrity)
}

pub(super) fn hit(hp_damage: i32, wear: i32) -> HitResult {
    HitResult {
        penetrating: PenetratingDamage::new(hp_damage.max(0)),
        hp_damage:   HpDamage::new(hp_damage),
        wear:        IntegrityWear::new(wear),
    }
}
