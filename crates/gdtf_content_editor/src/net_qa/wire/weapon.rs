//! The Weapon form's own field values: ballistics, handling, magazine, DOT and on-death.

use bevy::prelude::Deref;
use gdtf_battle_sim::{
    effects::{fields::FieldKey, on_death::ExplodeDamage},
    magazine::ReloadTu,
    weapon::{
        Accuracy, BaseSpread, DotDamage, Kickback, MagazineSize, Stable, TrajectoryStyle,
        WeaponSpec,
    },
};
use serde::{Deserialize, Serialize};

/// The cone a weapon opens before any mode multiplier.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct BaseSpreadNet(f32);

impl BaseSpreadNet {
    /// Wrap a base spread a client sent or the spec holds.
    pub(in crate::net_qa) const fn new(spread: f32) -> Self {
        Self(spread)
    }

    /// Read a client's base spread back as the sim's own.
    pub(in crate::net_qa) const fn to_spread(self) -> BaseSpread {
        BaseSpread::new(self.0)
    }
}

/// The accuracy a weapon lends its wielder.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct AccuracyNet(f32);

impl AccuracyNet {
    /// Wrap an accuracy a client sent or the spec holds.
    pub(in crate::net_qa) const fn new(accuracy: f32) -> Self {
        Self(accuracy)
    }

    /// Read a client's accuracy back as the sim's own.
    pub(in crate::net_qa) const fn to_accuracy(self) -> Accuracy {
        Accuracy::new(self.0)
    }
}

/// The kick a weapon throws onto the next shot.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct KickbackNet(f32);

impl KickbackNet {
    /// Wrap a kickback a client sent or the spec holds.
    pub(in crate::net_qa) const fn new(kickback: f32) -> Self {
        Self(kickback)
    }

    /// Read a client's kickback back as the sim's own.
    pub(in crate::net_qa) const fn to_kickback(self) -> Kickback {
        Kickback::new(self.0)
    }
}

/// The path a shot flies, mirroring the two the form's combo offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum TrajectoryStyleNet {
    /// Flat, blocked by anything in the way.
    Straight,
    /// Lobbed over cover.
    Arc,
}

impl TrajectoryStyleNet {
    /// Mirror the sim's own trajectory.
    pub(in crate::net_qa) const fn from_style(style: TrajectoryStyle) -> Self {
        match style {
            TrajectoryStyle::Straight => Self::Straight,
            TrajectoryStyle::Arc => Self::Arc,
        }
    }

    /// Read a client's trajectory back as the sim's own.
    pub(in crate::net_qa) const fn to_style(self) -> TrajectoryStyle {
        match self {
            Self::Straight => TrajectoryStyle::Straight,
            Self::Arc => TrajectoryStyle::Arc,
        }
    }
}

/// Whether a weapon is braced by design.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct StableNet(bool);

impl StableNet {
    /// Wrap the flag a client sent or the spec holds.
    pub(in crate::net_qa) const fn new(stable: bool) -> Self {
        Self(stable)
    }

    /// Read a client's flag back as the sim's own.
    pub(in crate::net_qa) const fn to_stable(self) -> Stable {
        Stable::new(self.0)
    }
}

/// How many rounds a magazine holds.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct MagazineSizeNet(u16);

impl MagazineSizeNet {
    /// Wrap a magazine size a client sent or the spec holds.
    pub(in crate::net_qa) const fn new(size: u16) -> Self {
        Self(size)
    }

    /// Read a client's magazine size back as the sim's own.
    pub(in crate::net_qa) const fn to_size(self) -> MagazineSize {
        MagazineSize::new(self.0)
    }
}

/// The time units a reload charges.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct ReloadTuNet(u8);

impl ReloadTuNet {
    /// Wrap a reload cost a client sent or the spec holds.
    pub(in crate::net_qa) const fn new(reload: u8) -> Self {
        Self(reload)
    }

    /// Read a client's reload cost back as the sim's own.
    pub(in crate::net_qa) const fn to_reload_tu(self) -> ReloadTu {
        ReloadTu::new(self.0)
    }
}

/// Whether the weapon authors a damage-over-time profile at all.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct DotEnabledNet(bool);

impl DotEnabledNet {
    /// Wrap a tick box, for a case that spells one a client could send.
    #[cfg(test)]
    pub(in crate::net_qa) const fn new(enabled: bool) -> Self {
        Self(enabled)
    }

    /// Whether the profile is authored.
    pub(in crate::net_qa) const fn is_enabled(self) -> bool {
        self.0
    }

    /// Read the tick box off a spec, the way the form's own checkbox reads it.
    pub(in crate::net_qa) const fn from_spec(spec: &WeaponSpec) -> Self {
        Self(spec.dot.is_some())
    }
}

/// The hit points a DOT tick takes each turn.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct DotDamageNet(u16);

impl DotDamageNet {
    /// Wrap a per-turn damage a client sent or the profile holds.
    pub(in crate::net_qa) const fn new(damage: u16) -> Self {
        Self(damage)
    }

    /// Read a client's per-turn damage back as the sim's own.
    pub(in crate::net_qa) const fn to_damage(self) -> DotDamage {
        DotDamage::new(self.0)
    }
}

/// How many turns a DOT profile runs for.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct DotTurnsNet(u8);

impl DotTurnsNet {
    /// Wrap a turn count a client sent or the profile holds.
    pub(in crate::net_qa) const fn new(turns: u8) -> Self {
        Self(turns)
    }
}

/// The damage an on-death blast deals.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct ExplodeDamageNet(u16);

impl ExplodeDamageNet {
    /// Wrap a blast damage a client sent or the effect holds.
    pub(in crate::net_qa) const fn new(damage: u16) -> Self {
        Self(damage)
    }

    /// Read a client's blast damage back as the sim's own.
    pub(in crate::net_qa) const fn to_damage(self) -> ExplodeDamage {
        ExplodeDamage::new(self.0)
    }
}

/// The registry key of the field an on-death effect leaves behind.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct FieldKeyNet(String);

impl FieldKeyNet {
    /// Mirror the sim's own field key.
    pub(in crate::net_qa) fn from_key(key: &FieldKey) -> Self {
        Self(key.as_str().to_owned())
    }

    /// Read a client's field key back as the sim's own.
    pub(in crate::net_qa) fn to_key(&self) -> FieldKey {
        FieldKey::new(self.0.clone())
    }
}
