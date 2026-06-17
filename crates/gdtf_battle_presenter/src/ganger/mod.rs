//! The ganger draw (GTW-48 S5 / GTW-219): each sim ganger drawn as a 16x16 top-down
//! SPRITE, kept current by Bevy change detection.
//!
//! This module reads the sim's per-field ganger components — [`Position`], [`Faction`],
//! [`Facing`], [`Stance`], [`Aiming`], [`LifeState`] — and mirrors each ganger as one
//! 16x16 character [`Sprite`] from the role-separated character sheet
//! ([`SheetRole::Characters`], `assets/tiles/alt_tileset_characters.png`). It is the
//! ganger arm of the change-driven sim->view mirror (ADR-0001): the presenter READS the
//! sim and renders it, the sim never reads the presenter.
//!
//! WHICH actor tile a faction draws is DATA-DRIVEN — a per-faction base actor
//! [`TileIndex`] authored in `assets/tiles/character_roles.ron` and resolved into the
//! [`CharacterRoles`] resource. HOW the sim's 8 facings collapse to the sheet's 4
//! sprite frames is a pure, documented mapping ([`facing_frame`]) — the sheet ships 4
//! frames per actor, not 8, so 8-direction sprite generation is not viable. The drawn
//! atlas index is `faction_base + facing_frame`, both terms read STRUCTURALLY (the base
//! from the data table, the offset from the map), never a hardcoded literal.
//!
//! # Draw lifecycle
//!
//! Driven entirely by change detection over the sim's ganger components (registered by
//! [`TopDownRendererPlugin`](crate::TopDownRendererPlugin) in the S4-defined
//! [`PresenterSystems::Draw`](crate::PresenterSystems) set, ordered `.after(SimSystems::Simulate)`):
//!
//! - [`spawn_ganger_sprites`] — [`Added<Position>`]: spawns one presenter [`Sprite`] for
//!   a ganger on the [`ActiveLevel`], recording its `sim Entity -> presenter Entity` in
//!   the [`GangerSprites`] map and tagging it with the [`GangerSprite`] marker.
//! - [`move_ganger_sprites`] — [`Changed<Position>`] (excluding the spawn): moves the
//!   existing presenter sprite's [`Transform`] (it does NOT respawn) and shows/hides it
//!   by whether the new `(cell, level)` is on the active level.
//! - [`reframe_ganger_sprites`] — [`Changed<Facing>`] / [`Changed<Stance>`] /
//!   [`Changed<Aiming>`]: recomputes the atlas index (facing reframe) and re-tints the
//!   sprite (the stance / aiming delta) in place.
//! - [`update_ganger_life_state`] — [`Changed<LifeState>`]: tints/reframes a `Downed`
//!   ganger and despawns a `Dead` one (dropping its [`GangerSprites`] entry).
//! - [`despawn_removed_ganger_sprites`] — [`RemovedComponents<Position>`]: despawns the
//!   mapped presenter sprite and drops its map entry.
//! - [`apply_active_level_filter`] — on an [`ActiveLevel`] change: hides off-level ganger
//!   sprites and shows on-level ones (the SAME active-level filter the S4 static draw
//!   uses — compare the ganger's `Position` `z` against `**ActiveLevel`).
//!
//! It draws ONLY ganger sprites — never the S4 [`TerrainSprite`](crate::TerrainSprite),
//! never the S2 [`WorldCamera`](crate::WorldCamera) — and adds ZERO sim setup/teardown
//! plumbing (S5 only READS the running battle's results).

mod frame;
mod roles;
mod sprite_map;
mod systems;
mod tint;

#[cfg(test)]
mod test;

pub use frame::{FacingFrame, facing_frame};
pub use roles::{
    CharacterRoles, CharacterRolesHandle, load_character_roles, resolve_character_roles,
};
pub use sprite_map::{GangerSprite, GangerSprites};
pub use systems::{
    apply_active_level_filter, despawn_removed_ganger_sprites, move_ganger_sprites,
    reframe_ganger_sprites, spawn_ganger_sprites, update_ganger_life_state,
};
