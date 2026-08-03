use bevy::prelude::Component;

/// The **`Weapon` MARKER** — a unit `#[derive(Component)]` tag (no data) marking an
/// `#[derive(Component)]` newtype on the same entity (`BaseSpread`, `Accuracy`,
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Weapon;

/// The **`MountedWeapon` MARKER** — a unit `#[derive(Component)]` tag (no data) marking a
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MountedWeapon;
