//! The static-battlefield terrain draw (GTW-48 S4 / GTW-218): the first VISUAL slice.
//!
//! This module reads the three sim-owned static-map resources — the
//! [`OccupancyGrid`](gdtf_battle_sim::occupancy::OccupancyGrid) terrain, the
//! [`CoverLedger`](gdtf_battle_sim::cover::CoverLedger) cover entries, and the
//! [`SurfaceGrid`](gdtf_battle_sim::surface::SurfaceGrid) slabs — for the presenter-owned
//! [`ActiveLevel`] and spawns one 16x16 top-down terrain tile per non-empty
//! `(cell, level)`, resolving each tile's PIXELS from its graphic name's sprite def
//! (GTW-665 — the [`resolve`] module over the GTW-663
//! [`SpriteDefRegistry`](gdtf_content_families::sprites::SpriteDefRegistry), loaded from
//! `assets/content/sprites/*.spritedef.ron`). It positions each
//! sprite via the S3 [`cell_to_world`](crate::cell_to_world) projection (plus the def's
//! authored ANCHOR offset) through the
//! S3 sprite-sizing recipe (`custom_size: Some(Vec2::splat(CELL_PX))`).
//!
//! Which sim fact maps to which fallback ROLE is owned HERE (the closed [`TileRole`]
//! vocabulary); which PIXELS a graphic name resolves to is data, read from the
//! sprite-def registry at draw time — never a hardcoded atlas index. The model never
//! reads the presenter (ADR-0001): this slice writes NOTHING
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
//! A sprite-def hot-reload never redraws: the GTW-666 restamp reaction
//! ([`restamp_tiles_on_def_change`]) re-resolves every already-drawn tile IN PLACE when
//! the [`SpriteDefRegistry`](gdtf_content_families::sprites::SpriteDefRegistry) changes —
//! mutate-not-respawn, tick-quiet for unchanged defs.
//!
//! It draws NO gangers (S5), NO FX (S6), and reads NO input (S7/S8) — it only REACTS
//! to [`ActiveLevel`] changing (S8's level-cycling input mutates the resource).

mod active_level;
mod band;
mod link_draw;
mod resolve;
mod restamp;
mod roles;
mod static_draw;
mod static_map;
mod swaps;
mod treatment;

#[cfg(test)]
mod test;

pub use active_level::{ActiveLevel, PresenterSystems, ViewMode};
pub use link_draw::{VerticalLinkSprite, draw_vertical_links};
pub use resolve::{
    MissingTileTexture, anchor_world_offset, resolve_sprite, setup_missing_tile_texture,
    single_rect_layout, source_parts, source_px_size, source_urect,
};
pub use restamp::{StampedGraphic, restamp_tiles_on_def_change};
pub use roles::TileRole;
pub use static_draw::{TerrainSprite, draw_static_battlefield};
pub use static_map::{SpriteResolveCtx, StaticMap};
pub use swaps::{indicate_emplacement_occupied, swap_destroyed_cover, swap_destroyed_slab};
pub use treatment::{ContextDepth, IsolateView, StoreyTreatment, StoreyViewMode, storey_treatment};
