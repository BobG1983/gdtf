//! The **Stability** attachment effect (GTW-549; GTW-558 one-file-per-effect) — its per-item
//! [`WeaponBraceBonus`] magnitude, the isolated [`ApplyStability`] behaviour, and the `impl`
//! that inserts the graduated brace bonus onto the weapon.

use bevy::prelude::{Component, Deref, EntityWorldMut};
use serde::Deserialize;

use super::ApplyAttachmentEffect;

/// A brace's **weapon-brace bonus** — the graduated per-item §1a stability-score
/// contribution a [`Stability`](super::AttachmentEffect::Stability) attachment adds to the
/// weapon's cone-stability composition (GTW-549). An additive stability term, the
/// [`SuppressionStability`](crate::SuppressionStability) / emplacement-stability precedent,
/// GRADUATED per item (not the boolean [`Stable`](crate::weapon::Stable) tag).
///
/// A `#[derive(Component)]` (no-bare-types: private inner + derived [`Deref`];
/// `#[serde(transparent)]` so it authors as a bare RON scalar — `Stability(12.0)`). The
/// isolated [`Stability`](super::AttachmentEffect::Stability) effect inserts it onto the
/// weapon entity and the §1a stability composer reads it as an additive score contribution.
/// Its magnitude lives HERE, on the attachment item, never in global tuning. `Default`
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
    /// the §1a composer folds when no brace bonus is present.
    #[must_use]
    pub const fn none() -> Self {
        Self(0.0)
    }
}

/// **Stability** — fits the weapon a graduated per-item [`WeaponBraceBonus`] of §1a
/// stability-score points (the NEW clean brace seam GTW-549 introduces, `::none()`-identity —
/// NOT the ripped-out sight-stability seam). GRADUATED, distinct from the boolean
/// [`ApplyBrace`](super::ApplyBrace) tag.
///
/// Inserts the bonus as a `Component` (the §1a stability composer folds it as an additive
/// contribution, the [`SuppressionStability`](crate::SuppressionStability) precedent).
pub struct ApplyStability {
    /// The graduated §1a stability-score points this brace contributes.
    bonus: WeaponBraceBonus,
}

impl ApplyStability {
    /// Build the stability effect from its per-item [`WeaponBraceBonus`].
    #[must_use]
    pub const fn new(bonus: WeaponBraceBonus) -> Self {
        Self { bonus }
    }
}

impl ApplyAttachmentEffect for ApplyStability {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        weapon.insert(self.bonus);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::World;

    use super::{ApplyAttachmentEffect, ApplyStability, WeaponBraceBonus};

    /// `ApplyStability` INSERTS a positive graduated `WeaponBraceBonus` — the NEW clean brace
    /// seam. Asserts the component lands with a positive magnitude (not a shipped value).
    #[test]
    fn stability_inserts_weapon_brace_bonus() {
        let mut world = World::new();
        let weapon = world.spawn_empty().id();
        let mut entity = world.entity_mut(weapon);
        ApplyStability::new(WeaponBraceBonus::new(12.0)).apply_to_weapon(&mut entity);
        let Some(bonus) = entity.get::<WeaponBraceBonus>() else {
            unreachable!("Stability must insert a WeaponBraceBonus component");
        };
        assert!(
            **bonus > 0.0,
            "Stability inserts a positive brace bonus (got {})",
            **bonus
        );
    }
}
