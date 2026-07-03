//! The ganger draw (GTW-48 S5 / GTW-219): each sim ganger drawn as a 16x16 top-down
//! SPRITE, kept current by Bevy change detection.
//!
//! This module reads the sim's per-field ganger components — [`Position`](gdtf_battle_sim::Position), [`Faction`](gdtf_battle_sim::Faction),
//! [`Facing`](gdtf_battle_sim::Facing), [`Stance`](gdtf_battle_sim::Stance), [`Aiming`](gdtf_battle_sim::Aiming), [`LifeState`](gdtf_battle_sim::LifeState) — and mirrors each ganger as one
//! 16x16 character [`Sprite`](bevy::sprite::Sprite) from the role-separated character sheet
//! ([`SheetRole::Characters`](crate::SheetRole::Characters), `assets/sprites/alt_tileset_characters.png`). It is the
//! ganger arm of the change-driven sim->view mirror (ADR-0001): the presenter READS the
//! sim and renders it, the sim never reads the presenter.
//!
//! WHICH actor tile a faction draws is DATA-DRIVEN — a per-faction base actor
//! [`TileIndex`](crate::TileIndex) authored in `assets/sprites/character_roles.spritedef.ron` and resolved into the
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
//! - [`spawn_ganger_sprites`] — `Added<Position>`: spawns one presenter [`Sprite`](bevy::sprite::Sprite) for
//!   a ganger on the [`ActiveLevel`](crate::ActiveLevel), recording its `sim Entity -> presenter Entity` in
//!   the [`GangerSprites`] map and tagging it with the [`GangerSprite`] marker.
//! - [`move_ganger_sprites`] — `Changed<Position>` (excluding the spawn): moves the
//!   existing presenter sprite's [`Transform`](bevy::transform::components::Transform) (it does NOT respawn) and shows/hides it
//!   by whether the new `(cell, level)` is on the active level.
//! - [`reframe_ganger_sprites`] — `Changed<Facing>` / `Changed<Stance>` /
//!   `Changed<Aiming>`: recomputes the atlas index (facing reframe) and re-tints the
//!   sprite (the stance / aiming delta) in place.
//! - [`update_ganger_life_state`] — `Changed<LifeState>`: tints/reframes a `Downed`
//!   ganger and despawns a `Dead` one — but a shot-kill whose tracer is still in flight is
//!   DEFERRED (GTW-331): it despawns only a death with no pending incoming shot (a non-shot
//!   death), dropping its [`GangerSprites`] entry.
//! - [`despawn_killed_ganger_on_impact`] — [`ShotImpactResolved`](crate::ShotImpactResolved):
//!   despawns a SHOT-killed ganger's sprite when its killing tracer LANDS (GTW-331), so the body
//!   does not vanish before the bolt reaches it; drops its [`GangerSprites`] entry.
//! - [`despawn_removed_ganger_sprites`] — [`RemovedComponents<Position>`](bevy::ecs::lifecycle::RemovedComponents): despawns the
//!   mapped presenter sprite and drops its map entry.
//! - [`apply_active_level_filter`] — on an [`ActiveLevel`](crate::ActiveLevel) change: hides off-level ganger
//!   sprites and shows on-level ones (the SAME active-level filter the S4 static draw
//!   uses — compare the ganger's `Position` `z` against `**ActiveLevel`).
//!
//! It draws ONLY ganger sprites — never the S4 [`TerrainSprite`](crate::TerrainSprite),
//! never the S2 [`WorldCamera`](crate::WorldCamera) — and adds ZERO sim setup/teardown
//! plumbing (S5 only READS the running battle's results).
//!
//! # GTW-520 — draw gangers on visible lower storeys
//!
//! The ganger-visibility model is a DRAWN-BAND decision, not a single-active-storey hard cut:
//! a live ganger on ANY storey within `0..=active` is drawn at its OWN storey's Z (peeking
//! through the floor-gaps GTW-519 already renders terrain for), and one strictly ABOVE the
//! active level is culled. The four visibility sites — [`spawn_ganger_sprites`],
//! [`move_ganger_sprites`], [`apply_active_level_filter`], AND the fog writer's
//! `present_actor_fog` (the single final [`Visibility`](bevy::prelude::Visibility)
//! writer) — all consult ONE shared predicate,
//! [`ActiveLevel::draws_storey`](crate::ActiveLevel::draws_storey), so they cannot drift. The
//! fog hard-cut is UNCHANGED (an unseen enemy on a lower drawn storey is still hidden); only
//! the storey axis widened.
//!
//! FOLLOW-ON (GTW-522, NOT built here): cross-storey TARGETING — clicking / firing a ganger
//! drawn on a LOWER storey. The cursor / selection / hover pick still binds the hovered cell to
//! the ACTIVE storey only (locked design #4; the pick path in `gdtf_battle_input` is
//! deliberately unchanged), so a drawn lower-storey ganger is VISIBLE but NOT pickable /
//! fireable. Making a drawn lower-storey unit a valid click/fire target (and the reticle across
//! storeys) is scoped to GTW-522.

mod frame;
mod roles;
mod sprite_map;
mod systems;
mod tint;
mod tween;

#[cfg(test)]
mod test;

pub use frame::{FacingFrame, facing_frame};
pub use roles::CharacterRoles;
pub(crate) use roles::register_character_roles_hot_ron;
pub use sprite_map::{GangerSprite, GangerSprites};
pub use systems::{
    apply_active_level_filter, despawn_killed_ganger_on_impact, despawn_removed_ganger_sprites,
    move_ganger_sprites, reframe_ganger_sprites, reindex_ganger_sprites_on_character_roles_change,
    spawn_ganger_sprites, update_ganger_life_state,
};
pub use tween::{SpriteTween, advance_sprite_tweens};
