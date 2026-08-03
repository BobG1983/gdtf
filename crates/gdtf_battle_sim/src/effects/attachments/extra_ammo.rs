//! Extra magazine capacity attachment effect.

use bevy::prelude::EntityWorldMut;

use super::ApplyAttachmentEffect;
use crate::{magazine::Magazine, weapon::MagazineSize};

/// Grows the weapon magazine by a fixed amount.
pub struct ApplyExtraAmmo {
    size_bonus: MagazineSize,
}

impl ApplyExtraAmmo {
    /// Build the applicator.
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
