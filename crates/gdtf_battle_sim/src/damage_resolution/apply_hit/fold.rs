use bevy::prelude::Entity;

use crate::{
    armor::{ArmorIntegrity, BodyPart},
    armor_wear::{ArmorWearOutcome, wear_armor},
    ganger::{Hp, LifeState, Wounds},
    inflicted_wound::{InflictedWound, InflictedWounds},
    resolve_hit::{HitResult, HpDamage},
    severity::Severity,
    tuning::{CombatTuning, WoundCost, WoundCosts},
};

pub struct GangerHitTarget<'a> {
        pub hp:        &'a mut Hp,
        pub wounds:    &'a mut Wounds,
        pub life:      &'a mut LifeState,
                    pub integrity: Option<&'a mut ArmorIntegrity>,
                pub inflicted: &'a mut InflictedWounds,
}

#[must_use]
pub(super) const fn wound_cost(severity: Severity, costs: WoundCosts) -> WoundCost {
    match severity {
        Severity::None | Severity::Fatal => WoundCost::new(0),
        Severity::Minor => costs.minor,
        Severity::Major => costs.major,
        Severity::Critical => costs.critical,
    }
}

/// localized `#[expect]` is the crate's guarded-cast idiom (see
fn hp_damage_to_u16(damage: HpDamage) -> Hp {
    #[expect(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        reason = "clamped into [0, u16::MAX] first, so the cast can neither wrap nor lose a sign"
    )]
    let clamped = damage.clamp(0, i32::from(u16::MAX)) as u16;
    Hp::new(clamped)
}

#[must_use]
pub fn apply_hit(
    target: GangerHitTarget<'_>,
    hit: &HitResult,
    severity: Severity,
    part: BodyPart,
    ganger: Entity,
    tuning: &CombatTuning,
) -> ArmorWearOutcome {
    if *target.life == LifeState::Dead {
        return ArmorWearOutcome::Unaffected;
    }

    let hp_loss = hp_damage_to_u16(hit.hp_damage);
    *target.hp = Hp::new(target.hp.saturating_sub(*hp_loss));

    if severity == Severity::Fatal {
        *target.wounds = Wounds::new(0);
    } else {
        let cost = *wound_cost(severity, tuning.wound_costs);
        *target.wounds = Wounds::new(target.wounds.saturating_sub(cost));
    }

    if severity != Severity::None {
        target.inflicted.record(InflictedWound::new(severity, part));
    }

    let wear_outcome = match target.integrity {
        Some(integrity) => wear_armor(integrity, part, hit.wear, ganger),
        None => ArmorWearOutcome::Unaffected,
    };

    if *target.wounds == Wounds::new(0) {
        *target.life = LifeState::Dead;
    } else if *target.hp == Hp::new(0) {
        *target.life = LifeState::Downed;
    }

    wear_outcome
}
