//! GTW-359 (E7 · GTW-12k) headless POSITIVE QA for the vertical-link tile draw + the
//! cross-storey ganger handoff + the movement tween — the C6 (b)/(c)/(d) clauses.
//!
//! These prove the DRAW LOGIC headless (component state — atlas index / Visibility /
//! Transform — is queryable); the PIXEL proof that the stair / ladder tiles actually
//! render NON-EMPTY is `vertical_link_readback.rs` (the dev screenshot capture is broken,
//! so a real-GPU readback closes that gap, `bevy-traps.md` #8). The harness is the
//! `ganger_draw.rs` pattern: a `DefaultPlugins`/`no_renderer` app with a live
//! `AssetServer` rooted at the workspace `assets/` (so `TileRoles` resolves to the
//! SHIPPED `tile_roles.ron`, `TopDownAtlases` loads) plus `TopDownRendererPlugin` and the
//! real `setup_battle` spawn path. The links + gangers are NOT hand-spawned — they are
//! poured through the real setup from a `Situation` carrying authored
//! [`VerticalLink`](gdtf_battle_sim::vertical::VerticalLink)s, so `setup_battle` validates + inserts
//! the `VerticalLinkGraph` the draw reads. `app.world_mut()` in the test body is the
//! accepted headless idiom (`bevy-traps.md` #7 carve-out (a)); no function here takes
//! `&mut World`/`&World`.
//!
//! POSITIVE — every assertion NAMES the indices (`stair_up` 29 / `stair_down` 28 / ladder
//! 235), cells, and Visibility / intermediate-Transform values it checks, not "something
//! renders".

mod ganger_mirror;
mod harness;
mod link_tiles;
