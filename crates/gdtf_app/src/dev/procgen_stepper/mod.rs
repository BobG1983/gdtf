//! The DEV-ONLY procgen load-time stepper (GTW-655): an egui Next/Auto/Skip overlay over
//! the sim's staged procgen driver, shown at every battle load while active. Replaces the
//! GTW-434 menu-invoked standalone procgen-visualizer scene (retired in the same change).
//!
//! See [`plugin`] for the wiring rationale and the module-doc walk-through of the seam.

mod commands;
mod drive;
mod gate;
mod plugin;
// The pure stage-summary formatter `ui` draws — split out (GTW-655 gate-fix) so it compiles,
// and unit-tests, under `test-support` too, unlike `ui` itself (see `summary`'s module doc).
mod summary;
// The egui panel itself needs a primary window — excluded from `test-support` builds (the
// headless integration test's `no_renderer.rs`-style app has none), mirroring the
// content-editor's own headless harness (see `plugin`'s module doc). The real binary never
// enables `test-support`, so this never changes shipped wiring.
#[cfg(not(feature = "test-support"))]
mod ui;

// `ProcgenStepperPlugin` is the only item the binary consumes (via the dev aggregate
// plugin, `crate::dev::plugin`), so it is re-exported in BOTH configurations, at the same
// `test-support` visibility flip the type itself uses (`support_item!` in `plugin`) —
// mirrors the `net_qa` plugin's own `support_item!` re-export precedent exactly.
crate::support_use!(plugin::ProcgenStepperPlugin;);

// `battle_setup_runs_directly` is consumed by `battle_sim::plugin` (the run condition that
// gates `request_battle_setup` off while the stepper is active) — a PRODUCTION consumer
// outside this module, so it stays `pub(crate)` unconditionally (not test-support-gated).
// The command/latch types + the env-var gate reader are consumed ONLY by the GTW-655
// integration test (which drives the ENGAGED path directly, bypassing egui — the egui
// closure never runs headlessly — and unit-checks the gate mirrors `from_env`, the house
// recognised-truthy convention). `pub` under `test-support` (the `test_support`
// re-export needs them), `pub(crate)` otherwise (the production egui panel + drive systems
// + this plugin's own `from_env` still use them in-crate).
#[cfg(feature = "test-support")]
pub use commands::{AutoRunning, AutoStepDelay, PendingStepCommand, StepCommand};
pub(crate) use gate::battle_setup_runs_directly;
#[cfg(feature = "test-support")]
pub use gate::stepper_enabled;
#[cfg(feature = "test-support")]
pub use summary::stage_summary;
