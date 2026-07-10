//! The **trajectory-style model** — the per-weapon [`TrajectoryStyle`] (`Straight` /
//! `Arc`) that selects how a weapon's projectile flies (GTW-546, child GTW-41d of
//! GTW-41).
//!
//! Most weapons fire a `Straight` round the §2 voxel DDA marches as a flat ray from the
//! muzzle ([`march_vector`](crate::march::march_vector)). A grenade / grenade-launcher
//! LOBS its charge over a **parabolic** [`Arc`](TrajectoryStyle::Arc): the arc rises over
//! same-level cover and comes down on a target cell at range, passing up/through a
//! [`SlabState::Destroyed`](crate::surface::SlabState) / `Absent` hole or window but
//! BLOCKED by an intact [`SlabState::Present`](crate::surface::SlabState) roof — the
//! [`march_arc`](crate::march::march_arc) traversal. A blind lob has NO line-of-sight
//! gate (`docs/combat/combat.md` names lobbed grenades among the advanced-effect weapons).

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// A weapon's **trajectory style** — the closed set of ways a weapon's projectile flies
/// (GTW-546): a flat [`Straight`](TrajectoryStyle::Straight) ray or a lobbed
/// [`Arc`](TrajectoryStyle::Arc) parabola.
///
/// A named domain enum (no-bare-types: a trajectory style is a domain value, not a bare
/// `bool`), a PER-WEAPON `#[derive(Component)]` sibling on the armed entity (the GTW-200
/// weapon-as-components model, like [`DamageType`](super::DamageType)): the fire path
/// reads it to pick the STRAIGHT [`march_vector`](crate::march::march_vector) or the
/// lobbed [`march_arc`](crate::march::march_arc). It also rides on the authoring
/// [`WeaponSpec`](super::WeaponSpec) (a `#[serde(default)]` field), so every EXISTING
/// weapon `.ron` — none of which author a `trajectory:` field — deserializes byte-identical
/// to a `Straight` weapon (the identity property, mirroring
/// [`HitType::Single`](super::HitType) / [`Shove`](super::Shove) opt-in defaults).
///
/// [`Straight`](TrajectoryStyle::Straight) is the [`Default`] (`#[serde(default)]` on the
/// spec field + a spawn-seed sentinel via `bsn!`), so an omitted `trajectory:` leaves a
/// weapon firing exactly as before GTW-546. `Deserialize` so a grenade's RON names its
/// style by variant; `Serialize` for the round-trip; `Copy`/`Eq` so it snapshots cheaply.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TrajectoryStyle {
    /// A **flat** projectile — the §2 voxel DDA marches it as a straight ray from the
    /// muzzle ([`march_vector`](crate::march::march_vector)). The DEFAULT: every existing
    /// weapon (a gun, a las, a bolter) flies straight.
    #[default]
    Straight,
    /// A **lobbed** projectile — the round follows a parabola from the muzzle to the target
    /// cell ([`march_arc`](crate::march::march_arc)): it rises over same-level cover and
    /// descends onto the target, passing through a
    /// [`SlabState::Destroyed`](crate::surface::SlabState) / `Absent` hole or window but
    /// BLOCKED by an intact roof. A blind lob needs no line of sight. The grenade / grenade
    /// launcher trajectory (GTW-546).
    Arc,
}

/// Whether a shot follows a lobbed parabolic arc rather than a straight line — the
/// throw / arc-march trajectory answer (GTW-546).
///
/// A named predicate newtype (no-bare-types: "the shot is lobbed" is a domain answer,
/// not a bare `bool`) over the [`TrajectoryStyle::is_arc`] read: `true` for an
/// [`Arc`](TrajectoryStyle::Arc) weapon (grenades / launchers), `false` for a
/// [`Straight`](TrajectoryStyle::Straight) one. Private inner, read through the derived
/// [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lobbed(bool);

impl TrajectoryStyle {
    /// Whether this style is a lobbed [`Arc`](TrajectoryStyle::Arc) — the throw / arc-march
    /// path (GTW-546). [`Lobbed`]`(false)` for a [`Straight`](TrajectoryStyle::Straight) weapon.
    #[must_use]
    pub const fn is_arc(self) -> Lobbed {
        Lobbed(matches!(self, Self::Arc))
    }
}
