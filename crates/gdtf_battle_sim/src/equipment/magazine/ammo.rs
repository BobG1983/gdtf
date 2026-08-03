use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

use crate::weapon::{AmmoType, MagazineSize, ModeShots};

/// [`Deref`]; `#[serde(transparent)]` parses a bare RON scalar ([`Serialize`] so the
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReloadTu(u8);

impl ReloadTu {
            #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

/// value): private inner + derived [`Deref`]; `#[serde(transparent)]` lets it default
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct LoadedRounds(u16);

impl LoadedRounds {
            #[must_use]
    pub const fn new(rounds: u16) -> Self {
        Self(rounds)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MagazineEmpty(bool);

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MagazineFull(bool);

/// the same schema it loads — with `rounds` `#[serde(skip_serializing)]`: the live
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Magazine {
            pub size:        MagazineSize,
            pub reload_tu:   ReloadTu,
                    #[serde(default, skip_serializing)]
    pub rounds:      LoadedRounds,
                                #[serde(default, skip_serializing)]
    pub loaded_ammo: AmmoType,
}

impl Magazine {
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

                                        #[must_use]
    pub const fn loaded(size: MagazineSize, reload_tu: ReloadTu) -> Self {
        Self::loaded_with(size, reload_tu, AmmoType::Slug)
    }

                                    #[must_use]
    pub const fn loaded_with(size: MagazineSize, reload_tu: ReloadTu, ammo: AmmoType) -> Self {
        Self {
            size,
            reload_tu,
            rounds: LoadedRounds(size.get()),
            loaded_ammo: ammo,
        }
    }

        #[must_use]
    pub const fn rounds(&self) -> LoadedRounds {
        self.rounds
    }

            #[must_use]
    pub const fn size(&self) -> MagazineSize {
        self.size
    }

            #[must_use]
    pub const fn reload_tu(&self) -> ReloadTu {
        self.reload_tu
    }

            #[must_use]
    pub const fn loaded_ammo(&self) -> AmmoType {
        self.loaded_ammo
    }

        #[must_use]
    pub const fn is_empty(&self) -> MagazineEmpty {
        MagazineEmpty(self.rounds.0 == 0)
    }

            #[must_use]
    pub const fn is_full(&self) -> MagazineFull {
        MagazineFull(self.rounds.0 >= self.size.get())
    }

                            pub const fn spend_round(&mut self) {
        self.rounds.0 = self.rounds.0.saturating_sub(1);
    }

                                pub const fn refill(&mut self) {
        self.rounds.0 = self.size.get();
    }

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

#[must_use]
pub fn ammo_compatible(accepted: AmmoType, candidate: AmmoType) -> AmmoCompatible {
    AmmoCompatible(accepted == candidate)
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmmoCompatible(bool);

#[must_use]
pub fn clamp_burst(shots: ModeShots, mag: &Magazine) -> ModeShots {
    ModeShots::new((*shots).min(*mag.rounds()))
}
