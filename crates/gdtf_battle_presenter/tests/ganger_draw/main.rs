//! GTW-219 (GTW-48 S5): headless draw-LOGIC tests for the ganger sprite draw — AC1
//! (an `Added<Position>` ganger on the active level spawns exactly one faction-coloured
//! sprite at `cell_to_world` with the facing-correct atlas index), AC3 (a
//! `Changed<Position>` MOVES the existing sprite and does not spawn a second), AC4 (a
//! `Changed<LifeState>` Downed re-tints and Dead despawns), AC5 (gangers are drawn for
//! the active level ONLY; an `ActiveLevel` change hides off-level and shows on-level).
//! AC2 (the pure 8->4 facing map) is the in-crate unit test in `src/ganger.rs`.
//!
//! These prove the DRAW LOGIC headless; "the right sprites appear on screen facing the
//! right way" is AC7's in-engine QA evidence. The harness is a `DefaultPlugins`/
//! `no_renderer` app (a live `AssetServer` rooted at the workspace `assets/` so
//! `TopDownAtlases` + the `character_roles.ron`-resolved `CharacterRoles` are resident)
//! plus `TopDownRendererPlugin` and the sim lifecycle pieces needed to drive the REAL
//! `setup_battle` spawn path (`SetupBattleRequested` -> `setup_battle_on_request`). The
//! gangers are NOT hand-spawned — they come from a `Situation` poured through the real
//! setup. `app.world_mut()` in the test body is the accepted headless idiom
//! (`bevy-traps.md` #7 carve-out (a)). No function here takes `&mut World`/`&World`.

mod fog_visibility;
mod harness;
mod life_state;
mod posture;
mod probes;
mod spawn_move;
mod storeys;
mod suppression;
