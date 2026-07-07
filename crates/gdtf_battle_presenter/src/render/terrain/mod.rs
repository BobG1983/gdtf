//! The static-battlefield terrain draw (GTW-48 S4 / GTW-218): the first VISUAL slice.
//!
//! This module reads the three sim-owned static-map resources — the
//! [`OccupancyGrid`](gdtf_battle_sim::occupancy::OccupancyGrid) terrain, the
//! [`CoverLedger`](gdtf_battle_sim::cover::CoverLedger) cover entries, and the
//! [`SurfaceGrid`](gdtf_battle_sim::surface::SurfaceGrid) slabs — for the presenter-owned
//! [`ActiveLevel`] and spawns one 16x16 top-down terrain [`Sprite`](bevy::sprite::Sprite) per non-empty
//! `(cell, level)`, choosing each tile's atlas index from a DATA-DRIVEN role table
//! ([`TileRoles`], loaded from `assets/sprites/tile_roles.spritedef.ron`). It positions each
//! sprite via the S3 [`cell_to_world`](crate::cell_to_world) projection through the
//! S3 sprite-sizing recipe (`custom_size: Some(Vec2::splat(CELL_PX))`).
//!
//! Which sim fact maps to which ROLE is owned HERE; which atlas INDEX a role resolves
//! to is data, read from the [`TileRoles`] resource at draw time — never a hardcoded
//! literal. The model never reads the presenter (ADR-0001): this slice writes NOTHING
//! back to the sim, it only projects sim state to terrain sprites.
//!
//! # Draw lifecycle
//!
//! The initial draw is a ONE-SHOT triggered by draining
//! [`MessageReader<BattleReady>`](gdtf_battle_sim::battle::BattleReady) — NOT per-frame polling
//! and NOT `Changed<Resource>` (the three grids are mutated in place with no per-cell
//! change detection). After the initial draw it reacts to exactly two further triggers:
//!
//! - an [`ActiveLevel`] change (`ActiveLevel::is_changed`): redraw the new level, the
//!   off-active-level terrain despawned (the despawn-all-then-respawn path also makes
//!   the first-ready double-fire idempotent), and
//! - a [`CoverDestroyed`](gdtf_battle_sim::occupancy_sync::CoverDestroyed) message: swap that cover
//!   cell's sprite to the RUBBLE tile.
//!
//! It draws NO gangers (S5), NO FX (S6), and reads NO input (S7/S8) — it only REACTS
//! to [`ActiveLevel`] changing (S8's level-cycling input mutates the resource).

mod active_level;
mod band;
mod link_draw;
mod roles;
mod static_draw;
mod static_map;
mod swaps;
mod treatment;

#[cfg(test)]
mod test;

pub use active_level::{ActiveLevel, PresenterSystems, ViewMode};
pub use link_draw::{VerticalLinkSprite, draw_vertical_links};
pub(crate) use roles::register_tile_roles_hot_ron;
pub use roles::{TileIndex, TileRole, TileRoles, tile_roles_hot_ron_chain};
pub use static_draw::{TerrainSprite, draw_static_battlefield};
pub use static_map::StaticMap;
pub use swaps::{indicate_emplacement_occupied, swap_destroyed_cover, swap_destroyed_slab};
pub use treatment::{ContextDepth, IsolateView, StoreyTreatment, StoreyViewMode, storey_treatment};
