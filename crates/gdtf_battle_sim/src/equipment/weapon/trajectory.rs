use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// `bool`), a PER-WEAPON `#[derive(Component)]` sibling on the armed entity (the GTW-200
/// [`WeaponSpec`](super::WeaponSpec) (a `#[serde(default)]` field), so every EXISTING
/// [`Straight`](TrajectoryStyle::Straight) is the [`Default`] (`#[serde(default)]` on the
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TrajectoryStyle {
                #[default]
    Straight,
                            Arc,
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lobbed(bool);

impl TrajectoryStyle {
            #[must_use]
    pub const fn is_arc(self) -> Lobbed {
        Lobbed(matches!(self, Self::Arc))
    }
}
