//! Top-down renderer plumbing: the px/coordinate bridge the 16×16 top-down sprite
//! renderer is built on.
//!
//! This module owns the presenter-side projection OF the sim's presentation-agnostic
//! cubic-voxel metric (`docs/combat/battle-space.md`): the sim reasons in cells +
//! levels, and turning those into on-screen world units is the presenter's job
//! (ADR-0001 — the presenter owns ALL sim→view projection; the sim never reads the
//! presenter). It defines [`CELL_PX`], the [`cell_to_world`] projection, the
//! role-keyed atlas [`TopDownAtlases`] resource, and the system
//! ([`load_topdown_atlases`]) that builds that resource ONCE from the three render
//! sheets (terrain / characters / effects).
//!
//! It spawns NO sprite and draws NOTHING — terrain is S4, characters S5, FX S6. It
//! only produces the loaded atlas resource, the [`CELL_PX`] scalar, the
//! [`cell_to_world`] projection, and the documented sprite-sizing recipe those later
//! slices call.
//!
//! # Sprite-construction recipe (for S4/S5/S6)
//!
//! Every cell sprite is sized to exactly one cell so a 16px source tile fills a
//! 16-world-unit cell regardless of camera scale:
//!
//! ```ignore
//! use bevy::prelude::*;
//! use bevy::image::TextureAtlas;
//! use gdtf_battle_presenter::topdown::{CELL_PX, SheetRole, TopDownAtlases};
//!
//! fn draw_one(mut commands: Commands, atlases: Res<TopDownAtlases>) {
//!     let Some(role) = atlases.role(SheetRole::Terrain) else { return };
//!     let mut sprite = Sprite::from_atlas_image(
//!         role.image.clone(),
//!         TextureAtlas { layout: role.layout.clone(), index: 0 },
//!     );
//!     sprite.custom_size = Some(Vec2::splat(CELL_PX));
//!     commands.spawn(sprite);
//! }
//! ```
//!
//! Equivalently, the explicit `Sprite { image, texture_atlas: Some(TextureAtlas {
//! layout, index }), custom_size: Some(Vec2::splat(CELL_PX)), .. }` form. Any
//! tile-INDEX value a later slice introduces must be a NAMED newtype over its
//! primitive (no-bare-types); S3 itself stores no per-glyph index.

mod atlases;
mod projection;
mod redrive;

#[cfg(test)]
mod test;

pub use atlases::{SheetAtlas, SheetRole, TileIndex, TopDownAtlases, load_topdown_atlases};
pub use projection::{
    CELL_PX, GANGER_Z_BIAS, Layer, cell_to_world, cell_to_world_layered, sim_pos_to_world,
};
pub use redrive::redrive_sheet_images_on_asset_event;
pub(crate) use redrive::register_sheet_image_redrive;
