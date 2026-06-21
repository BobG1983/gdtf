//! The presenter FOG WRITER (GTW-342, leaf 6 / the only VIEW leaf of the GTW-13 squad
//! fog-of-war epic).
//!
//! This module is the VIEW arm of the squad fog: the sim
//! ([`gdtf_battle_sim`]) is the source of truth — it owns the three
//! [`SquadVisibility`](gdtf_battle_sim::SquadVisibility) states (VISIBLE / EXPLORED /
//! UNSEEN) and the [`recompute_visibility`](gdtf_battle_sim::recompute_visibility)
//! writer (GTW-341). The presenter never owns fog; it READS the squad sets through the
//! pure seams ([`SquadVisibility::is_cell_visible`](gdtf_battle_sim::SquadVisibility::is_cell_visible)
//! / [`SquadVisibility::is_cell_explored`](gdtf_battle_sim::SquadVisibility::is_cell_explored)
//! / [`is_ganger_visible`](gdtf_battle_sim::is_ganger_visible)) and the
//! [`explored_dim`](gdtf_battle_sim::CombatTuning) tunable, then MODULATES the already-drawn
//! layer in place (`docs/combat/visibility.md` §"Composition with the view slice").
//!
//! # The rendered layer IS the fog mask
//!
//! Fog modulates the *current* rendered geometry — it never repaints from a snapshot
//! (`docs/combat/visibility.md` §"Per-ganger FOV, the squad union, the mission memory").
//! Only authored, still-standing terrain has a [`TerrainSprite`](crate::TerrainSprite)
//! cell to modulate, so fog can never present over void and destroyed terrain stays
//! destroyed. Per `(cell, level)`:
//!
//! - **VISIBLE** → full identity ([`Color::WHITE`] modulate — the atlas tile's own
//!   colours read through), shown;
//! - **EXPLORED** → the tile's RGB × `explored_dim` (alpha untouched — "memory, not live
//!   sight"), shown;
//! - **UNSEEN** → hidden ([`Visibility::Hidden`] — the dark clear colour reads through).
//!
//! # Composition with the view slice — never crossing writers
//!
//! Two visibility writers, never crossed (`docs/combat/visibility.md` §"Composition with
//! the view slice"): the [`ActiveLevel`](crate::ActiveLevel) **slice** owns LAYER visibility (a ganger off the
//! active storey is hard-hidden by [`apply_active_level_filter`](crate::apply_active_level_filter)
//! / the spawn / move systems); **fog** owns the per-cell terrain modulate plus each
//! actor entity's own fog flag. The design intent is "a thing draws iff fog shows its
//! cell/entity AND the slice shows its storey". This codebase has no layer-parent
//! hierarchy to inherit through, so the fog writer is the SINGLE FINAL writer of each
//! actor sprite's [`Visibility`]: ordered `.after` the slice's storey-filter systems, it
//! re-reads the same `pos.z == active` storey fact the slice uses and ANDs it with the
//! fog fact — so plan and render can never disagree and the two facts are composed by one
//! writer rather than two fighting over the same component.
//!
//! # Ordering (the CRITICAL clause)
//!
//! [`draw_static_battlefield`](crate::draw_static_battlefield) DESPAWNS-ALL-then-RESPAWNS
//! every [`TerrainSprite`](crate::TerrainSprite) on `BattleReady` OR an
//! [`ActiveLevel`](crate::ActiveLevel) change, and
//! [`swap_destroyed_cover`](crate::swap_destroyed_cover) edits sprites on
//! [`CoverDestroyed`](gdtf_battle_sim::CoverDestroyed). The fog writer MUST run strictly
//! `.after` both (within [`PresenterSystems::Draw`](crate::PresenterSystems)) or it would
//! colour stale / just-despawned entities or miss freshly-spawned ones on a level cycle
//! (`bevy-traps.md` #3). It is wired so in [`TopDownRendererPlugin`](crate::TopDownRendererPlugin).
//!
//! It mints NO fire / targeting fog-GATE UX (the reticle / "hold your fire" refusal) —
//! that is GTW-11, which consumes the SIM read seams, not this presenter writer.

mod present;

#[cfg(test)]
mod test;

pub use present::present_fog;
