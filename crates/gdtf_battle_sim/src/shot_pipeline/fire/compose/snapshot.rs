use bevy::prelude::{Deref, Entity};

use super::super::query::{MeleeQuery, MountedQuery, ShooterQuery, WeaponQuery, WieldsQuery};
use crate::{
    aim::Shooter,
    effects::attachments::WeaponBraceBonus,
    ganger::{Aiming, Facing, Luck, Position, Shooting, Stance, Suppressed, Tu, TuMax},
    injuries::{HandsAvailable, InflictedInjuries},
    magazine::Magazine,
    stability::EmplacementStability,
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageType, FatalBias, Handedness, Kickback, Stable, WeaponDamage,
        WeaponPunch, WeaponShred, WeaponStats,
    },
};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MountedShot(bool);

impl MountedShot {
        pub(super) const fn new(mounted: bool) -> Self {
        Self(mounted)
    }
}

pub(in crate::shot_pipeline::fire) struct ShooterSnapshot {
    pub(super) position:    Position,
    pub(super) facing:      Facing,
    pub(super) stance:      Stance,
    pub(super) aiming:      Aiming,
    pub(super) shooting:    Shooting,
    pub(super) luck:        Luck,
    pub(super) base_spread: BaseSpread,
    pub(super) accuracy:    Accuracy,
    pub(super) kickback:    Kickback,
    pub(super) fatal_bias:  FatalBias,
    pub(super) damage:      WeaponDamage,
    pub(super) punch:       WeaponPunch,
    pub(super) shred:       WeaponShred,
    pub(super) damage_type: DamageType,
    pub(super) stable:      Stable,
    pub(super) brace_bonus: Option<WeaponBraceBonus>,
    pub(super) suppressed:  Option<Suppressed>,
    pub(super) mounted:     MountedShot,
    pub(super) dot:         Option<crate::weapon::DotProfile>,
}

impl ShooterSnapshot {
                    pub(super) const fn weapon_stats(&self) -> WeaponStats<'_> {
        WeaponStats {
            base_spread: &self.base_spread,
            accuracy:    &self.accuracy,
            kickback:    &self.kickback,
            fatal_bias:  &self.fatal_bias,
            damage:      &self.damage,
            punch:       &self.punch,
            shred:       &self.shred,
            damage_type: &self.damage_type,
            stable:      &self.stable,
            brace_bonus: self.brace_bonus.as_ref(),
            dot:         self.dot.as_ref(),
        }
    }

            pub(super) const fn shooter_view(&self) -> Shooter<'_> {
        Shooter {
            stance:     &self.stance,
            aiming:     &self.aiming,
            position:   &self.position,
            facing:     &self.facing,
            suppressed: self.suppressed.as_ref(),
        }
    }

                                            pub(super) fn emplacement_stability(&self, tuning: &CombatTuning) -> EmplacementStability {
        if *self.mounted {
            EmplacementStability::new(*tuning.cone_stability.emplacement_stability_bonus)
        } else {
            EmplacementStability::none()
        }
    }
}

pub(in crate::shot_pipeline::fire) struct ShooterReads {
        pub(in crate::shot_pipeline::fire) snapshot:        ShooterSnapshot,
        pub(in crate::shot_pipeline::fire) weapon:          Entity,
        pub(in crate::shot_pipeline::fire) tu:              Tu,
        pub(in crate::shot_pipeline::fire) tu_max:          TuMax,
        pub(in crate::shot_pipeline::fire) aiming:          Aiming,
        pub(in crate::shot_pipeline::fire) magazine:        Magazine,
            pub(in crate::shot_pipeline::fire) handedness:      Handedness,
                pub(in crate::shot_pipeline::fire) hands_available: HandsAvailable,
}

pub(in crate::shot_pipeline::fire) fn read_shooter(
    shooter: Entity,
    shooters: &ShooterQuery,
    wields: &WieldsQuery,
    weapons: &WeaponQuery,
    melee: &MeleeQuery,
    mounted: &MountedQuery,
) -> Option<ShooterReads> {
    let ((position, facing, stance, aiming, shooting, luck, tu_max, suppressed), injuries, tu) =
        shooters.get(shooter).ok()?;
    let effective_shooter_luck = match injuries {
        Some(ledger) => crate::ganger::effective_luck(*luck, ledger),
        None => *luck,
    };
    let hands_available =
        injuries.map_or_else(HandsAvailable::default, InflictedInjuries::hands_available);
    let wielded = wields.get(shooter).ok()?;
    let weapon = wielded.firing_weapon(
        |entity| mounted.get(entity).is_ok(),
        |entity| melee.get(entity).is_ok(),
    )?;
    let is_mounted = mounted.get(weapon).is_ok();
    let (
        base_spread,
        accuracy,
        kickback,
        fatal_bias,
        damage,
        punch,
        shred,
        damage_type,
        stable,
        brace_bonus,
        handedness,
        magazine,
        dot,
    ) = weapons.get(weapon).ok()?;
    let snapshot = ShooterSnapshot {
        position:    *position,
        facing:      *facing,
        stance:      *stance,
        aiming:      *aiming,
        shooting:    *shooting,
        luck:        effective_shooter_luck,
        base_spread: *base_spread,
        accuracy:    *accuracy,
        kickback:    *kickback,
        fatal_bias:  *fatal_bias,
        damage:      *damage,
        punch:       *punch,
        shred:       *shred,
        damage_type: *damage_type,
        stable:      *stable,
        brace_bonus: brace_bonus.copied(),
        suppressed:  suppressed.copied(),
        mounted:     MountedShot::new(is_mounted),
        dot:         dot.copied(),
    };
    Some(ShooterReads {
        snapshot,
        weapon,
        tu: *tu,
        tu_max: *tu_max,
        aiming: *aiming,
        magazine: *magazine,
        handedness: *handedness,
        hands_available,
    })
}
