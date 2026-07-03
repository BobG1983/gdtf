//! The **`ReloadTime`** attachment effect (GTW-549 reload-speed attachment, renamed GTW-558)
//! — its per-item [`ReloadTimeScale`] magnitude, the isolated [`ApplyReloadTime`] behaviour,
//! and the `impl` that scales the weapon's reload cost.

use bevy::prelude::{Deref, EntityWorldMut};
use serde::Deserialize;

use super::ApplyAttachmentEffect;
use crate::magazine::{Magazine, ReloadTu};

/// A reload-time attachment's **reload-cost scale** — the per-item multiplier a
/// [`ReloadTime`](super::AttachmentEffect::ReloadTime) attachment applies to the weapon's
/// [`ReloadTu`](crate::magazine::ReloadTu) at spawn (GTW-549, renamed GTW-558). A
/// BIDIRECTIONAL factor: `< 1.0` speeds the reload (fewer TU to swap a magazine — a
/// speed-loader), `> 1.0` slows it (a bulky drum), `1.0` is the identity.
///
/// GTW-549 re-homes this magnitude ONTO the attachment item: the GTW-542 model read a
/// GLOBAL tuning factor (`combat.tuning.ron`), the exact defect that rework fixed. GTW-558
/// renames it from the GTW-549 misnomer that implied a speed-up only — the scale is
/// bidirectional (a `> 1.0` drum SLOWS the reload). A per-item
/// authoring magnitude (no-bare-types: private inner + derived [`Deref`];
/// `#[serde(transparent)]` so it authors as a bare RON scalar — `ReloadTime(0.5)`). NOT a
/// `Component` — it is an effect payload the application path reads to rebuild the weapon's
/// [`Magazine`](crate::magazine::Magazine) with a scaled reload cost.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ReloadTimeScale(f32);

impl ReloadTimeScale {
    /// Build a reload-cost scale from its multiplier magnitude (`< 1.0` speeds the reload,
    /// `> 1.0` slows it; `1.0` is the identity).
    #[must_use]
    pub const fn new(scale: f32) -> Self {
        Self(scale)
    }
}

/// **`ReloadTime`** — SCALES the weapon's [`ReloadTu`](crate::magazine::ReloadTu) reload cost
/// by its per-item [`ReloadTimeScale`] (`< 1.0` → faster, `> 1.0` → slower; GTW-549 re-homes
/// the magnitude onto the item, the exact defect the GTW-542 global-tuning model had).
///
/// Rebuilds the [`Magazine`](crate::magazine::Magazine) through its ctor preserving the
/// loaded count + capacity (the GTW-542 `scale_reload` logic, now isolated here). A weapon
/// with no magazine is left unchanged.
pub struct ApplyReloadTime {
    /// The per-item reload-cost multiplier (`< 1.0` speeds the reload, `> 1.0` slows it).
    scale: ReloadTimeScale,
}

impl ApplyReloadTime {
    /// Build the reload-time effect from its per-item [`ReloadTimeScale`].
    #[must_use]
    pub const fn new(scale: ReloadTimeScale) -> Self {
        Self { scale }
    }
}

impl ApplyAttachmentEffect for ApplyReloadTime {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        let Some(magazine) = weapon.get::<Magazine>() else {
            return;
        };
        let scaled = (f32::from(*magazine.reload_tu()) * *self.scale).max(0.0);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the scaled reload cost is clamped non-negative above and a reload TU is a \
                      small u8 count, so the f32 -> u8 floor cannot truncate meaningfully or \
                      sign-flip (the GTW-542 scale_reload precedent this isolates)"
        )]
        let tu = scaled as u8;
        let rebuilt = Magazine::new(*magazine.rounds(), magazine.size(), ReloadTu::new(tu));
        weapon.insert(rebuilt);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::World;

    use super::{ApplyAttachmentEffect, ApplyReloadTime, ReloadTimeScale};
    use crate::{
        magazine::{Magazine, ReloadTu},
        weapon::MagazineSize,
    };

    /// A `< 1.0` scale LOWERS the magazine's reload cost (a speed-loader).
    #[test]
    fn reload_time_scale_below_one_lowers_reload_tu() {
        let mut world = World::new();
        let weapon = world
            .spawn(Magazine::loaded(MagazineSize::new(20), ReloadTu::new(20)))
            .id();
        let mut entity = world.entity_mut(weapon);
        ApplyReloadTime::new(ReloadTimeScale::new(0.5)).apply_to_weapon(&mut entity);
        let Some(magazine) = entity.get::<Magazine>() else {
            unreachable!("the weapon must still carry a Magazine");
        };
        assert!(
            *magazine.reload_tu() < 20,
            "a < 1.0 ReloadTime scale lowers reload_tu below the 20 baseline (got {})",
            *magazine.reload_tu()
        );
    }

    /// A `> 1.0` scale RAISES the magazine's reload cost (the bulky-drum case — the rename's
    /// whole point — the old speed-up-only name was a misnomer).
    #[test]
    fn reload_time_scale_above_one_raises_reload_tu() {
        let mut world = World::new();
        let weapon = world
            .spawn(Magazine::loaded(MagazineSize::new(20), ReloadTu::new(20)))
            .id();
        let mut entity = world.entity_mut(weapon);
        ApplyReloadTime::new(ReloadTimeScale::new(1.5)).apply_to_weapon(&mut entity);
        let Some(magazine) = entity.get::<Magazine>() else {
            unreachable!("the weapon must still carry a Magazine");
        };
        assert!(
            *magazine.reload_tu() > 20,
            "a > 1.0 ReloadTime scale raises reload_tu above the 20 baseline (got {})",
            *magazine.reload_tu()
        );
    }
}
