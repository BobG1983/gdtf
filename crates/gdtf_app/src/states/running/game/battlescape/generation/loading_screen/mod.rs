//! The battlescape LOADING SCREEN sub-module of the Generation scene (GTW-419).
//!
//! A themed full-viewport overlay shown WHILE the sim assembles the level + builds the battle
//! (the brief `BattleScapeState::Generation` phase), removed the instant the assembled level
//! appears (`OnExit(Generation)` → `AnimateIn`). It guarantees NO frame of an unbuilt / partial
//! level is shown to the player: the opaque overlay covers the world-camera render beneath it
//! (`bevy-traps.md` #8 — occlusion working FOR us). A presenter / view artifact in the running
//! scene; it owns no combat rules and never touches the sim. The architectural choice was to
//! extend Generation IN-PLACE (spawn `OnEnter` / despawn `OnExit`) rather than add a sibling
//! `Loading` sub-state — the `BattleReady`-gated `Generation → AnimateIn` transition already
//! covers exactly the assembly phase, so no new state is needed (avoiding `bevy-traps.md` #5
//! sub-state complexity).

mod components;
mod plugin;
mod spawn;

pub(in crate::states::running::game::battlescape::generation) use plugin::LoadingScreenPlugin;

// Test-support-only re-export of the loading-screen root marker (GTW-419), so the headless
// `GdtfTestAppBuilder` AC tests can name it through `crate::test_support`. `support_use!` widens
// it to `pub` under the `test-support` feature and keeps it `pub(crate)` (and
// `unreachable_pub`-clean) otherwise (the contextual-panel marker re-export chain precedent). It
// climbs level by level up to `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use!(components::LoadingScreenRoot;);

// The DEV-ONLY loading-screen self-screenshot QA hook (the GTW-297 capture discipline): compiled
// in ONLY under a debug build with the opt-in `dev_capture` feature, and inert unless its env var
// is set. A release / default build never compiles it.
#[cfg(all(debug_assertions, feature = "dev_capture"))]
mod capture;
