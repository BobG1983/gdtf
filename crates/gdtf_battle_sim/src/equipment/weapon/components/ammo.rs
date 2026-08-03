//! shape (a closed, serde-authored enum) but a SEPARATE concept: ammo class is
use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

/// set of serde-authored variants, a `#[derive(Component)]` newtype), but a
/// the editor round-trips it. A `#[derive(Component)]` matching the
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, Default)]
pub enum AmmoType {
                #[default]
    Slug,
            Cell,
            Flask,
            Canister,
                Grenade,
}

impl AmmoType {
                pub const ALL: [Self; 5] = [
        Self::Slug,
        Self::Cell,
        Self::Flask,
        Self::Canister,
        Self::Grenade,
    ];
}
