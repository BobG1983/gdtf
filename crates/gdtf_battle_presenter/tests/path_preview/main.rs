//! GTW-358 / GTW-368 (C1 / C2 / C3 / C4 / C5 / C7): headless draw-LOGIC proof for the route
//! path-preview — the POSITIVE in-engine state assertions the contract demands.
//!
//! C1 (ROUTE, positive): with a `PathPreview` route at NAMED cells, the presenter draws a
//! `Visibility::Visible`, world-positioned `PathStepSprite` at each NAMED route cell ON the
//! active storey. The assertion NAMES the route cells that must appear.
//! C2 (GTW-368 COST-ON-TARGET, positive): with a previewed route, the presenter draws a SINGLE
//! `Visibility::Visible` `PathTargetLabel` `Text2d` at the NAMED target cell (the route's last
//! cell) reading the route cost as `"N TU"`; with NO target the label is HIDDEN.
//! C5 (HARD CUT): an off-`ActiveLevel` route cell is NOT drawn (and the route leaving the
//! storey draws a minimal marker at the link cell).
//! C4 (CLEARS / idle board): clearing the preview HIDES every step AND the cost label
//! (mutate-not-respawn), and the IDLE board has NO visible overlay entity (no per-cell labels —
//! the GTW-357 reachable overlay was removed entirely).
//!
//! This is the `fog_present.rs` pattern: a `DefaultPlugins`/`no_renderer` app with the real
//! `TopDownRendererPlugin`. The path-preview draw system gates on `BattleInProgress` +
//! `SquadVisibility` (a solid-tint sprite + a `Text2d`, no atlas), so the headless app drives
//! the REAL draw path. The `PathPreview` + `SquadVisibility` resources are authored DIRECTLY via
//! `app.world_mut()` in the test body (the accepted headless idiom, `bevy-traps.md` #7 carve-out
//! (a)) — standing in for the input crate's populate system, which is unit-tested in
//! `gdtf_battle_input` over the same resource.

mod cost_label;
mod harness;
mod steps;
