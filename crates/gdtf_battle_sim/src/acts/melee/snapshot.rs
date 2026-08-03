use bevy::prelude::Entity;

use crate::{
    ganger::{Facing, Faction, Fight, Luck, Position, Stance, Tu},
    melee::MeleeWeaponHit,
    rng::{FightRng, InjuryRng, SeverityRng, ShotRng},
    weapon::DamageType,
};

pub(super) struct AttackerSnapshot<'a> {
        pub(super) entity:             Entity,
        pub(super) position:           Position,
        pub(super) stance:             Stance,
        pub(super) facing:             Facing,
        pub(super) fight:              Fight,
        pub(super) faction:            Faction,
        pub(super) luck:               Luck,
        pub(super) weapon:             MeleeWeaponHit<'a>,
        pub(super) tu_cost:            Tu,
        pub(super) strike_damage_type: DamageType,
                pub(super) shove:              crate::weapon::Shove,
}

pub(super) struct MeleeStreams<'a> {
        pub(super) fight:    &'a mut FightRng,
        pub(super) shot:     &'a mut ShotRng,
        pub(super) severity: &'a mut SeverityRng,
            pub(super) injury:   &'a mut InjuryRng,
}
