use bevy::prelude::Bundle;

use super::{
    Accuracy, BaseSpread, DamageType, DotProfile, FatalBias, FireMode, Handedness, Kickback, Shove,
    Stable, TrajectoryStyle, Weapon, WeaponDamage, WeaponName, WeaponPunch, WeaponShred,
};
use crate::{effects::attachments::WeaponBraceBonus, magazine::Magazine};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeaponStats<'a> {
        pub base_spread: &'a BaseSpread,
        pub accuracy:    &'a Accuracy,
        pub kickback:    &'a Kickback,
        pub fatal_bias:  &'a FatalBias,
        pub damage:      &'a WeaponDamage,
        pub punch:       &'a WeaponPunch,
        pub shred:       &'a WeaponShred,
        pub damage_type: &'a DamageType,
                pub stable:      &'a Stable,
                                    pub brace_bonus: Option<&'a WeaponBraceBonus>,
                            pub dot:         Option<&'a DotProfile>,
}

#[derive(Bundle, Debug, Clone, PartialEq)]
pub struct WeaponBundle {
        pub marker:      Weapon,
        pub name:        WeaponName,
        pub base_spread: BaseSpread,
        pub accuracy:    Accuracy,
        pub kickback:    Kickback,
        pub fatal_bias:  FatalBias,
        pub damage:      WeaponDamage,
        pub punch:       WeaponPunch,
        pub shred:       WeaponShred,
        pub damage_type: DamageType,
            pub magazine:    Magazine,
        pub fire_mode:   FireMode,
        pub stable:      Stable,
                pub shove:       Shove,
            pub handedness:  Handedness,
                /// [`march_arc`](crate::march::march_arc). Spawned from the spec's `#[serde(default)]`
        pub trajectory:  TrajectoryStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DamageProfile {
        pub damage:      WeaponDamage,
        pub punch:       WeaponPunch,
        pub shred:       WeaponShred,
        pub damage_type: DamageType,
}

impl DamageProfile {
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

#[derive(Debug, Clone, PartialEq)]
pub struct HandlingProfile {
            pub magazine:   Magazine,
        pub fire_mode:  FireMode,
        pub stable:     Stable,
            pub shove:      Shove,
        pub handedness: Handedness,
            pub trajectory: TrajectoryStyle,
}

impl HandlingProfile {
                                #[must_use]
    pub const fn new(
        magazine: Magazine,
        fire_mode: FireMode,
        stable: Stable,
        shove: Shove,
        handedness: Handedness,
    ) -> Self {
        Self {
            magazine,
            fire_mode,
            stable,
            shove,
            handedness,
            trajectory: TrajectoryStyle::Straight,
        }
    }

                    #[must_use]
    pub const fn with_trajectory(mut self, trajectory: TrajectoryStyle) -> Self {
        self.trajectory = trajectory;
        self
    }
}

impl WeaponBundle {
                                        #[must_use]
    pub fn new(
        name: WeaponName,
        base_spread: BaseSpread,
        accuracy: Accuracy,
        kickback: Kickback,
        fatal_bias: FatalBias,
        damage: DamageProfile,
        handling: HandlingProfile,
    ) -> Self {
        Self {
            marker: Weapon,
            name,
            base_spread,
            accuracy,
            kickback,
            fatal_bias,
            damage: damage.damage,
            punch: damage.punch,
            shred: damage.shred,
            damage_type: damage.damage_type,
            magazine: handling.magazine,
            fire_mode: handling.fire_mode,
            stable: handling.stable,
            shove: handling.shove,
            handedness: handling.handedness,
            trajectory: handling.trajectory,
        }
    }

                    #[must_use]
    pub const fn stats(&self) -> WeaponStats<'_> {
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
            brace_bonus: None,
            dot:         None,
        }
    }
}
