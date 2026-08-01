//! The DEV-ONLY procgen load-time stepper (GTW-655): an egui Next/Auto/Skip overlay over
//! the sim's staged procgen driver, shown at every battle load while active. Replaces the
//! GTW-434 menu-invoked standalone procgen-visualizer scene (retired in the same change).
//!
//! See [`plugin`] for the wiring rationale and the module-doc walk-through of the wiring.

mod commands;
mod drive;
mod gate;
mod plugin;
// The pure schematic-overview painter `ui` draws (GTW-732) — NOT gated on `test-support` so its
// geometry compiles + unit-tests there too, unlike `ui` itself (which needs a primary egui
// context the headless harness lacks — see `schematic`'s module doc).
mod schematic;
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
// gates `request_battle_setup` off while the stepper is engaged) and `ProcgenStepperActive` by
// the Options screen's dev-only stepper toggle (`crate::states::running::options`, GTW-868),
// which inserts / removes it — both PRODUCTION consumers outside this module, so both stay
// `pub(crate)` unconditionally (not test-support-gated). `AutoStepDelay` is consumed ONLY by the
// GTW-655 integration test (which drives the ENGAGED path directly, bypassing egui — the egui
// closure never runs headlessly), so it widens to `pub` only under `test-support`.
#[cfg(feature = "test-support")]
pub use commands::AutoStepDelay;
// The command/latch TRIO (`StepCommand` / `PendingStepCommand` / `AutoRunning`) is reached
// in-crate via `super::commands::…` (drive / ui) and out of it only by the `test_support`
// ledger. GTW-766's `net_qa` stepper-drive dispatch was its second consumer; GTW-943 deleted
// that dispatch with the `StepperControl` request it served, so the re-export is now
// `test-support` only — an unconditional one would be an unused `pub(crate) use` in a plain
// `dev_tools` build.
#[cfg(feature = "test-support")]
pub use commands::{AutoRunning, PendingStepCommand, StepCommand};
pub(crate) use gate::battle_setup_runs_directly;
crate::support_use!(gate::ProcgenStepperActive;);
// The schematic painter (GTW-732) — re-exported under `test-support` so it stays a reachable
// public API item (keeping its geometry linked + unit-tested even when `ui`, the only in-crate
// caller, is excluded from headless builds).
#[cfg(feature = "test-support")]
pub use schematic::draw_schematic;
