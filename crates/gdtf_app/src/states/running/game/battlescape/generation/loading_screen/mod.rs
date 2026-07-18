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

/// Test-support re-exports for this panel (GTW-569 one-hop ledger): the loading-screen
/// root marker (GTW-419) the headless `GdtfTestAppBuilder` AC tests name through
/// `crate::test_support`. The crate-root ledger (`src/test_support.rs`) re-exports these
/// by explicit name directly from here — no intermediate `mod.rs` climb. `pub(crate)` on
/// the module (not `pub`) because the parent chain is `pub(crate)`, so a `pub mod` here
/// trips the workspace `unreachable_pub = deny`; the `pub use` items inside still widen
/// to crate-external through the ledger's own `pub use`.
#[cfg(feature = "test-support")]
pub(crate) mod test_support {
    pub use super::components::LoadingScreenRoot;
}

// The DEV-ONLY loading-screen self-screenshot QA hook (the GTW-297 capture discipline): compiled
// in ONLY under a debug build with the opt-in `net_qa` feature (GTW-749 retired the `dev_capture`
// feature this used to ride; `net_qa` already pulls in the `gdtf_screenshot` crate this hook
// builds on, so it rides that gate now), and inert unless its env var is set. A release / default
// build never compiles it.
#[cfg(all(debug_assertions, feature = "net_qa"))]
mod capture;
