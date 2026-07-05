//! GTW-220 (GTW-48 S6): headless draw-LOGIC tests for the transient FX-flash layer —
//! AC1 (a `Bleeding` spawns one cell-positioned `FxFlash` at the table's bleed index, with a
//! `*Wounds`-relation tint, and fail-closes with no `Position` / `Wounds`), AC2 (an
//! `ArmorBroken` spawns one at the table's armor-break index), AC3 (a `CoverDestroyed` spawns
//! one at `cell_to_world(at)` at the cover-destroyed index), AC4 (`FlashTtl` expiry despawns
//! the flash; a further empty update spawns nothing), AC5 (two `Bleeding` in one frame spawn
//! two independent flashes).
//!
//! These prove the DRAW + EXPIRY LOGIC headless; "the flashes visibly pop + fade" is AC7's
//! deferred S9-capstone in-engine QA. The harness is a `DefaultPlugins`/`no_renderer` app (a
//! live `AssetServer` rooted at the workspace `assets/` so `TopDownAtlases` + the
//! `effect_roles.ron`-resolved `EffectRoles` are resident) plus `TopDownRendererPlugin` and
//! the three sim FX message buffers the readers drain (`Bleeding` / `ArmorBroken` /
//! `CoverDestroyed`). The gangers + `BattleInProgress` are authored, and the FX messages are
//! written, DIRECTLY via `app.world_mut()` in the test body — the accepted headless idiom
//! (`bevy-traps.md` #7 carve-out (a)). No function here takes `&mut World`/`&World`.

mod blast;
mod consequence_fct;
mod death_and_fields;
mod fall_fx;
mod flash;
mod harness;
mod impact_fct;
mod injury_fct;
mod kill_despawn;
mod probes;
mod projectile;
mod registrar_contract;
mod volley_stagger;
