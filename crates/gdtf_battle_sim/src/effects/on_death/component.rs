//! The [`OnDeath`] authoring component — the weapon-entity carrier of an authored
//! [`OnDeathEffect`] (GTW-547, child GTW-41g; the effect vocabulary itself lives in the
//! GTW-552 palette, [`crate::effects::on_death`]).

use bevy::prelude::{Component, Deref};

use crate::effects::{fields::FieldKey, on_death::OnDeathEffect};

/// The **authoring component** carrying a weapon / gear entity's [`OnDeathEffect`]
/// (GTW-547).
///
/// Composed onto the wielded-weapon entity at the wielded-weapon spawn (the GTW-544
/// [`Dot`](crate::weapon::Dot) sibling precedent) from the authored
/// [`WeaponSpec::on_death`](crate::weapon::WeaponSpec) field. When the WIELDING ganger dies,
/// [`resolve_on_death`](super::resolve_on_death) reads this off the ganger's
/// [`Wields`](crate::weapon::Wields) weapon entity and fans its effect at the death cell —
/// modelling "the dying ganger's live grenade / unstable power cell detonates". Cover
/// on-death lives in the [`CoverOnDeathRegistry`](super::CoverOnDeathRegistry) (cover is not
/// an entity), NOT this component.
///
/// A MECHANICS carrier, not palette behaviour (the GTW-550 storage-stays-on-the-ledger
/// mirror): the effect's behaviour lives in its isolated
/// [`crate::effects::on_death`] palette file; this component only carries the authored
/// value to the resolver.
///
/// A newtype [`Component`] over the domain [`OnDeathEffect`] (no-bare-types: the wrapped
/// value is a domain enum). Private inner + derived [`Deref`]; derives [`Default`] (an
/// [`OnDeathEffect::LeaveField`] with an empty key — a spawn-seed sentinel that never fires:
/// the spawn only composes this component when the spec authored a real effect) so the
/// `template_value` sibling composition's `Clone + Default` bound is satisfied.
#[derive(Component, Deref, Debug, Clone, PartialEq)]
pub struct OnDeath(OnDeathEffect);

impl OnDeath {
    /// Build an on-death component from its authored effect.
    #[must_use]
    pub const fn new(effect: OnDeathEffect) -> Self {
        Self(effect)
    }

    /// Borrow the authored [`OnDeathEffect`] — the resolver reads this and fans it
    /// generically through the palette's
    /// [`ApplyOnDeathEffect`](crate::effects::on_death::ApplyOnDeathEffect) trait.
    #[must_use]
    pub const fn effect(&self) -> &OnDeathEffect {
        &self.0
    }
}

impl Default for OnDeath {
    /// The spawn-seed sentinel: a [`LeaveField`](OnDeathEffect::LeaveField) with an empty
    /// [`FieldKey`] (never resolves to a field, so it fires nothing). Never authored — the
    /// wielded-weapon spawn composes this component ONLY when the weapon spec authored a
    /// real [`on_death`](crate::weapon::WeaponSpec) effect; the default exists only so the
    /// `template_value` sibling composition's `Default` bound is satisfied (the
    /// [`DotProfile`](crate::weapon::DotProfile) default-sentinel precedent).
    fn default() -> Self {
        Self(OnDeathEffect::LeaveField {
            field: FieldKey::new(String::new()),
        })
    }
}
