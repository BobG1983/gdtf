//! Weapon panel spawn on the UI camera.
mod columns;
mod combined;
mod geometry;
mod item_aim_panels;
mod root;

pub(in crate::states::running::game::battlescape) use root::{
    despawn_weapon_panel, spawn_weapon_panel,
};
