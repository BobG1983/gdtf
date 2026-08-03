//! [`gdtf_ui`] panel group on the GTW-120 UI camera to the AUTHORITATIVE layout (GTW-298, user
mod columns;
mod combined;
mod geometry;
mod item_aim_panels;
mod root;

pub(in crate::states::running::game::battlescape) use root::{
    despawn_weapon_panel, spawn_weapon_panel,
};
