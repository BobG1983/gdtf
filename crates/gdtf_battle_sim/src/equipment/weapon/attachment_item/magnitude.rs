//! The per-item **magnitude newtypes** an [`AttachmentEffect`](super::AttachmentEffect)
//! carries where no existing weapon newtype fits — the GTW-549 net-new authoring types
//! that live ON the attachment item (never in global tuning).
//!
//! Every attachment magnitude that reuses an existing weapon newtype
//! ([`WeaponPunch`](crate::weapon::WeaponPunch), [`MagazineSize`](crate::weapon::MagazineSize),
//! [`FatalBias`](crate::weapon::FatalBias), …) is authored with that type directly; the two
//! types here are the ones GTW-549 introduces:
//!
//! - [`AimDelta`] — the per-item [`Accuracy`](crate::weapon::Accuracy) addend a SIGHT adds
//!   (the headline fix: a sight boosts AIM, not stability).
//! - [`WeaponBraceBonus`] — the graduated per-item §1a stability-score contribution a
//!   BRACE adds (the additive stability seam, a `Component` the PHASE 2 composer reads).
//! - [`ReloadScale`] — the per-item reload-cost multiplier a `FastReload` attachment applies
//!   (GTW-549 re-homes the magnitude ONTO the item; the GTW-542 model read a GLOBAL tuning
//!   `ReloadFactor`).

use bevy::prelude::{Component, Deref};
use serde::Deserialize;

/// A sight's **aim delta** — the per-item [`Accuracy`](crate::weapon::Accuracy) addend an
/// [`Aim`](super::AttachmentEffect::Aim) attachment adds to the weapon's in-cone
/// concentration exponent (GTW-549). A precision optic RAISES accuracy (clustering the
/// §1b draw toward centre), the lever DISTINCT from stability.
///
/// A per-item authoring magnitude (no-bare-types: private inner + derived [`Deref`];
/// `#[serde(transparent)]` so it authors as a bare RON scalar — `Aim(0.4)`). NOT a
/// `Component` — it is an effect payload the PHASE 2 application path reads to mutate the
/// weapon's [`Accuracy`](crate::weapon::Accuracy). Its magnitude lives HERE, on the
/// attachment item, never in global tuning (the GTW-549 headline fix).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct AimDelta(f32);

impl AimDelta {
    /// Build an aim delta from its [`Accuracy`](crate::weapon::Accuracy)-addend magnitude
    /// (dimensionless; a positive value tightens the in-cone draw toward centre).
    #[must_use]
    pub const fn new(delta: f32) -> Self {
        Self(delta)
    }
}

/// A brace's **weapon-brace bonus** — the graduated per-item §1a stability-score
/// contribution a [`Stability`](super::AttachmentEffect::Stability) attachment adds to the
/// weapon's cone-stability composition (GTW-549). An additive stability term, the
/// [`SuppressionStability`](crate::SuppressionStability) / emplacement-stability precedent,
/// GRADUATED per item (not the boolean [`Stable`](crate::weapon::Stable) tag).
///
/// A `#[derive(Component)]` (no-bare-types: private inner + derived [`Deref`];
/// `#[serde(transparent)]` so it authors as a bare RON scalar —
/// `Stability(12.0)`). PHASE 2 inserts it onto the weapon entity (the isolated
/// [`Stability`](super::AttachmentEffect::Stability) effect) and the §1a stability composer
/// reads it as an additive score contribution; PHASE 1 only defines the type. Its magnitude
/// lives HERE, on the attachment item, never in global tuning. `Default`
/// (`WeaponBraceBonus(0.0)`) is the identity contribution — a weapon with no brace effect
/// adds no stability points.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Deserialize, Default)]
#[serde(transparent)]
pub struct WeaponBraceBonus(f32);

impl WeaponBraceBonus {
    /// Build a weapon-brace bonus from its §1a stability-score-point magnitude (a positive
    /// value steadies the weapon — a tighter cone).
    #[must_use]
    pub const fn new(bonus: f32) -> Self {
        Self(bonus)
    }

    /// The identity brace bonus — zero stability-score points (a weapon with no
    /// [`Stability`](super::AttachmentEffect::Stability) effect). The `::none()` identity
    /// the PHASE 2 composer folds when no brace bonus is present.
    #[must_use]
    pub const fn none() -> Self {
        Self(0.0)
    }
}

/// A `FastReload` attachment's **reload-cost scale** — the per-item multiplier a
/// [`FastReload`](super::AttachmentEffect::FastReload) attachment applies to the weapon's
/// [`ReloadTu`](crate::magazine::ReloadTu) at spawn (GTW-549). A factor `< 1.0` speeds the
/// reload (fewer TU to swap a magazine); `1.0` is the identity.
///
/// GTW-549 re-homes this magnitude ONTO the attachment item: the GTW-542 model read a
/// GLOBAL tuning `ReloadFactor` (`combat.tuning.ron`), the exact defect this rework fixes.
/// A per-item authoring magnitude (no-bare-types: private inner + derived [`Deref`];
/// `#[serde(transparent)]` so it authors as a bare RON scalar — `FastReload(0.5)`). NOT a
/// `Component` — it is an effect payload the application path reads to rebuild the weapon's
/// [`Magazine`](crate::magazine::Magazine) with a scaled reload cost.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ReloadScale(f32);

impl ReloadScale {
    /// Build a reload-cost scale from its multiplier magnitude (`< 1.0` speeds the reload;
    /// `1.0` is the identity).
    #[must_use]
    pub const fn new(scale: f32) -> Self {
        Self(scale)
    }
}
