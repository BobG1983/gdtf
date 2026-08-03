use bevy::prelude::Bundle;

use super::{FightMode, MeleeWeapon, Reach};
use crate::weapon::{
    DamageType, FatalBias, Handedness, Shove, WeaponDamage, WeaponName, WeaponPunch, WeaponShred,
};

#[derive(Bundle, Debug, Clone, PartialEq)]
pub struct MeleeWeaponBundle {
            pub marker:      MeleeWeapon,
        pub name:        WeaponName,
        pub damage:      WeaponDamage,
        pub punch:       WeaponPunch,
        pub shred:       WeaponShred,
        pub damage_type: DamageType,
        pub fatal_bias:  FatalBias,
        pub handedness:  Handedness,
        pub reach:       Reach,
        pub fight_mode:  FightMode,
                    pub shove:       Shove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeleeDamageProfile {
        pub damage:      WeaponDamage,
        pub punch:       WeaponPunch,
        pub shred:       WeaponShred,
        pub damage_type: DamageType,
}

impl MeleeDamageProfile {
            #[must_use]
    pub const fn new(
        damage: WeaponDamage,
        punch: WeaponPunch,
        shred: WeaponShred,
        damage_type: DamageType,
    ) -> Self {
        Self {
            damage,
            punch,
            shred,
            damage_type,
        }
    }
}

impl MeleeWeaponBundle {
                                        #[must_use]
    pub const fn new(
        name: WeaponName,
        damage: MeleeDamageProfile,
        fatal_bias: FatalBias,
        handedness: Handedness,
        reach: Reach,
        fight_mode: FightMode,
        shove: Shove,
    ) -> Self {
        Self {
            marker: MeleeWeapon,
            name,
            damage: damage.damage,
            punch: damage.punch,
            shred: damage.shred,
            damage_type: damage.damage_type,
            fatal_bias,
            handedness,
            reach,
            fight_mode,
            shove,
        }
    }
}
