//! GTW-342 / GTW-348: headless draw-LOGIC tests for the presenter FOG WRITER — the
//! three-state terrain treatment (VISIBLE full colour `saturation` 1.0 / EXPLORED
//! full-brightness GREYSCALE `saturation` 0.0 / UNSEEN hidden), the enemy hard-cut +
//! player always-shown actor flags, and the CRITICAL post-level-cycle re-apply (proving
//! the fog runs `.after` `draw_static_battlefield` + `swap_destroyed_cover`).
//!
//! GTW-348 moved terrain from the `Sprite` path to a `Mesh2d` +
//! `MeshMaterial2d<TerrainFogMaterial>` so EXPLORED can render DESATURATED (the sprite
//! pipeline's multiply tint cannot desaturate). So the terrain assertions read each tile's
//! `TerrainFogMaterial.saturation` (from `Assets<TerrainFogMaterial>`) + `Visibility`,
//! NOT `Sprite.color`.
//!
//! These prove the WRITER LOGIC headless; "the fog horizon + enemy pop-in actually
//! render" is the in-engine QA evidence (the green suite + gate are blind to it — a
//! `Visible` sprite can still draw nothing, `bevy-traps.md` #8).
//!
//! The harness is the SAME `DefaultPlugins`/`no_renderer` app the terrain / ganger draw
//! tests use (a live `AssetServer` rooted at the workspace `assets/`, the
//! `TopDownRendererPlugin`, the sim lifecycle pieces for the REAL `setup_battle` spawn).
//! The gangers come from a `Situation` poured through the real setup; the
//! `SquadVisibility` fog + `CombatTuning` are authored DIRECTLY via `app.world_mut()` in
//! the test body — the accepted headless idiom (`bevy-traps.md` #7 carve-out (a)). No
//! function here takes `&mut World`/`&World`.

mod actors_and_reapply;
mod harness;
mod terrain_states;
