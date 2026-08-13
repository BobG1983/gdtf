//! Magazine capacity, loaded rounds, and ammo type.

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

use crate::weapon::{AmmoType, MagazineSize, ModeShots};

/// TU cost to reload.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReloadTu(u8);

impl ReloadTu {
    /// Wrap a reload cost.
    #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

/// Rounds currently in the magazine.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct LoadedRounds(u16);

impl LoadedRounds {
    /// Wrap a round count.
    #[must_use]
    pub const fn new(rounds: u16) -> Self {
        Self(rounds)
    }
}

/// Whether the magazine has zero rounds.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MagazineEmpty(bool);

/// Whether the magazine is at capacity.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MagazineFull(bool);

/// Live magazine on a weapon entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Magazine {
    /// Capacity.
    size:        MagazineSize,
    /// Reload TU cost.
    reload_tu:   ReloadTu,
    /// Current rounds (not serialized to content).
    #[serde(default, skip_serializing)]
    rounds:      LoadedRounds,
    /// Loaded ammo type (not serialized to content).
    #[serde(default, skip_serializing)]
    loaded_ammo: AmmoType,
}

impl Magazine {
    /// Build with clamped rounds.
    #[must_use]
    pub const fn new(rounds: LoadedRounds, size: MagazineSize, reload_tu: ReloadTu) -> Self {
        Self {
            size,
            reload_tu,
            rounds: LoadedRounds(if rounds.0 < size.get() {
                rounds.0
            } else {
                size.get()
            }),
            loaded_ammo: AmmoType::Slug,
        }
    }

    /// Full magazine of slug ammo.
    #[must_use]
    pub const fn loaded(size: MagazineSize, reload_tu: ReloadTu) -> Self {
        Self::loaded_with(size, reload_tu, AmmoType::Slug)
    }

    /// Full magazine of a given ammo type.
    #[must_use]
    pub const fn loaded_with(size: MagazineSize, reload_tu: ReloadTu, ammo: AmmoType) -> Self {
        Self {
            size,
            reload_tu,
            rounds: LoadedRounds(size.get()),
            loaded_ammo: ammo,
        }
    }

    /// Current rounds.
    #[must_use]
    pub const fn rounds(&self) -> LoadedRounds {
        self.rounds
    }

    /// Capacity.
    #[must_use]
    pub const fn size(&self) -> MagazineSize {
        self.size
    }

    /// Set capacity, clamping loaded rounds down to the new size.
    pub const fn set_size(&mut self, size: MagazineSize) {
        self.size = size;
        if self.rounds.0 > size.get() {
            self.rounds.0 = size.get();
        }
    }

    /// Reload cost.
    #[must_use]
    pub const fn reload_tu(&self) -> ReloadTu {
        self.reload_tu
    }

    /// Set the reload TU cost.
    pub const fn set_reload_tu(&mut self, reload_tu: ReloadTu) {
        self.reload_tu = reload_tu;
    }

    /// Loaded ammo type.
    #[must_use]
    pub const fn loaded_ammo(&self) -> AmmoType {
        self.loaded_ammo
    }

    /// True if empty.
    #[must_use]
    pub const fn is_empty(&self) -> MagazineEmpty {
        MagazineEmpty(self.rounds.0 == 0)
    }

    /// True if full.
    #[must_use]
    pub const fn is_full(&self) -> MagazineFull {
        MagazineFull(self.rounds.0 >= self.size.get())
    }

    /// Spend one round (saturating).
    pub const fn spend_round(&mut self) {
        self.rounds.0 = self.rounds.0.saturating_sub(1);
    }

    /// Fill to capacity.
    pub const fn refill(&mut self) {
        self.rounds.0 = self.size.get();
    }

    /// Load `candidate` if compatible with `accepted`; refill on success.
    #[must_use]
    pub fn load(&mut self, candidate: AmmoType, accepted: AmmoType) -> AmmoCompatible {
        let compatible = ammo_compatible(accepted, candidate);
        if *compatible {
            self.loaded_ammo = candidate;
            self.refill();
        }
        compatible
    }
}

/// True when candidate matches accepted ammo type.
#[must_use]
pub fn ammo_compatible(accepted: AmmoType, candidate: AmmoType) -> AmmoCompatible {
    AmmoCompatible(accepted == candidate)
}

/// Compatibility flag for a load attempt.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmmoCompatible(bool);

/// Cap burst size to remaining rounds.
#[must_use]
pub fn clamp_burst(shots: ModeShots, mag: &Magazine) -> ModeShots {
    ModeShots::new((*shots).min(*mag.rounds()))
}
