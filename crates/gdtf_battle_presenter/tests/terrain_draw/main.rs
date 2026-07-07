//! GTW-218 (GTW-48 S4): headless draw-LOGIC tests for the static-battlefield terrain
//! draw — AC2 (one-shot draw of role-correct, cell-positioned, CELL_PX-sized sprites),
//! AC3 (a `CoverDestroyed` swaps the cover cell to the rubble tile), and (GTW-519) the
//! multi-level draw core: raising `ActiveLevel` redraws the whole drawn band `[0..=active]`
//! (C1/C7), an open upper-storey cell peeks through to the storey below (C2), a tile on
//! storey `k+1` sorts strictly in front of the same `(x,y)` on storey `k` (per-storey Z, C3),
//! and a cover / slab destroyed on a DRAWN lower storey still swaps its tile (C6).
//!
//! These prove the DRAW LOGIC headless; "the right tiles appear on screen" is AC6's
//! in-engine QA evidence. The harness is a `DefaultPlugins`/`no_renderer` app (a live
//! `AssetServer` rooted at the workspace `assets/` so `TopDownAtlases` + the
//! `content/sprites/`-resolved `SpriteDefRegistry` is resident) plus `TopDownRendererPlugin` and
//! the two sim message buffers the draw reads (`BattleReady` / `CoverDestroyed`). The
//! three grids + `BattleInProgress` are authored, and `BattleReady` is written, DIRECTLY
//! via `app.world_mut()` in the test body — the accepted headless idiom (`bevy-traps.md`
//! #7 carve-out (a)). No function here takes `&mut World`/`&World`.

mod anchor;
mod destruction_swap;
mod door_stair_tiles;
mod emplacement;
mod harness;
mod level_band;
mod per_def_graphics;
mod static_draw;
mod storey_fog;
mod view_mode;
