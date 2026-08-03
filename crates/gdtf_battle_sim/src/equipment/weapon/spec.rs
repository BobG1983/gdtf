//! The **authoring spec** — the `WeaponSpec` an `assets/content/weapons/ranged/*.weapon.ron`
use bevy::{prelude::Component, reflect::TypePath};
use serde::{Deserialize, Serialize};

use super::{
    Accuracy, AmmoType, BaseSpread, DamageProfile, DamageType, DotProfile, FatalBias, FireMode,
    Handedness, HandlingProfile, Kickback, Shove, Stable, TrajectoryStyle, WeaponBundle,
    WeaponDamage, WeaponName, WeaponPunch, WeaponShred,
};
use crate::{
    effects::attachments::AttachmentEffect,
    equipment::attachments::{AttachmentName, WeaponSlots},
    magazine::Magazine,
};

/// `#[serde(transparent)]` bare RON scalar (the [`crate::tuning`] / GTW-200 house
/// mirror); the [`Magazine`]'s live `rounds` count is `#[serde(skip_serializing)]` on its
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TypePath)]
pub struct WeaponSpec {
        pub base_spread: BaseSpread,
        pub accuracy:    Accuracy,
        pub kickback:    Kickback,
        pub fatal_bias:  FatalBias,
        pub damage:      WeaponDamage,
        pub punch:       WeaponPunch,
        pub shred:       WeaponShred,
        pub damage_type: DamageType,
                                /// not the wheel node its hits resolve on. `#[serde(default)]` (defaulting to
                #[serde(default)]
    pub accepts:     AmmoType,
                    pub magazine:    Magazine,
                pub fire_mode:   FireMode,
        pub stable:      Stable,
        /// connecting shot (in addition to the shot's damage). `#[serde(default)]` so an
            /// shoving OFF (the [`Reach`](super::Reach) `#[serde(default)]` precedent), unlike
        #[serde(default)]
    pub shove:       Shove,
                    pub handedness:  Handedness,
            /// field. `#[serde(default)]` (defaulting to [`TrajectoryStyle::Straight`]) so an omitted
        /// `#[serde(default)]` precedent), so EVERY existing weapon `.ron` — none of which author
                #[serde(default)]
    pub trajectory:  TrajectoryStyle,
                    /// attachments each holds. `#[serde(default)]` so an omitted field is the EMPTY
                        #[serde(default)]
    pub slots:       WeaponSlots,
                /// the `attachments:` `.weapon.ron` field. `#[serde(default)]` so an omitted field falls
        /// [`Shove`] `#[serde(default)]` precedent), so EVERY existing weapon `.ron` — none of which
                                        #[serde(default)]
    pub attachments: Vec<AttachmentName>,
            /// authored as the `dot:` `.weapon.ron` field. `#[serde(default)]` (defaulting to `None`)
        /// [`Shove`] `#[serde(default)]` precedent), so EVERY existing weapon `.ron` — none of
                        #[serde(default)]
    pub dot:         Option<DotProfile>,
                /// `.weapon.ron` field. `#[serde(default)]` (defaulting to `None`) so an omitted field is a
        /// [`Shove`] `#[serde(default)]` precedent), so EVERY existing weapon `.ron` — none of which
                        #[serde(default)]
    pub on_death:    Option<crate::effects::on_death::OnDeathEffect>,
}

impl WeaponSpec {
                                                                                #[must_use]
    pub fn into_bundle(self, name: WeaponName) -> (WeaponBundle, WeaponSpawnSiblings) {
        let magazine = Magazine::loaded_with(
            self.magazine.size(),
            self.magazine.reload_tu(),
            self.accepts,
        );
        let bundle = WeaponBundle::new(
            name,
            self.base_spread,
            self.accuracy,
            self.kickback,
            self.fatal_bias,
            DamageProfile::new(self.damage, self.punch, self.shred, self.damage_type),
            HandlingProfile::new(
                magazine,
                self.fire_mode,
                self.stable,
                self.shove,
                self.handedness,
            )
            .with_trajectory(self.trajectory),
        );
        let siblings = WeaponSpawnSiblings::new(self.dot, self.on_death);
        (bundle, siblings)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct WeaponSpawnSiblings {
                dot:      Option<DotProfile>,
                on_death: Option<crate::effects::on_death::OnDeath>,
}

impl WeaponSpawnSiblings {
            #[must_use]
    pub fn new(
        dot: Option<DotProfile>,
        on_death: Option<crate::effects::on_death::OnDeathEffect>,
    ) -> Self {
        Self {
            dot,
            on_death: on_death.map(crate::effects::on_death::OnDeath::new),
        }
    }

            #[must_use]
    pub const fn dot(&self) -> Option<DotProfile> {
        self.dot
    }

            #[must_use]
    pub const fn on_death(&self) -> Option<&crate::effects::on_death::OnDeath> {
        self.on_death.as_ref()
    }
}

#[derive(Component, Debug, Clone, PartialEq, Default)]
pub struct PendingAttachments(Vec<AttachmentEffect>);

impl PendingAttachments {
            #[must_use]
    pub const fn new(effects: Vec<AttachmentEffect>) -> Self {
        Self(effects)
    }

                #[must_use]
    pub fn effects(&self) -> &[AttachmentEffect] {
        &self.0
    }
}
