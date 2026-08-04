//! Authored ranged weapon content and spawn helpers.

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

/// Deserialized weapon definition from content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TypePath)]
pub struct WeaponSpec {
    /// Base spread.
    pub base_spread: BaseSpread,
    /// Accuracy.
    pub accuracy:    Accuracy,
    /// Kickback.
    pub kickback:    Kickback,
    /// Fatal bias.
    pub fatal_bias:  FatalBias,
    /// Damage.
    pub damage:      WeaponDamage,
    /// Punch.
    pub punch:       WeaponPunch,
    /// Shred.
    pub shred:       WeaponShred,
    /// Damage type.
    pub damage_type: DamageType,
    /// Accepted ammo class.
    #[serde(default)]
    pub accepts:     AmmoType,
    /// Magazine template (size / reload; live rounds filled on spawn).
    pub magazine:    Magazine,
    /// Fire modes.
    pub fire_mode:   FireMode,
    /// Stability.
    pub stable:      Stable,
    /// Shove on hit.
    #[serde(default)]
    pub shove:       Shove,
    /// Handedness.
    pub handedness:  Handedness,
    /// Trajectory.
    #[serde(default)]
    pub trajectory:  TrajectoryStyle,
    /// Attachment slots.
    #[serde(default)]
    pub slots:       WeaponSlots,
    /// Pre-fitted attachment names.
    #[serde(default)]
    pub attachments: Vec<AttachmentName>,
    /// Optional DOT on hit.
    #[serde(default)]
    pub dot:         Option<DotProfile>,
    /// Optional on-death effect.
    #[serde(default)]
    pub on_death:    Option<crate::effects::on_death::OnDeathEffect>,
}

impl WeaponSpec {
    /// Build spawn bundle plus optional sibling components.
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

/// Extra components to attach after the main bundle (DOT, on-death).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WeaponSpawnSiblings {
    dot:      Option<DotProfile>,
    on_death: Option<crate::effects::on_death::OnDeath>,
}

impl WeaponSpawnSiblings {
    /// Build siblings from optional authored effects.
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

    /// Optional DOT profile.
    #[must_use]
    pub const fn dot(&self) -> Option<DotProfile> {
        self.dot
    }

    /// Optional on-death effect.
    #[must_use]
    pub const fn on_death(&self) -> Option<&crate::effects::on_death::OnDeath> {
        self.on_death.as_ref()
    }
}

/// Attachment effects waiting to be applied after spawn.
#[derive(Component, Debug, Clone, PartialEq, Default)]
pub struct PendingAttachments(Vec<AttachmentEffect>);

impl PendingAttachments {
    /// Wrap a list of effects.
    #[must_use]
    pub const fn new(effects: Vec<AttachmentEffect>) -> Self {
        Self(effects)
    }

    /// Borrow the effects.
    #[must_use]
    pub fn effects(&self) -> &[AttachmentEffect] {
        &self.0
    }
}
