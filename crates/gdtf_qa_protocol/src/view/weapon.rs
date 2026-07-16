//! The weapon view DTOs — the wielded weapon + its INDEXED fire-mode list (GTW-734).
//!
//! The wire mirror of a ganger's wielded weapon: its name plus the authored fire-mode
//! list flattened to `(index, label)` pairs. The index is exactly the
//! [`FireModeIndex`] a [`NetIntent::Fire`](crate::intent::NetIntent::Fire) selects; the
//! label is the sim
//! `ModeKind`'s `Display` string. The resolved `FireModeSpec` numbers (balance data,
//! some `f32`) never cross the wire.

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

use crate::ids::FireModeIndex;

/// A weapon's display **name** — the wire mirror of the sim `WeaponName` (`String`).
///
/// A name newtype (no-bare-types), serde-transparent. `Clone`-not-`Copy`.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WeaponNameNet(String);

impl WeaponNameNet {
    /// Build a weapon name from its display string.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// A fire mode's human-facing **label** — the sim `ModeKind`'s `Display` string (e.g.
/// `Single` / `Burst` / `Full`).
///
/// A label newtype (no-bare-types), serde-transparent. `Clone`-not-`Copy`.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FireModeLabel(String);

impl FireModeLabel {
    /// Build a fire-mode label from its display string.
    #[must_use]
    pub const fn new(label: String) -> Self {
        Self(label)
    }
}

/// One entry in a weapon's indexed fire-mode list — the `(index, label)` pair a QA
/// client selects a fire mode by.
///
/// The [`index`](Self::index) is the position into the weapon's authored mode list a
/// [`NetIntent::Fire`](crate::intent::NetIntent::Fire) names; the [`label`](Self::label)
/// is the mode's display string, so a client can pick "the Burst mode" by reading the
/// list. Serde default shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FireModeView {
    /// The mode's index in the weapon's authored list — the fire selector.
    pub index: FireModeIndex,
    /// The mode's human-facing display label.
    pub label: FireModeLabel,
}

impl FireModeView {
    /// Build a fire-mode entry from its list index and display label.
    #[must_use]
    pub const fn new(index: FireModeIndex, label: FireModeLabel) -> Self {
        Self { index, label }
    }
}

/// A ganger's wielded **weapon** — its name and its indexed fire-mode list.
///
/// The mirror of the sim weapon's `FireMode` selector, flattened to the wire: each
/// authored mode as a [`FireModeView`] `(index, label)`. Serde default shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WeaponView {
    /// The weapon's display name.
    pub name:  WeaponNameNet,
    /// The authored fire modes, each an `(index, label)` the client selects by.
    pub modes: Vec<FireModeView>,
}

impl WeaponView {
    /// Build a weapon view from its name and indexed fire-mode list.
    #[must_use]
    pub const fn new(name: WeaponNameNet, modes: Vec<FireModeView>) -> Self {
        Self { name, modes }
    }
}
