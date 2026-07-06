//! The presenter FOG WRITER (GTW-342, leaf 6 / the only VIEW leaf of the GTW-13 squad
//! fog-of-war epic).
//!
//! This module is the VIEW arm of the squad fog: the sim
//! ([`gdtf_battle_sim`]) is the source of truth — it owns the three
//! [`SquadVisibility`](gdtf_battle_sim::visibility::SquadVisibility) states (VISIBLE / EXPLORED /
//! UNSEEN) and the [`recompute_visibility`](gdtf_battle_sim::visibility::recompute_visibility)
//! writer (GTW-341). The presenter never owns fog; it READS the squad sets through the
//! pure seams ([`SquadVisibility::is_cell_visible`](gdtf_battle_sim::visibility::SquadVisibility::is_cell_visible)
//! / [`SquadVisibility::is_cell_explored`](gdtf_battle_sim::visibility::SquadVisibility::is_cell_explored)
//! / [`is_ganger_visible`](gdtf_battle_sim::visibility::is_ganger_visible)), then MODULATES the
//! already-drawn layer in place (`docs/combat/visibility.md` §"Composition with the view
//! slice").
//!
//! # The rendered layer IS the fog mask
//!
//! Fog modulates the *current* rendered geometry — it never repaints from a snapshot
//! (`docs/combat/visibility.md` §"Per-ganger FOV, the squad union, the mission memory").
//! Only authored, still-standing terrain has a [`TerrainSprite`](crate::TerrainSprite)
//! cell to modulate, so fog can never present over void and destroyed terrain stays
//! destroyed. Per `(cell, level)` (GTW-348 — colour-loss, not brightness-loss, as the
//! memory cue):
//!
//! - **VISIBLE** → full colour (the [`TerrainFogMaterial`] `saturation` is `1.0`, the atlas
//!   tile's own colours read through), shown;
//! - **EXPLORED** → FULL-brightness GREYSCALE (the material `saturation` is `0.0` — the
//!   tile's BT.709 luminance with its colour removed; "memory, not live sight"), shown;
//! - **UNSEEN** → hidden (`Visibility::Hidden` — the dark clear colour reads through).
//!
//! Terrain tiles render through a [`Material2d`](bevy::sprite_render::Material2d)
//! ([`TerrainFogMaterial`]) rather than a [`Sprite`](bevy::prelude::Sprite), because the
//! sprite pipeline's per-channel multiply tint cannot DESATURATE (GTW-348). Gangers stay
//! on the sprite path, and their fog hard-cut lives in the ganger-visibility RESOLVER
//! ([`resolve_ganger_visibility`](crate::resolve_ganger_visibility), GTW-627) — this
//! module writes terrain only.
//!
//! # Composition with the view slice
//!
//! Two visibility FACTS (`docs/combat/visibility.md` §"Composition with the view slice"):
//! the [`ActiveLevel`](crate::ActiveLevel) **slice** owns LAYER visibility (only storeys
//! within the drawn band `0..=active` are drawn); **fog** owns per-cell presentation and
//! actor flags. The design intent — "a thing draws iff fog shows its cell/entity AND the
//! slice draws its storey" — is composed per SURFACE: for terrain, the slice decides which
//! tiles EXIST (the band draw) and [`present_fog`] modulates every drawn tile; for actor
//! sprites, ONE pure classifier ANDs both facts and ONE resolver
//! ([`resolve_ganger_visibility`](crate::resolve_ganger_visibility)) writes the verdict
//! (GTW-627 — the writes are tick-quiet through the shared `actors/quiet.rs` seam).
//!
//! # Ordering (the CRITICAL clause)
//!
//! [`draw_static_battlefield`](crate::draw_static_battlefield) DESPAWNS-ALL-then-RESPAWNS
//! every [`TerrainSprite`](crate::TerrainSprite) on `BattleReady` OR an
//! [`ActiveLevel`](crate::ActiveLevel) change, and
//! [`swap_destroyed_cover`](crate::swap_destroyed_cover) edits sprites on
//! [`CoverDestroyed`](gdtf_battle_sim::occupancy_sync::CoverDestroyed). The fog writer MUST run strictly
//! after both, or it would colour stale / just-despawned entities or miss freshly-spawned
//! ones on a level cycle (`bevy-traps.md` #3). That ordering is STAGE MEMBERSHIP (GTW-623):
//! the fog runs in [`PresenterSystems::Compose`](crate::PresenterSystems), chained strictly
//! after the `Scene` stage holding every drawn-world writer — configured once in
//! [`TopDownRendererPlugin`](crate::TopDownRendererPlugin).
//!
//! It mints NO fire / targeting fog-GATE UX (the reticle / "hold your fire" refusal) —
//! that is GTW-11, which consumes the SIM read seams, not this presenter writer.

mod material;
mod present;

#[cfg(test)]
mod test;

pub(crate) use material::Saturation;
pub use material::{Brightness, TerrainFogMaterial, TerrainFogUniform};
pub use present::present_fog;
