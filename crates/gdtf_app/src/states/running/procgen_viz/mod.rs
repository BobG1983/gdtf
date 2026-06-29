//! The DEV-ONLY procgen STEP/AUTO visualizer scene (GTW-434) — a debug overlay that reveals
//! the procgen space-packing placement sequence one prefab at a time (STEP) or all at once
//! (AUTO), drawn as light tinted quads over a single dark whole-level quad.
//!
//! # Where it lives + how it is invoked
//!
//! It lives in the MAIN GAME (`gdtf_app`, the `grimdark_turfwar` binary) as a
//! [`RunningState::DebugProcgenVisualizer`](crate::states::RunningState) scene — NOT in
//! `gdtf_editor` / the map-editor binary. It is reached ONLY from the
//! `cfg(debug_assertions)`-gated "Procgen Viz" main-menu button (the GTW-420 gang-editor
//! precedent), so the entry point never compiles into a release binary. The whole module —
//! the model, the components, the systems, the scene plugin, and its registration — is itself
//! `#[cfg(debug_assertions)]`-gated, so the feature COMPILES OUT of release entirely (C4).
//!
//! # What it does
//!
//! `OnEnter` it runs the sim's space-packing pipeline
//! ([`assemble_placement`](gdtf_battle_sim::procgen::assemble_placement) +
//! [`fill_placement`](gdtf_battle_sim::procgen::fill_placement)) against the loaded
//! [`PrefabRegistry2`](gdtf_battle_sim::PrefabRegistry2) for the loaded situation's theme +
//! grid-size + a seed, and holds the ordered placement (player, enemy, then fill) in the
//! state-scoped [`ProcgenViz`](model::ProcgenViz) model. STEP reveals the next prefab quad;
//! AUTO reveals them all. The player-spawn quad is tinted GREEN, the enemy-spawn quad RED, and
//! the rest the neutral light tint (C3) — all drawn over the dark board quad (C2).
//!
//! It reads the sim placement and never mutates combat/sim rules — the one-way sim→presenter
//! boundary (`docs/decisions/0001-rust-bevy-rewrite.md`).

#[cfg(all(debug_assertions, feature = "dev_capture"))]
mod capture;
mod components;
mod model;
mod plugin;
mod systems;

pub(in crate::states::running) use plugin::ProcgenVizScenePlugin;

// Test-support-only re-export of the GTW-434 visualizer model + markers, gated so the binary
// build is `unreachable_pub`-clean. Carries them up toward `crate::test_support` so the
// headless integration test can drive the STEP / AUTO controls + assert on the revealed quads.
#[cfg(feature = "test-support")]
crate::support_use! {
    components::{AutoButton, BoardQuad, PrefabQuad, ProcgenVizRoot, StepButton};
}
#[cfg(feature = "test-support")]
crate::support_use! {
    model::{ProcgenViz, QuadTint};
}
