//! The **`ExtraAmmo`** attachment effect (GTW-549; GTW-558 one-file-per-effect) — the
//! isolated [`ApplyExtraAmmo`] behaviour and the `impl` that grows the weapon's magazine. No
//! per-item magnitude newtype — its payload is the reused weapon
//! [`MagazineSize`](crate::weapon::MagazineSize).

use bevy::prelude::EntityWorldMut;

use super::ApplyAttachmentEffect;
use crate::{magazine::Magazine, weapon::MagazineSize};

/// **`ExtraAmmo`** — GROWS the weapon's [`Magazine`](crate::magazine::Magazine) capacity by
/// its per-item [`MagazineSize`](crate::weapon::MagazineSize) and refills it to the new full
/// (an oversized drum, more rounds before a reload).
///
/// Rebuilds the [`Magazine`](crate::magazine::Magazine) through its ctor (the GTW-542
/// `grow_magazine` logic, now isolated here) so the spawn-full invariant holds for the
/// larger drum. A weapon with no magazine is left unchanged.
pub struct ApplyExtraAmmo {
    /// The extra capacity this drum adds to the magazine.
    size_bonus: MagazineSize,
}

impl ApplyExtraAmmo {
    /// Build the extra-ammo effect from the [`MagazineSize`](crate::weapon::MagazineSize)
    /// capacity it adds.
    #[must_use]
    pub const fn new(size_bonus: MagazineSize) -> Self {
        Self { size_bonus }
    }
}

impl ApplyAttachmentEffect for ApplyExtraAmmo {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        let Some(magazine) = weapon.get::<Magazine>() else {
            return;
        };
        let grown = MagazineSize::new(magazine.size().get().saturating_add(self.size_bonus.get()));
        let reload_tu = magazine.reload_tu();
        weapon.insert(Magazine::loaded(grown, reload_tu));
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::World;

    use super::{ApplyAttachmentEffect, ApplyExtraAmmo};
    use crate::{
        magazine::{Magazine, ReloadTu},
        weapon::MagazineSize,
    };

    /// `ApplyExtraAmmo` GROWS the magazine capacity and refills it to the new full.
    #[test]
    fn extra_ammo_grows_the_magazine() {
        let mut world = World::new();
        let weapon = world
            .spawn(Magazine::loaded(MagazineSize::new(20), ReloadTu::new(20)))
            .id();
        let mut entity = world.entity_mut(weapon);
        ApplyExtraAmmo::new(MagazineSize::new(6)).apply_to_weapon(&mut entity);
        let Some(magazine) = entity.get::<Magazine>() else {
            unreachable!("the weapon must still carry a Magazine");
        };
        assert!(
            magazine.size().get() > 20,
            "ExtraAmmo grows capacity above the 20 baseline (got {})",
            magazine.size().get()
        );
        assert!(
            *magazine.is_full(),
            "ExtraAmmo refills the grown magazine to full"
        );
    }
}
